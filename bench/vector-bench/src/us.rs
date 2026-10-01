//! Candidate B: usearch in-memory HNSW (i8 / b1 / f16), optional exact rescoring from mmapped f16.

use crate::common::*;
use crate::data::*;
use crate::gt::*;
use memmap2::Mmap;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind, b1x8};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    I8,     // own per-vector max-abs scaling to [-127, 127], cosine
    I8Auto, // f32 handed to an i8 index: usearch's built-in cast (unit-norm * 127)
    B1,     // sign of (x - corpus mean), Hamming
    F16,    // f16, cosine
}

impl Kind {
    fn parse(s: &str) -> Kind {
        match s {
            "i8" => Kind::I8,
            "i8auto" => Kind::I8Auto,
            "b1" => Kind::B1,
            "f16" => Kind::F16,
            _ => panic!("unknown kind {s}"),
        }
    }
    fn tag(&self) -> &'static str {
        match self {
            Kind::I8 => "i8",
            Kind::I8Auto => "i8auto",
            Kind::B1 => "b1",
            Kind::F16 => "f16",
        }
    }
}

fn names(a: &Args) -> (Kind, usize, usize, String, PathBuf) {
    let kind = Kind::parse(&a.str("kind", "i8"));
    let m = a.usize("m", 16);
    let efc = a.usize("efc", 128);
    let n = a.opt("n").map(|s| format!("_n{s}")).unwrap_or_default();
    let name = format!("us_{}_m{}_efc{}{}", kind.tag(), m, efc, n);
    let dir = synth_dir(a).join("usearch").join(a.str("data", "d1024"));
    (kind, m, efc, name, dir)
}

fn options(kind: Kind, dim: usize, m: usize, efc: usize) -> IndexOptions {
    IndexOptions {
        dimensions: dim,
        metric: if kind == Kind::B1 { MetricKind::Hamming } else { MetricKind::Cos },
        quantization: match kind {
            Kind::I8 | Kind::I8Auto => ScalarKind::I8,
            Kind::B1 => ScalarKind::B1,
            Kind::F16 => ScalarKind::F16,
        },
        connectivity: m,
        expansion_add: efc,
        expansion_search: 64,
        multi: false,
    }
}

#[inline]
fn enc_i8(x: &[f32], out: &mut [i8]) {
    let mx = x.iter().fold(0f32, |m, v| m.max(v.abs()));
    let s = if mx > 0.0 { 127.0 / mx } else { 0.0 };
    for (o, v) in out.iter_mut().zip(x) {
        *o = (v * s).round().clamp(-127.0, 127.0) as i8;
    }
}

#[inline]
fn enc_b1(x: &[f32], mean: &[f32], out: &mut [u8]) {
    out.fill(0);
    for (i, (v, m)) in x.iter().zip(mean).enumerate() {
        if v > m {
            out[i / 8] |= 1 << (i % 8);
        }
    }
}

/// Encoded vectors in the index's storage type.
enum Codes {
    I8(Vec<i8>),
    U8(Vec<u8>), // b1 packed, dim/8 bytes per vector
    F16(Vec<i16>),
    F32(Vec<f32>),
}

fn encode(kind: Kind, x: &[f32], dim: usize, mean: &[f32]) -> Codes {
    let n = x.len() / dim;
    match kind {
        Kind::I8 => {
            let mut c = vec![0i8; n * dim];
            c.par_chunks_mut(dim).zip(x.par_chunks(dim)).for_each(|(o, v)| enc_i8(v, o));
            Codes::I8(c)
        }
        Kind::B1 => {
            let mut c = vec![0u8; n * dim / 8];
            c.par_chunks_mut(dim / 8).zip(x.par_chunks(dim)).for_each(|(o, v)| enc_b1(v, mean, o));
            Codes::U8(c)
        }
        Kind::F16 => {
            let mut b = vec![0u16; n * dim];
            b.par_chunks_mut(8192).zip(x.par_chunks(8192)).for_each(|(o, v)| f32_to_f16(v, o));
            Codes::F16(b.into_iter().map(|v| v as i16).collect())
        }
        Kind::I8Auto => Codes::F32(x.to_vec()),
    }
}

