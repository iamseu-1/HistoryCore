//! 서재 원본 저장소 `library.db`(설계 5-7). 문서 · 판 · 서재 정체를 갖는다. 판은 지우지 않고 쌓는다(설계 5-2).
use crate::error::{Error, Result};
use crate::hlc::{Clock, Hlc, WallClock};
use crate::ids::{DeviceId, DocId, LibraryId, RevId};
use crate::ns::Namespace;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{params, Connection, ErrorCode, OpenFlags, OptionalExtension};
use std::path::{Path, PathBuf};

/// 앱 자료 폴더 아래 엔진의 자리. 옛 앱은 자료 폴더 바로 아래에 `library.db` · `index.db` · `local.db`를 두었으므로
/// 새 엔진은 그 자리에 아무것도 쓰지 않고 이 하위 폴더에만 쓴다.
pub const FOLDER: &str = "HistoryCore";
pub const FILE: &str = "library.db";
/// 서재 형식 판(`PRAGMA user_version`). 모양이 바뀌면 올리고 옮기는 길을 함께 둔다.
pub const FORMAT: i64 = 1;
/// `PRAGMA application_id` — "HCLB". 이 값이 아닌 SQLite 파일은 서재로 열지 않는다.
const APPLICATION_ID: i64 = 0x4843_4C42;

pub(crate) const SCHEMA: &str = "
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE doc(
  id      BLOB PRIMARY KEY,
  ns      INTEGER NOT NULL,
  title   TEXT NOT NULL,
  head    BLOB NOT NULL,
  created INTEGER NOT NULL,
  updated INTEGER NOT NULL,
  deleted INTEGER NOT NULL
) WITHOUT ROWID;
CREATE INDEX doc_title ON doc(ns, title, created);
CREATE INDEX doc_ns_deleted ON doc(ns, deleted);
CREATE TABLE rev(
  seq     INTEGER PRIMARY KEY,
  id      BLOB NOT NULL UNIQUE,
  doc     BLOB NOT NULL,
  parent  BLOB,
  parent2 BLOB,
  device  BLOB NOT NULL,
  hlc     INTEGER NOT NULL,
  kind    INTEGER NOT NULL,
  ns      INTEGER NOT NULL,
  title   TEXT NOT NULL,
  message TEXT NOT NULL,
  len     INTEGER NOT NULL,
  body    BLOB NOT NULL
);
CREATE INDEX rev_doc ON rev(doc, hlc);
CREATE INDEX rev_hlc ON rev(hlc);
";

/// 열 때 시계를 잇는 마지막 판 시각.
pub(crate) const LAST_HLC_SQL: &str = "SELECT COALESCE(MAX(hlc), 0) FROM rev";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryInfo {
    pub library: LibraryId,
    pub device: DeviceId,
    pub format: i64,
    /// 엔진의 자리(`<앱 자료 폴더>/HistoryCore`).
    pub folder: PathBuf,
}

/// 판의 종류. 지우기는 본문이 빈 표시 판이다(설계 5-2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevKind {
    Content,
    Deleted,
}

impl RevKind {
    pub fn code(self) -> &'static str {
        match self {
            RevKind::Content => "content",
            RevKind::Deleted => "deleted",
        }
    }
}

impl ToSql for RevKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(match self {
            RevKind::Content => 0i64,
            RevKind::Deleted => 1i64,
        }))
    }
}

impl FromSql for RevKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_i64()? {
            0 => Ok(RevKind::Content),
            1 => Ok(RevKind::Deleted),
            v => Err(FromSqlError::OutOfRange(v)),
        }
    }
}

