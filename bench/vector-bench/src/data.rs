//! Synthetic embedding generator ("vb-gen-1"), on-disk formats, and SIMD kernels.
//!
//! Model (all direction vectors unit-norm, energies are squared norms before the final normalisation):
//!   x = a_g*m + a_f*F[field(t)] + a_t*C[t] + U_t (a_s*w[t,u] + a_l*z) + a_e*eps,   then x /= |x|
//!   m      : one global direction shared by every vector (anisotropy -> positive random-pair cosine)
//!   F[f]   : 10 "field" directions; each topic belongs to one field
//!   C[t]   : 2,000 topic centres; topic sizes follow w_t ~ 1/(t + offset)^exp
//!   U_t    : per-topic basis, `rank` (48) columns with N(0, 1/d) entries (almost orthonormal)
//!   w[t,u] : 25 subtopic offsets per topic inside the topic subspace, N(0, diag(lambda))
//!   z      : per-vector low-rank variation, N(0, diag(lambda)); lambda_i ~ 1/(1 + i/decay), sum 1
//!   eps    : isotropic noise, N(0, I/d)
//! A fraction `dup_frac` of rows are near-duplicates (cosine ~ dup_cos) of an earlier row.
//! Queries: `q_centre_frac` come from topic centres (no subtopic/low-rank part), the rest are
//! perturbations of a uniformly chosen base row with cosine to it drawn from [q_cos_lo, q_cos_hi].
//! Every row/query has its own RNG stream (seed, stream, index), so output is independent of
//! thread count and block size. RNG: SplitMix64 seeding + xoshiro256++; normals via Marsaglia polar.

use crate::common::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct GenParams {
    pub generator: String,
    pub n_base: usize,
    pub n_query: usize,
    pub dim: usize,
    pub seed: u64,
    pub n_fields: usize,
    pub n_topics: usize,
    pub n_sub: usize,
    pub rank: usize,
    /// 0 = independent random topic centres; R > 0 = centres drawn from a shared R-dim latent
    /// space (normalize(B y), B ~ N(0, I/d) of size d x R), so related topics exist.
    #[serde(default)]
    pub topic_latent_rank: usize,
    pub zipf_offset: f64,
    pub zipf_exp: f64,
    pub spec_decay: f64,
    pub e_global: f64,
    pub e_field: f64,
    pub e_topic: f64,
    pub e_sub: f64,
    pub e_local: f64,
    pub e_iso: f64,
    pub dup_frac: f64,
    pub dup_cos: f64,
    pub q_centre_frac: f64,
    pub q_centre_iso: f64,
    pub q_cos_lo: f64,
    pub q_cos_hi: f64,
    pub q_lowrank_share: f64,
}

impl Default for GenParams {
    fn default() -> Self {
        GenParams {
            generator: "vb-gen-1".into(),
            n_base: 6_000_000,
            n_query: 10_000,
            dim: 1024,
            seed: 20260930,
            n_fields: 10,
            n_topics: 2000,
            n_sub: 25,
            rank: 48,
            topic_latent_rank: 64,
            zipf_offset: 20.0,
            zipf_exp: 1.0,
            spec_decay: 8.0,
            e_global: 0.15,
            e_field: 0.09,
            e_topic: 0.25,
            e_sub: 0.25,
            e_local: 0.21,
            e_iso: 0.05,
            dup_frac: 0.03,
            dup_cos: 0.97,
            q_centre_frac: 0.15,
            q_centre_iso: 0.15,
            q_cos_lo: 0.55,
            q_cos_hi: 0.80,
            q_lowrank_share: 0.9,
        }
    }
}

impl GenParams {
    /// Defaults, then `--n/--nq/--dim/--seed`, then `--set k=v,k=v` for any numeric field.
    pub fn from_args(a: &Args) -> GenParams {
        let mut p = GenParams::default();
        p.n_base = a.usize("n", p.n_base);
        p.n_query = a.usize("nq", p.n_query);
        p.dim = a.usize("dim", p.dim);
        p.seed = a.usize("seed", p.seed as usize) as u64;
        if let Some(s) = a.opt("set") {
            let mut v = serde_json::to_value(&p).unwrap();
            for kv in s.split(',') {
                let (k, val) = kv.split_once('=').expect("--set k=v");
                let num: f64 = val.parse().expect("numeric --set value");
                let o = v.as_object_mut().unwrap();
                assert!(o.contains_key(k), "unknown generator param {k}");
                o.insert(k.into(), if o[k].is_u64() { json!(num as u64) } else { json!(num) });
            }
            p = serde_json::from_value(v).unwrap();
        }
        p
    }
}

