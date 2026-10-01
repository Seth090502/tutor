//! Measures a directory's storage the way the vault uses it: sequential throughput, 4 KiB random reads at
//! queue depth 1 and under 32 concurrent readers, and small-file creation. Reads and writes bypass the OS
//! cache (FILE_FLAG_NO_BUFFERING), so results describe the device, not RAM. Prints one JSON object.
//!
//! Usage: iobench <dir> [--size-gib N] [--seconds S]
//! The scratch file and files are created under <dir>/.iobench-<pid> and removed at the end.

use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::windows::fs::{FileExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const NO_BUFFERING: u32 = 0x2000_0000;
const WRITE_THROUGH: u32 = 0x8000_0000;
const ALIGN: usize = 4096;
const SEQ_BLOCK: usize = 1 << 20;

struct Aligned {
    ptr: *mut u8,
    layout: Layout,
}

impl Aligned {
    fn new(len: usize) -> Self {
        let layout = Layout::from_size_align(len, ALIGN).expect("layout");
        let ptr = unsafe { alloc_zeroed(layout) };
        assert!(!ptr.is_null(), "allocation failed");
        Self { ptr, layout }
    }
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.layout.size()) }
    }
}

impl Drop for Aligned {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn percentile(sorted: &[u32], p: f64) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn latency_json(mut us: Vec<u32>) -> String {
    us.sort_unstable();
    format!(
        "{{\"p50_us\":{},\"p95_us\":{},\"p99_us\":{},\"max_us\":{}}}",
        percentile(&us, 0.50),
        percentile(&us, 0.95),
        percentile(&us, 0.99),
        us.last().copied().unwrap_or(0)
    )
}

fn open_unbuffered(path: &Path, write: bool) -> File {
    let mut o = OpenOptions::new();
    o.read(true);
    if write {
        o.write(true).create(true).truncate(true).custom_flags(NO_BUFFERING | WRITE_THROUGH);
    } else {
        o.custom_flags(NO_BUFFERING);
    }
    o.open(path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
}

fn seq_write(path: &Path, size: u64) -> f64 {
    let f = open_unbuffered(path, true);
    let mut buf = Aligned::new(SEQ_BLOCK);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for chunk in buf.as_mut().chunks_exact_mut(8) {
        chunk.copy_from_slice(&rng.next().to_le_bytes());
    }
    let start = Instant::now();
    let mut off = 0u64;
    while off < size {
        // Vary the first word per block so no layer can deduplicate blocks.
        buf.as_mut()[..8].copy_from_slice(&off.to_le_bytes());
        f.seek_write(buf.as_mut(), off).expect("write");
        off += SEQ_BLOCK as u64;
    }
    size as f64 / start.elapsed().as_secs_f64() / 1e6
}

fn seq_read(path: &Path, size: u64) -> f64 {
    let f = open_unbuffered(path, false);
    let mut buf = Aligned::new(SEQ_BLOCK);
    let start = Instant::now();
    let mut off = 0u64;
    while off < size {
        f.seek_read(buf.as_mut(), off).expect("read");
        off += SEQ_BLOCK as u64;
    }
    size as f64 / start.elapsed().as_secs_f64() / 1e6
}

/// 4 KiB random reads from `threads` concurrent readers for `secs` seconds.
fn rand_read(path: &Path, size: u64, threads: usize, secs: u64) -> String {
    let f = Arc::new(open_unbuffered(path, false));
    let blocks = size / ALIGN as u64;
    let deadline = Instant::now() + Duration::from_secs(secs);
    let start = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|t| {
            let f = Arc::clone(&f);
            std::thread::spawn(move || {
                let mut buf = Aligned::new(ALIGN);
                let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ (t as u64 + 1).wrapping_mul(0x9E37_79B9));
                let mut lat = Vec::with_capacity(1 << 16);
                while Instant::now() < deadline {
                    let off = (rng.next() % blocks) * ALIGN as u64;
                    let t0 = Instant::now();
                    f.seek_read(buf.as_mut(), off).expect("read");
                    lat.push(t0.elapsed().as_micros() as u32);
                }
                lat
            })
        })
        .collect();
    let mut all = Vec::new();
    for h in handles {
        all.extend(h.join().expect("reader thread"));
    }
    let elapsed = start.elapsed().as_secs_f64();
    let iops = all.len() as f64 / elapsed;
    format!(
        "{{\"threads\":{threads},\"ops\":{},\"iops\":{:.0},\"mb_s\":{:.1},\"latency\":{}}}",
        all.len(),
        iops,
        iops * ALIGN as f64 / 1e6,
        latency_json(all)
    )
}

/// Creates `count` files of `bytes` each (flushed to the device), like intake writing text-store files.
fn small_files(dir: &Path, count: usize, bytes: usize) -> String {
    let sub = dir.join("small");
    fs::create_dir_all(&sub).expect("mkdir");
    let payload = vec![b'x'; bytes];
    let mut lat = Vec::with_capacity(count);
    let start = Instant::now();
    for i in 0..count {
        let t0 = Instant::now();
        let mut f = File::create(sub.join(format!("{i:05}.md"))).expect("create");
        f.write_all(&payload).expect("write");
        f.sync_all().expect("sync");
        lat.push(t0.elapsed().as_micros() as u32);
    }
    let rate = count as f64 / start.elapsed().as_secs_f64();
    format!(
        "{{\"count\":{count},\"bytes_each\":{bytes},\"files_per_s\":{rate:.0},\"latency\":{}}}",
        latency_json(lat)
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(root) = args.get(1).map(PathBuf::from) else {
        eprintln!("usage: iobench <dir> [--size-gib N] [--seconds S]");
        std::process::exit(2);
    };
    let flag = |name: &str, default: u64| -> u64 {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .map(|v| v.parse().unwrap_or_else(|_| panic!("{name} needs a number")))
            .unwrap_or(default)
    };
    let size = flag("--size-gib", 4) << 30;
    let secs = flag("--seconds", 10);

    let dir = root.join(format!(".iobench-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create scratch dir");
    let file = dir.join("scratch.bin");

    let write_mb_s = seq_write(&file, size);
    let read_mb_s = seq_read(&file, size);
    let qd1 = rand_read(&file, size, 1, secs);
    let qd32 = rand_read(&file, size, 32, secs);
    let small = small_files(&dir, 2000, 16 * 1024);
    fs::remove_dir_all(&dir).expect("remove scratch dir");

    println!(
        "{{\"root\":{:?},\"size_bytes\":{size},\"seq_write_mb_s\":{write_mb_s:.1},\"seq_read_mb_s\":{read_mb_s:.1},\
         \"rand_read_4k_qd1\":{qd1},\"rand_read_4k_32_threads\":{qd32},\"small_file_create\":{small}}}",
        root.display().to_string()
    );
}
