//! Start and stop exactly one llama-server process.

use anyhow::{Context, Result, bail};
use std::fs::File;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Embed,
    Rerank,
}

#[derive(Clone, Debug)]
pub struct ServerSpec {
    pub bin: PathBuf,
    pub model: PathBuf,
    pub role: Role,
    /// Pooling the model requires: `last` (Qwen3-Embedding), `cls` (BGE-M3), `rank` (rerankers).
    pub pooling: String,
    pub ngl: i32,
    pub np: u32,
    pub slot_ctx: u32,
    pub ubatch: u32,
    pub threads: u32,
    pub device: Option<String>,
    pub extra: Vec<String>,
}

impl ServerSpec {
    pub fn args(&self, port: u16) -> Vec<String> {
        let mut a: Vec<String> = vec![
            "-m".into(), self.model.display().to_string(),
            "--host".into(), "127.0.0.1".into(),
            "--port".into(), port.to_string(),
            "-c".into(), (self.np * self.slot_ctx).to_string(),
            "-np".into(), self.np.to_string(),
            "-b".into(), self.ubatch.to_string(),
            "-ub".into(), self.ubatch.to_string(),
            "-ngl".into(), self.ngl.to_string(),
            "-t".into(), self.threads.to_string(),
        ];
        if let Some(d) = &self.device {
            a.extend(["--device".into(), d.clone()]);
        }
        match self.role {
            Role::Embed => a.extend(["--embedding".into(), "--pooling".into(), self.pooling.clone()]),
            Role::Rerank => a.extend(["--reranking".into(), "--pooling".into(), self.pooling.clone()]),
        }
        a.extend(self.extra.iter().cloned());
        a
    }
}

pub struct Server {
    child: Child,
    pub base: String,
    pub log_path: PathBuf,
    pub args: Vec<String>,
}

fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

pub fn log_tail(path: &Path, bytes: usize) -> String {
    let b = std::fs::read(path).unwrap_or_default();
    String::from_utf8_lossy(&b[b.len().saturating_sub(bytes)..]).into_owned()
}

impl Server {
    pub fn start(spec: &ServerSpec, log_path: PathBuf) -> Result<Self> {
        if let Some(dir) = log_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let port = free_port()?;
        let args = spec.args(port);
        let log = File::create(&log_path)?;
        let mut cmd = Command::new(&spec.bin);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log));
        if let Some(dir) = spec.bin.parent() {
            cmd.current_dir(dir);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let child = cmd.spawn().with_context(|| format!("spawn {}", spec.bin.display()))?;
        Ok(Self { child, base: format!("http://127.0.0.1:{port}"), log_path, args })
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Poll /health until it returns 200. Fails fast if the process exits.
    pub fn wait_ready(&mut self, timeout: Duration) -> Result<()> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(5)))
            .build()
            .into();
        let t0 = Instant::now();
        loop {
            if let Some(st) = self.child.try_wait()? {
                bail!("llama-server exited early with {st}; log tail:\n{}", log_tail(&self.log_path, 3000));
            }
            if let Ok(r) = agent.get(&format!("{}/health", self.base)).call() {
                if r.status().as_u16() == 200 {
                    return Ok(());
                }
            }
            if t0.elapsed() > timeout {
                bail!("llama-server not healthy after {:?}; log tail:\n{}", timeout, log_tail(&self.log_path, 3000));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// Stop exactly the process this struct started.
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}