// ---------------------------------------------------------------- RNG

#[inline]
fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub struct Rng {
    s: [u64; 4],
    spare: Option<f64>,
}

impl Rng {
    pub fn stream(seed: u64, stream: u64, idx: u64) -> Rng {
        let mut x = mix64(mix64(mix64(seed) ^ stream) ^ idx);
        let mut s = [0u64; 4];
        for v in s.iter_mut() {
            x = mix64(x);
            *v = x;
        }
        Rng { s, spare: None }
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.s;
        let r = (s[0].wrapping_add(s[3])).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        r
    }
    #[inline]
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    #[inline]
    pub fn below(&mut self, n: u64) -> u64 {
        ((self.next_u64() as u128 * n as u128) >> 64) as u64
    }
    #[inline]
    pub fn normal(&mut self) -> f64 {
        if let Some(v) = self.spare.take() {
            return v;
        }
        loop {
            let u = 2.0 * self.uniform() - 1.0;
            let v = 2.0 * self.uniform() - 1.0;
            let s = u * u + v * v;
            if s > 0.0 && s < 1.0 {
                let m = (-2.0 * s.ln() / s).sqrt();
                self.spare = Some(v * m);
                return u * m;
            }
        }
    }
}

// ---------------------------------------------------------------- model

#[derive(Clone, Copy, Debug, Default)]
pub struct RowMeta {
    pub topic: u16,
    pub sub: u8,
    pub field: u8,
    pub rattr: u8,
    pub dup: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct QueryMeta {
    pub src: u64, // u64::MAX for topic-centre queries
    pub topic: u16,
    pub field: u8,
    pub kind: u8, // 0 = perturbation of a base row, 1 = topic centre
}

pub struct Model {
    pub p: GenParams,
    m: Vec<f32>,
    fields: Vec<f32>,
    centres: Vec<f32>,
    basis: Vec<f32>, // n_topics * rank * dim
    sub: Vec<f32>,   // n_topics * n_sub * rank (already scaled by sqrt(lambda))
    lam_sqrt: Vec<f32>,
    cdf: Vec<f64>,
    pub topic_field: Vec<u8>,
}

fn unit_gaussian(r: &mut Rng, d: usize) -> Vec<f32> {
    let mut v: Vec<f32> = (0..d).map(|_| r.normal() as f32).collect();
    normalize(&mut v);
    v
}

pub fn normalize(v: &mut [f32]) {
    let n = dot_f32(v, v).sqrt();
    if n > 0.0 {
        let inv = 1.0 / n;
        v.iter_mut().for_each(|x| *x *= inv);
    }
}

const S_MODEL: u64 = 100;
const S_CORE: u64 = 1;
const S_DUP: u64 = 2;
const S_QUERY: u64 = 3;

impl Model {
    pub fn new(p: &GenParams) -> Model {
        let (d, nt, ns, rk) = (p.dim, p.n_topics, p.n_sub, p.rank);
        let mut r = Rng::stream(p.seed, S_MODEL, 0);
        let m = unit_gaussian(&mut r, d);
        let fields: Vec<f32> = (0..p.n_fields).flat_map(|_| unit_gaussian(&mut r, d)).collect();
        let lam: Vec<f64> = (0..rk).map(|i| 1.0 / (1.0 + i as f64 / p.spec_decay)).collect();
        let lsum: f64 = lam.iter().sum();
        let lam_sqrt: Vec<f32> = lam.iter().map(|l| (l / lsum).sqrt() as f32).collect();
        let mut w: Vec<f64> = (0..nt).map(|t| 1.0 / (t as f64 + p.zipf_offset).powf(p.zipf_exp)).collect();
        let wsum: f64 = w.iter().sum();
        let mut acc = 0.0;
        for x in w.iter_mut() {
            acc += *x / wsum;
            *x = acc;
        }
        let topic_field: Vec<u8> = (0..nt).map(|_| r.below(p.n_fields as u64) as u8).collect();
        let latent: Vec<f32> = (0..p.topic_latent_rank * d).map(|_| r.normal() as f32).collect();
        // Per-topic structures, each from its own stream so they can be built in parallel.
        let per_topic: Vec<(Vec<f32>, Vec<f32>, Vec<f32>)> = (0..nt)
            .into_par_iter()
            .map(|t| {
                let mut r = Rng::stream(p.seed, S_MODEL + 1, t as u64);
                let c = if p.topic_latent_rank == 0 {
                    unit_gaussian(&mut r, d)
                } else {
                    let y: Vec<f32> = (0..p.topic_latent_rank).map(|_| r.normal() as f32).collect();
                    let mut c = vec![0f32; d];
                    for (i, &yi) in y.iter().enumerate() {
                        for (cj, &bj) in c.iter_mut().zip(&latent[i * d..(i + 1) * d]) {
                            *cj += yi * bj;
                        }
                    }
                    normalize(&mut c);
                    c
                };
                let sc = 1.0 / (d as f64).sqrt();
                let b: Vec<f32> = (0..rk * d).map(|_| (r.normal() * sc) as f32).collect();
                let s: Vec<f32> =
                    (0..ns * rk).map(|j| r.normal() as f32 * lam_sqrt[j % rk]).collect();
                (c, b, s)
            })
            .collect();
        let mut centres = Vec::with_capacity(nt * d);
        let mut basis = Vec::with_capacity(nt * rk * d);
        let mut sub = Vec::with_capacity(nt * ns * rk);
        for (c, b, s) in per_topic {
            centres.extend(c);
            basis.extend(b);
            sub.extend(s);
        }
        Model { p: p.clone(), m, fields, centres, basis, sub, lam_sqrt, cdf: w, topic_field }
    }

