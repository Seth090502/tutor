//! The two sidecars (DeepTutor backend, rag-engine): how each is launched, and a supervisor
//! that starts them in Job Objects, captures their output and polls their health.

use std::collections::{BTreeMap, VecDeque};
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

use crate::job::Job;

/// Browser origins the app is served from (contract header and C3). DeepTutor's CORS and WebSocket
/// origin guard must allow them, and only they get the microphone. The Vite dev server's origins
/// are trusted in debug builds only.
pub const APP_ORIGINS: &[&str] = app_origins(cfg!(debug_assertions));

const fn app_origins(debug: bool) -> &'static [&'static str] {
    if debug {
        &[
            "http://localhost:1420",
            "http://127.0.0.1:1420",
            "http://tauri.localhost",
            "tauri://localhost",
        ]
    } else {
        &["http://tauri.localhost", "tauri://localhost"]
    }
}

const POLL_EVERY: Duration = Duration::from_secs(2);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
const LOG_LINES: usize = 200;
const LOG_LINE_CHARS: usize = 1000;
const DETAIL_LINES: usize = 20;

/// DeepTutor derives `--ws-max-size` from its attachment settings; every launch point asks it,
/// as the dev Dockerfile stage does (tests/runtime/test_uvicorn_launch_flags.py).
const SERVING_QUERY: &str =
    "from deeptutor.services.config import get_ws_max_size, HTTP_KEEP_ALIVE_TIMEOUT; \
print(get_ws_max_size(), HTTP_KEEP_ALIVE_TIMEOUT)";

const BACKEND_SECRETS: [(&str, &str); 3] = [
    ("opencode", "OPENCODE_API_KEY"),
    ("anthropic", "ANTHROPIC_API_KEY"),
    ("openrouter", "OPENROUTER_API_KEY"),
];
const RAG_ENGINE_SECRETS: [(&str, &str); 1] = [("voyage", "VOYAGE_API_KEY")];

pub type Secrets = BTreeMap<&'static str, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceName {
    Backend,
    RagEngine,
}

impl ServiceName {
    pub const ALL: [ServiceName; 2] = [ServiceName::Backend, ServiceName::RagEngine];

    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "backend" => Ok(Self::Backend),
            "rag_engine" => Ok(Self::RagEngine),
            _ => Err(format!(
                "unknown service {name:?}; expected backend or rag_engine"
            )),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Backend => "backend",
            Self::RagEngine => "rag_engine",
        }
    }

    pub fn url(self) -> &'static str {
        match self {
            Self::Backend => "http://127.0.0.1:8001",
            Self::RagEngine => "http://127.0.0.1:8811",
        }
    }

    fn port(self) -> u16 {
        match self {
            Self::Backend => 8001,
            Self::RagEngine => 8811,
        }
    }

    /// `/health/ready` rather than `/health/live`: the backend answers "live" before its
    /// startup finishes, and "ready" only once it can serve (deeptutor/api/main.py).
    fn health_url(self) -> String {
        match self {
            Self::Backend => format!("{}/health/ready", self.url()),
            Self::RagEngine => format!("{}/health", self.url()),
        }
    }

    /// DeepTutor has been seen needing 78 s to start on slow machines (launcher.py, #1435).
    fn startup_timeout(self) -> Duration {
        match self {
            Self::Backend => Duration::from_secs(120),
            Self::RagEngine => Duration::from_secs(60),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Starting,
    Running,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ServiceStatus {
    pub state: State,
    pub url: String,
    pub detail: Option<String>,
}

impl ServiceStatus {
    fn new(name: ServiceName, state: State, detail: Option<String>) -> Self {
        Self {
            state,
            url: name.url().to_string(),
            detail,
        }
    }
}

/// A process to start. Holds secrets in `env`, so it is deliberately not `Debug`.
pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}

impl LaunchSpec {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            .current_dir(&self.cwd);
        command
    }
}

fn secret_env(secrets: &Secrets, names: &[(&str, &str)]) -> Vec<(String, String)> {
    names
        .iter()
        .filter_map(|(name, var)| {
            secrets
                .get(name)
                .map(|value| (var.to_string(), value.clone()))
        })
        .collect()
}

fn python(root: &Path) -> PathBuf {
    root.join(".venv").join("Scripts").join("python.exe")
}