impl Codes {
    fn width(&self, dim: usize) -> usize {
        match self {
            Codes::U8(_) => dim / 8,
            _ => dim,
        }
    }
    fn add(&self, index: &Index, key: u64, i: usize, dim: usize) {
        let w = self.width(dim);
        let r = match self {
            Codes::I8(c) => index.add::<i8>(key, &c[i * w..(i + 1) * w]),
            Codes::U8(c) => index.add::<b1x8>(key, b1x8::from_u8s(&c[i * w..(i + 1) * w])),
            Codes::F16(c) => index.add::<usearch::f16>(key, usearch::f16::from_i16s(&c[i * w..(i + 1) * w])),
            Codes::F32(c) => index.add::<f32>(key, &c[i * w..(i + 1) * w]),
        };
        r.expect("usearch add");
    }
    fn search(&self, index: &Index, i: usize, dim: usize, count: usize, filter: Option<&(dyn Fn(u64) -> bool + Sync)>) -> Vec<u64> {
        let w = self.width(dim);
        let m = match (self, filter) {
            (Codes::I8(c), None) => index.search::<i8>(&c[i * w..(i + 1) * w], count),
            (Codes::U8(c), None) => index.search::<b1x8>(b1x8::from_u8s(&c[i * w..(i + 1) * w]), count),
            (Codes::F16(c), None) => index.search::<usearch::f16>(usearch::f16::from_i16s(&c[i * w..(i + 1) * w]), count),
            (Codes::F32(c), None) => index.search::<f32>(&c[i * w..(i + 1) * w], count),
            (Codes::I8(c), Some(f)) => index.filtered_search::<i8, _>(&c[i * w..(i + 1) * w], count, f),
            (Codes::U8(c), Some(f)) => index.filtered_search::<b1x8, _>(b1x8::from_u8s(&c[i * w..(i + 1) * w]), count, f),
            (Codes::F16(c), Some(f)) => {
                index.filtered_search::<usearch::f16, _>(usearch::f16::from_i16s(&c[i * w..(i + 1) * w]), count, f)
            }
            (Codes::F32(c), Some(f)) => index.filtered_search::<f32, _>(&c[i * w..(i + 1) * w], count, f),
        };
        m.expect("usearch search").keys
    }
}

/// Mean vector of the first `rows` base rows (for centred binary codes).
fn corpus_mean(dir: &std::path::Path, n: usize, d: usize, rows: usize) -> Vec<f32> {
    let mut rd = BaseReader::open(dir, n.min(rows), d);
    let (mut bits, mut blk) = (Vec::new(), Vec::new());
    let mut acc = vec![0f64; d];
    let mut cnt = 0usize;
    while let Some((_, r)) = rd.next_f32(65536, &mut bits, &mut blk) {
        for row in blk.chunks_exact(d) {
            for (a, v) in acc.iter_mut().zip(row) {
                *a += *v as f64;
            }
        }
        cnt += r;
    }
    acc.iter().map(|a| (a / cnt as f64) as f32).collect()
}

fn bytes_per_vector(kind: Kind, dim: usize) -> usize {
    match kind {
        Kind::I8 | Kind::I8Auto => dim,
        Kind::B1 => dim / 8,
        Kind::F16 => dim * 2,
    }
}