    fn sample_topic(&self, u: f64) -> usize {
        self.cdf.partition_point(|&c| c < u).min(self.p.n_topics - 1)
    }

    fn core_meta(&self, idx: u64) -> RowMeta {
        let mut r = Rng::stream(self.p.seed, S_CORE, idx);
        let t = self.sample_topic(r.uniform());
        let u = r.below(self.p.n_sub as u64) as u8;
        let rattr = r.below(10) as u8;
        RowMeta { topic: t as u16, sub: u, field: self.topic_field[t], rattr, dup: 0 }
    }

    /// Metadata of a row without generating its vector.
    pub fn row_meta(&self, idx: u64) -> RowMeta {
        let mut r = Rng::stream(self.p.seed, S_DUP, idx);
        if idx > 0 && r.uniform() < self.p.dup_frac {
            let src = r.below(idx);
            let mut m = self.core_meta(src);
            m.rattr = r.below(10) as u8;
            m.dup = 1;
            m
        } else {
            self.core_meta(idx)
        }
    }

    fn add_topic_part(&self, t: usize, v: &[f32], out: &mut [f32]) {
        let d = self.p.dim;
        for (i, &vi) in v.iter().enumerate() {
            let ui = &self.basis[(t * self.p.rank + i) * d..(t * self.p.rank + i + 1) * d];
            for (o, &b) in out.iter_mut().zip(ui) {
                *o += vi * b;
            }
        }
    }

    fn gen_core(&self, idx: u64, out: &mut [f32]) -> RowMeta {
        let p = &self.p;
        let d = p.dim;
        let mut r = Rng::stream(p.seed, S_CORE, idx);
        let t = self.sample_topic(r.uniform());
        let u = r.below(p.n_sub as u64) as usize;
        let rattr = r.below(10) as u8;
        let f = self.topic_field[t] as usize;
        let (ag, af, at) = (p.e_global.sqrt() as f32, p.e_field.sqrt() as f32, p.e_topic.sqrt() as f32);
        let fv = &self.fields[f * d..(f + 1) * d];
        let cv = &self.centres[t * d..(t + 1) * d];
        for j in 0..d {
            out[j] = ag * self.m[j] + af * fv[j] + at * cv[j];
        }
        let (as_, al) = (p.e_sub.sqrt() as f32, p.e_local.sqrt() as f32);
        let sw = &self.sub[(t * p.n_sub + u) * p.rank..(t * p.n_sub + u + 1) * p.rank];
        let v: Vec<f32> = (0..p.rank)
            .map(|i| as_ * sw[i] + al * self.lam_sqrt[i] * r.normal() as f32)
            .collect();
        self.add_topic_part(t, &v, out);
        let ae = (p.e_iso / d as f64).sqrt();
        for o in out.iter_mut() {
            *o += (ae * r.normal()) as f32;
        }
        normalize(out);
        RowMeta { topic: t as u16, sub: u as u8, field: f as u8, rattr, dup: 0 }
    }