/// The shell's own origins, so a debug shell can run a release rag-engine (C3), plus its keys.
fn rag_engine_env(secrets: &Secrets) -> Vec<(String, String)> {
    let mut env = vec![("RAG_APP_ORIGINS".to_string(), APP_ORIGINS.join(","))];
    env.extend(secret_env(secrets, &RAG_ENGINE_SECRETS));
    env
}

fn backend_env(secrets: &Secrets) -> Vec<(String, String)> {
    let mut env = vec![
        ("CORS_ORIGINS".to_string(), APP_ORIGINS.join(",")),
        // Only the app's own origins: not the upstream web frontend's defaults (C3).
        ("DEEPTUTOR_WEB_FRONTEND".to_string(), "0".to_string()),
        ("PYTHONUTF8".to_string(), "1".to_string()),
        ("PYTHONIOENCODING".to_string(), "utf-8".to_string()),
        // Stream logs into the ring buffer as they happen (launcher.py sets it too).
        ("PYTHONUNBUFFERED".to_string(), "1".to_string()),
    ];
    env.extend(secret_env(secrets, &BACKEND_SECRETS));
    env
}

/// Asks DeepTutor, in the backend's own environment, for its serving settings.
pub fn serving_query_spec(root: &Path, secrets: &Secrets) -> LaunchSpec {
    LaunchSpec {
        program: python(root),
        args: vec!["-c".into(), SERVING_QUERY.into()],
        env: backend_env(secrets),
        cwd: root.to_path_buf(),
    }
}

#[derive(Debug, PartialEq)]
pub struct Serving {
    pub ws_max_size: u64,
    pub keep_alive: u64,
}

pub fn parse_serving(stdout: &str) -> Result<Serving, String> {
    let numbers: Vec<u64> = stdout
        .split_whitespace()
        .map_while(|n| n.parse().ok())
        .collect();
    match numbers[..] {
        [ws_max_size, keep_alive] => Ok(Serving {
            ws_max_size,
            keep_alive,
        }),
        _ => Err(format!(
            "unexpected serving settings output {:?}",
            stdout.trim()
        )),
    }
}

/// Same uvicorn flags as `deeptutor start` (deeptutor/runtime/launcher.py). `--workers` is left
/// at uvicorn's default of 1, which is also DeepTutor's default.
pub fn backend_spec(root: &Path, secrets: &Secrets, serving: &Serving) -> LaunchSpec {
    let args = [
        "-m",
        "uvicorn",
        "deeptutor.api.main:app",
        "--host",
        "127.0.0.1",
        "--port",
        "8001",
        "--log-level",
        "info",
        "--no-access-log",
        "--no-proxy-headers",
        "--ws-max-size",
        &serving.ws_max_size.to_string(),
        "--timeout-keep-alive",
        &serving.keep_alive.to_string(),
    ];
    LaunchSpec {
        program: python(root),
        args: args.iter().map(|a| a.to_string()).collect(),
        env: backend_env(secrets),
        cwd: root.to_path_buf(),
    }
}

pub fn rag_engine_spec(root: &Path, secrets: &Secrets) -> Result<LaunchSpec, String> {
    let program = ["release", "debug"]
        .iter()
        .map(|profile| root.join("target").join(profile).join("rag-engine.exe"))
        .find(|path| path.is_file())
        .ok_or("build it: cargo build -p rag-engine --release")?;
    let arg = |path: PathBuf| path.to_string_lossy().into_owned();
    Ok(LaunchSpec {
        program,
        args: vec![
            "serve".into(),
            "--corpus".into(),
            arg(root.join("corpus")),
            "--notes".into(),
            arg(root.join("content").join("notes")),
            "--data".into(),
            arg(root.join("data").join("rag")),
            "--port".into(),
            "8811".into(),
        ],
        env: rag_engine_env(secrets),
        cwd: root.to_path_buf(),
    })
}

fn prepare_backend(root: &Path, secrets: &Secrets) -> Result<LaunchSpec, String> {
    let python = python(root);
    if !python.is_file() {
        return Err(format!(
            "{} not found; create the venv (see TUTOR.md)",
            python.display()
        ));
    }
    let output = serving_query_spec(root, secrets)
        .command()
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run {}: {e}", python.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        let tail = lines[lines.len().saturating_sub(DETAIL_LINES)..].join("\n");
        return Err(format!(
            "could not read DeepTutor's serving settings ({}):\n{tail}",
            output.status
        ));
    }
    let serving = parse_serving(&String::from_utf8_lossy(&output.stdout))?;
    Ok(backend_spec(root, secrets, &serving))
}

