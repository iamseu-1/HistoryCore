//! 엔진 = 서재(원본) + 색인(파생). 쓰는 일은 서재에 먼저 커밋하고, 같은 호출 안에서 그 문서를 색인한다.
//! 색인은 `rev.seq` 눈금과 `stale` 표로 어디까지 했는지 안다 — 끊긴 저장 · 엔진 없이 들어온 판 · 바뀐 틀은 `step`이 따라잡는다.
//! 열 때 통째로 다시 짓지 않는다(설계 5-5).
use crate::extract::{extract, Extracted, PER_DOC};
use crate::store::{meta_i64, set_meta, Index};
use crate::tree::{Children, Tree};
use history_library::{DocId, DocSummary, Error, Library, Namespace, RevId, Renamed, Result, Saved};
use history_namumark::{parse, plain_document, render, ObjectRef, Options, RenderOptions, Templates};
use rusqlite::{params, Connection, OptionalExtension};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

const ZSTD_LEVEL: i32 = 3;
/// `step`이 한 번에 읽는 판 수.
const STEP_BATCH: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndexStatus {
    /// 다시 색인해야 하는 문서 수.
    pub stale: u64,
    /// 색인에 든 문서 수(지운 것 포함).
    pub total: u64,
    /// 통째 짓는 중.
    pub building: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepReport {
    pub done: u64,
    pub stale: u64,
}

/// `doc.open`의 답.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    pub doc: DocSummary,
    pub deleted: bool,
    /// 그리기 모델 JSON 글 그대로(spec/namumark.md) — 다시 읽지 않는다(깊은 중첩도 그대로 넘긴다).
    pub model: String,
    pub redirected_from: Option<DocSummary>,
    /// 색인이 머리 판보다 낡아 이 호출에서 새로 그렸다.
    pub stale: bool,
}

pub struct Engine {
    pub lib: Library,
    pub(crate) idx: Index,
    unz: zstd::bulk::Decompressor<'static>,
    /// 분류 나무 — 처음 물을 때 지어 두고, 색인이 바뀌면 버린다.
    tree: Option<Tree>,
    /// 시험용 — 참이면 저장 뒤 색인 트랜잭션을 커밋하기 직전에 끊는다(서재 커밋은 된 채).
    #[doc(hidden)]
    pub fail_before_index_commit: bool,
    /// 시험용 — 참이면 틀 원문 읽기가 모두 실패한다.
    #[doc(hidden)]
    pub fail_template_reads: bool,
}

/// 한 문서를 색인할 것 — 읽기(서재 · 틀)와 셈은 여기까지, 쓰기는 `write`가 한다.
pub(crate) struct Prepared {
    pub(crate) doc: DocId,
    pub(crate) rev: RevId,
    pub(crate) ns: Namespace,
    pub(crate) title: String,
    pub(crate) created: u64,
    pub(crate) updated: u64,
    pub(crate) deleted: bool,
    pub(crate) home: Option<String>,
    pub(crate) redirect: Option<(Namespace, String)>,
    pub(crate) model: Vec<u8>,
    pub(crate) plain: Vec<u8>,
    pub(crate) tokens: String,
    pub(crate) ex: Extracted,
}

/// 서재에서 틀 원문을 읽는다. 읽기 오류는 붙잡아 두었다가 그리기가 끝난 뒤 오류로 낸다 — `missing`으로 굳지 않도록(P2-2 미룬 것).
pub struct LibraryTemplates<'a> {
    lib: &'a Library,
    error: RefCell<Option<Error>>,
    fail: bool,
}

impl<'a> LibraryTemplates<'a> {
    pub fn new(lib: &'a Library) -> Self {
        LibraryTemplates { lib, error: RefCell::new(None), fail: false }
    }

    /// 시험용 — 모든 읽기가 실패한다.
    #[doc(hidden)]
    pub fn failing(lib: &'a Library) -> Self {
        LibraryTemplates { lib, error: RefCell::new(None), fail: true }
    }