    pub fn gen_row(&self, idx: u64, out: &mut [f32]) -> RowMeta {
        let mut r = Rng::stream(self.p.seed, S_DUP, idx);
        if idx > 0 && r.uniform() < self.p.dup_frac {
            let src = r.below(idx);
            let mut m = self.gen_core(src, out);
            m.rattr = r.below(10) as u8;
            m.dup = 1;
            let e = 1.0 / (self.p.dup_cos * self.p.dup_cos) - 1.0;
            let s = (e / self.p.dim as f64).sqrt();
            for o in out.iter_mut() {
                *o += (s * r.normal()) as f32;
            }
            normalize(out);
            m
        } else {
            self.gen_core(idx, out)
        }
    }

    pub fn gen_query(&self, qi: u64, out: &mut [f32]) -> QueryMeta {
        let p = &self.p;
        let d = p.dim;
        let mut r = Rng::stream(p.seed, S_QUERY, qi);
        if r.uniform() < p.q_centre_frac {
            let t = self.sample_topic(r.uniform());
            let f = self.topic_field[t] as usize;
            let (ag, af, at) =
                (p.e_global.sqrt() as f32, p.e_field.sqrt() as f32, p.e_topic.sqrt() as f32);
            let ae = (p.q_centre_iso / d as f64).sqrt();
            for j in 0..d {
                out[j] = ag * self.m[j]
                    + af * self.fields[f * d + j]
                    + at * self.centres[t * d + j]
                    + (ae * r.normal()) as f32;
            }
            normalize(out);
            QueryMeta { src: u64::MAX, topic: t as u16, field: f as u8, kind: 1 }
        } else {
            let src = r.below(p.n_base as u64);
            let m = self.gen_row(src, out);
            let c = p.q_cos_lo + (p.q_cos_hi - p.q_cos_lo) * r.uniform();
            let e = 1.0 / (c * c) - 1.0;
            let (sl, si) = ((e * p.q_lowrank_share).sqrt(), (e * (1.0 - p.q_lowrank_share) / d as f64).sqrt());
            let v: Vec<f32> =
                (0..p.rank).map(|i| (sl * self.lam_sqrt[i] as f64 * r.normal()) as f32).collect();
            self.add_topic_part(m.topic as usize, &v, out);
            for o in out.iter_mut() {
                *o += (si * r.normal()) as f32;
            }
            normalize(out);
            QueryMeta { src, topic: m.topic, field: m.field, kind: 0 }
        }
    }
}

// ---------------------------------------------------------------- kernels (AVX2 + FMA + F16C)

use std::arch::x86_64::*;

#[inline]
unsafe fn hsum256(v: __m256) -> f32 {
    unsafe {
        let lo = _mm256_castps256_ps128(v);
        let hi = _mm256_extractf128_ps(v, 1);
        let s = _mm_add_ps(lo, hi);
        let s = _mm_hadd_ps(s, s);
        let s = _mm_hadd_ps(s, s);
        _mm_cvtss_f32(s)
    }
}

/// Dot product, len must be a multiple of 8.
#[inline]
pub fn dot_f32(a: &[f32], b: &[f32]) -> f32 {
    assert!(a.len() == b.len() && a.len() % 8 == 0);
    let n = a.len();
    unsafe {
        let (pa, pb) = (a.as_ptr(), b.as_ptr());
        let mut acc = [_mm256_setzero_ps(); 4];
        let mut i = 0;
        while i + 32 <= n {
            for k in 0..4 {
                acc[k] = _mm256_fmadd_ps(
                    _mm256_loadu_ps(pa.add(i + 8 * k)),
                    _mm256_loadu_ps(pb.add(i + 8 * k)),
                    acc[k],
                );
            }
            i += 32;
        }
        while i < n {
            acc[0] = _mm256_fmadd_ps(_mm256_loadu_ps(pa.add(i)), _mm256_loadu_ps(pb.add(i)), acc[0]);
            i += 8;
        }
        hsum256(_mm256_add_ps(_mm256_add_ps(acc[0], acc[1]), _mm256_add_ps(acc[2], acc[3])))
    }
}

/// One row against four queries (row loaded once).
#[inline]
pub fn dot_1x4(row: &[f32], q: [&[f32]; 4]) -> [f32; 4] {
    let n = row.len();
    assert!(n % 8 == 0 && q.iter().all(|x| x.len() == n));
    unsafe {
        let pr = row.as_ptr();
        let pq = [q[0].as_ptr(), q[1].as_ptr(), q[2].as_ptr(), q[3].as_ptr()];
        let mut acc = [_mm256_setzero_ps(); 8];
        let mut i = 0;
        while i + 16 <= n {
            let r0 = _mm256_loadu_ps(pr.add(i));
            let r1 = _mm256_loadu_ps(pr.add(i + 8));
            for k in 0..4 {
                acc[k] = _mm256_fmadd_ps(r0, _mm256_loadu_ps(pq[k].add(i)), acc[k]);
                acc[4 + k] = _mm256_fmadd_ps(r1, _mm256_loadu_ps(pq[k].add(i + 8)), acc[4 + k]);
            }
            i += 16;
        }
        while i < n {
            let r0 = _mm256_loadu_ps(pr.add(i));
            for k in 0..4 {
                acc[k] = _mm256_fmadd_ps(r0, _mm256_loadu_ps(pq[k].add(i)), acc[k]);
            }
            i += 8;
        }
        [
            hsum256(_mm256_add_ps(acc[0], acc[4])),
            hsum256(_mm256_add_ps(acc[1], acc[5])),
            hsum256(_mm256_add_ps(acc[2], acc[6])),
            hsum256(_mm256_add_ps(acc[3], acc[7])),
        ]
    }
}

/// f32 query against an f16 row (raw IEEE half bits).
#[inline]
pub fn dot_f32_f16(q: &[f32], row: &[u16]) -> f32 {
    assert!(q.len() == row.len() && q.len() % 16 == 0);
    unsafe {
        let (pq, pr) = (q.as_ptr(), row.as_ptr());
        let mut a0 = _mm256_setzero_ps();
        let mut a1 = _mm256_setzero_ps();
        let mut i = 0;
        while i < q.len() {
            let h0 = _mm256_cvtph_ps(_mm_loadu_si128(pr.add(i) as *const __m128i));
            let h1 = _mm256_cvtph_ps(_mm_loadu_si128(pr.add(i + 8) as *const __m128i));
            a0 = _mm256_fmadd_ps(h0, _mm256_loadu_ps(pq.add(i)), a0);
            a1 = _mm256_fmadd_ps(h1, _mm256_loadu_ps(pq.add(i + 8)), a1);
            i += 16;
        }
        hsum256(_mm256_add_ps(a0, a1))
    }
}

pub fn f16_to_f32(src: &[u16], dst: &mut [f32]) {
    assert!(src.len() == dst.len() && src.len() % 8 == 0);
    unsafe {
        for i in (0..src.len()).step_by(8) {
            let h = _mm256_cvtph_ps(_mm_loadu_si128(src.as_ptr().add(i) as *const __m128i));
            _mm256_storeu_ps(dst.as_mut_ptr().add(i), h);
        }
    }
}

pub fn f32_to_f16(src: &[f32], dst: &mut [u16]) {
    assert!(src.len() == dst.len() && src.len() % 8 == 0);
    unsafe {
        for i in (0..src.len()).step_by(8) {
            let h = _mm256_cvtps_ph::<_MM_FROUND_TO_NEAREST_INT>(_mm256_loadu_ps(src.as_ptr().add(i)));
            _mm_storeu_si128(dst.as_mut_ptr().add(i) as *mut __m128i, h);
        }
    }
}

/// Bit-level f16 -> f32 without F16C, used by the independent naive ground-truth check.
pub fn f16_bits_to_f64_scalar(h: u16) -> f64 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = ((h >> 10) & 0x1f) as i32;
    let man = (h & 0x3ff) as f64;
    let mag = match exp {
        0 => man * 2f64.powi(-24),
        31 => f64::INFINITY,
        e => (1.0 + man / 1024.0) * 2f64.powi(e - 15),
    };
    sign * mag
}

