//! 통째 짓기(측정기 · 새 기기) — 일꾼 스레드가 해석 · 그리기 · 뽑기 · 조각내기를 하고, 쓰는 스레드 하나가 본 표를,
//! 조각 색인 넷은 저마다의 스레드가 따로 된 임시 파일에 나란히 짓는다. 다 지으면 조각 색인을 본 파일로 옮기고 `index.db`와 바꿔 끼운다.
//! 끊기면 임시 파일만 남고(다음 열기에서 지운다) 옛 색인은 그대로다.
use crate::engine::{prepare_doc, write, Engine};
use crate::pipeline::ordered;
use crate::search::{shard_of, shard_schema, SHARDS, SHARD_TABLES};
use crate::stats::count_edits;
use crate::store::{self, set_meta, Index, BUILDING, FILE};
use history_library::{Doc, Error, Library, Namespace, Reader, Result};
use history_namumark::{ObjectRef, Templates};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::Mutex;

/// 쓰는 쪽 트랜잭션을 이만큼마다 끊는다.
const COMMIT_EVERY: u64 = 20_000;
/// 짓는 동안 조각 색인 하나의 FTS5 메모리 해시(바이트).
const SHARD_HASH: i64 = 64 << 20;

/// 통째 짓기의 틀 원문 — 틀 이름공간은 미리 메모리에 읽고, 다른 이름공간을 부르는 드문 경우만 서재에서 읽는다.
struct RebuildTemplates {
    templates: HashMap<String, String>,
    other: Mutex<Reader>,
    error: Mutex<Option<Error>>,
}

impl Templates for RebuildTemplates {
    fn source(&self, target: &ObjectRef) -> Option<String> {
        let ns = Namespace::ALL.into_iter().find(|n| n.prefix() == target.ns)?;
        let key = history_library::normalize_title(&target.key).ok()?;
        if ns == Namespace::Template {
            return self.templates.get(&key).cloned();
        }
        let reader = self.other.lock().unwrap_or_else(|e| e.into_inner());
        let found = reader.first_by_title(ns, &key).and_then(|id| match id {
            Some(id) => reader.get(id),
            None => Ok(None),
        });
        match found {
            Ok(d) => d.filter(|d| !d.deleted).map(|d| d.body),
            Err(e) => {
                self.error.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(e);
                None
            }
        }
    }
}

/// 문서 ID 차례로 읽는 원천 — 읽기 전용 연결을 갖고 일꾼 스레드 사이를 옮겨 다닌다.
struct Source {
    reader: Reader,
    ids: std::vec::IntoIter<history_library::DocId>,
}