/// The last lines a service printed, bounded in count and line length.
#[derive(Default)]
struct LogRing {
    lines: VecDeque<String>,
}

impl LogRing {
    fn push(&mut self, line: &str) {
        if self.lines.len() == LOG_LINES {
            self.lines.pop_front();
        }
        self.lines
            .push_back(line.chars().take(LOG_LINE_CHARS).collect());
    }

    fn tail(&self, n: usize) -> String {
        let skip = self.lines.len().saturating_sub(n);
        self.lines
            .iter()
            .skip(skip)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A running service. Field order matters: the child handle closes, then the job, which kills
/// the whole process tree.
struct Proc {
    child: Child,
    log: Arc<Mutex<LogRing>>,
    _job: Job,
}

impl Proc {
    fn spawn(spec: &LaunchSpec) -> Result<Proc, String> {
        let job = Job::new().map_err(|e| format!("could not create a job object: {e}"))?;
        let mut command = spec.command();
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = job
            .spawn(&mut command)
            .map_err(|e| format!("could not start {}: {e}", spec.program.display()))?;
        let log = Arc::new(Mutex::new(LogRing::default()));
        let pipes: [Box<dyn std::io::Read + Send>; 2] = [
            Box::new(child.stdout.take().expect("stdout is piped")),
            Box::new(child.stderr.take().expect("stderr is piped")),
        ];
        for pipe in pipes {
            let log = log.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(pipe);
                let mut line = Vec::new();
                while matches!(reader.read_until(b'\n', &mut line), Ok(n) if n > 0) {
                    let text = String::from_utf8_lossy(&line);
                    lock(&log).push(text.trim_end());
                    line.clear();
                }
            });
        }
        Ok(Proc {
            child,
            log,
            _job: job,
        })
    }

    fn tail(&self) -> String {
        lock(&self.log).tail(DETAIL_LINES)
    }

    /// `Some(detail)` once the process has exited.
    fn exit_detail(&mut self) -> Option<String> {
        let status = match self.child.try_wait() {
            Ok(None) => return None,
            Ok(Some(status)) => status.to_string(),
            Err(e) => format!("unknown status ({e})"),
        };
        // Let the reader threads drain what the process wrote last.
        std::thread::sleep(Duration::from_millis(100));
        Some(with_tail(format!("exited ({status})"), &self.tail()))
    }

    /// Kill the tree and wait for the process, so its port is free for a successor.
    fn stop(self) {
        let Proc {
            mut child,
            _job: job,
            ..
        } = self;
        drop(job);
        let _ = child.wait();
    }
}