// ---------------------------------------------------------------- files

pub struct DataInfo {
    pub params: GenParams,
    pub manifest: Value,
    pub filter_field: u8,
    pub filter_rattr: u8,
}

pub fn read_manifest(dir: &Path) -> DataInfo {
    let s = std::fs::read_to_string(dir.join("data.json")).expect("data.json missing: run gen first");
    let v: Value = serde_json::from_str(&s).unwrap();
    let params: GenParams = serde_json::from_value(v["params"].clone()).unwrap();
    DataInfo {
        params,
        filter_field: v["filter_field"].as_u64().unwrap() as u8,
        filter_rattr: v["filter_rattr"].as_u64().unwrap() as u8,
        manifest: v,
    }
}

/// Compact dataset description for JSONL lines.
pub fn dataset_json(info: &DataInfo) -> Value {
    json!({
        "dir": info.manifest["dir"],
        "params": info.params,
        "filter_field": info.filter_field,
        "filter_field_share": info.manifest["field_shares"][info.filter_field as usize],
        "filter_rattr": info.filter_rattr,
        "base_crc32": info.manifest["files"]["base_f16.bin"]["crc32"],
    })
}

pub fn load_base_meta(dir: &Path, n: usize) -> Vec<RowMeta> {
    let b = std::fs::read(dir.join("base_meta.bin")).unwrap();
    assert_eq!(b.len(), n * 8);
    b.chunks_exact(8)
        .map(|c| RowMeta {
            topic: u16::from_le_bytes([c[0], c[1]]),
            sub: c[2],
            field: c[3],
            rattr: c[4],
            dup: c[5],
        })
        .collect()
}

