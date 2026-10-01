//! Candidate A: lancedb + lance, disk-based IVF_PQ / IVF_RQ (RaBitQ) / IVF_HNSW_SQ on f16 vectors.
//! One dataset holds the vectors; each index configuration is built with replace=true and its
//! dataset version is recorded, so benchmarks check out that version (old index files are kept).

use crate::common::*;
use crate::data::*;
use crate::gt::*;
use futures::{StreamExt, TryStreamExt};
use lancedb::DistanceType;
use lancedb::arrow::arrow::array::{
    Array, ArrayData, ArrayRef, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchReader, UInt8Array,
    UInt16Array, UInt64Array, make_array,
};
use lancedb::arrow::arrow::buffer::Buffer;
use lancedb::arrow::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use lancedb::arrow::arrow::error::ArrowError;
use lancedb::database::CreateTableMode;
use lancedb::index::Index;
use lancedb::index::scalar::BitmapIndexBuilder;
use lancedb::index::vector::{IvfHnswSqIndexBuilder, IvfPqIndexBuilder, IvfRqIndexBuilder};
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use lancedb::Table;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn lance_dir(a: &Args) -> PathBuf {
    synth_dir(a).join("lance").join(a.str("data", "d1024"))
}

fn state_path(a: &Args) -> PathBuf {
    lance_dir(a).join("bench_state.json")
}

fn read_state(a: &Args) -> Value {
    std::fs::read_to_string(state_path(a)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(json!({"indexes": {}}))
}

fn write_state(a: &Args, v: &Value) {
    std::fs::write(state_path(a), serde_json::to_string_pretty(v).unwrap()).unwrap();
}

fn runtime(threads: usize) -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().worker_threads(threads).enable_all().build().unwrap()
}

/// Lance reads LANCE_CPU_THREADS once per process; set it before any lance call.
fn set_lance_threads(threads: usize) {
    unsafe { std::env::set_var("LANCE_CPU_THREADS", threads.to_string()) };
}

struct BaseBatches {
    rd: BaseReader,
    meta: Vec<RowMeta>,
    schema: SchemaRef,
    bits: Vec<u16>,
}

impl Iterator for BaseBatches {
    type Item = Result<RecordBatch, ArrowError>;
    fn next(&mut self) -> Option<Self::Item> {
        let (start, rows) = self.rd.next_bits(65536, &mut self.bits)?;
        let d = self.rd.dim;
        let m = &self.meta[start..start + rows];
        let ids = UInt64Array::from_iter_values(start as u64..(start + rows) as u64);
        let field = UInt8Array::from_iter_values(m.iter().map(|r| r.field));
        let rattr = UInt8Array::from_iter_values(m.iter().map(|r| r.rattr));
        let topic = UInt16Array::from_iter_values(m.iter().map(|r| r.topic));
        let data = ArrayData::builder(DataType::Float16)
            .len(rows * d)
            .add_buffer(Buffer::from_vec(self.bits.clone()))
            .build()
            .unwrap();
        let values = make_array(data);
        let vecs = FixedSizeListArray::try_new(Arc::new(Field::new("item", DataType::Float16, true)), d as i32, values, None)
            .unwrap();
        let cols: Vec<ArrayRef> = vec![Arc::new(ids), Arc::new(field), Arc::new(rattr), Arc::new(topic), Arc::new(vecs)];
        Some(RecordBatch::try_new(self.schema.clone(), cols))
    }
}

