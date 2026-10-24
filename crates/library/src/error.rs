use crate::ids::{DocId, LibraryId, RevId};
use std::fmt;
use std::path::PathBuf;

/// 서재 엔진의 오류. `code()`는 다리(JSON)로 나가는 짧은 이름이다(spec/engine-api.md).
#[derive(Debug)]
pub enum Error {
    /// 제목으로 쓸 수 없음.
    BadTitle(String),
    /// 그런 문서 · 판이 없음.
    NotFound(String),
    /// 지운 문서에는 할 수 없는 일.
    Deleted(DocId),
    /// 고치기 시작한 판(base)이 지금 머리가 아님 — 그 사이 다른 저장이 있었다. 덮어쓰지 않았다.
    StaleBase { head: RevId },
    /// 서재 자리에 다른 파일이 있음(옛 앱 DB · 깨진 파일). 건드리지 않았다.
    NotALibrary(PathBuf),
    /// 이 엔진보다 새 형식의 서재. 건드리지 않았다.
    NewerFormat { found: i64, supported: i64 },
    /// 저장소 안의 값이 약속과 다름.
    Corrupt(String),
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    /// 다른 서재의 묶음 — 들지 않았다(spec/drive-format.md 4절).
    WrongLibrary(LibraryId),
    /// 묶음을 풀 수 없거나 모양이 틀림.
    BadPack(String),
    /// 견주어 합칠 판이 그 문서의 다른 내용 잎이 아님.
    NotALeaf(RevId),
    /// 시험용 실패 주입(`docs.rs` 단위 검사). 보통 실행에서는 생기지 않는다.
    Injected,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Error::BadTitle(_) => "bad_title",
            Error::NotFound(_) => "not_found",
            Error::Deleted(_) => "deleted",
            Error::StaleBase { .. } => "stale_base",
            Error::NotALibrary(_) => "not_a_library",
            Error::NewerFormat { .. } => "newer_format",
            Error::Corrupt(_) => "corrupt",
            Error::Sqlite(_) => "sqlite",
            Error::Io(_) => "io",
            Error::WrongLibrary(_) => "wrong_library",
            Error::BadPack(_) => "bad_pack",
            Error::NotALeaf(_) => "not_a_leaf",
            Error::Injected => "injected",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadTitle(why) => write!(f, "제목으로 쓸 수 없음: {why}"),
            Error::NotFound(what) => write!(f, "없음: {what}"),
            Error::Deleted(doc) => write!(f, "지운 문서: {doc}"),
            Error::StaleBase { head } => write!(f, "그 사이 다른 판이 저장됨(지금 머리 {head})"),
            Error::NotALibrary(path) => write!(f, "서재 파일이 아님 — 건드리지 않음: {}", path.display()),
            Error::NewerFormat { found, supported } => {
                write!(f, "더 새 형식의 서재(형식 {found}, 이 엔진은 {supported}까지) — 건드리지 않음")
            }
            Error::Corrupt(what) => write!(f, "저장소 값이 이상함: {what}"),
            Error::Sqlite(e) => write!(f, "SQLite: {e}"),
            Error::Io(e) => write!(f, "파일: {e}"),
            Error::WrongLibrary(found) => write!(f, "다른 서재의 묶음(서재 {found}) — 들지 않음"),
            Error::BadPack(why) => write!(f, "묶음을 읽을 수 없음: {why}"),
            Error::NotALeaf(rev) => write!(f, "합칠 수 있는 다른 판이 아님: {rev}"),
            Error::Injected => write!(f, "시험용 실패 주입"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Sqlite(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