impl Iterator for Source {
    type Item = Result<Doc>;
    fn next(&mut self) -> Option<Result<Doc>> {
        loop {
            let id = self.ids.next()?;
            match self.reader.get(id) {
                Ok(Some(d)) => return Some(Ok(d)),
                Ok(None) => continue,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

fn shard_path(folder: &Path, k: usize) -> PathBuf {
    folder.join(format!("{BUILDING}-{}", SHARD_TABLES[k]))
}

/// 남은 임시 파일(짓다 끊긴 것)을 지운다.
pub(crate) fn remove_leftovers(folder: &Path) -> Result<()> {
    store::remove_db(&folder.join(BUILDING))?;
    for k in 0..SHARDS {
        store::remove_db(&shard_path(folder, k))?;
    }
    Ok(())
}

/// 조각 색인 하나를 짓는다 — 받은 (문서 번호, 조각)을 넣고, 끝에 설정을 보통으로 되돌리고 optimize.
fn build_shard(path: &Path, k: usize, rx: Receiver<(i64, String)>) -> Result<()> {
    let db = Connection::open(path)?;
    db.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF; PRAGMA cache_size = -131072;")?;
    let t = SHARD_TABLES[k];
    db.execute_batch(&shard_schema(t))?;
    db.execute(&format!("INSERT INTO {t}({t}, rank) VALUES ('hashsize', ?1)"), [SHARD_HASH])?;
    db.execute(&format!("INSERT INTO {t}({t}, rank) VALUES ('automerge', 0)"), [])?;
    db.execute_batch("BEGIN")?;
    let mut n = 0u64;
    for (rowid, tokens) in rx {
        db.prepare_cached(&format!("INSERT INTO {t}(rowid, t) VALUES (?1, ?2)"))?.execute(rusqlite::params![rowid, tokens])?;
        n += 1;
        if n % 50_000 == 0 {
            db.execute_batch("COMMIT; BEGIN")?;
        }
    }
    db.execute_batch("COMMIT")?;
    // 옮겨 간 뒤 하나씩 저장할 때를 위해 설정을 FTS5 기본값으로 되돌린다(설정도 함께 옮겨진다).
    db.execute(&format!("INSERT INTO {t}({t}, rank) VALUES ('automerge', 4)"), [])?;
    db.execute(&format!("INSERT INTO {t}({t}, rank) VALUES ('hashsize', 1048576)"), [])?;
    db.execute(&format!("INSERT INTO {t}({t}) VALUES ('optimize')"), [])?;
    Ok(())
}

/// 조각 색인 파일의 FTS5 그림자 표를 본 파일의 같은 표로 옮긴다(같은 선택으로 만든 빈 표 위에).
fn copy_shard(db: &Connection, path: &Path, k: usize) -> Result<()> {
    let t = SHARD_TABLES[k];
    db.execute("ATTACH DATABASE ?1 AS shard", [path.to_string_lossy()])?;
    let shadows: Vec<String> = {
        let mut st = db.prepare("SELECT name FROM shard.sqlite_master WHERE type = 'table' AND name LIKE ?1 ESCAPE '\\'")?;
        let rows = st.query_map([format!("{t}\\_%")], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let tx = db.unchecked_transaction()?;
    for s in &shadows {
        tx.execute_batch(&format!("DELETE FROM main.\"{s}\"; INSERT INTO main.\"{s}\" SELECT * FROM shard.\"{s}\";"))?;
    }
    tx.commit()?;
    db.execute_batch("DETACH DATABASE shard")?;
    Ok(())
}

impl Engine {
    /// 색인을 통째로 다시 짓는다. `workers`는 해석 · 그리기 일꾼 수(쓰는 스레드 다섯은 따로).
    pub fn rebuild_index(&mut self, workers: usize) -> Result<()> {
        let folder = self.lib.info().folder.clone();
        let library = self.lib.info().library;
        remove_leftovers(&folder)?;
        let building = folder.join(BUILDING);
        let last = self.lib.last_seq()?;
        let ids = self.lib.doc_ids()?;
        let hlcs = self.lib.revision_times(last)?;
        let mut templates = HashMap::new();
        for t in self.lib.live_docs(Namespace::Template)? {
            if !templates.contains_key(&t.title) {
                if let Some(d) = self.lib.get(t.id)? {
                    templates.insert(t.title, d.body);
                }
            }
        }
        let templates = RebuildTemplates { templates, other: Mutex::new(self.lib.reader()?), error: Mutex::new(None) };
        let fail_after = self.fail_rebuild_after;
        build(&self.lib, &building, &folder, library, ids, &hlcs, last, &templates, workers, fail_after)?;
        if let Some(e) = templates.error.into_inner().unwrap_or_else(|e| e.into_inner()) {
            return Err(e);
        }
        // 바꿔 끼운다 — Windows에서는 열린 파일을 지우거나 덮을 수 없어 지금 색인 연결을 먼저 닫는다.
        self.tree = None;
        drop(std::mem::replace(&mut self.idx, Index::placeholder()?));
        store::remove_db(&folder.join(FILE))?;
        std::fs::rename(&building, folder.join(FILE))?;
        remove_leftovers(&folder)?;
        self.idx = Index::open(&folder, library, &crate::extra_schema())?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn build(
    lib: &Library,
    building: &Path,
    folder: &Path,
    library: history_library::LibraryId,
    ids: Vec<history_library::DocId>,
    hlcs: &[history_library::Hlc],
    last: i64,
    templates: &RebuildTemplates,
    workers: usize,
    fail_after: Option<u64>,
) -> Result<()> {
    let db = Connection::open(building)?;
    db.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF; PRAGMA cache_size = -262144;")?;
    store::create(&db, library, &crate::extra_schema())?;
    std::thread::scope(|s| -> Result<()> {
        let mut senders = Vec::with_capacity(SHARDS);
        let mut shards = Vec::with_capacity(SHARDS);
        for k in 0..SHARDS {
            let (tx, rx) = sync_channel::<(i64, String)>(4_096);
            let path = shard_path(folder, k);
            shards.push(s.spawn(move || build_shard(&path, k, rx)));
            senders.push(tx);
        }
        db.execute_batch("BEGIN")?;
        let mut n = 0u64;
        let source = Source { reader: lib.reader()?, ids: ids.into_iter() };
        let run = ordered(
            source,
            workers,
            |d: Result<Doc>| d.and_then(|d| prepare_doc(d, templates)),
            |p: Result<crate::engine::Prepared>| -> Result<()> {
                let p = p?;
                let rowid = write(&db, &p, false, false)?;
                if !p.deleted && !p.tokens.is_empty() && senders[shard_of(rowid)].send((rowid, p.tokens)).is_err() {
                    return Err(Error::Corrupt("조각 색인 짓기가 멈췄다".into()));
                }
                n += 1;
                if fail_after == Some(n) {
                    return Err(Error::Injected);
                }
                if n % COMMIT_EVERY == 0 {
                    db.execute_batch("COMMIT; BEGIN")?;
                }
                Ok(())
            },
        );
        drop(senders);
        let mut shard_result = Ok(());
        for h in shards {
            let r = h.join().unwrap_or_else(|_| Err(Error::Corrupt("조각 색인 스레드가 멈췄다".into())));
            if shard_result.is_ok() {
                shard_result = r;
            }
        }
        run?;
        shard_result?;
        count_edits(&db, hlcs)?;
        set_meta(&db, "seq", last)?;
        set_meta(&db, "stats_dirty", 1)?;
        db.execute_batch("COMMIT")?;
        Ok(())
    })?;
    for k in 0..SHARDS {
        copy_shard(&db, &shard_path(folder, k), k)?;
    }
    Ok(())
}