pub fn cmd_us_build(a: &Args) {
    let (kind, m, efc, name, idir) = names(a);
    std::fs::create_dir_all(&idir).unwrap();
    let ddir = data_dir(a);
    let info = read_manifest(&ddir);
    let d = info.params.dim;
    let n = a.usize("n", info.params.n_base);
    let path = idir.join(format!("{name}.usearch"));
    let man = idir.join(format!("{name}.json"));
    if man.exists() && path.exists() {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&man).unwrap()).unwrap();
        if v["file_bytes"].as_u64() == Some(std::fs::metadata(&path).unwrap().len()) {
            log(format!("us-build: {name} already built, skipping"));
            return;
        }
    }
    let threads = a.usize("threads", 12);
    init_rayon(threads);
    let est = (n * (bytes_per_vector(kind, d) + 16 + m * 2 * 4 + 64)) as u64;
    require_free_ram("us-build", est + (1u64 << 30));
    start_ram_watchdog("us-build");
    let mean = if kind == Kind::B1 { corpus_mean(&ddir, n, d, 200_000) } else { Vec::new() };
    let opts = options(kind, d, m, efc);
    let index = Index::new(&opts).expect("index");
    index.reserve_capacity_and_threads(n, threads + 4).expect("reserve");
    let t0 = Instant::now();
    let mut rd = BaseReader::open(&ddir, n, d);
    let (mut bits, mut blk) = (Vec::new(), Vec::new());
    while let Some((start, rows)) = rd.next_f32(65536, &mut bits, &mut blk) {
        let codes = encode(kind, &blk, d, &mean);
        (0..rows).into_par_iter().for_each(|i| codes.add(&index, (start + i) as u64, i, d));
        let done = start + rows;
        if (done / 65536) % 8 == 0 || done == n {
            let mi = mem_info();
            log(format!(
                "us-build {name}: {done}/{n} ({:.0} vec/s), ws {:.2} GiB",
                done as f64 / t0.elapsed().as_secs_f64(),
                mi.ws as f64 / GiB
            ));
        }
    }
    let build_s = t0.elapsed().as_secs_f64();
    let mem_usage = index.memory_usage();
    let ms = index.memory_stats();
    let st = index.stats();
    let tmp = idir.join(format!("{name}.usearch.tmp"));
    let t1 = Instant::now();
    index.save(tmp.to_str().unwrap()).expect("save");
    std::fs::rename(&tmp, &path).unwrap();
    let save_s = t1.elapsed().as_secs_f64();
    if kind == Kind::B1 {
        let mb: Vec<u8> = mean.iter().flat_map(|v| v.to_le_bytes()).collect();
        std::fs::write(idir.join(format!("{name}.mean.f32")), mb).unwrap();
    }
    let file_bytes = std::fs::metadata(&path).unwrap().len();
    let rec = json!({
        "run_id": format!("us-build:{}:{name}", a.str("data", "d1024")),
        "kind": "build", "candidate": "B_usearch",
        "index": {"name": name, "kind": kind.tag(), "connectivity": m, "expansion_add": efc, "n": n, "dim": d,
                   "metric": format!("{:?}", opts.metric), "quantization": format!("{:?}", opts.quantization),
                   "i8_encoding": if kind == Kind::I8 {"per-vector max-abs scaling to +-127 (cosine is scale invariant)"} else {""},
                   "b1_encoding": if kind == Kind::B1 {"sign(x - mean of first 200k rows), LSB-first"} else {""}},
        "build_seconds": build_s, "save_seconds": save_s, "build_threads": threads,
        "insert_rate_vec_per_s": n as f64 / build_s,
        "file_bytes": file_bytes, "memory_usage_bytes": mem_usage,
        "memory_stats": {"graph_allocated": ms.graph_allocated, "vectors_allocated": ms.vectors_allocated,
                          "graph_wasted": ms.graph_wasted, "vectors_wasted": ms.vectors_wasted},
        "graph_stats": {"nodes": st.nodes, "edges": st.edges, "max_edges": st.max_edges, "allocated_bytes": st.allocated_bytes},
        "isa": index.hardware_acceleration(),
        "mem": mem_json(),
    });
    std::fs::write(&man, serde_json::to_string_pretty(&json!({"file_bytes": file_bytes, "record": rec})).unwrap()).unwrap();
    append_jsonl(&results_path(a), rec, &env_json(threads, None), &dataset_json(&info));
    log(format!("us-build {name}: done in {build_s:.0}s, {:.2} GiB file", file_bytes as f64 / GiB));
}

// ------------------------------------------------------------------ NumKong kernel override

unsafe extern "C" {
    fn nk_find_kernel_punned(kind: i32, dtype: i32, viable: u64, kernel: *mut *const std::ffi::c_void, cap: *mut u64);
    fn nk_capabilities() -> u64;
}
type NkKernel = unsafe extern "C" fn(*const std::ffi::c_void, *const std::ffi::c_void, u64, *mut std::ffi::c_void);

