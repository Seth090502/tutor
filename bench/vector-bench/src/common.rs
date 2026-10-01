//! Shared plumbing: paths, CLI args, JSONL records, Windows memory/affinity, timing helpers.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DEFAULT_SYNTH: &str = r"C:\Users\sethy\Projects\vault-synth";
pub const DEFAULT_RESULTS: &str =
    r"C:\Users\sethy\Projects\vault\docs\phase0\vector-bench-results.jsonl";
pub const LANCEDB_VERSION: &str = "0.39.0";
pub const LANCE_VERSION: &str = "12.0.0";
pub const USEARCH_CRATE_VERSION: &str = "2.26.2";
pub const NUMKONG_VERSION: &str = "7.8.3";
pub const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// `--key value` and `--flag` style arguments.
pub struct Args {
    pub cmd: String,
    kv: HashMap<String, String>,
}

impl Args {
    pub fn parse() -> Args {
        let mut it = std::env::args().skip(1);
        let cmd = it.next().unwrap_or_default();
        let rest: Vec<String> = it.collect();
        let mut kv = HashMap::new();
        let mut i = 0;
        while i < rest.len() {
            let k = rest[i].trim_start_matches("--").to_string();
            if i + 1 < rest.len() && !rest[i + 1].starts_with("--") {
                kv.insert(k, rest[i + 1].clone());
                i += 2;
            } else {
                kv.insert(k, "true".to_string());
                i += 1;
            }
        }
        Args { cmd, kv }
    }
    pub fn str(&self, k: &str, d: &str) -> String {
        self.kv.get(k).cloned().unwrap_or_else(|| d.to_string())
    }
    pub fn opt(&self, k: &str) -> Option<String> {
        self.kv.get(k).cloned()
    }
    pub fn usize(&self, k: &str, d: usize) -> usize {
        self.kv.get(k).map(|v| v.parse().expect(k)).unwrap_or(d)
    }
    pub fn f64(&self, k: &str, d: f64) -> f64 {
        self.kv.get(k).map(|v| v.parse().expect(k)).unwrap_or(d)
    }
    pub fn flag(&self, k: &str) -> bool {
        self.kv.get(k).map(|v| v == "true").unwrap_or(false)
    }
}

pub fn synth_dir(a: &Args) -> PathBuf {
    PathBuf::from(a.str("synth", DEFAULT_SYNTH))
}
pub fn data_dir(a: &Args) -> PathBuf {
    synth_dir(a).join(a.str("data", "d1024"))
}
pub fn results_path(a: &Args) -> PathBuf {
    PathBuf::from(a.str("results", DEFAULT_RESULTS))
}

pub fn log(msg: impl AsRef<str>) {
    println!("[{}] {}", now_rfc3339(), msg.as_ref());
    let _ = std::io::stdout().flush();
}

/// RFC 3339 UTC timestamp without external crates (civil-from-days algorithm).
pub fn now_rfc3339() -> String {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = d.as_secs() as i64;
    let (days, sod) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dd = doy - (153 * mp + 2) / 5 + 1;
    let mm = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mm <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        mm,
        dd,
        sod / 3600,
        (sod / 60) % 60,
        sod % 60
    )
}

pub fn cpu_brand() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::__cpuid;
        let mut s = Vec::new();
        for leaf in 0x8000_0002u32..=0x8000_0004 {
            let r = __cpuid(leaf);
            for reg in [r.eax, r.ebx, r.ecx, r.edx] {
                s.extend_from_slice(&reg.to_le_bytes());
            }
        }
        String::from_utf8_lossy(&s).trim_matches(char::from(0)).trim().to_string()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MemInfo {
    pub ws: u64,
    pub peak_ws: u64,
    pub private: u64,
    pub peak_private: u64,
}