fn with_tail(summary: String, tail: &str) -> String {
    if tail.is_empty() {
        summary
    } else {
        format!("{summary}\n{tail}")
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Waits briefly: a killed tree (the venv's python.exe is a launcher for the real interpreter)
/// can release its port a moment after the process we waited on has exited.
fn wait_for_free_port(port: u16) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
        if Instant::now() >= deadline {
            return Err(format!(
                "port {port} is already in use by another process; stop it, then restart"
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

async fn check_health(http: &reqwest::Client, url: &str) -> Result<(), String> {
    let response = http
        .get(url)
        .timeout(HEALTH_TIMEOUT)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("{url} answered {}", response.status()))
    }
}

struct Slot {
    status: ServiceStatus,
    /// Bumped by every start, so work for a replaced process stops touching the slot.
    generation: u64,
    proc: Option<Proc>,
}

struct Service {
    name: ServiceName,
    /// Serializes launches of this service, so a restart never races the previous one.
    launching: Mutex<()>,
    slot: Mutex<Slot>,
}

pub struct Supervisor {
    root: Result<PathBuf, String>,
    http: reqwest::Client,
    services: [Service; 2],
}

impl Supervisor {
    pub fn new(root: Result<PathBuf, String>, http: reqwest::Client) -> Self {
        let service = |name: ServiceName| Service {
            name,
            launching: Mutex::new(()),
            slot: Mutex::new(Slot {
                status: ServiceStatus::new(name, State::Stopped, None),
                generation: 0,
                proc: None,
            }),
        };
        Self {
            root,
            http,
            services: ServiceName::ALL.map(service),
        }
    }

    fn service(&self, name: ServiceName) -> &Service {
        &self.services[name as usize]
    }

    pub fn status(&self) -> BTreeMap<&'static str, ServiceStatus> {
        self.services
            .iter()
            .map(|s| (s.name.key(), lock(&s.slot).status.clone()))
            .collect()
    }

    /// Start (or restart) a service in the background: stop the old process tree, launch a
    /// new one, then poll its health.
    pub fn start(self: &Arc<Self>, name: ServiceName) {
        let generation = {
            let mut slot = lock(&self.service(name).slot);
            slot.generation += 1;
            set_status(name, &mut slot, State::Starting, None);
            slot.generation
        };
        let this = self.clone();
        tauri::async_runtime::spawn_blocking(move || this.launch(name, generation));
    }

    fn launch(self: Arc<Self>, name: ServiceName, generation: u64) {
        let service = self.service(name);
        let _serial = lock(&service.launching);
        let old = {
            let mut slot = lock(&service.slot);
            if slot.generation != generation {
                return;
            }
            slot.proc.take()
        };
        if let Some(old) = old {
            old.stop();
        }
        let started = self.prepare(name).and_then(|spec| Proc::spawn(&spec));
        let mut slot = lock(&service.slot);
        if slot.generation != generation {
            drop(slot);
            if let Ok(proc) = started {
                proc.stop();
            }
            return;
        }
        match started {
            Ok(proc) => {
                slot.proc = Some(proc);
                drop(slot);
                tauri::async_runtime::spawn(self.clone().monitor(name, generation));
            }
            Err(detail) => set_status(name, &mut slot, State::Failed, Some(detail)),
        }
    }

    fn prepare(&self, name: ServiceName) -> Result<LaunchSpec, String> {
        let root = self.root.as_ref().map_err(Clone::clone)?;
        wait_for_free_port(name.port())?;
        let secrets = crate::secrets::stored()?;
        match name {
            ServiceName::Backend => prepare_backend(root, &secrets),
            ServiceName::RagEngine => rag_engine_spec(root, &secrets),
        }
    }

    /// Poll every 2 s while this generation's process is alive: exit means failed, a healthy
    /// answer means running, and no healthy answer within the startup timeout means failed.
    async fn monitor(self: Arc<Self>, name: ServiceName, generation: u64) {
        let service = self.service(name);
        let started = Instant::now();
        loop {
            tokio::time::sleep(POLL_EVERY).await;
            {
                let mut slot = lock(&service.slot);
                if slot.generation != generation {
                    return;
                }
                let Some(proc) = slot.proc.as_mut() else {
                    return;
                };
                if let Some(detail) = proc.exit_detail() {
                    slot.proc = None; // the job kills anything it left behind
                    set_status(name, &mut slot, State::Failed, Some(detail));
                    return;
                }
            }
            let health = check_health(&self.http, &name.health_url()).await;
            let mut slot = lock(&service.slot);
            if slot.generation != generation || slot.proc.is_none() {
                return;
            }
            match health {
                Ok(()) => {
                    if slot.status.state != State::Running {
                        set_status(name, &mut slot, State::Running, None);
                    }
                }
                Err(e) if slot.status.state == State::Starting => {
                    let timeout = name.startup_timeout();
                    if started.elapsed() >= timeout {
                        let proc = slot.proc.take().expect("checked above");
                        let detail = with_tail(
                            format!("not healthy after {} s: {e}", timeout.as_secs()),
                            &proc.tail(),
                        );
                        set_status(name, &mut slot, State::Failed, Some(detail));
                        drop(slot);
                        proc.stop();
                        return;
                    }
                }
                Err(e) if slot.status.state == State::Running => {
                    let detail = with_tail(
                        format!("health check failing: {e}"),
                        &slot.proc.as_ref().expect("checked above").tail(),
                    );
                    set_status(name, &mut slot, State::Failed, Some(detail));
                }
                Err(_) => {}
            }
        }
    }

    /// Kill every service tree (on app exit; a crash gets the same from the kernel).
    pub fn stop_all(&self) {
        for service in &self.services {
            let proc = {
                let mut slot = lock(&service.slot);
                slot.generation += 1;
                set_status(service.name, &mut slot, State::Stopped, None);
                slot.proc.take()
            };
            if let Some(proc) = proc {
                proc.stop();
            }
        }
    }
}

fn set_status(name: ServiceName, slot: &mut Slot, state: State, detail: Option<String>) {
    let first_line = detail
        .as_deref()
        .and_then(|d| d.lines().next())
        .unwrap_or("");
    eprintln!("[tutor-shell] {}: {state:?} {first_line}", name.key());
    slot.status = ServiceStatus::new(name, state, detail);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn secrets() -> Secrets {
        [
            "elevenlabs",
            "opencode",
            "anthropic",
            "openrouter",
            "voyage",
        ]
        .into_iter()
        .map(|name| (name, format!("{name}-value")))
        .collect()
    }

    fn env_map(spec: &LaunchSpec) -> BTreeMap<&str, &str> {
        spec.env
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    #[test]
    fn backend_command_line_mirrors_deeptutor_launcher() {
        let root = Path::new(r"C:\repo");
        let spec = backend_spec(
            root,
            &secrets(),
            &Serving {
                ws_max_size: 43341141,
                keep_alive: 300,
            },
        );
        assert_eq!(spec.program, Path::new(r"C:\repo\.venv\Scripts\python.exe"));
        assert_eq!(spec.cwd, root);
        assert_eq!(
            spec.args,
            [
                "-m",
                "uvicorn",
                "deeptutor.api.main:app",
                "--host",
                "127.0.0.1",
                "--port",
                "8001",
                "--log-level",
                "info",
                "--no-access-log",
                "--no-proxy-headers",
                "--ws-max-size",
                "43341141",
                "--timeout-keep-alive",
                "300",
            ]
        );
        let env = env_map(&spec);
        let origins = APP_ORIGINS.join(",");
        assert_eq!(
            env,
            BTreeMap::from([
                ("CORS_ORIGINS", origins.as_str()),
                ("PYTHONUTF8", "1"),
                ("PYTHONIOENCODING", "utf-8"),
                ("PYTHONUNBUFFERED", "1"),
                ("DEEPTUTOR_WEB_FRONTEND", "0"),
                ("OPENCODE_API_KEY", "opencode-value"),
                ("ANTHROPIC_API_KEY", "anthropic-value"),
                ("OPENROUTER_API_KEY", "openrouter-value"),
            ])
        );
    }

    #[test]
    fn release_builds_trust_only_the_built_app() {
        assert_eq!(
            app_origins(false),
            ["http://tauri.localhost", "tauri://localhost"]
        );
        assert_eq!(
            app_origins(true),
            [
                "http://localhost:1420",
                "http://127.0.0.1:1420",
                "http://tauri.localhost",
                "tauri://localhost",
            ]
        );
        assert_eq!(APP_ORIGINS, app_origins(cfg!(debug_assertions)));
    }

    #[test]
    fn backend_gets_only_the_keys_that_are_stored() {
        let stored = Secrets::from([("anthropic", "a".to_string()), ("voyage", "v".to_string())]);
        let spec = backend_spec(
            Path::new("r"),
            &stored,
            &Serving {
                ws_max_size: 1,
                keep_alive: 2,
            },
        );
        let env = env_map(&spec);
        assert_eq!(env.get("ANTHROPIC_API_KEY"), Some(&"a"));
        assert!(!env.contains_key("OPENCODE_API_KEY") && !env.contains_key("VOYAGE_API_KEY"));
    }

    #[test]
    fn serving_query_runs_in_the_backend_environment() {
        let root = Path::new(r"C:\repo");
        let query = serving_query_spec(root, &secrets());
        let backend = backend_spec(
            root,
            &secrets(),
            &Serving {
                ws_max_size: 1,
                keep_alive: 2,
            },
        );
        assert_eq!(query.program, backend.program);
        assert_eq!(query.env, backend.env);
        assert_eq!(query.cwd, backend.cwd);
        assert_eq!(query.args[0], "-c");
        assert!(
            query.args[1].contains("get_ws_max_size()")
                && query.args[1].contains("HTTP_KEEP_ALIVE_TIMEOUT")
        );
    }

    #[test]
    fn parses_serving_settings() {
        assert_eq!(
            parse_serving("43341141 300\r\n"),
            Ok(Serving {
                ws_max_size: 43341141,
                keep_alive: 300
            })
        );
        assert!(parse_serving("").is_err());
        assert!(parse_serving("12").is_err());
        assert!(parse_serving("Traceback (most recent call last)").is_err());
    }

    #[test]
    fn missing_venv_is_a_readable_failure() {
        let root = tempfile::tempdir().unwrap();
        let err = prepare_backend(root.path(), &Secrets::new()).err().unwrap();
        assert!(
            err.ends_with("python.exe not found; create the venv (see TUTOR.md)"),
            "{err}"
        );
    }

    #[test]
    fn rag_engine_prefers_release_then_debug_and_says_how_to_build() {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        let err = rag_engine_spec(r, &secrets()).err().unwrap();
        assert_eq!(err, "build it: cargo build -p rag-engine --release");

        std::fs::create_dir_all(r.join("target/debug")).unwrap();
        std::fs::write(r.join("target/debug/rag-engine.exe"), "").unwrap();
        assert_eq!(
            rag_engine_spec(r, &secrets()).unwrap().program,
            r.join("target/debug/rag-engine.exe")
        );

        std::fs::create_dir_all(r.join("target/release")).unwrap();
        std::fs::write(r.join("target/release/rag-engine.exe"), "").unwrap();
        let spec = rag_engine_spec(r, &secrets()).unwrap();
        assert_eq!(
            spec.program,
            r.join("target").join("release").join("rag-engine.exe")
        );
        assert_eq!(spec.cwd, r);
        let path = |p: PathBuf| p.to_string_lossy().into_owned();
        assert_eq!(
            spec.args,
            [
                "serve".to_string(),
                "--corpus".into(),
                path(r.join("corpus")),
                "--notes".into(),
                path(r.join("content").join("notes")),
                "--data".into(),
                path(r.join("data").join("rag")),
                "--port".into(),
                "8811".into(),
            ]
        );
        let origins = APP_ORIGINS.join(",");
        assert_eq!(
            env_map(&spec),
            BTreeMap::from([
                ("RAG_APP_ORIGINS", origins.as_str()),
                ("VOYAGE_API_KEY", "voyage-value")
            ])
        );
        let keyless = rag_engine_spec(r, &Secrets::new()).unwrap();
        assert_eq!(
            env_map(&keyless),
            BTreeMap::from([("RAG_APP_ORIGINS", origins.as_str())])
        );
    }

    #[test]
    fn service_names_and_status_shape_match_the_contract() {
        assert_eq!(ServiceName::parse("backend"), Ok(ServiceName::Backend));
        assert_eq!(ServiceName::parse("rag_engine"), Ok(ServiceName::RagEngine));
        assert!(ServiceName::parse("rag-engine").is_err());
        let supervisor = Supervisor::new(Err("no root".into()), crate::http_client());
        assert_eq!(
            serde_json::to_value(supervisor.status()).unwrap(),
            json!({
                "backend": {"state": "stopped", "url": "http://127.0.0.1:8001", "detail": null},
                "rag_engine": {"state": "stopped", "url": "http://127.0.0.1:8811", "detail": null},
            })
        );
    }

    #[test]
    fn log_ring_keeps_the_last_lines_and_bounds_their_length() {
        let mut ring = LogRing::default();
        for i in 0..LOG_LINES + 5 {
            ring.push(&format!("line {i}"));
        }
        assert_eq!(ring.lines.len(), LOG_LINES);
        assert_eq!(
            ring.tail(2),
            format!("line {}\nline {}", LOG_LINES + 3, LOG_LINES + 4)
        );
        ring.push(&"x".repeat(5000));
        assert_eq!(ring.tail(1).len(), LOG_LINE_CHARS);
    }

    #[test]
    fn a_failed_process_reports_its_exit_and_last_output() {
        let spec = LaunchSpec {
            program: "cmd.exe".into(),
            args: vec!["/c".into(), "echo starting& echo boom 1>&2& exit 3".into()],
            env: vec![],
            cwd: std::env::temp_dir(),
        };
        let mut proc = Proc::spawn(&spec).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let detail = loop {
            if let Some(detail) = proc.exit_detail() {
                break detail;
            }
            assert!(Instant::now() < deadline, "process did not exit");
            std::thread::sleep(Duration::from_millis(50));
        };
        let mut lines = detail.lines();
        assert_eq!(lines.next(), Some("exited (exit code: 3)"));
        let mut output: Vec<_> = lines.collect();
        output.sort(); // stdout and stderr are read concurrently
        assert_eq!(output, ["boom", "starting"]);
    }

    #[test]
    fn health_is_a_2xx_answer() {
        let (base, _) =
            crate::test_support::mock_http("200 OK", r#"{"ok": true, "version": "0.1.0"}"#);
        let http = crate::http_client();
        tauri::async_runtime::block_on(check_health(&http, &format!("{base}/health"))).unwrap();

        let (base, _) =
            crate::test_support::mock_http("503 Service Unavailable", r#"{"status": "not_ready"}"#);
        let err =
            tauri::async_runtime::block_on(check_health(&http, &format!("{base}/health/ready")))
                .unwrap_err();
        assert!(
            err.ends_with("/health/ready answered 503 Service Unavailable"),
            "{err}"
        );
    }
}