impl RecordBatchReader for BaseBatches {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

async fn open_table(a: &Args) -> Table {
    let db = lancedb::connect(lance_dir(a).to_str().unwrap()).execute().await.unwrap();
    db.open_table("base").execute().await.unwrap()
}

pub fn cmd_lance_ingest(a: &Args) {
    let threads = a.usize("threads", 12);
    set_lance_threads(threads);
    init_rayon(threads);
    let rt = runtime(threads);
    rt.block_on(async {
        let ddir = data_dir(a);
        let info = read_manifest(&ddir);
        let (n, d) = (info.params.n_base, info.params.dim);
        let ldir = lance_dir(a);
        std::fs::create_dir_all(&ldir).unwrap();
        let st = read_state(a);
        if st["ingested_rows"].as_u64() == Some(n as u64) {
            let t = open_table(a).await;
            if t.count_rows(None).await.unwrap() == n {
                log("lance-ingest: already complete, skipping");
                return;
            }
        }
        require_free_ram("lance-ingest", 2u64 << 30);
        start_ram_watchdog("lance-ingest");
        let db = lancedb::connect(ldir.to_str().unwrap()).execute().await.unwrap();
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::UInt64, false),
            Field::new("field", DataType::UInt8, false),
            Field::new("rattr", DataType::UInt8, false),
            Field::new("topic", DataType::UInt16, false),
            Field::new("vector", DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float16, true)), d as i32), false),
        ]));
        let reader = BaseBatches { rd: BaseReader::open(&ddir, n, d), meta: load_base_meta(&ddir, n), schema, bits: Vec::new() };
        let t0 = Instant::now();
        let table = db
            .create_table("base", Box::new(reader) as Box<dyn RecordBatchReader + Send>)
            .mode(CreateTableMode::Overwrite)
            .execute()
            .await
            .unwrap();
        let ingest_s = t0.elapsed().as_secs_f64();
        let rows = table.count_rows(None).await.unwrap();
        assert_eq!(rows, n);
        let data_bytes = dir_size(&ldir);
        let t1 = Instant::now();
        for col in ["field", "rattr"] {
            table.create_index(&[col], Index::Bitmap(BitmapIndexBuilder::default())).execute().await.unwrap();
        }
        let scalar_s = t1.elapsed().as_secs_f64();
        let version = table.version().await.unwrap();
        let total_bytes = dir_size(&ldir);
        let rec = json!({"run_id": format!("lance-ingest:{}", a.str("data", "d1024")), "kind": "ingest", "candidate": "A_lance",
            "rows": rows, "vector_type": "FixedSizeList<Float16>", "ingest_seconds": ingest_s, "data_bytes": data_bytes,
            "scalar_indexes": {"columns": ["field", "rattr"], "type": "BITMAP", "build_seconds": scalar_s, "bytes": total_bytes - data_bytes},
            "version": version, "mem": mem_json()});
        append_jsonl(&results_path(a), rec, &env_json(threads, None), &dataset_json(&info));
        let mut st = read_state(a);
        st["ingested_rows"] = json!(rows);
        st["base_version"] = json!(version);
        write_state(a, &st);
        log(format!("lance-ingest: {rows} rows in {ingest_s:.0}s, {:.2} GIB", total_bytes as f64 / GIB));
    });
}

fn index_name(a: &Args) -> String {
    let kind = a.str("kind", "rq");
    let p = a.usize("parts", 2450);
    match kind.as_str() {
        "rq" => format!("ivf_rq_p{p}_b{}", a.usize("bits", 4)),
        "pq" => format!("ivf_pq_p{p}_m{}", a.usize("subvec", 128)),
        "hnswsq" => format!("ivf_hnsw_sq_p{p}_m{}_efc{}", a.usize("m", 20), a.usize("efc", 300)),
        _ => panic!("kind rq|pq|hnswsq"),
    }
}