    /// 그리는 동안 난 첫 읽기 오류.
    pub fn take_error(&self) -> Option<Error> {
        self.error.borrow_mut().take()
    }

    fn read(&self, target: &ObjectRef) -> Result<Option<String>> {
        if self.fail {
            return Err(Error::Injected);
        }
        let Some(ns) = Namespace::ALL.into_iter().find(|n| n.prefix() == target.ns) else {
            return Ok(None);
        };
        let Some(first) = self.lib.by_title(ns, &target.key)?.into_iter().next() else {
            return Ok(None);
        };
        Ok(self.lib.get(first.id)?.filter(|d| !d.deleted).map(|d| d.body))
    }
}

impl Templates for LibraryTemplates<'_> {
    fn source(&self, target: &ObjectRef) -> Option<String> {
        match self.read(target) {
            Ok(s) => s,
            Err(e) => {
                self.error.borrow_mut().get_or_insert(e);
                None
            }
        }
    }
}

/// 링크 대상의 이름공간이 서재 것이면 (이름공간, 제목).
pub(crate) fn library_target(r: &ObjectRef) -> Option<(Namespace, String)> {
    Namespace::ALL.into_iter().find(|n| n.prefix() == r.ns).map(|n| (n, r.key.clone()))
}

/// 한 문서를 읽고 그린다. 서재에 없으면 None. 읽기 오류(서재 · 틀)는 Err — 부르는 쪽이 그 문서를 낡음으로 남긴다.
pub(crate) fn prepare(lib: &Library, doc: DocId, fail_templates: bool) -> Result<Option<Prepared>> {
    let Some(d) = lib.get(doc)? else {
        return Ok(None);
    };
    let base = |model: Vec<u8>, plain: Vec<u8>, tokens: String, home, redirect, ex| Prepared {
        doc,
        rev: d.head,
        ns: d.ns,
        title: d.title.clone(),
        created: d.created.0,
        updated: d.updated.0,
        deleted: d.deleted,
        home,
        redirect,
        model,
        plain,
        tokens,
        ex,
    };
    if d.deleted {
        let model = serde_json::to_vec(&render("", &RenderOptions::default(), &history_namumark::NoTemplates)).expect("모델 JSON");
        return Ok(Some(base(zip(&model)?, zip(b"")?, String::new(), None, None, Extracted::default())));
    }
    let templates = if fail_templates { LibraryTemplates::failing(lib) } else { LibraryTemplates::new(lib) };
    let options = RenderOptions { title: Some(d.title.clone()), template: d.ns == Namespace::Template };
    let model = render(&d.body, &options, &templates);
    if let Some(e) = templates.take_error() {
        return Err(e);
    }
    let tree = parse(&d.body, &Options { template: options.template, current: options.title.clone(), ..Options::default() });
    let plain = crate::search::stored_plain(&plain_document(&tree));
    let tokens = crate::search::index_tokens(&plain).join(" ");
    let home = tree.categories.first().cloned();
    let redirect = model.redirect.as_ref().and_then(library_target);
    let ex = extract(&model, &tree);
    let json = serde_json::to_vec(&model).expect("모델 JSON");
    Ok(Some(base(zip(&json)?, zip(plain.as_bytes())?, tokens, home, redirect, ex)))
}

pub(crate) fn zip(bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(zstd::bulk::compress(bytes, ZSTD_LEVEL)?)
}