/// Route distance computations through a NumKong kernel picked from a restricted ISA mask:
/// "full" = everything this CPU supports, "avx2" = serial + haswell (AVX2/FMA/F16C) only.
fn override_metric(index: &mut Index, kind: Kind, dim: usize, mode: &str) -> String {
    let viable = match mode {
        "avx2" => 1u64 | (1 << 2),
        "full" => unsafe { nk_capabilities() },
        _ => panic!("metric override {mode}"),
    };
    let (nk_kind, dtype) = match kind {
        Kind::I8 => (b'a' as i32, 1 << 2),
        Kind::B1 => (b'h' as i32, 1 << 1),
        _ => panic!("override only for i8 / b1"),
    };
    let mut kp: *const std::ffi::c_void = std::ptr::null();
    let mut cap = 0u64;
    unsafe { nk_find_kernel_punned(nk_kind, dtype, viable, &mut kp, &mut cap) };
    assert!(!kp.is_null(), "no NumKong kernel for {mode}");
    let f: NkKernel = unsafe { std::mem::transmute(kp) };
    match kind {
        Kind::I8 => index.change_metric::<i8>(Box::new(move |x: *const i8, y: *const i8| {
            let mut r = 0f32;
            unsafe { f(x as _, y as _, dim as u64, &mut r as *mut f32 as _) };
            r
        })),
        Kind::B1 => index.change_metric::<b1x8>(Box::new(move |x: *const b1x8, y: *const b1x8| {
            let mut r = 0u32;
            unsafe { f(x as _, y as _, dim as u64, &mut r as *mut u32 as _) };
            r as f32
        })),
        _ => unreachable!(),
    }
    format!("numkong viable={viable:#x} selected_cap={cap:#x}")
}

// ------------------------------------------------------------------ bench

#[derive(Clone, Copy, Debug)]
struct Point {
    ef: usize,
    cand: usize,
    rescore: bool,
}

fn default_grid(kind: Kind) -> Vec<Point> {
    let p = |ef, cand, rescore| Point { ef, cand, rescore };
    match kind {
        Kind::I8 | Kind::I8Auto | Kind::F16 => vec![
            p(100, 100, false),
            p(128, 100, false),
            p(192, 100, false),
            p(256, 100, false),
            p(384, 100, false),
            p(512, 100, false),
            p(128, 128, true),
            p(192, 192, true),
            p(256, 256, true),
            p(384, 384, true),
            p(512, 512, true),
        ],
        Kind::B1 => vec![
            p(256, 256, true),
            p(512, 512, true),
            p(1024, 1024, true),
            p(2048, 1024, true),
            p(2048, 2048, true),
        ],
    }
}

fn parse_grid(s: &str) -> Vec<Point> {
    // "ef:cand:r,ef:cand:n"
    s.split(',')
        .map(|t| {
            let v: Vec<&str> = t.split(':').collect();
            Point { ef: v[0].parse().unwrap(), cand: v[1].parse().unwrap(), rescore: v[2] == "r" }
        })
        .collect()
}