pub fn cmd_lance_build(a: &Args) {
    let threads = a.usize("threads", 12);
    set_lance_threads(threads);
    init_rayon(threads);
    let rt = runtime(threads);
    rt.block_on(async {
        let info = read_manifest(&data_dir(a));
        let d = info.params.dim;
        let name = index_name(a);
        let st = read_state(a);
        if let Some(v) = st["indexes"][&name]["version"].as_u64() {
            let t = open_table(a).await;
            t.checkout(v).await.unwrap();
            let vec_idx = vector_indexes(&t).await;
            if vec_idx == vec![name.clone()] {
                log(format!("lance-build: {name} already built at version {v}, skipping"));
                return;
            }
            if vec_idx.contains(&name) {
                // Older harness versions left other vector indexes on the column; a query would
                // then use the first one. Drop the others at the latest version and re-point.
                t.checkout_latest().await.unwrap();
                for other in vector_indexes(&t).await.into_iter().filter(|n| *n != name) {
                    t.drop_index(&other).await.unwrap();
                }
                assert_eq!(vector_indexes(&t).await, vec![name.clone()]);
                let nv = t.version().await.unwrap();
                let mut st = read_state(a);
                st["indexes"][&name]["version"] = json!(nv);
                st["indexes"][&name]["isolated_from_version"] = json!(v);
                write_state(a, &st);
                log(format!("lance-build: {name} isolated at version {nv} (was {v} with {vec_idx:?})"));
                return;
            }
        }
        let p = a.usize("parts", 2450) as u32;
        // Keep the k-means training sample at or under ~700k vectors (f32 copy ~2.9 GB at 1024 d).
        let sr = a.usize("sample_rate", (256usize).min(700_000 / p as usize)) as u32;
        let sample_bytes = (sr as u64) * (p as u64) * (d as u64) * 4;
        require_free_ram("lance-build", sample_bytes * 2 + (3u64 << 30));
        start_ram_watchdog("lance-build");
        let kind = a.str("kind", "rq");
        let idx = match kind.as_str() {
            "rq" => Index::IvfRq(
                IvfRqIndexBuilder::default()
                    .distance_type(DistanceType::L2)
                    .num_partitions(p)
                    .num_bits(a.usize("bits", 4) as u32)
                    .sample_rate(sr),
            ),
            "pq" => Index::IvfPq(
                IvfPqIndexBuilder::default()
                    .distance_type(DistanceType::L2)
                    .num_partitions(p)
                    .num_sub_vectors(a.usize("subvec", 128) as u32)
                    .num_bits(8)
                    .sample_rate(sr),
            ),
            "hnswsq" => Index::IvfHnswSq(
                IvfHnswSqIndexBuilder::default()
                    .distance_type(DistanceType::L2)
                    .num_partitions(p)
                    .num_edges(a.usize("m", 20) as u32)
                    .ef_construction(a.usize("efc", 300) as u32)
                    .sample_rate(sr),
            ),
            _ => unreachable!(),
        };
        let table = open_table(a).await;
        // One vector index per dataset version: lancedb queries cannot pick an index by name.
        for other in vector_indexes(&table).await {
            table.drop_index(&other).await.unwrap();
        }
        let before = dir_size(&lance_dir(a));
        let t0 = Instant::now();
        table.create_index(&["vector"], idx).name(name.clone()).replace(true).execute().await.unwrap();
        let build_s = t0.elapsed().as_secs_f64();
        let version = table.version().await.unwrap();
        assert_eq!(vector_indexes(&table).await, vec![name.clone()]);
        let cfg = table.list_indices().await.unwrap().into_iter().find(|i| i.name == name).expect("index listed");
        let uuid = cfg.index_uuid.clone().unwrap_or_default();
        let idx_dir = lance_dir(a).join("base.lance").join("_indices").join(&uuid);
        let idx_bytes = if uuid.is_empty() { dir_size(&lance_dir(a)) - before } else { dir_size(&idx_dir) };
        let stats = table.index_stats(&name).await.unwrap();
        let rec = json!({"run_id": format!("lance-build:{}:{name}", a.str("data", "d1024")), "kind": "build", "candidate": "A_lance",
            "index": {"name": name, "type": kind, "num_partitions": p, "sample_rate": sr, "distance": "L2 (vectors unit-normalised)",
                      "num_bits": if kind == "rq" { json!(a.usize("bits", 4)) } else if kind == "pq" { json!(8) } else { Value::Null },
                      "num_sub_vectors": if kind == "pq" { json!(a.usize("subvec", 128)) } else { Value::Null },
                      "hnsw": if kind == "hnswsq" { json!({"m": a.usize("m", 20), "ef_construction": a.usize("efc", 300)}) } else { Value::Null },
                      "uuid": uuid, "stats": stats.map(|s| format!("{s:?}"))},
            "build_seconds": build_s, "build_threads": threads, "index_bytes": idx_bytes,
            "dataset_bytes_total": dir_size(&lance_dir(a)), "version": version, "mem": mem_json()});
        append_jsonl(&results_path(a), rec, &env_json(threads, None), &dataset_json(&info));
        let mut st = read_state(a);
        st["indexes"][&name] = json!({"version": version, "uuid": cfg.index_uuid, "build_seconds": build_s, "index_bytes": idx_bytes});
        write_state(a, &st);
        log(format!("lance-build {name}: {build_s:.0}s, {:.2} GIB index, version {version}", idx_bytes as f64 / GIB));
    });
}

