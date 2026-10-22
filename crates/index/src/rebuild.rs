//! 통째 짓기(측정기 · 새 기기) — 일꾼 스레드가 해석 · 그리기 · 뽑기 · 조각내기를 하고, 쓰는 스레드 하나가 본 표를,
//! 조각 색인 넷은 저마다의 스레드가 따로 된 임시 파일에 나란히 짓는다. 다 지으면 조각 색인을 본 파일로 옮기고 `index.db`와 바꿔 끼운다.
//! 끊기면 임시 파일만 남고(다음 열기에서 지운다) 옛 색인은 그대로다.
use crate::engine::{prepare_doc, Engine, Prepared};
use crate::pipeline::ordered;
use crate::search::{shard_of, shard_schema, SHARDS, SHARD_TABLES};
use crate::stats::{count_edits, decades};
use crate::store::{self, set_meta, Index, BUILDING, FILE};
use history_library::{Doc, Error, Library, Namespace, Reader, Result};
use history_namumark::{ObjectRef, Templates};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

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

thread_local! {
    /// 일꾼마다 서재 읽기 연결 하나 — 원천이 잠금 하나 뒤에서 혼자 읽으면 일꾼이 굶는다(100만 편에서 20분).
    static READER: std::cell::RefCell<Option<Reader>> = const { std::cell::RefCell::new(None) };
}

fn read_doc(library_db: &Path, id: history_library::DocId) -> Result<Option<Doc>> {
    READER.with(|r| {
        let mut r = r.borrow_mut();
        if r.is_none() {
            *r = Some(Reader::open(library_db)?);
        }
        r.as_ref().expect("읽기 연결").get(id)
    })
}

/// 통째 짓기의 쓰기 — 빈 색인에 넣기만 한다(이전 행 · 셈 · 있는지 찾기 없음). 셈은 다 넣은 뒤 `finish`가 한 번에 한다.
fn write_fresh(db: &Connection, rowid: i64, p: &Prepared, by_decade: &mut HashMap<i32, i64>) -> Result<()> {
    use rusqlite::params;
    let (rns, rkey) = match &p.redirect {
        Some((ns, key)) => (Some(*ns), Some(key.as_str())),
        None => (None, None),
    };
    db.prepare_cached("INSERT INTO item(rowid, doc, rev, ns, title, home, rns, rkey, created, updated, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)")?
        .execute(params![rowid, p.doc, p.rev, p.ns, p.title, p.home, rns, rkey, p.created as i64, p.updated as i64, p.deleted])?;
    db.prepare_cached("INSERT INTO model(rowid, z) VALUES (?1, ?2)")?.execute(params![rowid, p.model])?;
    db.prepare_cached("INSERT INTO plain(rowid, z) VALUES (?1, ?2)")?.execute(params![rowid, p.plain])?;
    let ex = &p.ex;
    for (kind, ns, key) in &ex.links {
        db.prepare_cached("INSERT OR IGNORE INTO link(src, kind, ns, key) VALUES (?1, ?2, ?3, ?4)")?.execute(params![rowid, *kind as i64, ns, key])?;
    }
    for (i, c) in ex.categories.iter().enumerate() {
        db.prepare_cached("INSERT OR IGNORE INTO member(doc, category, first) VALUES (?1, ?2, ?3)")?.execute(params![rowid, c, i == 0])?;
    }
    for s in &ex.sources {
        db.prepare_cached("INSERT INTO source(doc, name) VALUES (?1, ?2)")?.execute(params![rowid, s])?;
    }
    for e in &ex.entities {
        db.prepare_cached("INSERT INTO entity(doc, entity) VALUES (?1, ?2)")?.execute(params![rowid, e])?;
    }
    for t in &ex.templates {
        db.prepare_cached("INSERT INTO uses(doc, template) VALUES (?1, ?2)")?.execute(params![rowid, t])?;
    }
    if let Some((y0, y1)) = ex.years {
        db.prepare_cached("INSERT INTO years(doc, y0, y1) VALUES (?1, ?2, ?3)")?.execute(params![rowid, y0, y1])?;
        for d in decades(y0, y1) {
            *by_decade.entry(d).or_default() += 1;
        }
    }
    Ok(())
}