pub fn mem_info() -> MemInfo {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let mut c: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut c as *mut _ as *mut PROCESS_MEMORY_COUNTERS,
            c.cb,
        );
        MemInfo {
            ws: c.WorkingSetSize as u64,
            peak_ws: c.PeakWorkingSetSize as u64,
            private: c.PrivateUsage as u64,
            peak_private: c.PeakPagefileUsage as u64,
        }
    }
}

pub fn mem_json() -> Value {
    let m = mem_info();
    json!({
        "working_set_gib": m.ws as f64 / GIB,
        "peak_working_set_gib": m.peak_ws as f64 / GIB,
        "private_gib": m.private as f64 / GIB,
        "peak_private_gib": m.peak_private as f64 / GIB,
        "mapped_view_trims": WS_TRIMS.load(Ordering::SeqCst),
    })
}

/// Available physical memory (free + standby) and total, in bytes.
pub fn sys_mem() -> (u64, u64) {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    unsafe {
        let mut s: MEMORYSTATUSEX = std::mem::zeroed();
        s.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        GlobalMemoryStatusEx(&mut s);
        (s.ullAvailPhys, s.ullTotalPhys)
    }
}

/// Refuse to start a step that would push available RAM under the 6 GIB floor.
pub fn require_free_ram(step: &str, need_bytes: u64) {
    let (avail, _) = sys_mem();
    let floor = 6u64 << 30;
    log(format!(
        "RAM check for {step}: available {:.1} GIB, step needs ~{:.1} GIB",
        avail as f64 / GIB,
        need_bytes as f64 / GIB
    ));
    if avail < need_bytes + floor {
        log(format!("ABORT {step}: available RAM would fall under 6 GIB"));
        std::process::exit(3);
    }
}

static MAP_PTR: AtomicUsize = AtomicUsize::new(0);
static MAP_LEN: AtomicUsize = AtomicUsize::new(0);
pub static WS_TRIMS: AtomicUsize = AtomicUsize::new(0);

/// Register a read-only file mapping whose pages the watchdog may drop from the working set.
pub fn register_mapping(ptr: *const u8, len: usize) {
    MAP_PTR.store(ptr as usize, Ordering::SeqCst);
    MAP_LEN.store(len, Ordering::SeqCst);
}

/// Background watchdog: trims the registered file mapping from the working set above 13 GiB
/// (pages stay in the OS file cache and soft-fault back), and aborts the process if available
/// RAM drops below 6 GiB or the working set still exceeds the 16 GiB benchmark limit.
pub fn start_ram_watchdog(step: &'static str) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let (avail, _) = sys_mem();
            let mut m = mem_info();
            let (p, l) = (MAP_PTR.load(Ordering::SeqCst), MAP_LEN.load(Ordering::SeqCst));
            if m.ws > (13u64 << 30) && p != 0 {
                // VirtualUnlock on unlocked pages removes them from the working set (documented).
                unsafe { windows_sys::Win32::System::Memory::VirtualUnlock(p as *const _, l) };
                WS_TRIMS.fetch_add(1, Ordering::SeqCst);
                m = mem_info();
            }
            if avail < (6u64 << 30) || m.ws > (16u64 << 30) {
                log(format!(
                    "WATCHDOG ABORT in {step}: available {:.2} GIB, working set {:.2} GIB",
                    avail as f64 / GIB,
                    m.ws as f64 / GIB
                ));
                std::process::exit(4);
            }
        }
    });
}

/// Pin the process to `threads` logical processors in a layout that mimics a 6-core part:
/// 6 -> one logical CPU on each of physical cores 0..5 (no SMT siblings),
/// 12 -> both SMT siblings of physical cores 0..5 (6C/12T, like the Ryzen 5 5600X).
/// Windows numbers SMT siblings adjacently (LP 2k and 2k+1 share core k).
pub fn set_affinity(threads: usize) -> u64 {
    let mask: u64 = match threads {
        6 => 0b0101_0101_0101,
        12 => 0xFFF,
        n => (1u64 << n) - 1,
    };
    unsafe {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessAffinityMask};
        let ok = SetProcessAffinityMask(GetCurrentProcess(), mask as usize);
        assert!(ok != 0, "SetProcessAffinityMask failed");
    }
    mask
}