/// 저장 한 번의 결과. `changed`가 거짓이면 본문이 같아 판을 더하지 않았다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved {
    pub doc: DocId,
    pub rev: RevId,
    pub hlc: Hlc,
    pub changed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Renamed {
    pub rev: RevId,
    pub hlc: Hlc,
    /// 옛 제목에 남긴 넘겨주기 문서.
    pub redirect: Option<DocId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    pub id: DocId,
    pub ns: Namespace,
    pub title: String,
    pub head: RevId,
    pub created: Hlc,
    pub updated: Hlc,
    pub deleted: bool,
    /// 머리 판의 본문(지운 문서는 빈 글).
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocSummary {
    pub id: DocId,
    pub ns: Namespace,
    pub title: String,
    pub head: RevId,
    pub created: Hlc,
    pub updated: Hlc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevInfo {
    pub id: RevId,
    pub doc: DocId,
    pub parent: Option<RevId>,
    /// 합친 판의 둘째 앞 판(P4).
    pub parent2: Option<RevId>,
    pub device: DeviceId,
    pub hlc: Hlc,
    pub kind: RevKind,
    pub ns: Namespace,
    pub title: String,
    pub message: String,
    /// 본문의 UTF-8 바이트 수.
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub info: RevInfo,
    pub body: String,
}

/// 이름공간별 살아 있는 문서 수와 지운 문서 수.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub live: [u64; 5],
    pub deleted: u64,
}

impl Counts {
    pub fn total(&self) -> u64 {
        self.live.iter().sum()
    }
    pub fn of(&self, ns: Namespace) -> u64 {
        self.live[ns as usize]
    }
}

/// 열린 서재 하나. 한 스레드에서 쓴다(다리는 Mutex 안에 둔다).
pub struct Library {
    pub(crate) db: Connection,
    pub(crate) info: LibraryInfo,
    pub(crate) clock: Clock,
    /// 시험용 실패 주입 — 참이면 판을 넣은 뒤 문서 머리를 옮기기 전에 `Error::Injected`로 끊는다.
    pub(crate) fail_after_rev: bool,
}

impl Library {
    /// 앱 자료 폴더 `app_dir`의 `HistoryCore/library.db`를 연다(없으면 만든다). `app_dir` 바로 아래에는 쓰지 않는다.
    pub fn open(app_dir: &Path) -> Result<Library> {
        Self::open_with_clock(app_dir, Clock::system())
    }

    /// 벽시계를 바꿔 끼워 연다(시험 — 시계가 뒤로 가는 경우).
    pub fn open_with_clock(app_dir: &Path, wall: WallClock) -> Result<Library> {
        let folder = app_dir.join(FOLDER);
        std::fs::create_dir_all(&folder)?;
        let path = folder.join(FILE);
        if std::fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false) {
            // 먼저 읽기 전용으로만 본다 — 읽기 · 쓰기로 열면 SQLite가 남의 파일에 남은 WAL · 롤백 저널을
            // 되감거나 합쳐 넣어(파일을 바꿔) 버린다. 우리 서재로 확인된 뒤에만 읽기 · 쓰기로 연다.
            let probe = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
                .map_err(|e| identity_error(e, &path))?;
            check_identity(&probe, &path)?;
        }
        let db = Connection::open(&path)?;
        let tables = check_identity(&db, &path)?;
        db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;")?;
        if tables == 0 {
            create_schema(&db)?;
        }
        let info = LibraryInfo {
            library: meta_id(&db, "library", LibraryId::parse)?,
            device: meta_id(&db, "device", DeviceId::parse)?,
            format: FORMAT,
            folder,
        };
        let last: i64 = db.query_row(LAST_HLC_SQL, [], |r| r.get(0))?;
        Ok(Library { db, info, clock: Clock::new(Hlc(last as u64), wall), fail_after_rev: false })
    }

    pub fn info(&self) -> &LibraryInfo {
        &self.info
    }
}

/// 이 파일이 우리 서재인지 본다 — 아무것도 쓰지 않는다. 표 수를 돌려준다(0이면 빈 새 파일).
fn check_identity(db: &Connection, path: &Path) -> Result<i64> {
    let not_ours = || Error::NotALibrary(path.to_path_buf());
    let app_id: i64 = db.pragma_query_value(None, "application_id", |r| r.get(0)).map_err(|e| identity_error(e, path))?;
    let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0)).map_err(|e| identity_error(e, path))?;
    let tables: i64 = db.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0)).map_err(|e| identity_error(e, path))?;
    if tables == 0 && (app_id == 0 || app_id == APPLICATION_ID) {
        return Ok(0);
    }
    if app_id != APPLICATION_ID {
        return Err(not_ours());
    }
    if version > FORMAT {
        return Err(Error::NewerFormat { found: version, supported: FORMAT });
    }
    Ok(tables)
}

/// SQLite가 "DB 파일이 아님"이라 할 때만 남의 파일로 본다. 잠김 · 권한 · 입출력 오류는 그대로 올린다 —
/// 내 서재가 잠깐 잠겼을 뿐인데 "서재 파일이 아님"이라 하면 사용자가 진짜 서재를 지울 수 있다.
fn identity_error(e: rusqlite::Error, path: &Path) -> Error {
    match &e {
        rusqlite::Error::SqliteFailure(f, _) if f.code == ErrorCode::NotADatabase => Error::NotALibrary(path.to_path_buf()),
        _ => Error::Sqlite(e),
    }
}

fn create_schema(db: &Connection) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(&format!("PRAGMA application_id = {APPLICATION_ID}; PRAGMA user_version = {FORMAT};"))?;
    tx.execute_batch(SCHEMA)?;
    for (key, value) in [("library", LibraryId::new().to_string()), ("device", DeviceId::new().to_string()), ("format", FORMAT.to_string())] {
        tx.execute("INSERT INTO meta(key, value) VALUES (?1, ?2)", params![key, value])?;
    }
    tx.commit()?;
    Ok(())
}

fn meta_id<T>(db: &Connection, key: &str, parse: fn(&str) -> Option<T>) -> Result<T> {
    let value: String = db
        .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| r.get(0))
        .optional()?
        .ok_or_else(|| Error::Corrupt(format!("meta.{key} 없음")))?;
    parse(&value).ok_or_else(|| Error::Corrupt(format!("meta.{key} = {value}")))
}