/// Names of the vector indexes on the "vector" column at the table's checked-out version.
async fn vector_indexes(t: &Table) -> Vec<String> {
    t.list_indices().await.unwrap().into_iter().filter(|i| i.columns == ["vector"]).map(|i| i.name).collect()
}

#[derive(Clone, Copy, Debug)]
struct LPoint {
    nprobes: usize,
    refine: Option<u32>,
}

async fn lq(t: &Table, q: &[f32], pt: LPoint, filter: Option<&str>, k: usize) -> Vec<u64> {
    let mut b = t
        .query()
        .nearest_to(q)
        .unwrap()
        .column("vector")
        .distance_type(DistanceType::L2)
        .nprobes(pt.nprobes)
        .limit(k)
        .select(Select::columns(&["id"]));
    if let Some(r) = pt.refine {
        b = b.refine_factor(r);
    }
    if let Some(f) = filter {
        b = b.only_if(f);
    }
    let batches: Vec<RecordBatch> = b.execute().await.unwrap().try_collect().await.unwrap();
    let mut v: Vec<(f32, u64)> = Vec::with_capacity(k);
    for rb in &batches {
        let ids = rb.column_by_name("id").unwrap().as_any().downcast_ref::<UInt64Array>().unwrap();
        let dist = rb.column_by_name("_distance").unwrap().as_any().downcast_ref::<Float32Array>().unwrap();
        for i in 0..rb.num_rows() {
            v.push((dist.value(i), ids.value(i)));
        }
    }
    v.sort_by(|x, y| x.0.total_cmp(&y.0));
    v.into_iter().map(|x| x.1).collect()
}

fn parse_lgrid(s: &str) -> Vec<LPoint> {
    // "nprobes:refine" with refine 0 = none, e.g. "20:0,40:5"
    s.split(',')
        .map(|t| {
            let (np, r) = t.split_once(':').unwrap();
            let r: u32 = r.parse().unwrap();
            LPoint { nprobes: np.parse().unwrap(), refine: if r == 0 { None } else { Some(r) } }
        })
        .collect()
}

