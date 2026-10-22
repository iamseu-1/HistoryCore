//! 색인 파일 `HistoryCore/index.db`(설계 5-5 · 5-7) — 파생물이라 지워도 `library.db`에서 다시 짓는다.
//! 정체(application_id "HCIX")가 우리 것이면 깨졌거나 판이 달라도 그 파일만 다시 짓고, 남의 것이면 건드리지 않는다.
use history_library::{Error, LibraryId, Result};
use rusqlite::{Connection, ErrorCode, OpenFlags, OptionalExtension};
use std::path::{Path, PathBuf};

pub const FILE: &str = "index.db";
/// 통째 짓기가 쓰는 임시 파일. 다 지으면 `index.db`와 바꿔 끼운다.
pub const BUILDING: &str = "index.db.building";
/// 색인 형식 판. 바뀌면 옮기지 않고 다시 짓는다(파생물).
pub const FORMAT: i64 = 1;
/// "HCIX".
pub const APPLICATION_ID: i64 = 0x4843_4958;

pub(crate) const SCHEMA: &str = "
CREATE TABLE meta(key TEXT PRIMARY KEY, value) WITHOUT ROWID;
-- 문서 하나 = 번호 하나(fts · plain · model의 rowid). rev는 색인한 판.
CREATE TABLE item(
  rowid   INTEGER PRIMARY KEY,
  doc     BLOB NOT NULL UNIQUE,
  rev     BLOB NOT NULL,
  ns      INTEGER NOT NULL,
  title   TEXT NOT NULL,
  home    TEXT,
  rns     INTEGER,
  rkey    TEXT,
  created INTEGER NOT NULL,
  updated INTEGER NOT NULL,
  deleted INTEGER NOT NULL
);
CREATE INDEX item_updated ON item(updated) WHERE deleted = 0;
CREATE TABLE model(rowid INTEGER PRIMARY KEY, z BLOB NOT NULL);
CREATE TABLE plain(rowid INTEGER PRIMARY KEY, z BLOB NOT NULL);
-- 다시 그려야 하는 문서(바뀐 틀을 쓰는 문서 · 색인하다 실패한 문서).
CREATE TABLE stale(doc BLOB PRIMARY KEY) WITHOUT ROWID;
";

/// 열린 색인. 연결은 엔진이 갖는다.
pub(crate) struct Index {
    pub(crate) db: Connection,
    #[allow(dead_code)] // 통째 짓기(Task 7)가 바꿔 끼울 때 쓴다
    pub(crate) path: PathBuf,
}

impl Index {
    /// `folder`(= `<앱 자료 폴더>/HistoryCore`)의 index.db를 연다. 없거나, 우리 것인데 판이 다르거나 깨졌거나, 다른 서재의 색인이면
    /// 새로 만든다(그 파일만). 남의 SQLite면 `NotALibrary`로 건드리지 않는다. 남은 `.building`은 지운다.
    pub(crate) fn open(folder: &Path, library: LibraryId, extra_schema: &str) -> Result<Index> {
        let path = folder.join(FILE);
        crate::rebuild::remove_leftovers(folder)?;
        match probe(&path, library)? {
            Probe::Ours => {}
            Probe::Missing => {}
            Probe::Rebuild => remove_db(&path)?,
        }
        let db = Connection::open(&path)?;
        db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        let tables: i64 = db.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))?;
        if tables == 0 {
            create(&db, library, extra_schema)?;
        }
        Ok(Index { db, path })
    }

    /// 통째 짓기가 파일을 바꿔 끼우는 동안 잠깐 드는 빈 색인.
    pub(crate) fn placeholder() -> Result<Index> {
        Ok(Index { db: Connection::open_in_memory()?, path: PathBuf::new() })
    }

    /// 눈금 — 이 판 번호(`rev.seq`)까지의 판은 모두 색인에 들었다.
    pub(crate) fn seq(&self) -> Result<i64> {
        meta_i64(&self.db, "seq")
    }
}

pub(crate) fn meta_i64(db: &Connection, key: &str) -> Result<i64> {
    let v: Option<i64> = db.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()?;
    Ok(v.unwrap_or(0))
}

pub(crate) fn set_meta(db: &Connection, key: &str, value: i64) -> Result<()> {
    db.prepare_cached("INSERT INTO meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value")?.execute(rusqlite::params![key, value])?;
    Ok(())
}

/// 빈 파일에 스키마를 만든다(통째 짓기의 임시 파일도 같은 것을 쓴다).
pub(crate) fn create(db: &Connection, library: LibraryId, extra_schema: &str) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(&format!("PRAGMA application_id = {APPLICATION_ID}; PRAGMA user_version = {FORMAT};"))?;
    tx.execute_batch(SCHEMA)?;
    for t in crate::search::SHARD_TABLES {
        tx.execute_batch(&crate::search::shard_schema(t))?;
    }
    tx.execute_batch(extra_schema)?;
    tx.execute("INSERT INTO meta(key, value) VALUES ('library', ?1)", [library.to_string()])?;
    tx.execute("INSERT INTO meta(key, value) VALUES ('seq', 0)", [])?;
    tx.commit()?;
    Ok(())
}

enum Probe {
    Missing,
    Ours,
    Rebuild,
}

/// 읽기 전용으로만 보고 정한다 — 남의 파일이면 WAL 되감기조차 일으키지 않는다.
fn probe(path: &Path, library: LibraryId) -> Result<Probe> {
    if !std::fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(if path.exists() { Probe::Rebuild } else { Probe::Missing });
    }
    let db = match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX) {
        Ok(db) => db,
        Err(e) if not_a_database(&e) => return Ok(Probe::Rebuild),
        Err(e) => return Err(e.into()),
    };
    let app_id: i64 = match db.pragma_query_value(None, "application_id", |r| r.get(0)) {
        Ok(v) => v,
        Err(e) if not_a_database(&e) => return Ok(Probe::Rebuild),
        Err(e) => return Err(e.into()),
    };
    if app_id != APPLICATION_ID {
        let tables: i64 = db.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0)).unwrap_or(1);
        if app_id == 0 && tables == 0 {
            return Ok(Probe::Rebuild);
        }
        return Err(Error::NotALibrary(path.to_path_buf()));
    }
    let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let ours: Option<String> = db.query_row("SELECT value FROM meta WHERE key = 'library'", [], |r| r.get(0)).optional().unwrap_or(None);
    if version != FORMAT || ours.as_deref() != Some(&library.to_string()) {
        return Ok(Probe::Rebuild);
    }
    Ok(Probe::Ours)
}

fn not_a_database(e: &rusqlite::Error) -> bool {
    matches!(e, rusqlite::Error::SqliteFailure(f, _) if f.code == ErrorCode::NotADatabase)
}

/// 우리 색인 파일과 그 WAL · 공유 메모리 파일을 지운다.
pub(crate) fn remove_db(path: &Path) -> Result<()> {
    for suffix in ["", "-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{suffix}", path.display()));
        match std::fs::remove_file(&p) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