pub fn init_rayon(threads: usize) {
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().unwrap();
}

/// Environment block recorded on every JSONL line.
pub fn env_json(threads: usize, affinity: Option<u64>) -> Value {
    let (_, total) = sys_mem();
    json!({
        "machine": {
            "host": "main-pc",
            "cpu": cpu_brand(),
            "logical_cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
            "total_ram_gib": total as f64 / GIB,
            "os": "Windows 11 Home 10.0.26200",
            "storage": "Samsung 990 PRO NVMe",
            "lance_simd_tier": lance_core::utils::cpu::simd_info().tier.to_string(),
            "usearch_isa_available": usearch::hardware_acceleration_available(),
        },
        "build": {
            "rustc": env!("VB_RUSTC_VERSION"),
            "rustflags": env!("VB_RUSTFLAGS"),
            "profile": "release",
        },
        "versions": {
            "lancedb": LANCEDB_VERSION,
            "lance": LANCE_VERSION,
            "usearch_crate": USEARCH_CRATE_VERSION,
            "usearch_runtime": usearch::version(),
            "numkong": NUMKONG_VERSION,
        },
        "threads": threads,
        "affinity_mask": affinity.map(|m| format!("{m:#x}")),
    })
}

pub fn append_jsonl(path: &Path, mut rec: Value, env: &Value, dataset: &Value) {
    let obj = rec.as_object_mut().unwrap();
    obj.insert("ts".into(), json!(now_rfc3339()));
    obj.insert("env".into(), env.clone());
    obj.insert("dataset".into(), dataset.clone());
    let mut f = OpenOptions::new().create(true).append(true).open(path).expect("open results");
    writeln!(f, "{}", serde_json::to_string(&rec).unwrap()).unwrap();
}

/// Lines already in the results file for a given `run_id` (resumability).
pub fn has_record(path: &Path, run_id: &str) -> bool {
    let Ok(s) = std::fs::read_to_string(path) else { return false };
    let needle = format!("\"run_id\":\"{run_id}\"");
    s.lines().any(|l| l.contains(&needle))
}

pub fn dir_size(p: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let md = match e.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.is_dir() {
                total += dir_size(&e.path());
            } else {
                total += md.len();
            }
        }
    }
    total
}

/// Latency summary in milliseconds.
pub fn lat_summary(ns: &mut [u64]) -> Value {
    if ns.is_empty() {
        return Value::Null;
    }
    ns.sort_unstable();
    let p = |q: f64| {
        let i = ((ns.len() as f64 - 1.0) * q).round() as usize;
        ns[i] as f64 / 1e6
    };
    let mean = ns.iter().sum::<u64>() as f64 / ns.len() as f64 / 1e6;
    json!({"n": ns.len(), "mean_ms": mean, "p50_ms": p(0.5), "p95_ms": p(0.95), "p99_ms": p(0.99), "max_ms": p(1.0)})
}

pub fn elapsed_ns(t: Instant) -> u64 {
    t.elapsed().as_nanos() as u64
}

/// Recall of `got` against the first `k` ids of `truth`.
pub fn recall_at(got: &[u64], truth: &[u32], k: usize) -> f64 {
    let k = k.min(truth.len());
    let t: std::collections::HashSet<u32> = truth[..k].iter().copied().collect();
    let hit = got.iter().take(k).filter(|g| t.contains(&(**g as u32))).count();
    hit as f64 / k as f64
}

pub fn percentiles(v: &mut [f64]) -> Value {
    if v.is_empty() {
        return Value::Null;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p = |q: f64| v[((v.len() as f64 - 1.0) * q).round() as usize];
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let sd = (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / v.len() as f64).sqrt();
    json!({"n": v.len(), "mean": mean, "sd": sd, "p05": p(0.05), "p25": p(0.25), "p50": p(0.5), "p75": p(0.75), "p95": p(0.95)})
}
