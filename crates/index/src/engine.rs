//! 엔진 = 서재(원본) + 색인(파생). 쓰는 일은 서재에 먼저 커밋하고, 같은 호출 안에서 그 문서를 색인한다.
//! 색인은 `rev.seq` 눈금과 `stale` 표로 어디까지 했는지 안다 — 끊긴 저장 · 엔진 없이 들어온 판 · 바뀐 틀은 `step`이 따라잡는다.
//! 열 때 통째로 다시 짓지 않는다(설계 5-5).
use crate::extract::{extract, Extracted, PER_DOC};
use crate::stats::{bump_new, bump_ns, bump_source, bump_years, count_edits, HomeOptions};
use crate::store::{meta_i64, set_meta, Index};
use crate::search::{fts_match, parse_query, snippet, verify, SHARDS, SHARD_TABLES};
use crate::tree::{summary_row, Children, Tree};
use history_library::{Applied, Doc, DocId, DocSummary, Error, Hlc, Library, LibraryId, Namespace, Reader, RevId, Renamed, Result, Saved, WallClock};
use history_namumark::{plain_document, render, render_document, Document, Model, ObjectRef, RenderOptions, Templates};
use rusqlite::{params, Connection, OptionalExtension};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

/// 모델 · 글자 압축 — 1이 3보다 두 배 가깝게 빠르고 크기는 조금 크다(통째 짓기에서 일꾼 몫이 크다).
const ZSTD_LEVEL: i32 = 1;
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

/// 본문 찾기 한 건.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub doc: DocSummary,
    /// 첫 맞은 곳 앞뒤 글자(표식 없음).
    pub snippet: String,
}

/// 없는 문서를 가리킨 링크(만들 문서).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    pub ns: Namespace,
    pub title: String,
    /// 가리킨 문서 수.
    pub refs: u64,
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
    pub(crate) tree: Option<Tree>,
    /// 시험용 — 참이면 저장 뒤 색인 트랜잭션을 커밋하기 직전에 끊는다(서재 커밋은 된 채).
    #[doc(hidden)]
    pub fail_before_index_commit: bool,
    /// 시험용 — 참이면 틀 원문 읽기가 모두 실패한다.
    #[doc(hidden)]
    pub fail_template_reads: bool,
    /// 시험용 — 통째 짓기를 이만큼 쓴 뒤 끊는다.
    #[doc(hidden)]
    pub fail_rebuild_after: Option<u64>,
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
    let templates = if fail_templates { LibraryTemplates::failing(lib) } else { LibraryTemplates::new(lib) };
    let p = prepare_doc(d, &templates)?;
    if let Some(e) = templates.take_error() {
        return Err(e);
    }
    Ok(Some(p))
}

/// 읽어 둔 문서를 그리고 뽑는다. 틀 읽기 오류는 `templates`가 붙잡는다.
pub(crate) fn prepare_doc(d: Doc, templates: &dyn Templates) -> Result<Prepared> {
    let doc = d.id;
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
        return Ok(base(zip(&model)?, zip(b"")?, String::new(), None, None, Extracted::default()));
    }
    let (model, tree) = draw(&d, templates);
    let plain = crate::search::stored_plain(&plain_document(&tree));
    let tokens = crate::search::index_text(&plain);
    let home = tree.categories.first().cloned();
    let redirect = model.redirect.as_ref().and_then(library_target);
    let ex = extract(&model, &tree);
    let json = serde_json::to_vec(&model).expect("모델 JSON");
    Ok(base(zip(&json)?, zip(plain.as_bytes())?, tokens, home, redirect, ex))
}

/// 살아 있는 문서의 머리 판을 그린다 — 색인(`prepare_doc`)과 서재 창구(`Window::doc`)가 같은 선택으로 그려야 같은 모델이 나온다.
pub(crate) fn draw(d: &Doc, templates: &dyn Templates) -> (Model, Document) {
    let options = RenderOptions { title: Some(d.title.clone()), template: d.ns == Namespace::Template };
    render_document(&d.body, &options, templates)
}

