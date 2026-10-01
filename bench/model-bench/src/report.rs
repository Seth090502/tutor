//! Result lines (JSONL), timestamps, and the optional nvidia-smi VRAM monitor.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn iso_now() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

pub fn log(msg: impl AsRef<str>) {
    eprintln!("[{}] {}", iso_now(), msg.as_ref());
}

/// Append one JSON object as one line.
pub fn append_result(path: &Path, v: &Value) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)
        .with_context(|| format!("open results file {}", path.display()))?;
    let mut line = serde_json::to_string(v)?;
    line.push('\n');
    f.write_all(line.as_bytes())?;
    f.flush()?;
    Ok(())
}

/// One long-running `nvidia-smi -lms 250` child writing to a file. Started only for GPU runs.
pub struct VramMonitor {
    child: Child,
    path: PathBuf,
}

impl VramMonitor {
    pub fn start(path: PathBuf) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let out = File::create(&path)?;
        let child = Command::new("nvidia-smi")
            .args(["--query-gpu=memory.used", "--format=csv,noheader,nounits", "-lms", "250"])
            .stdout(Stdio::from(out))
            .stderr(Stdio::null())
            .spawn()
            .context("start nvidia-smi")?;
        Ok(Self { child, path })
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Kill exactly the child we started and summarise the samples.
    pub fn stop(mut self) -> Value {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        let s: Vec<u64> = text.lines().filter_map(|l| l.trim().parse().ok()).collect();
        if s.is_empty() {
            return json!({"samples": 0, "note": "no samples parsed (nvidia-smi output buffered or failed)",
                          "file": self.path.display().to_string()});
        }
        let mut head: Vec<u64> = s.iter().take(4).copied().collect();
        head.sort_unstable();
        let baseline = head[head.len() / 2];
        let peak = *s.iter().max().unwrap();
        json!({"samples": s.len(), "baseline_mib": baseline, "peak_mib": peak,
               "delta_mib": peak as i64 - baseline as i64, "file": self.path.display().to_string()})
    }
}

impl Drop for VramMonitor {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
