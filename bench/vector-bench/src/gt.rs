//! Exact ground truth (brute-force inner product), dataset statistics, and generator tuning.

use crate::common::*;
use crate::data::*;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
struct Cand(f32, u32);
impl Eq for Cand {}
impl PartialOrd for Cand {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cand {
    // Reverse on score so BinaryHeap is a min-heap on score; ties broken by id.
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then(self.1.cmp(&o.1))
    }
}

pub struct TopK {
    k: usize,
    h: BinaryHeap<Cand>,
    pub min: f32,
}
impl TopK {
    pub fn new(k: usize) -> TopK {
        TopK { k, h: BinaryHeap::with_capacity(k + 1), min: f32::NEG_INFINITY }
    }
    #[inline]
    pub fn push(&mut self, s: f32, id: u32) {
        if self.h.len() < self.k {
            self.h.push(Cand(s, id));
            if self.h.len() == self.k {
                self.min = self.h.peek().unwrap().0;
            }
        } else if s > self.min {
            self.h.pop();
            self.h.push(Cand(s, id));
            self.min = self.h.peek().unwrap().0;
        }
    }
    /// Descending by score.
    pub fn sorted(&self) -> Vec<(f32, u32)> {
        let mut v: Vec<(f32, u32)> = self.h.iter().map(|c| (c.0, c.1)).collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        v
    }
}

/// Levina-Bickel MLE of local intrinsic dimensionality from the k nearest Euclidean distances.
pub fn lid_mle(scores_desc: &[f32], k: usize) -> Option<f64> {
    if scores_desc.len() < k {
        return None;
    }
    let r: Vec<f64> = scores_desc[..k].iter().map(|&s| (2.0 - 2.0 * s as f64).max(1e-12).sqrt()).collect();
    let rk = r[k - 1];
    let s: f64 = r[..k - 1].iter().map(|&ri| (ri / rk).ln()).sum::<f64>() / (k - 1) as f64;
    if s < 0.0 { Some(-1.0 / s) } else { None }
}

/// Summary statistics of neighbour lists (each sorted by descending cosine).
pub fn neighbour_stats(lists: &[Vec<f32>]) -> Value {
    let mut top1: Vec<f64> = lists.iter().map(|l| l[0] as f64).collect();
    let mut top10: Vec<f64> = lists.iter().map(|l| l[9] as f64).collect();
    let mut top100: Vec<f64> = lists.iter().filter(|l| l.len() >= 100).map(|l| l[99] as f64).collect();
    let mut gap: Vec<f64> = lists.iter().map(|l| (l[0] - l[9]) as f64).collect();
    let mut gap100: Vec<f64> =
        lists.iter().filter(|l| l.len() >= 100).map(|l| (l[0] - l[99]) as f64).collect();
    let mut lid20: Vec<f64> = lists.iter().filter_map(|l| lid_mle(l, 20)).collect();
    let mut lid100: Vec<f64> = lists.iter().filter_map(|l| lid_mle(l, 100)).collect();
    json!({
        "n": lists.len(),
        "cos_top1": percentiles(&mut top1),
        "cos_top10": percentiles(&mut top10),
        "cos_top100": percentiles(&mut top100),
        "gap_top1_top10": percentiles(&mut gap),
        "gap_top1_top100": percentiles(&mut gap100),
        "lid_mle_k20": percentiles(&mut lid20),
        "lid_mle_k100": percentiles(&mut lid100),
    })
}

fn random_pair_stats(vecs: &[f32], d: usize, pairs: usize, seed: u64) -> Value {
    let n = vecs.len() / d;
    let mut r = Rng::stream(seed, 77, 0);
    let mut c: Vec<f64> = (0..pairs)
        .map(|_| {
            let (i, j) = (r.below(n as u64) as usize, r.below(n as u64) as usize);
            dot_f32(&vecs[i * d..(i + 1) * d], &vecs[j * d..(j + 1) * d]) as f64
        })
        .collect();
    percentiles(&mut c)
}