pub fn cmd_us_bench(a: &Args) {
    let (kind, m, _efc, name, idir) = names(a);
    let threads = a.usize("threads", 12);
    let mo = a.str("metric", "native");
    let do_recall = a.flag("recall");
    let data_tag = a.str("data", "d1024");
    let done_id = format!("us-bench:{data_tag}:{name}:t{threads}:{mo}:{}", if do_recall { "recall" } else { "lat" });
    let rpath = results_path(a);
    if has_record(&rpath, &done_id) {
        log(format!("us-bench: {done_id} already recorded, skipping"));
        return;
    }
    let mask = set_affinity(threads);
    init_rayon(threads);
    let ddir = data_dir(a);
    let info = read_manifest(&ddir);
    let d = info.params.dim;
    let path = idir.join(format!("{name}.usearch"));
    let fsize = std::fs::metadata(&path).expect("index missing: run us-build").len();
    require_free_ram("us-bench", fsize + (1u64 << 30));
    start_ram_watchdog("us-bench");
    let env = env_json(threads, Some(mask));
    let ds = dataset_json(&info);
    let gt = load_gt(&ddir);
    let (qv, qm) = load_queries(&ddir, &info);
    let n_base = a.usize("n", info.params.n_base);
    let meta = load_base_meta(&ddir, info.params.n_base);
    let field: Vec<u8> = meta.iter().map(|r| r.field).collect();
    let rattr: Vec<u8> = meta.iter().map(|r| r.rattr).collect();
    drop(meta);
    let mean: Vec<f32> = if kind == Kind::B1 {
        std::fs::read(idir.join(format!("{name}.mean.f32")))
            .unwrap()
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    } else {
        Vec::new()
    };
    // Load (in-memory copy of the whole index).
    let t0 = Instant::now();
    let mut index = Index::restore(path.to_str().unwrap()).expect("restore");
    let load_s = t0.elapsed().as_secs_f64();
    index.reserve_capacity_and_threads(index.size(), 64).expect("reserve threads");
    let isa = index.hardware_acceleration();
    let metric_note = if mo == "native" { String::new() } else { override_metric(&mut index, kind, d, &mo) };
    let mi_loaded = mem_json();
    let file = std::fs::File::open(ddir.join("base_f16.bin")).unwrap();
    let mmap = unsafe { Mmap::map(&file).unwrap() };
    let base16: &[u16] = unsafe { std::slice::from_raw_parts(mmap.as_ptr() as *const u16, info.params.n_base * d) };
    let codes = encode(kind, &qv, d, &mean);
    let k = 100usize;
    let search = |qi: usize, pt: &Point, filt: Option<&(dyn Fn(u64) -> bool + Sync)>| -> Vec<u64> {
        let keys = codes.search(&index, qi, d, pt.cand.max(k), filt);
        if !pt.rescore {
            return keys.into_iter().take(k).collect();
        }
        let q = &qv[qi * d..(qi + 1) * d];
        let mut sc: Vec<(f32, u64)> = keys
            .iter()
            .map(|&key| (dot_f32_f16(q, &base16[key as usize * d..(key as usize + 1) * d]), key))
            .collect();
        sc.sort_by(|x, y| y.0.total_cmp(&x.0));
        sc.into_iter().take(k).map(|x| x.1).collect()
    };
    let grid = a.opt("grid").map(|s| parse_grid(&s)).unwrap_or_else(|| default_grid(kind));
    let index_json = json!({"name": name, "kind": kind.tag(), "connectivity": m, "file_bytes": fsize,
        "size": index.size(), "memory_usage_bytes": index.memory_usage(), "isa_native": isa,
        "metric_override": mo, "metric_note": metric_note, "load_seconds": load_s, "n_base": n_base});
    // Cold: first queries after loading in this fresh process (approximate: OS file cache not dropped).
    let cold_pt = grid[grid.len() / 2];
    index.change_expansion_search(cold_pt.ef);
    let mut cold_ns: Vec<u64> = (1000..1020)
        .map(|qi| {
            let t = Instant::now();
            std::hint::black_box(search(qi, &cold_pt, None));
            elapsed_ns(t)
        })
        .collect();
    let cold_first = cold_ns[0] as f64 / 1e6;
    append_jsonl(&rpath, json!({"run_id": format!("{done_id}:cold"), "kind": "cold_start", "candidate": "B_usearch",
        "index": index_json, "query": {"ef": cold_pt.ef, "cand": cold_pt.cand, "rescore": cold_pt.rescore, "k": k},
        "approximate": "fresh process after index load; OS page cache could not be dropped (no admin), so the index file and base_f16.bin may be cached",
        "load_seconds": load_s, "first_query_ms": cold_first, "first20": lat_summary(&mut cold_ns), "mem_after_load": mi_loaded}), &env, &ds);
    // Warm-up.
    for qi in 1020..1520 {
        std::hint::black_box(search(qi, &cold_pt, None));
    }
    let n_single = a.usize("nsingle", 1000);
    let n_batch = a.usize("nbatch", 250);
    let tput_s = a.f64("tput_seconds", 5.0);
    for pt in &grid {
        index.change_expansion_search(pt.ef);
        let recall = if do_recall { Some(recall_block(&gt, &qm, |qi| search(qi, pt, None), 0, info.filter_field)) } else { None };
        let mut single: Vec<u64> = (0..n_single)
            .map(|i| {
                let t = Instant::now();
                std::hint::black_box(search(2000 + i % 1000, pt, None));
                elapsed_ns(t)
            })
            .collect();
        let mut batch: Vec<u64> = (0..n_batch)
            .map(|b| {
                let t = Instant::now();
                (0..4).into_par_iter().for_each(|j| {
                    std::hint::black_box(search(3000 + (4 * b + j) % 1000, pt, None));
                });
                elapsed_ns(t)
            })
            .collect();
        let tp: Vec<Value> = [8usize, 16].iter().map(|&c| clients(c, tput_s, |qi| { std::hint::black_box(search(qi, pt, None)); })).collect();
        append_jsonl(&rpath, json!({
            "run_id": format!("{done_id}:ef{}:c{}:{}", pt.ef, pt.cand, if pt.rescore {"r"} else {"n"}),
            "kind": "query_point", "candidate": "B_usearch", "index": index_json,
            "query": {"ef": pt.ef, "cand": pt.cand, "rescore": pt.rescore, "rescore_source": if pt.rescore {"mmapped base_f16.bin"} else {""}, "k": k},
            "recall": recall, "latency_single": lat_summary(&mut single), "latency_batch4": lat_summary(&mut batch),
            "throughput": {"clients8": tp[0], "clients16": tp[1]}, "mem": mem_json()}), &env, &ds);
        log(format!("us-bench {name} t{threads} ef{} c{} r{}: {}", pt.ef, pt.cand, pt.rescore,
            recall.as_ref().map(|r| r.to_string()).unwrap_or_default()));
    }
    // Filters: keep ~10% of rows, either one field (topic-correlated) or a random attribute.
    if !a.flag("nofilter") {
        let fgrid: Vec<Point> = match kind {
            Kind::B1 => vec![Point { ef: 1024, cand: 1024, rescore: true }, Point { ef: 2048, cand: 2048, rescore: true }],
            _ => vec![
                Point { ef: 256, cand: 256, rescore: true },
                Point { ef: 512, cand: 512, rescore: true },
                Point { ef: 1024, cand: 1024, rescore: true },
            ],
        };
        let ff = info.filter_field;
        let fr = info.filter_rattr;
        let pf = |key: u64| field[key as usize] == ff;
        let pr = |key: u64| rattr[key as usize] == fr;
        for pt in &fgrid {
            index.change_expansion_search(pt.ef);
            for (set, label, pred) in [(1usize, "field", &pf as &(dyn Fn(u64) -> bool + Sync)), (2, "rattr", &pr as _)] {
                let recall = if do_recall { Some(recall_block(&gt, &qm, |qi| search(qi, pt, Some(pred)), set, ff)) } else { None };
                let mut single: Vec<u64> = (0..300)
                    .map(|i| {
                        let t = Instant::now();
                        std::hint::black_box(search(2000 + i, pt, Some(pred)));
                        elapsed_ns(t)
                    })
                    .collect();
                append_jsonl(&rpath, json!({
                    "run_id": format!("{done_id}:filter_{label}:ef{}", pt.ef),
                    "kind": "filter_point", "candidate": "B_usearch", "index": index_json,
                    "filter": {"type": label, "value": if set == 1 { ff } else { fr }, "method": "usearch filtered_search predicate during HNSW traversal"},
                    "query": {"ef": pt.ef, "cand": pt.cand, "rescore": pt.rescore, "k": k},
                    "recall": recall, "latency_single": lat_summary(&mut single), "mem": mem_json()}), &env, &ds);
            }
        }
    }
    append_jsonl(&rpath, json!({"run_id": done_id, "kind": "bench_done", "candidate": "B_usearch", "index": index_json, "mem_final": mem_json()}), &env, &ds);
    log(format!("us-bench {name} t{threads}: done, peak ws {:.2} GiB", mem_info().peak_ws as f64 / GiB));
}