pub fn cmd_lance_bench(a: &Args) {
    let threads = a.usize("threads", 12);
    let name = index_name(a);
    let do_recall = a.flag("recall");
    let data_tag = a.str("data", "d1024");
    let done_id = format!("lance-bench:{data_tag}:{name}:t{threads}:{}", if do_recall { "recall" } else { "lat" })
        + &a.opt("tag").map(|t| format!(":{t}")).unwrap_or_default();
    let rpath = results_path(a);
    if has_record(&rpath, &done_id) {
        log(format!("lance-bench: {done_id} already recorded, skipping"));
        return;
    }
    set_lance_threads(threads);
    let mask = set_affinity(threads);
    init_rayon(threads);
    let rt = runtime(threads);
    let st = read_state(a);
    let version = st["indexes"][&name]["version"].as_u64().expect("index not built");
    let idx_bytes = st["indexes"][&name]["index_bytes"].as_u64().unwrap_or(0);
    require_free_ram("lance-bench", 8u64 << 30);
    start_ram_watchdog("lance-bench");
    let ddir = data_dir(a);
    let info = read_manifest(&ddir);
    let env = env_json(threads, Some(mask));
    let ds = dataset_json(&info);
    let gt = load_gt(&ddir);
    let (qv, qm) = load_queries(&ddir, &info);
    let qv = Arc::new(qv);
    let d = info.params.dim;
    let k = 100usize;
    let grid = a.opt("grid").map(|s| parse_lgrid(&s)).unwrap_or_else(|| {
        parse_lgrid("10:0,20:0,40:0,80:0,160:0,20:2,40:2,80:2,20:5,40:5,80:5")
    });
    rt.block_on(async {
        let t0 = Instant::now();
        let table = open_table(a).await;
        table.checkout(version).await.unwrap();
        let open_s = t0.elapsed().as_secs_f64();
        assert_eq!(vector_indexes(&table).await, vec![name.clone()], "version {version} must hold only {name}");
        let index_json = json!({"name": name, "version": version, "index_bytes": idx_bytes, "open_seconds": open_s,
            "index_cache_default_bytes": 6u64 << 30, "lance_cpu_threads": threads, "filter_mode": "prefilter (lancedb default)"});
        let q = |i: usize| qv[i * d..(i + 1) * d].to_vec();
        // Cold start (approximate; OS page cache not dropped).
        let cold_pt = LPoint { nprobes: 20, refine: None };
        let mut cold = Vec::new();
        for qi in 1000..1020 {
            let t = Instant::now();
            std::hint::black_box(lq(&table, &q(qi), cold_pt, None, k).await);
            cold.push(elapsed_ns(t));
        }
        let first = cold[0] as f64 / 1e6;
        append_jsonl(&rpath, json!({"run_id": format!("{done_id}:cold"), "kind": "cold_start", "candidate": "A_lance",
            "index": index_json, "query": {"nprobes": 20, "refine": null, "k": k},
            "approximate": "fresh process; dataset opened at the index version; OS page cache could not be dropped (no admin)",
            "open_seconds": open_s, "first_query_ms": first, "first20": lat_summary(&mut cold), "mem": mem_json()}), &env, &ds);
        // Warm-up across many partitions so the index cache is populated.
        for qi in 1020..1320 {
            std::hint::black_box(lq(&table, &q(qi), LPoint { nprobes: 80, refine: None }, None, k).await);
        }
        let n_single = a.usize("nsingle", 500);
        let n_batch = a.usize("nbatch", 125);
        let tput_s = a.f64("tput_seconds", 5.0);
        let grid_run: Vec<LPoint> = if a.flag("filter-only") { Vec::new() } else { grid.clone() };
        for pt in &grid_run {
            let recall = if do_recall {
                let lists = run_concurrent(&table, &qv, d, (0..gt.n_q).collect(), *pt, None, k, threads).await;
                Some(recall_summary(&gt, &qm, &lists, 0, info.filter_field))
            } else {
                None
            };
            let mut single = Vec::new();
            for i in 0..n_single {
                let t = Instant::now();
                std::hint::black_box(lq(&table, &q(2000 + i % 1000), *pt, None, k).await);
                single.push(elapsed_ns(t));
            }
            let mut batch = Vec::new();
            for b in 0..n_batch {
                let t = Instant::now();
                let futs = (0..4).map(|j| {
                    let tb = table.clone();
                    let v = q(3000 + (4 * b + j) % 1000);
                    let p = *pt;
                    tokio::spawn(async move { lq(&tb, &v, p, None, k).await })
                });
                for r in futures::future::join_all(futs).await {
                    std::hint::black_box(r.unwrap());
                }
                batch.push(elapsed_ns(t));
            }
            let mut tp = Vec::new();
            for c in [8usize, 16] {
                tp.push(lance_clients(&table, &qv, d, *pt, c, tput_s, k).await);
            }
            append_jsonl(&rpath, json!({
                "run_id": format!("{done_id}:np{}:rf{}", pt.nprobes, pt.refine.unwrap_or(0)),
                "kind": "query_point", "candidate": "A_lance", "index": index_json,
                "query": {"nprobes": pt.nprobes, "refine_factor": pt.refine, "k": k},
                "recall": recall, "latency_single": lat_summary(&mut single), "latency_batch4": lat_summary(&mut batch),
                "throughput": {"clients8": tp[0], "clients16": tp[1]}, "mem": mem_json()}), &env, &ds);
            log(format!("lance-bench {name} t{threads} np{} rf{:?}: {}", pt.nprobes, pt.refine,
                recall.as_ref().map(|r| r.to_string()).unwrap_or_default()));
        }
        if !a.flag("nofilter") {
            let fgrid = a.opt("fgrid").map(|s| parse_lgrid(&s)).unwrap_or_else(|| parse_lgrid("20:0,40:0,40:2"));
            for pt in &fgrid {
                for (set, label, expr) in [(1usize, "field", format!("field = {}", info.filter_field)), (2, "rattr", format!("rattr = {}", info.filter_rattr))] {
                    let recall = if do_recall {
                        let lists = run_concurrent(&table, &qv, d, (0..gt.n_q).collect(), *pt, Some(expr.clone()), k, threads).await;
                        Some(recall_summary(&gt, &qm, &lists, set, info.filter_field))
                    } else {
                        None
                    };
                    let mut single = Vec::new();
                    for i in 0..300 {
                        let t = Instant::now();
                        std::hint::black_box(lq(&table, &q(2000 + i), *pt, Some(&expr), k).await);
                        single.push(elapsed_ns(t));
                    }
                    let mut single_in = Vec::new();
                    if set == 1 {
                        let ids: Vec<usize> = (2000..info.params.n_query).filter(|&i| qm[i].field == info.filter_field).take(300).collect();
                        for qi in ids {
                            let t = Instant::now();
                            std::hint::black_box(lq(&table, &q(qi), *pt, Some(&expr), k).await);
                            single_in.push(elapsed_ns(t));
                        }
                    }
                    append_jsonl(&rpath, json!({
                        "latency_single_query_in_field": lat_summary(&mut single_in),
                        "run_id": format!("{done_id}:filter_{label}:np{}:rf{}", pt.nprobes, pt.refine.unwrap_or(0)),
                        "kind": "filter_point", "candidate": "A_lance", "index": index_json,
                        "filter": {"type": label, "expr": expr, "method": "lancedb only_if, prefilter, BITMAP scalar index"},
                        "query": {"nprobes": pt.nprobes, "refine_factor": pt.refine, "k": k},
                        "recall": recall, "latency_single": lat_summary(&mut single), "mem": mem_json()}), &env, &ds);
                }
            }
        }
        append_jsonl(&rpath, json!({"run_id": done_id, "kind": "bench_done", "candidate": "A_lance", "index": index_json, "mem_final": mem_json()}), &env, &ds);
        log(format!("lance-bench {name} t{threads}: done, peak ws {:.2} GIB", mem_info().peak_ws as f64 / GIB));
    });
}