/// Fast generator tuning: full-density rows of a random topic subset, plus all-topic samples.
pub fn cmd_tune(a: &Args) {
    let p = GenParams::from_args(a);
    init_rayon(a.usize("threads", 12));
    let t0 = Instant::now();
    let model = Model::new(&p);
    let d = p.dim;
    let n_topics_sample = a.usize("topics", 40);
    let mut r = Rng::stream(p.seed, 900, 0);
    let mut in_set = vec![false; p.n_topics];
    while in_set.iter().filter(|x| **x).count() < n_topics_sample {
        in_set[r.below(p.n_topics as u64) as usize] = true;
    }
    let rows: Vec<u64> = (0..p.n_base as u64)
        .into_par_iter()
        .filter(|&i| in_set[model.row_meta(i).topic as usize])
        .collect();
    let mut sv = vec![0f32; rows.len() * d];
    sv.par_chunks_mut(d).zip(rows.par_iter()).for_each(|(v, &i)| {
        model.gen_row(i, v);
    });
    let nq = a.usize("nq_scan", 10000).min(p.n_query);
    let mut qv = vec![0f32; nq * d];
    let mut qm = vec![QueryMeta::default(); nq];
    qv.par_chunks_mut(d).zip(qm.par_iter_mut()).enumerate().for_each(|(i, (v, m))| {
        *m = model.gen_query(i as u64, v)
    });
    let qsel: Vec<usize> = (0..nq).filter(|&i| in_set[qm[i].topic as usize]).collect();
    let knn = |q: &[f32], skip: Option<usize>| -> Vec<f32> {
        let mut t = TopK::new(100);
        for (j, row) in sv.chunks_exact(d).enumerate() {
            if Some(j) != skip {
                t.push(dot_f32(q, row), j as u32);
            }
        }
        t.sorted().into_iter().map(|x| x.0).collect()
    };
    let lists: Vec<(u8, Vec<f32>)> =
        qsel.par_iter().map(|&i| (qm[i].kind, knn(&qv[i * d..(i + 1) * d], None))).collect();
    let pert: Vec<Vec<f32>> = lists.iter().filter(|x| x.0 == 0).map(|x| x.1.clone()).collect();
    let cent: Vec<Vec<f32>> = lists.iter().filter(|x| x.0 == 1).map(|x| x.1.clone()).collect();
    let docs: Vec<usize> = (0..300).map(|_| r.below(rows.len() as u64) as usize).collect();
    let doc_lists: Vec<Vec<f32>> =
        docs.par_iter().map(|&j| knn(&sv[j * d..(j + 1) * d], Some(j))).collect();
    // All-topic random sample for random-pair cosine.
    let mut rv = vec![0f32; 4000 * d];
    rv.par_chunks_mut(d).enumerate().for_each(|(i, v)| {
        let idx = Rng::stream(p.seed, 901, i as u64).below(p.n_base as u64);
        model.gen_row(idx, v);
    });
    let out = json!({
        "params": p,
        "subset_rows": rows.len(),
        "random_pair_cos": random_pair_stats(&rv, d, 200_000, p.seed),
        "queries_perturbation": neighbour_stats(&pert),
        "queries_centre": neighbour_stats(&cent),
        "doc_to_doc": neighbour_stats(&doc_lists),
        "seconds": t0.elapsed().as_secs_f64(),
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

pub struct Gt {
    pub n_q: usize,
    pub k: usize,
    pub ids: Vec<u32>, // [n_q][3][k]: sets all, field, rattr
}
impl Gt {
    pub fn get(&self, q: usize, set: usize) -> &[u32] {
        let o = (q * 3 + set) * self.k;
        &self.ids[o..o + self.k]
    }
}

pub fn load_gt(dir: &Path) -> Gt {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("gt.json")).expect("run gt first")).unwrap();
    let n_q = v["n_q"].as_u64().unwrap() as usize;
    let k = v["k"].as_u64().unwrap() as usize;
    let raw = std::fs::read(dir.join("gt_ids.u32")).unwrap();
    let ids: Vec<u32> = raw.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    assert_eq!(ids.len(), n_q * 3 * k);
    Gt { n_q, k, ids }
}

struct QState {
    q: Vec<f32>,
    all: TopK,
    field: Option<TopK>,
    rattr: Option<TopK>,
    skip: u32, // row to exclude (doc-to-doc self match), u32::MAX = none
}

pub fn cmd_gt(a: &Args) {
    let dir = data_dir(a);
    let info = read_manifest(&dir);
    let p = &info.params;
    let (n, d) = (p.n_base, p.dim);
    let n_gt = a.usize("ngt", 1000);
    let k = 100;
    let n_doc = a.usize("ndoc", 500);
    let n_naive = a.usize("nnaive", 5);
    if dir.join("gt.json").exists() {
        let g = load_gt(&dir);
        if g.n_q == n_gt {
            log("gt: already complete, skipping");
            return;
        }
    }
    let threads = a.usize("threads", 12);
    init_rayon(threads);
    require_free_ram("gt", 2u64 << 30);
    let t0 = Instant::now();
    let meta = load_base_meta(&dir, n);
    let (qv, qm) = load_queries(&dir, &info);
    let (ff, fr) = (info.filter_field, info.filter_rattr);
    let mut rr = Rng::stream(p.seed, 950, 0);
    let mut doc_ids: Vec<u64> = Vec::new();
    while doc_ids.len() < n_doc {
        let x = rr.below(n as u64);
        if !doc_ids.contains(&x) {
            doc_ids.push(x);
        }
    }
    let doc_vecs = read_rows(&dir, d, &doc_ids);
    let mut states: Vec<QState> = (0..n_gt)
        .map(|i| QState {
            q: qv[i * d..(i + 1) * d].to_vec(),
            all: TopK::new(k),
            field: Some(TopK::new(k)),
            rattr: Some(TopK::new(k)),
            skip: u32::MAX,
        })
        .chain((0..n_doc).map(|j| QState {
            q: doc_vecs[j * d..(j + 1) * d].to_vec(),
            all: TopK::new(k),
            field: None,
            rattr: None,
            skip: doc_ids[j] as u32,
        }))
        .collect();
    while states.len() % 4 != 0 {
        // Pad to a multiple of 4 with a dummy query that is discarded.
        states.push(QState { q: vec![0.0; d], all: TopK::new(1), field: None, rattr: None, skip: u32::MAX });
    }
    // Independent naive check: f64 accumulation over scalar-decoded f16, separate code path.
    let lut: Vec<f64> = (0..=u16::MAX).map(f16_bits_to_f64_scalar).collect();
    let naive_q: Vec<Vec<f64>> =
        (0..n_naive).map(|i| qv[i * d..(i + 1) * d].iter().map(|&x| x as f64).collect()).collect();
    let mut naive: Vec<Vec<(f64, u32)>> = vec![Vec::new(); n_naive];
    let mut rd = BaseReader::open(&dir, n, d);
    let (mut bits, mut blk) = (Vec::new(), Vec::new());
    let block = 8192;
    while let Some((start, rows)) = rd.next_f32(block, &mut bits, &mut blk) {
        let blk_ref = &blk;
        let meta_ref = &meta;
        states.par_chunks_mut(4).for_each(|g| {
            let qs = [&g[0].q[..], &g[1].q[..], &g[2].q[..], &g[3].q[..]];
            for r in 0..rows {
                let id = (start + r) as u32;
                let s = dot_1x4(&blk_ref[r * d..(r + 1) * d], qs);
                let m = meta_ref[start + r];
                for (st, &sc) in g.iter_mut().zip(s.iter()) {
                    if id == st.skip {
                        continue;
                    }
                    st.all.push(sc, id);
                    if let Some(t) = st.field.as_mut() {
                        if m.field == ff {
                            t.push(sc, id);
                        }
                    }
                    if let Some(t) = st.rattr.as_mut() {
                        if m.rattr == fr {
                            t.push(sc, id);
                        }
                    }
                }
            }
        });
        // Naive: every row, scalar f64, keep everything above a loose bound, trimmed later.
        let bits_ref = &bits;
        let lut = &lut;
        let part: Vec<Vec<(f64, u32)>> = naive_q
            .par_iter()
            .map(|q| {
                let mut v: Vec<(f64, u32)> = (0..rows)
                    .map(|r| {
                        let row = &bits_ref[r * d..(r + 1) * d];
                        let s: f64 = row.iter().zip(q).map(|(&h, &x)| lut[h as usize] * x).sum();
                        (s, (start + r) as u32)
                    })
                    .collect();
                v.sort_by(|a, b| b.0.total_cmp(&a.0));
                v.truncate(k);
                v
            })
            .collect();
        for (acc, p) in naive.iter_mut().zip(part) {
            acc.extend(p);
            acc.sort_by(|a, b| b.0.total_cmp(&a.0));
            acc.truncate(k);
        }
        if (start / block) % 40 == 0 {
            log(format!("gt: {}/{} rows, {:.0}s", start + rows, n, t0.elapsed().as_secs_f64()));
        }
    }
    let scan_s = t0.elapsed().as_secs_f64();
    // Verify the SIMD path against the naive path.
    let mut verify = Vec::new();
    for i in 0..n_naive {
        let fast = states[i].all.sorted();
        let fast_ids: std::collections::HashSet<u32> = fast.iter().map(|x| x.1).collect();
        let kth = naive[i][k - 1].0;
        let mut mismatches = 0;
        let mut unexplained = 0;
        for &(s, id) in &naive[i] {
            if !fast_ids.contains(&id) {
                mismatches += 1;
                if (s - kth).abs() > 1e-5 {
                    unexplained += 1;
                }
            }
        }
        let max_score_diff = fast
            .iter()
            .zip(naive[i].iter())
            .map(|(f, n)| (f.0 as f64 - n.0).abs())
            .fold(0.0, f64::max);
        verify.push(json!({"query": i, "set_mismatches": mismatches, "unexplained_by_ties": unexplained, "max_rank_score_diff": max_score_diff}));
        assert_eq!(unexplained, 0, "ground truth mismatch vs naive for query {i}");
    }
    // Write outputs.
    let mut ids = Vec::with_capacity(n_gt * 3 * k * 4);
    let mut scores = Vec::with_capacity(n_gt * 3 * k * 4);
    let mut q_lists: Vec<(u8, Vec<f32>)> = Vec::new();
    let mut field_lists = Vec::new();
    for (i, st) in states.iter().take(n_gt).enumerate() {
        let sets = [st.all.sorted(), st.field.as_ref().unwrap().sorted(), st.rattr.as_ref().unwrap().sorted()];
        for s in &sets {
            assert_eq!(s.len(), k);
            for &(sc, id) in s {
                ids.extend_from_slice(&id.to_le_bytes());
                scores.extend_from_slice(&sc.to_le_bytes());
            }
        }
        q_lists.push((qm[i].kind, sets[0].iter().map(|x| x.0).collect()));
        field_lists.push(sets[1].iter().map(|x| x.0).collect::<Vec<f32>>());
    }
    std::fs::write(dir.join("gt_ids.u32"), &ids).unwrap();
    std::fs::write(dir.join("gt_scores.f32"), &scores).unwrap();
    let doc_lists: Vec<Vec<f32>> =
        states[n_gt..n_gt + n_doc].iter().map(|s| s.all.sorted().iter().map(|x| x.0).collect()).collect();
    let pert: Vec<Vec<f32>> = q_lists.iter().filter(|x| x.0 == 0).map(|x| x.1.clone()).collect();
    let cent: Vec<Vec<f32>> = q_lists.iter().filter(|x| x.0 == 1).map(|x| x.1.clone()).collect();
    let all: Vec<Vec<f32>> = q_lists.iter().map(|x| x.1.clone()).collect();
    // Random-pair cosine from 20k random rows.
    let sample_ids: Vec<u64> = (0..20_000).map(|_| rr.below(n as u64)).collect();
    let sample = read_rows(&dir, d, &sample_ids);
    let stats = json!({
        "random_pair_cos": random_pair_stats(&sample, d, 200_000, p.seed),
        "queries_all": neighbour_stats(&all),
        "queries_perturbation": neighbour_stats(&pert),
        "queries_centre": neighbour_stats(&cent),
        "doc_to_doc": neighbour_stats(&doc_lists),
        "queries_field_filtered": neighbour_stats(&field_lists),
    });
    let gtj = json!({
        "n_q": n_gt, "k": k, "sets": ["all", "field", "rattr"],
        "layout": "gt_ids.u32 / gt_scores.f32: [n_q][3][k] little endian, scores = inner product, descending",
        "filter_field": ff, "filter_rattr": fr,
        "query_ids": "queries 0..n_q of queries_f16.bin",
        "doc_query_rows": doc_ids.len(),
        "naive_verification": verify,
        "scan_seconds": scan_s,
        "threads": threads,
        "stats": stats,
    });
    std::fs::write(dir.join("gt.json"), serde_json::to_string_pretty(&gtj).unwrap()).unwrap();
    let env = env_json(threads, None);
    append_jsonl(
        &results_path(a),
        json!({"run_id": format!("gt:{}", a.str("data", "d1024")), "kind": "ground_truth", "n_q": n_gt, "k": k,
               "scan_seconds": scan_s, "naive_verification": verify, "stats": stats, "mem": mem_json()}),
        &env,
        &dataset_json(&info),
    );
    log(format!("gt: done in {:.1}s", t0.elapsed().as_secs_f64()));
}

/// Recall@10/@100 of result lists against ground-truth set `set`, overall and by query group.
pub fn recall_summary(gt: &Gt, qm: &[QueryMeta], lists: &[Vec<u64>], set: usize, ff: u8) -> Value {
    let res: Vec<(u8, bool, f64, f64)> = lists
        .iter()
        .enumerate()
        .map(|(qi, got)| {
            let t = gt.get(qi, set);
            (qm[qi].kind, qm[qi].field == ff, recall_at(got, t, 10), recall_at(got, t, 100))
        })
        .collect();
    let avg = |sel: &dyn Fn(&(u8, bool, f64, f64)) -> bool| {
        let v: Vec<&(u8, bool, f64, f64)> = res.iter().filter(|x| sel(x)).collect();
        let n = v.len().max(1) as f64;
        json!({"n": v.len(), "r10": v.iter().map(|x| x.2).sum::<f64>() / n, "r100": v.iter().map(|x| x.3).sum::<f64>() / n})
    };
    json!({"all": avg(&|_| true), "perturbation": avg(&|x| x.0 == 0), "centre": avg(&|x| x.0 == 1),
           "query_in_filter_field": avg(&|x| x.1), "query_outside_filter_field": avg(&|x| !x.1),
           "r10_min": res.iter().map(|x| x.2).fold(1.0, f64::min)})
}