pub fn load_queries(dir: &Path, info: &DataInfo) -> (Vec<f32>, Vec<QueryMeta>) {
    let d = info.params.dim;
    let nq = info.params.n_query;
    let raw = std::fs::read(dir.join("queries_f16.bin")).unwrap();
    assert_eq!(raw.len(), nq * d * 2);
    let bits: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let mut q = vec![0f32; nq * d];
    f16_to_f32(&bits, &mut q);
    let mb = std::fs::read(dir.join("queries_meta.bin")).unwrap();
    let meta = mb
        .chunks_exact(16)
        .map(|c| QueryMeta {
            src: u64::from_le_bytes(c[0..8].try_into().unwrap()),
            topic: u16::from_le_bytes([c[8], c[9]]),
            field: c[10],
            kind: c[11],
        })
        .collect();
    (q, meta)
}

/// Sequential block reader over base_f16.bin (plain reads: pages land in the OS cache,
/// not in this process's working set).
pub struct BaseReader {
    f: File,
    pub dim: usize,
    pub n: usize,
    pub pos: usize,
    raw: Vec<u8>,
}

impl BaseReader {
    pub fn open(dir: &Path, n: usize, dim: usize) -> BaseReader {
        BaseReader { f: File::open(dir.join("base_f16.bin")).unwrap(), dim, n, pos: 0, raw: Vec::new() }
    }
    /// Reads up to `rows` rows as f16 bits into `bits`; returns (start_row, rows_read).
    pub fn next_bits(&mut self, rows: usize, bits: &mut Vec<u16>) -> Option<(usize, usize)> {
        if self.pos >= self.n {
            return None;
        }
        let r = rows.min(self.n - self.pos);
        self.raw.resize(r * self.dim * 2, 0);
        self.f.read_exact(&mut self.raw).unwrap();
        bits.resize(r * self.dim, 0);
        for (b, c) in bits.iter_mut().zip(self.raw.chunks_exact(2)) {
            *b = u16::from_le_bytes([c[0], c[1]]);
        }
        let start = self.pos;
        self.pos += r;
        Some((start, r))
    }
    pub fn next_f32(&mut self, rows: usize, bits: &mut Vec<u16>, out: &mut Vec<f32>) -> Option<(usize, usize)> {
        let (s, r) = self.next_bits(rows, bits)?;
        out.resize(r * self.dim, 0.0);
        out.par_chunks_mut(8192).zip(bits.par_chunks(8192)).for_each(|(o, b)| f16_to_f32(b, o));
        Some((s, r))
    }
}