/// 색인한 판이 머리 판인지 견줄 서재 — 쓰는 엔진의 `Library`와 서재 창구의 `Reader`.
pub(crate) trait Heads {
    fn last_seq(&self) -> Result<i64>;
    fn head(&self, doc: DocId) -> Result<Option<RevId>>;
}

impl Heads for Library {
    fn last_seq(&self) -> Result<i64> {
        Library::last_seq(self)
    }
    fn head(&self, doc: DocId) -> Result<Option<RevId>> {
        Ok(self.summary(doc)?.map(|(s, _)| s.head))
    }
}

impl Heads for Reader {
    fn last_seq(&self) -> Result<i64> {
        Reader::last_seq(self)
    }
    fn head(&self, doc: DocId) -> Result<Option<RevId>> {
        Ok(self.summary(doc)?.map(|(s, _)| s.head))
    }
}

/// 이 연표 항목을 잇는 살아 있는 문서(연표 링크 · 연표 띠), 같은 문서는 한 번 · 제목 차례 — `timeline.docs`와 `window.docs_for_entity`가 함께 쓴다.
pub(crate) fn timeline_docs_in(db: &Connection, entity: &str) -> Result<Vec<DocSummary>> {
    let mut st = db.prepare_cached(
        "SELECT DISTINCT i.doc, i.ns, i.title, i.rev, i.created, i.updated FROM entity e JOIN item i ON i.rowid = e.doc
         WHERE e.entity = ?1 AND i.deleted = 0 ORDER BY i.title",
    )?;
    let rows = st.query_map([entity], summary_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// 있는 문서 ID — 같은 차례, 없으면 None. 제목은 정규화하고, 같은 제목이 둘이면 먼저 만든 것(`titles.exist` · `window.titles_exist`).
pub(crate) fn titles_exist_in(db: &Connection, titles: &[(Namespace, String)]) -> Result<Vec<Option<DocId>>> {
    let mut st = db.prepare_cached("SELECT doc FROM item INDEXED BY item_title WHERE ns = ?1 AND title = ?2 AND deleted = 0 ORDER BY ns, title, created LIMIT 1")?;
    let mut out = Vec::with_capacity(titles.len());
    for (ns, t) in titles {
        let found = match history_library::normalize_title(t) {
            Ok(t) => st.query_row(params![ns, t], |r| r.get(0)).optional()?,
            Err(_) => None,
        };
        out.push(found);
    }
    Ok(out)
}

/// 제목이 `prefix`로 시작하는 살아 있는 문서(제목 차례) — 덮는 제목 색인 한 범위(`search.titles` · `window.search`).
pub(crate) fn search_titles_in(db: &Connection, ns: Namespace, prefix: &str, limit: usize) -> Result<Vec<DocSummary>> {
    use unicode_normalization::UnicodeNormalization;
    let prefix: String = prefix.trim_start().nfc().collect();
    if prefix.is_empty() {
        return Ok(Vec::new());
    }
    let upper = format!("{prefix}\u{10FFFF}");
    let mut st = db.prepare_cached(
        "SELECT doc, ns, title, rev, created, updated FROM item INDEXED BY item_title
         WHERE ns = ?1 AND title >= ?2 AND title < ?3 AND deleted = 0 ORDER BY ns, title, created LIMIT ?4",
    )?;
    let rows = st.query_map(params![ns, prefix, upper, limit as i64], summary_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// 본문 찾기 — 조각 색인 넷을 문서 번호 차례로 합쳐 후보를 받고, 지운 · 낡은 문서는 건너뛰고, 확인을 지난 것만 `limit`까지
/// (`search.body` · `window.search`). 색인이 서재와 같으면(눈금이 마지막 판 · 낡음 없음) 후보마다 머리 판을 묻지 않는다.
pub(crate) fn search_body_in(db: &Connection, unz: &mut zstd::bulk::Decompressor<'static>, lib: &dyn Heads, query: &str, limit: usize) -> Result<Vec<Hit>> {
    let Some(q) = parse_query(query) else {
        return Ok(Vec::new());
    };
    let m = fts_match(&q);
    let stale_rows: i64 = db.query_row("SELECT count(*) FROM stale", [], |r| r.get(0))?;
    let fresh = stale_rows == 0 && meta_i64(db, "seq")? == lib.last_seq()?;
    let mut stmts = Vec::with_capacity(SHARDS);
    for t in SHARD_TABLES {
        stmts.push(db.prepare_cached(&format!("SELECT rowid FROM {t} WHERE {t} MATCH ?1"))?);
    }
    let mut rows = Vec::with_capacity(SHARDS);
    for s in stmts.iter_mut() {
        rows.push(s.query([&m])?);
    }
    let mut heads = Vec::with_capacity(SHARDS);
    for r in rows.iter_mut() {
        heads.push(next_rowid(r)?);
    }
    let mut hits = Vec::new();
    while hits.len() < limit {
        let Some((k, rowid)) = heads.iter().enumerate().filter_map(|(k, h)| h.map(|r| (k, r))).min_by_key(|&(_, r)| r) else {
            break;
        };
        heads[k] = next_rowid(&mut rows[k])?;
        let row: Option<(DocSummary, bool, Vec<u8>)> = db
            .prepare_cached(
                "SELECT i.doc, i.ns, i.title, i.rev, i.created, i.updated, i.deleted, p.z FROM item i JOIN plain p ON p.rowid = i.rowid WHERE i.rowid = ?1",
            )?
            .query_row([rowid], |r| Ok((summary_row(r)?, r.get(6)?, r.get(7)?)))
            .optional()?;
        let Some((doc, deleted, z)) = row else { continue };
        if deleted || (!fresh && lib.head(doc.id)? != Some(doc.head)) {
            continue; // 지웠거나, 색인한 글이 머리 판보다 낡았다
        }
        let plain = unzip_with(unz, &z)?;
        if !verify(&plain, &q) {
            continue;
        }
        hits.push(Hit { snippet: snippet(&plain, &q, 40), doc });
    }
    Ok(hits)
}

thread_local! {
    /// 스레드마다 압축 문맥 하나 — 부를 때마다 만들면 작은 글에서는 압축보다 문맥 만들기가 더 든다.
    static ZIP: std::cell::RefCell<Option<zstd::bulk::Compressor<'static>>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn zip(bytes: &[u8]) -> Result<Vec<u8>> {
    ZIP.with(|z| {
        let mut z = z.borrow_mut();
        if z.is_none() {
            *z = Some(zstd::bulk::Compressor::new(ZSTD_LEVEL)?);
        }
        Ok(z.as_mut().expect("압축 문맥").compress(bytes)?)
    })
}

/// 한 문서의 색인 행을 모두 새로 쓴다. 문서 번호(rowid)는 처음 색인할 때 정해지고 바뀌지 않는다.
/// `mark_users`면, 틀 문서일 때 그 틀(옛 제목 · 새 제목)을 쓰는 문서를 낡음으로 넣는다(통째 짓기는 모두 새로 그리므로 끈다).
/// `fts`가 거짓이면 조각 색인은 건드리지 않는다(통째 짓기는 따로 나란히 짓는다).
pub(crate) fn write(db: &Connection, p: &Prepared, mark_users: bool, fts: bool) -> Result<i64> {
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
    if let Some((_, ns, _, _, deleted)) = &prev {
        bump_ns(db, *ns, *deleted, -1)?;
    }
    bump_ns(db, p.ns, p.deleted, 1)?;
    // 달마다 새 문서: 살아 있는 일반 문서가 되거나 그만둔 만큼(만든 시각의 달).
    let was_live = matches!(&prev, Some((_, Namespace::Doc, _, _, false)));
    let is_live = p.ns == Namespace::Doc && !p.deleted;
    if was_live != is_live {
        bump_new(db, Hlc(p.created), if is_live { 1 } else { -1 })?;
    }
    let home_moved = match &prev {
        Some((_, ns, _, home, deleted)) => *ns != p.ns || *deleted != p.deleted || *home != p.home,
        None => true,
    };
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
    let old = prev.as_ref().map(|(rowid, ..)| *rowid);
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
            if fts {
                db.prepare_cached(&format!("DELETE FROM {} WHERE rowid = ?1", fts_of(rowid)))?.execute([rowid])?;
            }
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
    if fts && !p.deleted && !p.tokens.is_empty() {
        db.prepare_cached(&format!("INSERT INTO {}(rowid, t) VALUES (?1, ?2)", fts_of(rowid)))?.execute(params![rowid, p.tokens])?;
    }
    let links_changed = write_extracted(db, rowid, old.is_some(), &p.ex)?;
    // 갈래 사이 링크는 링크 · 집 · 분류 나무가 바뀔 때만 다시 센다(본문 글만 고친 저장은 세지 않는다).
    if links_changed || home_moved || p.ns == Namespace::Category {
        set_meta(db, "stats_dirty", 1)?;
    }
    // 이 제목을 가리키던 링크가 만들 문서인지 다시 정한다(새 제목은 있음, 떠난 옛 제목은 다른 문서가 남았는지).
    if !p.deleted {
        db.prepare_cached("UPDATE target SET present = 1 WHERE ns = ?1 AND key = ?2")?.execute(params![p.ns.prefix(), p.title])?;
    }
    if let Some((_, ns, title, _, was_deleted)) = &prev {
        if !*was_deleted && (p.deleted || *ns != p.ns || *title != p.title) {
            db.prepare_cached(
                "UPDATE target SET present = EXISTS(SELECT 1 FROM item INDEXED BY item_title WHERE ns = ?3 AND title = ?2 AND deleted = 0) WHERE ns = ?1 AND key = ?2",
            )?
            .execute(params![ns.prefix(), title, ns])?;
        }
    }
    db.prepare_cached("DELETE FROM stale WHERE doc = ?1")?.execute(params![p.doc])?;
    Ok(rowid)
}

/// 뽑기 표를 이 문서 번호의 새 행으로 바꾼다. 본문 링크는 대상별 문서 수(`target`), 사료 · 해는 셈 표도 고친다.
/// 본문 링크 모음이 바뀌었으면 참.
fn write_extracted(db: &Connection, rowid: i64, existed: bool, ex: &Extracted) -> Result<bool> {
    let mut new_links: Vec<(String, String)> =
        ex.links.iter().filter(|(k, ..)| *k == history_namumark::LinkKind::Body).map(|(_, ns, key)| (ns.clone(), key.clone())).collect();
    new_links.sort();
    let mut links_changed = !new_links.is_empty();
    if existed {
        let old: Vec<(String, String)> = {
            let mut st = db.prepare_cached("SELECT ns, key FROM link WHERE src = ?1 AND kind = 0 ORDER BY ns, key")?;
            let rows = st.query_map([rowid], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        links_changed = old != new_links;
        let sources: Vec<String> = {
            let mut st = db.prepare_cached("SELECT name FROM source WHERE doc = ?1")?;
            let rows = st.query_map([rowid], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for s in sources {
            bump_source(db, &s, -1)?;
        }
        let years: Option<(i32, i32)> = db.prepare_cached("SELECT y0, y1 FROM years WHERE doc = ?1")?.query_row([rowid], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
        if let Some((y0, y1)) = years {
            bump_years(db, y0, y1, -1)?;
        }
        for (ns, key) in old {
            db.prepare_cached("UPDATE target SET refs = refs - 1 WHERE ns = ?1 AND key = ?2")?.execute(params![ns, key])?;
            db.prepare_cached("DELETE FROM target WHERE ns = ?1 AND key = ?2 AND refs <= 0")?.execute(params![ns, key])?;
        }
        for (table, col) in PER_DOC {
            db.prepare_cached(&format!("DELETE FROM {table} WHERE {col} = ?1"))?.execute([rowid])?;
        }
    }
    for (kind, ns, key) in &ex.links {
        db.prepare_cached("INSERT OR IGNORE INTO link(src, kind, ns, key) VALUES (?1, ?2, ?3, ?4)")?.execute(params![rowid, *kind as i64, ns, key])?;
        if *kind == history_namumark::LinkKind::Body {
            // 있던 대상이면 수만 올린다. 새 대상일 때만 그 제목의 문서가 있는지 찾는다(저장마다 링크 수만큼 찾지 않도록).
            let bumped = db.prepare_cached("UPDATE target SET refs = refs + 1 WHERE ns = ?1 AND key = ?2")?.execute(params![ns, key])?;
            if bumped == 0 {
                let present = match Namespace::ALL.into_iter().find(|n| n.prefix() == ns) {
                    Some(code) => db
                        .prepare_cached("SELECT EXISTS(SELECT 1 FROM item INDEXED BY item_title WHERE ns = ?1 AND title = ?2 AND deleted = 0)")?
                        .query_row(params![code, key], |r| r.get::<_, bool>(0))?,
                    None => false,
                };
                db.prepare_cached("INSERT INTO target(ns, key, refs, present) VALUES (?1, ?2, 1, ?3)")?.execute(params![ns, key, present])?;
            }
        }
    }
    for (i, c) in ex.categories.iter().enumerate() {
        db.prepare_cached("INSERT OR IGNORE INTO member(doc, category, first) VALUES (?1, ?2, ?3)")?.execute(params![rowid, c, i == 0])?;
    }
    for s in &ex.sources {
        db.prepare_cached("INSERT INTO source(doc, name) VALUES (?1, ?2)")?.execute(params![rowid, s])?;
        bump_source(db, s, 1)?;
    }
    for e in &ex.entities {
        db.prepare_cached("INSERT INTO entity(doc, entity) VALUES (?1, ?2)")?.execute(params![rowid, e])?;
    }
    for t in &ex.templates {
        db.prepare_cached("INSERT INTO uses(doc, template) VALUES (?1, ?2)")?.execute(params![rowid, t])?;
    }
    if let Some((y0, y1)) = ex.years {
        db.prepare_cached("INSERT INTO years(doc, y0, y1) VALUES (?1, ?2, ?3)")?.execute(params![rowid, y0, y1])?;
        bump_years(db, y0, y1, 1)?;
    }
    Ok(links_changed)
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
        Self::with_library(Library::open(app_dir)?)
    }

    /// 벽시계를 바꿔 끼워 연다(시험 — 날 · 달 셈).
    pub fn open_with_clock(app_dir: &Path, wall: WallClock) -> Result<Engine> {
        Self::with_library(Library::open_with_clock(app_dir, wall)?)
    }

    fn with_library(lib: Library) -> Result<Engine> {
        let idx = Index::open(&lib.info().folder, lib.info().library, &crate::extra_schema())?;
        Ok(Engine {
            lib,
            idx,
            unz: zstd::bulk::Decompressor::new()?,
            tree: None,
            fail_before_index_commit: false,
            fail_template_reads: false,
            fail_rebuild_after: None,
        })
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
            let (mut seen, mut new_w, mut times) = (HashSet::new(), w, Vec::new());
            for (seq, doc, hlc) in revs {
                if seen.insert(doc) {
                    index_one(&self.lib, &tx, doc, self.fail_template_reads)?;
                    done += 1;
                }
                new_w = seq;
                times.push(hlc); // 판마다 한 번 — 눈금이 판을 한 번만 지난다
                if start.elapsed() >= max {
                    count_edits(&tx, &times)?;
                    set_meta(&tx, "seq", new_w)?;
                    tx.commit()?;
                    break 'revs;
                }
            }
            count_edits(&tx, &times)?;
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
                        write(&tx, &p, true, true)?;
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
        // 남는 시간에 갈래 사이 링크를 센다.
        if start.elapsed() < max && crate::stats::pending(&self.idx.db)? {
            if self.tree.is_none() {
                self.tree = Some(Tree::load(&self.idx.db)?);
            }
            let tree = self.tree.as_ref().expect("나무");
            crate::stats::count_group_links(&self.idx.db, tree, start + max)?;
        }
        Ok(StepReport { done, stale: self.status()?.stale })
    }

    /// 갈래 사이 링크를 다시 세야 하거나 세는 중인가.
    pub fn stats_pending(&self) -> Result<bool> {
        crate::stats::pending(&self.idx.db)
    }

    /// 대문 모음(설계 8-2).
    pub fn home(&mut self, o: &HomeOptions) -> Result<crate::stats::Home> {
        let counts = crate::stats::counts(&self.idx.db)?;
        let wanted = self.links_wanted(o.wanted)?;
        if self.tree.is_none() {
            self.tree = Some(Tree::load(&self.idx.db)?);
        }
        crate::stats::home(&self.idx.db, self.tree.as_ref().expect("나무"), counts, wanted, o)
    }

    /// 서재 통계. 날 · 달은 `tz_minutes` 시간대로, 「오늘」은 `today_ms`(없으면 지금).
    pub fn stats(&self, tz_minutes: i32, today_ms: Option<u64>) -> Result<crate::stats::Stats> {
        crate::stats::stats(&self.idx.db, tz_minutes, today_ms)
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

    /// 묶음을 든다(spec/drive-format.md 4절) — 판이 든 문서는 `doc.*`처럼 같은 호출에서 색인한다.
    pub fn apply(&mut self, pack: &[u8]) -> Result<Applied> {
        let before = self.lib.last_seq()?;
        let r = self.lib.apply(pack)?;
        if !r.touched.is_empty() {
            self.after_write(before, &r.touched);
        }
        Ok(r)
    }

    pub fn save_merge(&mut self, doc: DocId, base: RevId, merge: RevId, body: &str, message: &str) -> Result<Saved> {
        let before = self.lib.last_seq()?;
        let s = self.lib.save_merge(doc, base, merge, body, message)?;
        self.after_write(before, &[doc]);
        Ok(s)
    }

    /// 드라이브의 서재에 합류한다 — 색인의 서재 ID도 바꾼다. 안 그러면 다음 열기에서 다른 서재의 색인으로 보여 통째로 다시 짓는다.
    pub fn join(&mut self, library: LibraryId) -> Result<()> {
        self.lib.join(library)?;
        self.idx.db.execute("UPDATE meta SET value = ?1 WHERE key = 'library'", [library.to_string()])?;
        Ok(())
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
            // 이 저장 앞의 판은 모두 색인에 있었다 — 눈금을 이 저장 뒤로 옮기고, 그 판들을 날마다 고친 횟수에 센다.
            let times: Vec<Hlc> = self.lib.revisions_after(before, usize::MAX)?.into_iter().map(|(_, _, h)| h).collect();
            count_edits(&tx, &times)?;
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
                let rowid = write(&tx, &p, true, true)?;
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
        unzip_with(&mut self.unz, z)
    }

    /// 있는 문서 ID — 같은 차례, 없으면 None. 제목은 정규화하고, 같은 제목이 둘이면 먼저 만든 것.
    /// 색인의 덮는 제목 색인에서 읽는다(서재 표를 차갑게 찾아가지 않는다 — 색인이 낡았으면 그만큼 늦다).
    pub fn titles_exist(&self, titles: &[(Namespace, String)]) -> Result<Vec<Option<DocId>>> {
        titles_exist_in(&self.idx.db, titles)
    }

    /// 제목이 `prefix`로 시작하는 살아 있는 문서(제목 차례) — 덮는 제목 색인 한 범위.
    pub fn search_titles(&self, ns: Namespace, prefix: &str, limit: usize) -> Result<Vec<DocSummary>> {
        search_titles_in(&self.idx.db, ns, prefix, limit)
    }

    /// 본문 찾기 — 조각 색인 넷을 문서 번호 차례로 합쳐 후보를 받고, 지운 · 낡은 문서는 건너뛰고, 확인을 지난 것만 `limit`까지.
    pub fn search_body(&mut self, query: &str, limit: usize) -> Result<Vec<Hit>> {
        search_body_in(&self.idx.db, &mut self.unz, &self.lib, query, limit)
    }

    /// 이 문서 제목으로 링크(본문 · 넘겨주기)한 살아 있는 문서, 제목 차례.
    pub fn links_back(&self, doc: DocId, limit: usize) -> Result<Vec<DocSummary>> {
        let Some((me, _)) = self.lib.summary(doc)? else {
            return Ok(Vec::new());
        };
        let mut st = self.idx.db.prepare_cached(
            "SELECT DISTINCT i.doc, i.ns, i.title, i.rev, i.created, i.updated FROM link l JOIN item i ON i.rowid = l.src
             WHERE l.ns = ?1 AND l.key = ?2 AND l.kind IN (0, 2) AND i.deleted = 0 AND i.doc != ?3 ORDER BY i.title LIMIT ?4",
        )?;
        let rows = st.query_map(params![me.ns.prefix(), me.title, doc, limit as i64], summary_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// 만들 문서 — 없는 문서를 가리킨 본문 링크, 가리킨 문서 수 많은 차례.
    pub fn links_wanted(&self, limit: usize) -> Result<Vec<Wanted>> {
        let mut st = self.idx.db.prepare_cached("SELECT ns, key, refs FROM target WHERE present = 0 ORDER BY refs DESC, ns, key LIMIT ?1")?;
        let rows = st.query_map([limit as i64], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (ns, title, refs) = row?;
            if let Some(ns) = Namespace::ALL.into_iter().find(|n| n.prefix() == ns) {
                out.push(Wanted { ns, title, refs: refs as u64 });
            }
        }
        Ok(out)
    }

    /// 이 연표 항목을 잇는 문서(연표 링크 · 연표 띠), 제목 차례.
    pub fn timeline_docs(&self, entity: &str) -> Result<Vec<DocSummary>> {
        timeline_docs_in(&self.idx.db, entity)
    }

    /// 분류 폴더 하나 — `None`이면 맨 위(「분류 안 됨」 수 포함), 빈 이름이면 「분류 안 됨」 문서.
    pub fn children(&mut self, category: Option<&str>, limit: usize, after: Option<&str>) -> Result<Children> {
        if self.tree.is_none() {
            self.tree = Some(Tree::load(&self.idx.db)?);
        }
        self.tree.as_ref().expect("나무").children(&self.idx.db, category, limit, after)
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

fn next_rowid(rows: &mut rusqlite::Rows<'_>) -> Result<Option<i64>> {
    Ok(match rows.next()? {
        Some(r) => Some(r.get(0)?),
        None => None,
    })
}

pub(crate) fn unzip_with(unz: &mut zstd::bulk::Decompressor<'static>, z: &[u8]) -> Result<String> {
    let cap = zstd::zstd_safe::get_frame_content_size(z).ok().flatten().unwrap_or(64 << 20) as usize;
    let bytes = unz.decompress(z, cap.max(1))?;
    String::from_utf8(bytes).map_err(|_| Error::Corrupt("색인 글이 UTF-8이 아님".into()))
}

/// 한 문서를 색인한다. 읽기 오류면 `stale`에 넣고 넘어간다(색인 쓰기 오류만 올린다).
fn index_one(lib: &Library, tx: &Connection, doc: DocId, fail_templates: bool) -> Result<()> {
    match prepare(lib, doc, fail_templates) {
        Ok(Some(p)) => {
            write(tx, &p, true, true)?;
        }
        Ok(None) => {}
        Err(_) => mark_stale(tx, doc)?,
    }
    Ok(())
}
