//! 서재 엔진(설계 4-1). 원본 저장소 `library.db` · 판 · 다리가 부르는 일.

/// 엔진 판 — 작업공간 판과 같다.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// 다리 · 창구 약속의 판. 요청 · 답의 모양이 바뀌면 올린다(op를 더하는 것은 바꿈이 아니다).
pub const ABI_VERSION: u32 = 1;
/// 서재 창구 약속의 판(spec/windows-api.md 1절 — `window.*` 답마다 `api_version`). 칸 · 질의를 더하는 것은 올리지 않는다.
pub const WINDOW_API_VERSION: u32 = 1;

mod docs;
pub mod error;
pub mod hlc;
pub mod ids;
pub mod ns;
pub mod store;
mod sync;
pub mod title;

pub use error::{Error, Result};
pub use hlc::{Clock, Hlc, WallClock};
pub use ids::{DeviceId, DocId, LibraryId, RevId};
pub use ns::{Namespace, RESERVED_PREFIXES};
pub use title::{normalize_title, MAX_TITLE_CHARS};
pub use docs::Reader;
pub use sync::{Applied, Exported, SyncStatus};
pub use store::{in_read_tx, open_read_only, Counts, Doc, DocSummary, Library, LibraryInfo, NewDoc, RevInfo, RevKind, Renamed, Revision, Saved, SideFiles, FILE, FOLDER, FORMAT};

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_three_numbers() {
        assert_eq!(super::ENGINE_VERSION.split('.').count(), 3);
        assert_eq!(super::ABI_VERSION, 1);
        assert_eq!(super::WINDOW_API_VERSION, 1);
    }
}
