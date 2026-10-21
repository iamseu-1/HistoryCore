//! P1의 기준 구현 — SQLite 하나에 본문 그대로 + FTS5 trigram. 서재 엔진이 넘어야 할 바닥 수치를 낸다.
use crate::{Aggregates, Library, Result};
use history_corpus::{GenDoc, Ns};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

const FILE: &str = "naive.db";
const PRAGMAS: &str = "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;";
const SCHEMA: &str = "
CREATE TABLE doc(id INTEGER PRIMARY KEY, ns INTEGER NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL, grp TEXT, updated INTEGER NOT NULL, UNIQUE(ns, title));
CREATE INDEX doc_updated ON doc(updated DESC);
CREATE TABLE cat_member(cat TEXT NOT NULL, doc INTEGER NOT NULL, PRIMARY KEY(cat, doc)) WITHOUT ROWID;
CREATE VIRTUAL TABLE fts USING fts5(body, content='doc', content_rowid='id', tokenize='trigram');
";

pub struct NaiveSqlite {
    dir: PathBuf,
    db: Connection,
    clock: i64,
}

fn connect(path: &Path) -> Result<Connection> {
    let db = Connection::open(path)?;
    db.execute_batch(PRAGMAS)?;
    Ok(db)
}

/// 본문 첫 줄의 [[분류:…]]들 — 첫 것이 집.
fn categories(body: &str) -> Vec<&str> {
    let first = body.lines().next().unwrap_or("");
    first.split("[[분류:").skip(1).filter_map(|p| p.split("]]").next()).collect()
}

fn ids(rows: impl Iterator<Item = rusqlite::Result<u32>>) -> Result<Vec<u32>> {
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

impl NaiveSqlite {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let db = connect(&dir.join(FILE))?;
        let clock = db
            .query_row("SELECT COALESCE(MAX(updated), 0) FROM doc", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(NaiveSqlite { dir: dir.to_path_buf(), db, clock })
    }

    /// 옛 파일을 모두 지우고 빈 표로 다시 연다(끊긴 측정이 남긴 것 포함).
    fn reset(&mut self) -> Result<()> {
        self.db = Connection::open_in_memory()?; // 옛 파일을 놓는다
        for f in [FILE, "naive.db-wal", "naive.db-shm"] {
            let p = self.dir.join(f);
            if p.exists() {
                std::fs::remove_file(p)?;
            }
        }
        let db = connect(&self.dir.join(FILE))?;
        db.execute_batch(SCHEMA)?;
        self.db = db;
        self.clock = 0;
        Ok(())
    }
}

impl Library for NaiveSqlite {
    fn name(&self) -> &'static str {
        "naive-sqlite"
    }

    fn load(&mut self, docs: &mut dyn Iterator<Item = GenDoc>) -> Result<()> {
        self.reset()?;
        let mut n = 0i64;
        self.db.execute_batch("BEGIN")?;
        {
            let mut ins = self.db.prepare("INSERT INTO doc(id, ns, title, body, grp, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
            let mut mem = self.db.prepare("INSERT OR IGNORE INTO cat_member(cat, doc) VALUES (?1, ?2)")?;
            for d in docs {
                n += 1;
                let ns = if d.ns == Ns::Category { 1 } else { 0 };
                ins.execute(params![d.id, ns, d.title, d.body, d.group, n])?;
                if d.ns == Ns::Doc {
                    for c in categories(&d.body) {
                        mem.execute(params![c, d.id])?;
                    }
                }
                if n % 20_000 == 0 {
                    self.db.execute_batch("COMMIT; BEGIN")?;
                }
            }
        }
        self.db.execute_batch("COMMIT")?;
        self.db.execute_batch("INSERT INTO fts(fts) VALUES('rebuild'); PRAGMA wal_checkpoint(TRUNCATE);")?;
        self.clock = n;
        Ok(())
    }

    fn find_title(&self, prefix: &str, limit: usize) -> Result<Vec<u32>> {
        let upper = format!("{prefix}\u{10FFFF}");
        let mut st = self.db.prepare_cached("SELECT id FROM doc WHERE ns = 0 AND title >= ?1 AND title < ?2 ORDER BY title LIMIT ?3")?;
        let rows = st.query_map(params![prefix, upper, limit as i64], |r| r.get::<_, u32>(0))?;
        ids(rows)
    }

    fn search_body(&self, query: &str, limit: usize) -> Result<Vec<u32>> {
        if query.chars().count() >= 3 {
            let phrase = format!("\"{}\"", query.replace('"', "\"\""));
            let mut st = self.db.prepare_cached("SELECT rowid FROM fts WHERE fts MATCH ?1 LIMIT ?2")?;
            let rows = st.query_map(params![phrase, limit as i64], |r| r.get::<_, u32>(0))?;
            ids(rows)
        } else {
            // trigram은 3글자 미만을 못 찾는다 — 기준 구현은 통째로 훑는다(엔진이 2글자 조각 색인으로 넘을 곳).
            let mut st = self.db.prepare_cached("SELECT id FROM doc WHERE body LIKE ?1 LIMIT ?2")?;
            let rows = st.query_map(params![format!("%{query}%"), limit as i64], |r| r.get::<_, u32>(0))?;
            ids(rows)
        }
    }

    fn open(&self, id: u32) -> Result<Option<String>> {
        let mut st = self.db.prepare_cached("SELECT body FROM doc WHERE id = ?1")?;
        let body = st.query_row([id], |r| r.get(0)).optional()?;
        Ok(body)
    }

    fn save(&mut self, id: u32, body: &str) -> Result<()> {
        let now = self.clock + 1;
        let tx = self.db.transaction()?;
        let old: Option<String> = tx.query_row("SELECT body FROM doc WHERE id = ?1", [id], |r| r.get(0)).optional()?;
        let Some(old) = old else {
            return Err(format!("없는 문서 {id}").into());
        };
        tx.execute("INSERT INTO fts(fts, rowid, body) VALUES('delete', ?1, ?2)", params![id, old])?;
        tx.execute("UPDATE doc SET body = ?2, updated = ?3 WHERE id = ?1", params![id, body, now])?;
        tx.execute("INSERT INTO fts(rowid, body) VALUES (?1, ?2)", params![id, body])?;
        tx.commit()?;
        self.clock = now;
        Ok(())
    }

    fn category_children(&self, category: &str) -> Result<Vec<u32>> {
        let mut st = self.db.prepare_cached("SELECT doc FROM cat_member WHERE cat = ?1")?;
        let rows = st.query_map([category], |r| r.get::<_, u32>(0))?;
        ids(rows)
    }

    fn home_aggregates(&self) -> Result<Aggregates> {
        let mut st = self.db.prepare_cached("SELECT COALESCE(grp, '분류 안 됨'), COUNT(*) FROM doc WHERE ns = 0 GROUP BY 1 ORDER BY 2 DESC")?;
        let groups = st
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut st = self.db.prepare_cached("SELECT id FROM doc WHERE ns = 0 ORDER BY updated DESC LIMIT 10")?;
        let recent = ids(st.query_map([], |r| r.get::<_, u32>(0))?)?;
        Ok(Aggregates { groups, recent })
    }

    fn reopen(&mut self) -> Result<()> {
        self.db = Connection::open_in_memory()?;
        self.db = connect(&self.dir.join(FILE))?;
        Ok(())
    }

    fn bytes_on_disk(&self) -> Result<u64> {
        let mut total = 0;
        for f in [FILE, "naive.db-wal", "naive.db-shm"] {
            if let Ok(m) = std::fs::metadata(self.dir.join(f)) {
                total += m.len();
            }
        }
        Ok(total)
    }
}
