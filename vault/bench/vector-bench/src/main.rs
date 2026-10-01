//! vector-bench: Phase 0 benchmark of dense-vector index candidates for libraryd.
//! Candidate A = lancedb/lance (IVF_PQ, IVF_RQ, IVF_HNSW_SQ), candidate B = usearch HNSW.
//! See docs/phase0/vector-bench.md for method and results. Every command is resumable.

mod common;
mod data;
mod gt;
mod lan;
mod us;

fn main() {
    let a = common::Args::parse();
    match a.cmd.as_str() {
        "tune" => gt::cmd_tune(&a),
        "gen" => data::cmd_gen(&a),
        "gt" => gt::cmd_gt(&a),
        "us-build" => us::cmd_us_build(&a),
        "us-bench" => us::cmd_us_bench(&a),
        "lance-ingest" => lan::cmd_lance_ingest(&a),
        "lance-build" => lan::cmd_lance_build(&a),
        "lance-bench" => lan::cmd_lance_bench(&a),
        _ => {
            eprintln!(
                "usage: vector-bench <tune|gen|gt|us-build|us-bench|lance-ingest|lance-build|lance-bench> [--key value ...]"
            );
            std::process::exit(2);
        }
    }
}