/// 다 넣은 뒤 — 보조 색인을 만들고, 하나씩 저장할 때 고쳐 두는 셈을 같은 규칙으로 한 번에 센다.
fn finish(db: &Connection, deferred: &[String], by_decade: &HashMap<i32, i64>) -> Result<()> {
    for sql in deferred {
        db.execute_batch(sql)?;
    }
    let ns_code = history_library::Namespace::ALL.iter().map(|n| format!("WHEN '{}' THEN {}", n.prefix(), *n as i64)).collect::<Vec<_>>().join(" ");
    db.execute_batch(&format!(
        "INSERT INTO target(ns, key, refs, present) SELECT ns, key, count(*), 0 FROM link WHERE kind = 0 GROUP BY ns, key;
         UPDATE target SET present = EXISTS(SELECT 1 FROM item INDEXED BY item_title WHERE item.ns = (CASE target.ns {ns_code} END) AND item.title = target.key AND item.deleted = 0);
         INSERT INTO folder(name, direct) SELECT COALESCE(home, ''), count(*) FROM item WHERE ns = 0 AND deleted = 0 GROUP BY COALESCE(home, '');
         INSERT INTO ns_count(ns, deleted, n) SELECT ns, deleted, count(*) FROM item GROUP BY ns, deleted;
         INSERT INTO stat_new(hour, n) SELECT (created >> 16) / 3600000, count(*) FROM item WHERE ns = 0 AND deleted = 0 GROUP BY 1;
         INSERT INTO source_count(name, refs) SELECT name, count(*) FROM source GROUP BY name;"
    ))?;
    for (d, n) in by_decade {
        db.prepare_cached("INSERT INTO decade(decade, docs) VALUES (?1, ?2)")?.execute(rusqlite::params![d, n])?;
    }
    Ok(())
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
/// 조각 색인 하나를 짓는다 — 쓰는 데 든 시간(ms, 받기를 기다린 시간 뺌).
fn build_shard(path: &Path, k: usize, rx: Receiver<(i64, String)>) -> Result<u64> {
    let mut busy = std::time::Duration::ZERO;
    let db = Connection::open(path)?;
    db.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF; PRAGMA cache_size = -131072;")?;
    let t = SHARD_TABLES[k];
    db.execute_batch(&shard_schema(t))?;
    db.execute(&format!("INSERT INTO {t}({t}, rank) VALUES ('hashsize', ?1)"), [SHARD_HASH])?;
    db.execute(&format!("INSERT INTO {t}({t}, rank) VALUES ('automerge', 0)"), [])?;
    db.execute_batch("BEGIN")?;
    let mut n = 0u64;
    for (rowid, tokens) in rx {
        let at = Instant::now();
        db.prepare_cached(&format!("INSERT INTO {t}(rowid, t) VALUES (?1, ?2)"))?.execute(rusqlite::params![rowid, tokens])?;
        n += 1;
        if n % 50_000 == 0 {
            db.execute_batch("COMMIT; BEGIN")?;
        }
        busy += at.elapsed();
    }
    let at = Instant::now();
    db.execute_batch("COMMIT")?;
    // 옮겨 간 뒤 하나씩 저장할 때를 위해 설정을 FTS5 기본값으로 되돌린다(설정도 함께 옮겨진다).
    db.execute(&format!("INSERT INTO {t}({t}, rank) VALUES ('automerge', 4)"), [])?;
    db.execute(&format!("INSERT INTO {t}({t}, rank) VALUES ('hashsize', 1048576)"), [])?;
    db.execute(&format!("INSERT INTO {t}({t}) VALUES ('optimize')"), [])?;
    busy += at.elapsed();
    Ok(busy.as_millis() as u64)
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

/// 통째 짓기가 단계마다 쓴 시간(ms).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RebuildReport {
    pub docs: u64,
    /// 문서 ID · 판 시각 · 틀 원문 읽기.
    pub prepare_ms: u64,
    /// 해석 · 그리기 · 본 표 쓰기 · 조각 색인 넷 짓기(나란히).
    pub build_ms: u64,
    /// 조각 색인을 본 파일로 옮기기.
    pub copy_ms: u64,
    /// 바꿔 끼우고 다시 열기.
    pub swap_ms: u64,
    /// 일꾼들이 해석 · 그리기 · 뽑기에 쓴 시간의 합.
    pub worker_ms: u64,
    /// 쓰는 스레드가 본 표에 쓴 시간.
    pub writer_ms: u64,
    /// 조각 색인 넷 가운데 가장 오래 쓴 것.
    pub shard_ms: u64,
    /// 그 가운데 다 넣은 뒤 보조 색인 · 셈.
    pub finish_ms: u64,
    /// 그 가운데 ANALYZE.
    pub analyze_ms: u64,
}

impl Engine {
    /// 색인을 통째로 다시 짓는다. `workers`는 해석 · 그리기 일꾼 수(쓰는 스레드 다섯은 따로).
    pub fn rebuild_index(&mut self, workers: usize) -> Result<RebuildReport> {
        let started = std::time::Instant::now();
        let mut report = RebuildReport::default();
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
        report.docs = ids.len() as u64;
        report.prepare_ms = started.elapsed().as_millis() as u64;
        build(&self.lib, &building, &folder, library, ids, &hlcs, last, &templates, workers, fail_after, &mut report)?;
        let swap = std::time::Instant::now();
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
        report.swap_ms = swap.elapsed().as_millis() as u64;
        Ok(report)
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
    report: &mut RebuildReport,
) -> Result<()> {
    let started = Instant::now();
    let worker_ns = AtomicU64::new(0);
    let mut writer_ns = 0u64;
    let db = Connection::open(building)?;
    db.execute_batch("PRAGMA page_size = 8192; PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF; PRAGMA cache_size = -262144;")?;
    let deferred = store::create(&db, library, &crate::extra_schema(), true)?;
    let mut by_decade: HashMap<i32, i64> = HashMap::new();
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
        let library_db = lib.info().folder.join(history_library::FILE);
        let run = ordered(
            ids.into_iter(),
            workers,
            |id| {
                let t = Instant::now();
                let p = read_doc(&library_db, id).and_then(|d| d.map(|d| prepare_doc(d, templates)).transpose());
                worker_ns.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
                p
            },
            |p: Result<Option<Prepared>>| -> Result<()> {
                let Some(p) = p? else { return Ok(()) };
                let t = Instant::now();
                let rowid = n as i64 + 1;
                write_fresh(&db, rowid, &p, &mut by_decade)?;
                writer_ns += t.elapsed().as_nanos() as u64;
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
            match h.join().unwrap_or_else(|_| Err(Error::Corrupt("조각 색인 스레드가 멈췄다".into()))) {
                Ok(ms) => report.shard_ms = report.shard_ms.max(ms),
                Err(e) => {
                    if shard_result.is_ok() {
                        shard_result = Err(e);
                    }
                }
            }
        }
        run?;
        shard_result?;
        let t = Instant::now();
        finish(&db, &deferred, &by_decade)?;
        report.finish_ms = t.elapsed().as_millis() as u64;
        count_edits(&db, hlcs)?;
        set_meta(&db, "seq", last)?;
        set_meta(&db, "stats_dirty", 1)?;
        db.execute_batch("COMMIT")?;
        Ok(())
    })?;
    report.build_ms = started.elapsed().as_millis() as u64;
    report.worker_ms = worker_ns.into_inner() / 1_000_000;
    report.writer_ms = writer_ns / 1_000_000;
    let copy = Instant::now();
    for k in 0..SHARDS {
        copy_shard(&db, &shard_path(folder, k), k)?;
    }
    report.copy_ms = copy.elapsed().as_millis() as u64;
    // 플래너가 색인을 고를 수 있게 대강의 통계를 남긴다(색인마다 400행만 본다).
    let analyze = Instant::now();
    db.execute_batch("PRAGMA analysis_limit = 400; ANALYZE;")?;
    report.analyze_ms = analyze.elapsed().as_millis() as u64;
    Ok(())
}
