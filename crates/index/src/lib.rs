//! 서재 색인(설계 5-5 · 5-7) — 파생 파일 `HistoryCore/index.db`. 지워도 원본 `library.db`에서 다시 짓는다.
pub mod engine;
pub(crate) mod extract;
pub mod pipeline;
pub mod search;
pub mod store;
pub mod tree;

pub use engine::{Engine, Hit, IndexStatus, LibraryTemplates, Opened, StepReport, Wanted};
pub use tree::{Children, Folder};

/// 색인 스키마 중 뽑기 표.
pub(crate) use extract::SCHEMA as EXTRA_SCHEMA;
