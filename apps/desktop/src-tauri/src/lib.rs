//! Tauri shell: supervises the backend and rag-engine, keeps API keys in Windows Credential
//! Manager, and exposes the commands in contract C3 (docs/tutor/CONTRACTS.md).

mod config;
mod elevenlabs;
pub mod job;
mod learning;
mod nback;
mod root;
mod secrets;
mod services;
mod speech;
mod tracker;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};

use tauri::webview::{PermissionKind, PermissionResponse};
use tauri::{Manager, RunEvent, State};

use services::{ServiceName, ServiceStatus, Supervisor, APP_ORIGINS};

struct AppState {
    root: Result<PathBuf, String>,
    config_path: PathBuf,
    http: reqwest::Client,
    supervisor: Arc<Supervisor>,
}

impl AppState {
    fn root(&self) -> Result<&Path, String> {
        self.root.as_deref().map_err(Clone::clone)
    }
}

/// One HTTP client for health checks and ElevenLabs. Proxies are ignored: every call is either
/// to loopback or to ElevenLabs directly.
pub(crate) fn http_client() -> reqwest::Client {
    static CRYPTO: Once = Once::new();
    CRYPTO.call_once(|| {
        // Err only means a provider is already installed, which is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("HTTP client builds")
}

/// Whether a page URL is one of the app's own origins (scheme, host and port all match).
fn is_app_origin(url: &tauri::Url) -> bool {
    let origin = match (url.host_str(), url.port()) {
        (Some(host), Some(port)) => format!("{}://{host}:{port}", url.scheme()),
        (Some(host), None) => format!("{}://{host}", url.scheme()),
        _ => return false,
    };
    APP_ORIGINS.contains(&origin.as_str())
}

#[tauri::command]
fn services_status(state: State<'_, AppState>) -> BTreeMap<&'static str, ServiceStatus> {
    state.supervisor.status()
}

#[tauri::command]
fn services_restart(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.supervisor.start(ServiceName::parse(&name)?);
    Ok(())
}

#[tauri::command(async)]
fn secrets_status() -> Result<BTreeMap<&'static str, bool>, String> {
    secrets::status()
}

#[tauri::command(async)]
fn secret_set(name: String, value: String) -> Result<(), String> {
    secrets::set(&name, &value)
}

#[tauri::command(async)]
fn secret_delete(name: String) -> Result<(), String> {
    secrets::delete(&name)
}

#[tauri::command]
async fn elevenlabs_conversation_token(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<elevenlabs::Token, String> {
    let key =
        secrets::get("elevenlabs")?.ok_or("No ElevenLabs API key saved. Add it in Settings.")?;
    elevenlabs::conversation_token(&state.http, elevenlabs::API_BASE, &key, &agent_id).await
}

#[tauri::command]
fn config_get(state: State<'_, AppState>) -> Result<config::Config, String> {
    config::load(&state.config_path)
}

#[tauri::command]
fn config_set(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    config::set(&state.config_path, &key, &value)
}

#[tauri::command(async)]
fn evals_runs(state: State<'_, AppState>) -> Result<tracker::EvalRuns, String> {
    Ok(tracker::eval_runs(state.root()?))
}

#[tauri::command(async)]
fn plan_markdown(state: State<'_, AppState>) -> Result<tracker::Plan, String> {
    tracker::plan_markdown(state.root()?)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // The microphone only for the app's own pages; every other request keeps WebView2's default.
        .on_permission_request(|webview, kind| match kind {
            PermissionKind::Microphone if webview.url().is_ok_and(|url| is_app_origin(&url)) => {
                PermissionResponse::Allow
            }
            PermissionKind::Microphone => PermissionResponse::Deny,
            _ => PermissionResponse::Default,
        })
        .setup(|app| {
            let root = root::discover(std::env::var_os("TUTOR_ROOT"));
            match &root {
                Ok(path) => eprintln!("[tutor-shell] root: {}", path.display()),
                Err(e) => eprintln!("[tutor-shell] {e}"),
            }
            let config_path = app.path().app_config_dir()?.join(config::FILE_NAME);
            let http = http_client();
            let supervisor = Arc::new(Supervisor::new(root.clone(), http.clone()));
            for name in ServiceName::ALL {
                supervisor.start(name);
            }
            app.manage(AppState {
                root,
                config_path,
                http,
                supervisor,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            services_status,
            services_restart,
            secrets_status,
            secret_set,
            secret_delete,
            elevenlabs_conversation_token,
            config_get,
            config_set,
            evals_runs,
            plan_markdown,
            nback::nback_save_session,
            nback::nback_sessions,
            nback::nback_session,
            nback::nback_export_csv,
            speech::tts_audio,
            speech::tts_voices,
            speech::stt_transcribe,
            learning::learning_skills,
            learning::learning_plan,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<AppState>().supervisor.stop_all();
            }
        });
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// Serves one HTTP response on 127.0.0.1 and reports the request head it received.
    pub fn mock_http(status: &str, body: &str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
                head.push_str(&line);
            }
            (&stream).write_all(response.as_bytes()).unwrap();
            let _ = tx.send(head);
        });
        (base, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn microphone_origins_are_exactly_the_app_origins() {
        for ok in ["http://tauri.localhost/", "tauri://localhost/index.html"] {
            assert!(is_app_origin(&ok.parse().unwrap()), "{ok}");
        }
        // The Vite dev server's origins count only in debug builds (C3).
        for dev in ["http://localhost:1420/#/tutor", "http://127.0.0.1:1420"] {
            assert_eq!(
                is_app_origin(&dev.parse().unwrap()),
                cfg!(debug_assertions),
                "{dev}"
            );
        }
        for bad in [
            "https://tauri.localhost/",
            "http://tauri.localhost.evil.com/",
            "http://localhost:1421/",
            "http://localhost/",
            "https://elevenlabs.io/",
            "file:///C:/x.html",
            "data:text/html,hi",
        ] {
            assert!(!is_app_origin(&bad.parse().unwrap()), "{bad}");
        }
    }

    /// Every source the CSP allows, and why. The CSP applies to the built app (served from
    /// http://tauri.localhost); `tauri dev` loads the Vite server directly, without it.
    const CSP: &[(&str, &[(&str, &str)])] = &[
        (
            "default-src",
            &[(
                "'self'",
                "the app's own bundle; everything not listed below",
            )],
        ),
        (
            "script-src",
            &[
                ("'self'", "the app's own bundle"),
                (
                    "blob:",
                    "@elevenlabs/client loads its AudioWorklet processors from blob: URLs",
                ),
            ],
        ),
        (
            "style-src",
            &[
                ("'self'", "the bundled stylesheet"),
                (
                    "'unsafe-inline'",
                    "style elements injected at runtime by UI libraries; styles cannot run script",
                ),
            ],
        ),
        (
            "img-src",
            &[
                ("'self'", "bundled images"),
                ("blob:", "three.js texture loading"),
                ("data:", "images Vite inlines and three.js data textures"),
                (
                    "https://storage.googleapis.com",
                    "the orb's perlin-noise texture (eleven-public-cdn)",
                ),
            ],
        ),
        (
            "connect-src",
            &[
                ("'self'", "the app's own bundle"),
                ("ipc:", "Tauri IPC (macOS, Linux)"),
                ("http://ipc.localhost", "Tauri IPC (Windows)"),
                ("http://127.0.0.1:8001", "DeepTutor backend"),
                ("ws://127.0.0.1:8001", "DeepTutor WebSockets"),
                ("http://127.0.0.1:8811", "rag-engine"),
                ("ws://127.0.0.1:8811", "rag-engine WebSockets"),
                (
                    "https://api.elevenlabs.io",
                    "ElevenLabs REST calls made by @elevenlabs/client (feedback, uploads)",
                ),
                (
                    "wss://api.elevenlabs.io",
                    "ElevenLabs conversation over WebSocket",
                ),
                (
                    "wss://livekit.rtc.elevenlabs.io",
                    "ElevenLabs conversation over WebRTC: LiveKit signalling",
                ),
                (
                    "https://livekit.rtc.elevenlabs.io",
                    "LiveKit reachability checks (HEAD, /rtc/validate)",
                ),
            ],
        ),
        (
            "media-src",
            &[
                ("'self'", "bundled audio"),
                (
                    "blob:",
                    "text-to-speech clips played from object URLs of the shell's MP3 bytes",
                ),
            ],
        ),
        ("object-src", &[("'none'", "no plugins")]),
        (
            "base-uri",
            &[("'self'", "no <base> retargeting of relative URLs")],
        ),
    ];

    #[test]
    fn csp_allows_only_the_documented_sources() {
        let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let csp = conf["app"]["security"]["csp"]
            .as_object()
            .expect("csp is a directive map");
        let actual: Vec<(&str, Vec<&str>)> = csp
            .iter()
            .map(|(directive, sources)| {
                let sources = sources.as_array().expect("sources are a list");
                (
                    directive.as_str(),
                    sources.iter().map(|s| s.as_str().unwrap()).collect(),
                )
            })
            .collect();
        let mut expected: Vec<(&str, Vec<&str>)> = CSP
            .iter()
            .map(|(directive, sources)| {
                (
                    *directive,
                    sources.iter().map(|(source, _why)| *source).collect(),
                )
            })
            .collect();
        expected.sort();
        let mut actual = actual;
        actual.sort();
        assert_eq!(actual, expected);
    }
}