/// 한 문서의 색인 행을 모두 새로 쓴다. 문서 번호(rowid)는 처음 색인할 때 정해지고 바뀌지 않는다.
/// `mark_users`면, 틀 문서일 때 그 틀(옛 제목 · 새 제목)을 쓰는 문서를 낡음으로 넣는다(통째 짓기는 모두 새로 그리므로 끈다).
pub(crate) fn write(db: &Connection, p: &Prepared, mark_users: bool) -> Result<i64> {
    let prev: Option<(i64, Namespace, String, Option<String>, bool)> = db
        .prepare_cached("SELECT rowid, ns, title, home, deleted FROM item WHERE doc = ?1")?
        .query_row(params![p.doc], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
        .optional()?;
    // 폴더 수: 살아 있는 일반 문서의 집이 바뀐 만큼.
    if let Some((_, Namespace::Doc, _, home, false)) = &prev {
        bump_folder(db, home.as_deref().unwrap_or(""), -1)?;
    }
    if p.ns == Namespace::Doc && !p.deleted {
        bump_folder(db, p.home.as_deref().unwrap_or(""), 1)?;
    }
    if mark_users {
        let mut paths = Vec::new();
        if p.ns == Namespace::Template {
            paths.push(p.title.clone());
        }
        if let Some((_, Namespace::Template, t, _, _)) = &prev {
            if *t != p.title {
                paths.push(t.clone());
            }
        }
        for t in paths {
            db.prepare_cached(
                "INSERT OR IGNORE INTO stale(doc) SELECT i.doc FROM uses u JOIN item i ON i.rowid = u.doc WHERE u.template = ?1 AND i.doc != ?2",
            )?
            .execute(params![format!("{}:{t}", Namespace::Template.prefix()), p.doc])?;
        }
    }
    let old = prev.map(|(rowid, ..)| rowid);
    let (rns, rkey) = match &p.redirect {
        Some((ns, key)) => (Some(*ns), Some(key.as_str())),
        None => (None, None),
    };
    let rowid = match old {
        Some(rowid) => {
            db.prepare_cached(
                "UPDATE item SET rev = ?2, ns = ?3, title = ?4, home = ?5, rns = ?6, rkey = ?7, created = ?8, updated = ?9, deleted = ?10 WHERE rowid = ?1",
            )?
            .execute(params![rowid, p.rev, p.ns, p.title, p.home, rns, rkey, p.created as i64, p.updated as i64, p.deleted])?;
            db.prepare_cached(&format!("DELETE FROM {} WHERE rowid = ?1", fts_of(rowid)))?.execute([rowid])?;
            rowid
        }
        None => {
            db.prepare_cached("INSERT INTO item(doc, rev, ns, title, home, rns, rkey, created, updated, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)")?
                .execute(params![p.doc, p.rev, p.ns, p.title, p.home, rns, rkey, p.created as i64, p.updated as i64, p.deleted])?;
            db.last_insert_rowid()
        }
    };
    db.prepare_cached("INSERT OR REPLACE INTO model(rowid, z) VALUES (?1, ?2)")?.execute(params![rowid, p.model])?;
    db.prepare_cached("INSERT OR REPLACE INTO plain(rowid, z) VALUES (?1, ?2)")?.execute(params![rowid, p.plain])?;
    if !p.deleted && !p.tokens.is_empty() {
        db.prepare_cached(&format!("INSERT INTO {}(rowid, t) VALUES (?1, ?2)", fts_of(rowid)))?.execute(params![rowid, p.tokens])?;
    }
    write_extracted(db, rowid, old.is_some(), &p.ex)?;
    db.prepare_cached("DELETE FROM stale WHERE doc = ?1")?.execute(params![p.doc])?;
    Ok(rowid)
}

/// 뽑기 표를 이 문서 번호의 새 행으로 바꾼다. 본문 링크는 대상별 문서 수(`target`)도 고친다.
fn write_extracted(db: &Connection, rowid: i64, existed: bool, ex: &Extracted) -> Result<()> {
    if existed {
        let old: Vec<(String, String)> = {
            let mut st = db.prepare_cached("SELECT ns, key FROM link WHERE src = ?1 AND kind = 0")?;
            let rows = st.query_map([rowid], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for (ns, key) in old {
            db.prepare_cached("UPDATE target SET refs = refs - 1 WHERE ns = ?1 AND key = ?2")?.execute(params![ns, key])?;
            db.prepare_cached("DELETE FROM target WHERE ns = ?1 AND key = ?2 AND refs <= 0")?.execute(params![ns, key])?;
        }
        for (table, col) in PER_DOC {
            db.prepare_cached(&format!("DELETE FROM {table} WHERE {col} = ?1"))?.execute([rowid])?;
        }
    }
    for (kind, ns, key) in &ex.links {
        db.prepare_cached("INSERT INTO link(src, kind, ns, key) VALUES (?1, ?2, ?3, ?4)")?.execute(params![rowid, *kind as i64, ns, key])?;
        if *kind == history_namumark::LinkKind::Body {
            db.prepare_cached("INSERT INTO target(ns, key, refs) VALUES (?1, ?2, 1) ON CONFLICT(ns, key) DO UPDATE SET refs = refs + 1")?
                .execute(params![ns, key])?;
        }
    }
    for (i, c) in ex.categories.iter().enumerate() {
        db.prepare_cached("INSERT INTO member(doc, category, first) VALUES (?1, ?2, ?3)")?.execute(params![rowid, c, i == 0])?;
    }
    for (i, f) in ex.footnotes.iter().enumerate() {
        db.prepare_cached("INSERT INTO note(doc, n, text) VALUES (?1, ?2, ?3)")?.execute(params![rowid, i as i64 + 1, f])?;
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
    }
    Ok(())
}

fn bump_folder(db: &Connection, name: &str, by: i64) -> Result<()> {
    db.prepare_cached("INSERT INTO folder(name, direct) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET direct = direct + excluded.direct")?
        .execute(params![name, by])?;
    db.prepare_cached("DELETE FROM folder WHERE name = ?1 AND direct <= 0")?.execute([name])?;
    Ok(())
}

/// 이 문서 번호의 조각 색인 표.
pub(crate) fn fts_of(rowid: i64) -> &'static str {
    crate::search::SHARD_TABLES[crate::search::shard_of(rowid)]
}

fn mark_stale(db: &Connection, doc: DocId) -> Result<()> {
    db.prepare_cached("INSERT OR IGNORE INTO stale(doc) VALUES (?1)")?.execute(params![doc])?;
    Ok(())
}

impl Engine {
    /// 앱 자료 폴더의 `HistoryCore/`에서 서재와 색인을 연다. 색인이 없거나 다시 지어야 하면 빈 색인을 만들 뿐, 짓지는 않는다.
    pub fn open(app_dir: &Path) -> Result<Engine> {
        let lib = Library::open(app_dir)?;
        let idx = Index::open(&lib.info().folder, lib.info().library, crate::EXTRA_SCHEMA)?;
        Ok(Engine { lib, idx, unz: zstd::bulk::Decompressor::new()?, tree: None, fail_before_index_commit: false, fail_template_reads: false })
    }

    pub fn status(&self) -> Result<IndexStatus> {
        let w = self.idx.seq()?;
        let changed = self.lib.docs_changed_after(w)?;
        let mut extra = 0u64;
        let mut st = self.idx.db.prepare_cached("SELECT doc FROM stale")?;
        for doc in st.query_map([], |r| r.get::<_, DocId>(0))? {
            if !self.lib.changed_after(doc?, w)? {
                extra += 1;
            }
        }
        let total: i64 = self.idx.db.query_row("SELECT count(*) FROM item", [], |r| r.get(0))?;
        Ok(IndexStatus { stale: changed + extra, total: total as u64, building: false })
    }

    /// 낡은 문서를 `max` 동안 고친다 — 먼저 판 눈금 뒤의 판, 다음 `stale` 표.
    pub fn step(&mut self, max: Duration) -> Result<StepReport> {
        self.tree = None;
        let start = Instant::now();
        let mut done = 0u64;
        'revs: loop {
            let w = self.idx.seq()?;
            let revs = self.lib.revisions_after(w, STEP_BATCH)?;
            if revs.is_empty() {
                break;
            }
            let tx = self.idx.db.transaction()?;
            let (mut seen, mut new_w) = (HashSet::new(), w);
            for (seq, doc) in revs {
                if seen.insert(doc) {
                    index_one(&self.lib, &tx, doc, self.fail_template_reads)?;
                    done += 1;
                }
                new_w = seq;
                if start.elapsed() >= max {
                    set_meta(&tx, "seq", new_w)?;
                    tx.commit()?;
                    break 'revs;
                }
            }
            set_meta(&tx, "seq", new_w)?;
            tx.commit()?;
        }
        while start.elapsed() < max {
            let docs: Vec<DocId> = {
                let mut st = self.idx.db.prepare_cached("SELECT doc FROM stale LIMIT 64")?;
                let rows = st.query_map([], |r| r.get(0))?;
                rows.collect::<rusqlite::Result<_>>()?
            };
            if docs.is_empty() {
                break;
            }
            let tx = self.idx.db.transaction()?;
            let mut progressed = false;
            for doc in docs {
                match prepare(&self.lib, doc, self.fail_template_reads) {
                    Ok(Some(p)) => {
                        write(&tx, &p, true)?;
                        progressed = true;
                    }
                    Ok(None) => {
                        tx.execute("DELETE FROM stale WHERE doc = ?1", params![doc])?;
                        progressed = true;
                    }
                    Err(_) => {} // 아직 읽을 수 없다 — 남겨 두고 다음에
                }
                done += 1;
                if start.elapsed() >= max {
                    break;
                }
            }
            tx.commit()?;
            if !progressed {
                break;
            }
        }
        Ok(StepReport { done, stale: self.status()?.stale })
    }

    pub fn create(&mut self, ns: Namespace, title: &str, body: &str, message: &str) -> Result<Saved> {
        let before = self.lib.last_seq()?;
        let s = self.lib.create(ns, title, body, message)?;
        self.after_write(before, &[s.doc]);
        Ok(s)
    }

    pub fn save(&mut self, doc: DocId, base: RevId, body: &str, message: &str) -> Result<Saved> {
        let before = self.lib.last_seq()?;
        let s = self.lib.save(doc, base, body, message)?;
        if s.changed {
            self.after_write(before, &[doc]);
        }
        Ok(s)
    }

    pub fn rename(&mut self, doc: DocId, base: RevId, title: &str, keep_redirect: bool, message: &str) -> Result<Renamed> {
        let before = self.lib.last_seq()?;
        let r = self.lib.rename(doc, base, title, keep_redirect, message)?;
        let mut docs = vec![doc];
        docs.extend(r.redirect);
        self.after_write(before, &docs);
        Ok(r)
    }

    pub fn delete(&mut self, doc: DocId, base: RevId, message: &str) -> Result<Saved> {
        let before = self.lib.last_seq()?;
        let s = self.lib.delete(doc, base, message)?;
        if s.changed {
            self.after_write(before, &[doc]);
        }
        Ok(s)
    }

    /// 서재 커밋 뒤 그 문서들을 색인한다. 실패해도 서재 저장은 그대로다 — 문서는 낡음으로 남고 `step`이 다시 한다.
    fn after_write(&mut self, before: i64, docs: &[DocId]) {
        self.tree = None;
        let _ = self.index_now(before, docs);
    }

    fn index_now(&mut self, before: i64, docs: &[DocId]) -> Result<()> {
        let w = self.idx.seq()?;
        let last = self.lib.last_seq()?;
        let tx = self.idx.db.transaction()?;
        for &doc in docs {
            index_one(&self.lib, &tx, doc, self.fail_template_reads)?;
        }
        if w == before {
            // 이 저장 앞의 판은 모두 색인에 있었다 — 눈금을 이 저장 뒤로 옮긴다.
            set_meta(&tx, "seq", last)?;
        }
        if self.fail_before_index_commit {
            return Err(Error::Injected);
        }
        tx.commit()?;
        Ok(())
    }

    /// 머리 판의 그리기 모델. 색인이 낡았으면 그 자리에서 그려 저장한다. `follow`면 넘겨주기를 한 번 따라간다.
    pub fn open_doc(&mut self, doc: DocId, follow: bool) -> Result<Option<Opened>> {
        let Some((summary, deleted)) = self.lib.summary(doc)? else {
            return Ok(None);
        };
        let row: Option<(i64, RevId, Option<Namespace>, Option<String>)> = self
            .idx
            .db
            .prepare_cached("SELECT rowid, rev, rns, rkey FROM item WHERE doc = ?1")?
            .query_row(params![doc], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .optional()?;
        let (rowid, redirect, stale) = match row {
            Some((rowid, rev, rns, rkey)) if rev == summary.head => (rowid, rns.zip(rkey), false),
            _ => {
                let p = prepare(&self.lib, doc, self.fail_template_reads)?.ok_or_else(|| Error::NotFound(format!("문서 {doc}")))?;
                let redirect = p.redirect.clone();
                let tx = self.idx.db.transaction()?;
                let rowid = write(&tx, &p, true)?;
                tx.commit()?;
                self.tree = None;
                (rowid, redirect, true)
            }
        };
        if follow {
            if let Some((ns, key)) = &redirect {
                if let Some(target) = self.lib.by_title(*ns, key)?.into_iter().next() {
                    if target.id != doc {
                        if let Some(mut o) = self.open_doc(target.id, false)? {
                            o.redirected_from = Some(summary);
                            return Ok(Some(o));
                        }
                    }
                }
            }
        }
        let z: Vec<u8> = self.idx.db.prepare_cached("SELECT z FROM model WHERE rowid = ?1")?.query_row([rowid], |r| r.get(0))?;
        let model = self.unzip(&z)?;
        Ok(Some(Opened { doc: summary, deleted, model, redirected_from: None, stale }))
    }

    pub(crate) fn unzip(&mut self, z: &[u8]) -> Result<String> {
        let cap = zstd::zstd_safe::get_frame_content_size(z).ok().flatten().unwrap_or(64 << 20) as usize;
        let bytes = self.unz.decompress(z, cap.max(1))?;
        String::from_utf8(bytes).map_err(|_| Error::Corrupt("색인 글이 UTF-8이 아님".into()))
    }

    /// 분류 폴더 하나 — `None`이면 맨 위(「분류 안 됨」 수 포함), 빈 이름이면 「분류 안 됨」 문서.
    pub fn children(&mut self, category: Option<&str>, limit: usize, after: Option<&str>) -> Result<Children> {
        if self.tree.is_none() {
            self.tree = Some(Tree::load(&self.idx.db)?);
        }
        self.tree.as_ref().expect("나무").children(&self.idx.db, category, limit, after)
    }

    /// 분류 나무(대문 · 통계가 갈래를 셀 때).
    pub(crate) fn tree(&mut self) -> Result<&Tree> {
        if self.tree.is_none() {
            self.tree = Some(Tree::load(&self.idx.db)?);
        }
        Ok(self.tree.as_ref().expect("나무"))
    }

    /// 색인 연결(시험 · 측정이 표를 들여다볼 때).
    #[doc(hidden)]
    pub fn debug_index(&self) -> &Connection {
        &self.idx.db
    }

    /// 색인의 눈금(시험 · 측정).
    #[doc(hidden)]
    pub fn index_seq(&self) -> Result<i64> {
        meta_i64(&self.idx.db, "seq")
    }
}

/// 한 문서를 색인한다. 읽기 오류면 `stale`에 넣고 넘어간다(색인 쓰기 오류만 올린다).
fn index_one(lib: &Library, tx: &Connection, doc: DocId, fail_templates: bool) -> Result<()> {
    match prepare(lib, doc, fail_templates) {
        Ok(Some(p)) => {
            write(tx, &p, true)?;
        }
        Ok(None) => {}
        Err(_) => mark_stale(tx, doc)?,
    }
    Ok(())
}
