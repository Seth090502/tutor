//! Pure learner projections for the Tutor app (CONTRACTS.md C13): skill states from closed attempts
//! (`skills`) and a study plan from review candidates (`plan`).
//!
//! No I/O, no clock, no global state: every time and timezone is an argument and the output is
//! deterministic. Every number is a provisional local policy default, not a learning claim.

pub mod plan;
pub mod skills;