#[allow(clippy::too_many_arguments)]
async fn run_concurrent(table: &Table, qv: &Arc<Vec<f32>>, d: usize, ids: Vec<usize>, pt: LPoint, filter: Option<String>, k: usize, conc: usize) -> Vec<Vec<u64>> {
    let mut out: Vec<(usize, Vec<u64>)> = futures::stream::iter(ids)
        .map(|qi| {
            let tb = table.clone();
            let qv = qv.clone();
            let f = filter.clone();
            tokio::spawn(async move { (qi, lq(&tb, &qv[qi * d..(qi + 1) * d], pt, f.as_deref(), k).await) })
        })
        .buffer_unordered(conc)
        .map(|r| r.unwrap())
        .collect()
        .await;
    out.sort_by_key(|x| x.0);
    out.into_iter().map(|x| x.1).collect()
}

async fn lance_clients(table: &Table, qv: &Arc<Vec<f32>>, d: usize, pt: LPoint, c: usize, secs: f64, k: usize) -> Value {
    let next = Arc::new(AtomicUsize::new(0));
    let t0 = Instant::now();
    let deadline = t0 + Duration::from_secs_f64(secs);
    let tasks: Vec<_> = (0..c)
        .map(|_| {
            let tb = table.clone();
            let qv = qv.clone();
            let next = next.clone();
            tokio::spawn(async move {
                let mut mine = Vec::new();
                while Instant::now() < deadline {
                    let i = 4000 + next.fetch_add(1, Ordering::Relaxed) % 6000;
                    let t = Instant::now();
                    std::hint::black_box(lq(&tb, &qv[i * d..(i + 1) * d], pt, None, k).await);
                    mine.push(elapsed_ns(t));
                }
                mine
            })
        })
        .collect();
    let mut l = Vec::new();
    for t in tasks {
        l.extend(t.await.unwrap());
    }
    let el = t0.elapsed().as_secs_f64();
    let n = l.len();
    json!({"clients": c, "seconds": el, "queries": n, "qps": n as f64 / el, "latency": lat_summary(&mut l)})
}