/// Recall@10/@100 over the ground-truth queries (searches run in parallel on the rayon pool).
pub fn recall_block(gt: &Gt, qm: &[QueryMeta], f: impl Fn(usize) -> Vec<u64> + Sync, set: usize, ff: u8) -> Value {
    let lists: Vec<Vec<u64>> = (0..gt.n_q).into_par_iter().map(f).collect();
    recall_summary(gt, qm, &lists, set, ff)
}

/// Closed-loop throughput: `c` client threads issue queries back to back for `secs` seconds.
/// Threads stay inside the process affinity mask, so CPU use never exceeds the thread setting.
pub fn clients(c: usize, secs: f64, f: impl Fn(usize) + Sync) -> Value {
    let next = AtomicUsize::new(0);
    let lats = Mutex::new(Vec::new());
    let t0 = Instant::now();
    let deadline = t0 + Duration::from_secs_f64(secs);
    std::thread::scope(|s| {
        for _ in 0..c {
            s.spawn(|| {
                let mut mine = Vec::new();
                while Instant::now() < deadline {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let t = Instant::now();
                    f(4000 + i % 6000);
                    mine.push(elapsed_ns(t));
                }
                lats.lock().unwrap().extend(mine);
            });
        }
    });
    let el = t0.elapsed().as_secs_f64();
    let mut l = lats.into_inner().unwrap();
    let n = l.len();
    json!({"clients": c, "seconds": el, "queries": n, "qps": n as f64 / el, "latency": lat_summary(&mut l)})
}