/// Positioned read of individual rows (f32).
pub fn read_rows(dir: &Path, dim: usize, ids: &[u64]) -> Vec<f32> {
    use std::os::windows::fs::FileExt;
    let f = File::open(dir.join("base_f16.bin")).unwrap();
    let mut out = vec![0f32; ids.len() * dim];
    let mut raw = vec![0u8; dim * 2];
    let mut bits = vec![0u16; dim];
    for (k, &id) in ids.iter().enumerate() {
        let mut off = 0;
        while off < raw.len() {
            off += f.seek_read(&mut raw[off..], id * dim as u64 * 2 + off as u64).unwrap();
        }
        for (b, c) in bits.iter_mut().zip(raw.chunks_exact(2)) {
            *b = u16::from_le_bytes([c[0], c[1]]);
        }
        f16_to_f32(&bits, &mut out[k * dim..(k + 1) * dim]);
    }
    out
}

// ---------------------------------------------------------------- gen command

fn file_crc(path: &Path) -> u32 {
    let mut f = File::open(path).unwrap();
    let mut h = crc32fast::Hasher::new();
    let mut buf = vec![0u8; 8 << 20];
    loop {
        let n = f.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    h.finalize()
}

pub fn cmd_gen(a: &Args) {
    let p = GenParams::from_args(a);
    let dir = data_dir(a);
    std::fs::create_dir_all(&dir).unwrap();
    let man = dir.join("data.json");
    if man.exists() {
        let info = read_manifest(&dir);
        let size_ok = std::fs::metadata(dir.join("base_f16.bin")).map(|m| m.len()).unwrap_or(0)
            == (p.n_base * p.dim * 2) as u64;
        if info.params == p && size_ok {
            if a.flag("verify") {
                let crc = file_crc(&dir.join("base_f16.bin"));
                assert_eq!(json!(crc), info.manifest["files"]["base_f16.bin"]["crc32"], "base crc mismatch");
                log("gen: verified base crc");
            }
            log(format!("gen: {} already complete, skipping", dir.display()));
            return;
        }
        panic!("gen: {} holds a different dataset; remove it or use another --data", dir.display());
    }
    let threads = a.usize("threads", 12);
    init_rayon(threads);
    let t0 = Instant::now();
    let model = Model::new(&p);
    log(format!("gen: model ready in {:.1}s", t0.elapsed().as_secs_f64()));
    let d = p.dim;
    let block = 32768;
    let mut fb = BufWriter::with_capacity(16 << 20, File::create(dir.join("base_f16.bin.tmp")).unwrap());
    let mut fm = BufWriter::new(File::create(dir.join("base_meta.bin.tmp")).unwrap());
    let mut crc_b = crc32fast::Hasher::new();
    let mut field_counts = vec![0u64; p.n_fields];
    let mut rattr_counts = [0u64; 10];
    let mut dups = 0u64;
    let mut vecs = vec![0f32; block * d];
    let mut bits = vec![0u16; block * d];
    let mut metas = vec![RowMeta::default(); block];
    let mut bytes = Vec::with_capacity(block * d * 2);
    let mut start = 0;
    while start < p.n_base {
        let r = block.min(p.n_base - start);
        vecs[..r * d]
            .par_chunks_mut(d)
            .zip(metas[..r].par_iter_mut())
            .enumerate()
            .for_each(|(i, (v, m))| *m = model.gen_row((start + i) as u64, v));
        bits[..r * d]
            .par_chunks_mut(d)
            .zip(vecs[..r * d].par_chunks(d))
            .for_each(|(b, v)| f32_to_f16(v, b));
        bytes.clear();
        for b in &bits[..r * d] {
            bytes.extend_from_slice(&b.to_le_bytes());
        }
        crc_b.update(&bytes);
        fb.write_all(&bytes).unwrap();
        for m in &metas[..r] {
            fm.write_all(&[(m.topic & 0xff) as u8, (m.topic >> 8) as u8, m.sub, m.field, m.rattr, m.dup, 0, 0])
                .unwrap();
            field_counts[m.field as usize] += 1;
            rattr_counts[m.rattr as usize] += 1;
            dups += m.dup as u64;
        }
        start += r;
        if (start / block) % 20 == 0 || start == p.n_base {
            log(format!(
                "gen: {start}/{} rows, {:.0} rows/s",
                p.n_base,
                start as f64 / t0.elapsed().as_secs_f64()
            ));
        }
    }
    fb.flush().unwrap();
    fm.flush().unwrap();
    drop((fb, fm));
    // Queries.
    let mut qv = vec![0f32; p.n_query * d];
    let mut qm = vec![QueryMeta::default(); p.n_query];
    qv.par_chunks_mut(d)
        .zip(qm.par_iter_mut())
        .enumerate()
        .for_each(|(i, (v, m))| *m = model.gen_query(i as u64, v));
    let mut qbits = vec![0u16; p.n_query * d];
    f32_to_f16(&qv, &mut qbits);
    let qbytes: Vec<u8> = qbits.iter().flat_map(|b| b.to_le_bytes()).collect();
    std::fs::write(dir.join("queries_f16.bin.tmp"), &qbytes).unwrap();
    let mut qmb = Vec::with_capacity(p.n_query * 16);
    for m in &qm {
        qmb.extend_from_slice(&m.src.to_le_bytes());
        qmb.extend_from_slice(&m.topic.to_le_bytes());
        qmb.extend_from_slice(&[m.field, m.kind, 0, 0, 0, 0]);
    }
    std::fs::write(dir.join("queries_meta.bin.tmp"), &qmb).unwrap();
    for f in ["base_f16.bin", "base_meta.bin", "queries_f16.bin", "queries_meta.bin"] {
        std::fs::rename(dir.join(format!("{f}.tmp")), dir.join(f)).unwrap();
    }
    let shares: Vec<f64> = field_counts.iter().map(|&c| c as f64 / p.n_base as f64).collect();
    let filter_field = (0..p.n_fields)
        .min_by(|&x, &y| (shares[x] - 0.1).abs().partial_cmp(&(shares[y] - 0.1).abs()).unwrap())
        .unwrap();
    let manifest = json!({
        "dir": dir.display().to_string(),
        "params": p,
        "created": now_rfc3339(),
        "gen_seconds": t0.elapsed().as_secs_f64(),
        "threads": threads,
        "format": {
            "base_f16.bin": "n_base x dim IEEE f16, little endian, row-major, unit-normalised in f32 before rounding",
            "base_meta.bin": "n_base x 8 bytes: topic u16 LE, subtopic u8, field u8, rattr u8 (uniform 0..9), dup u8, 2 pad",
            "queries_f16.bin": "n_query x dim f16; the first 1000 are the ground-truth queries",
            "queries_meta.bin": "n_query x 16 bytes: src row u64 (u64::MAX = topic-centre query), topic u16, field u8, kind u8 (0 perturbation, 1 topic centre), 4 pad",
        },
        "files": {
            "base_f16.bin": {"bytes": (p.n_base * d * 2) as u64, "crc32": crc_b.finalize()},
            "queries_f16.bin": {"bytes": qbytes.len(), "crc32": crc32fast::hash(&qbytes)},
        },
        "field_shares": shares,
        "rattr_shares": rattr_counts.iter().map(|&c| c as f64 / p.n_base as f64).collect::<Vec<_>>(),
        "dup_rows": dups,
        "query_kinds": {"centre": qm.iter().filter(|m| m.kind == 1).count(), "perturbation": qm.iter().filter(|m| m.kind == 0).count()},
        "filter_field": filter_field,
        "filter_rattr": 3,
    });
    std::fs::write(&man, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    log(format!("gen: done in {:.1}s -> {}", t0.elapsed().as_secs_f64(), dir.display()));
}
