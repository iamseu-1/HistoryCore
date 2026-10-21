//! 100만 건 측정 판 — 서재 구현 하나(`Library`)를 같은 일감으로 재고 예산표(설계 9-2)와 견준다.
pub mod library;
pub mod report;
pub mod run;
pub mod stats;
pub mod workload;

pub use library::{Aggregates, Library, Result};
pub use report::{Budget, Report, Stat, Verdict, BUDGETS};
pub use run::{run, RunPlan};
pub use stats::{nearest_rank, Op, OpStats};
pub use workload::Workload;
