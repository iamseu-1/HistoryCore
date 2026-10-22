//! 서재 색인(설계 5-5 · 5-7) — 파생 파일 `HistoryCore/index.db`. 지워도 원본 `library.db`에서 다시 짓는다.
pub mod engine;
pub(crate) mod extract;
pub mod rebuild;
pub mod stats;
pub mod pipeline;
pub mod search;
pub mod store;
pub mod tree;

pub use engine::{Engine, Hit, IndexStatus, LibraryTemplates, Opened, StepReport, Wanted};
pub use stats::{DayEdits, DecadeCount, Group, GroupLink, Home, HomeOptions, Linked, MonthNew, SourceCount, Stats};
pub use tree::{Children, Folder};

/// 색인 스키마 중 뽑기 · 통계 표.
pub(crate) fn extra_schema() -> String {
    format!("{}{}", extract::SCHEMA, stats::SCHEMA)
}
