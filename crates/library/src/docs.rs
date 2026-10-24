//! 문서 일 — 만들기 · 저장 · 이름 바꾸기 · 지우기 · 읽기(설계 5-1 · 5-2). 쓰는 일은 모두 SQLite 트랜잭션 하나다.
use crate::error::{Error, Result};
use crate::hlc::Hlc;
use crate::ids::{DeviceId, DocId, RevId};
use crate::ns::Namespace;
use crate::store::{Counts, Doc, DocSummary, Library, NewDoc, Renamed, RevInfo, RevKind, Revision, Saved};
use crate::title::normalize_title;
use rusqlite::{params, Connection, OptionalExtension};
use std::borrow::Cow;

const ZSTD_LEVEL: i32 = 3;
/// 이름공간 · 지움별 문서 수.
const COUNTS_SQL: &str = "SELECT ns, deleted, count(*) FROM doc GROUP BY ns, deleted";
const REV_COLUMNS: &str = "id, doc, parent, parent2, device, hlc, kind, ns, title, message, len";

pub(crate) struct Head {
    pub(crate) ns: Namespace,
    pub(crate) title: String,
    pub(crate) head: RevId,
    pub(crate) created: Hlc,
    pub(crate) updated: Hlc,
    pub(crate) deleted: bool,
}

struct NewRev<'a> {
    id: RevId,
    doc: DocId,
    parent: Option<RevId>,
    kind: RevKind,
    ns: Namespace,
    title: &'a str,
    message: &'a str,
    body: &'a str,
}

pub(crate) fn load_head(db: &Connection, doc: DocId) -> Result<Head> {
    db.prepare_cached("SELECT ns, title, head, created, updated, deleted FROM doc WHERE id = ?1")?.query_row(params![doc], |r| {
        Ok(Head { ns: r.get(0)?, title: r.get(1)?, head: r.get(2)?, created: r.get(3)?, updated: r.get(4)?, deleted: r.get(5)? })
    })
    .optional()?
    .ok_or_else(|| Error::NotFound(format!("문서 {doc}")))
}

/// 줄끝을 `\n`으로 맞춘다 — 그리기 모델의 문단 범위(UTF-16)가 저장된 본문의 위치와 맞도록(spec/namumark.md).
pub(crate) fn lf(body: &str) -> Cow<'_, str> {
    if body.contains('\r') {
        Cow::Owned(body.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(body)
    }
}

pub(crate) fn encode(body: &str) -> Result<Vec<u8>> {
    Ok(zstd::bulk::compress(body.as_bytes(), ZSTD_LEVEL)?)
}

thread_local! {
    /// 스레드마다 풀기 문맥 하나 — 부를 때마다 만들면 짧은 본문에서는 풀기보다 문맥 만들기가 더 든다.
    static UNZ: std::cell::RefCell<Option<zstd::bulk::Decompressor<'static>>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn decode(blob: &[u8]) -> Result<String> {
    let bytes = match zstd::zstd_safe::get_frame_content_size(blob) {
        Ok(Some(size)) => UNZ.with(|u| {
            let mut u = u.borrow_mut();
            if u.is_none() {
                *u = Some(zstd::bulk::Decompressor::new()?);
            }
            u.as_mut().expect("풀기 문맥").decompress(blob, size as usize)
        }),
        // 크기가 적히지 않은 묶음(없어야 하지만)은 흐름으로 푼다.
        _ => zstd::stream::decode_all(blob),
    }
    .map_err(|e| Error::Corrupt(format!("본문 풀기: {e}")))?;
    String::from_utf8(bytes).map_err(|_| Error::Corrupt("본문이 UTF-8이 아님".into()))
}

fn read_body(db: &Connection, rev: RevId) -> Result<String> {
    let blob: Vec<u8> = db
        .prepare_cached("SELECT body FROM rev WHERE id = ?1")?
        .query_row(params![rev], |r| r.get(0))
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("판 {rev}")))?;
    decode(&blob)
}

fn insert_rev(db: &Connection, device: DeviceId, hlc: Hlc, r: &NewRev) -> Result<()> {
    db.prepare_cached(
        "INSERT INTO rev(id, doc, parent, parent2, device, hlc, kind, ns, title, message, len, body)
         VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )?
    .execute(params![r.id, r.doc, r.parent, device, hlc, r.kind, r.ns, r.title, r.message, r.body.len() as i64, encode(r.body)?])?;
    Ok(())
}

fn insert_doc(db: &Connection, doc: DocId, ns: Namespace, title: &str, head: RevId, hlc: Hlc) -> Result<()> {
    db.prepare_cached("INSERT INTO doc(id, ns, title, head, created, updated, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)")?
        .execute(params![doc, ns, title, head, hlc])?;
    Ok(())
}

fn rev_info(r: &rusqlite::Row<'_>) -> rusqlite::Result<RevInfo> {
    Ok(RevInfo {
        id: r.get(0)?,
        doc: r.get(1)?,
        parent: r.get(2)?,
        parent2: r.get(3)?,
        device: r.get(4)?,
        hlc: r.get(5)?,
        kind: r.get(6)?,
        ns: r.get(7)?,
        title: r.get(8)?,
        message: r.get(9)?,
        len: r.get::<_, i64>(10)? as u64,
    })
}

impl Library {
    pub fn create(&mut self, ns: Namespace, title: &str, body: &str, message: &str) -> Result<Saved> {
        let title = normalize_title(title)?;
        let body = &*lf(body);
        let (doc, rev, hlc, device) = (DocId::new(), RevId::new(), self.clock.now(), self.info.device);
        let tx = self.db.transaction()?;
        insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: None, kind: RevKind::Content, ns, title: &title, message, body })?;
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        insert_doc(&tx, doc, ns, &title, rev, hlc)?;
        tx.commit()?;
        Ok(Saved { doc, rev, hlc, changed: true, head: rev })
    }

    pub fn save(&mut self, doc: DocId, base: RevId, body: &str, message: &str) -> Result<Saved> {
        let body = &*lf(body);
        let device = self.info.device;
        let tx = self.db.transaction()?;
        let h = load_head(&tx, doc)?;
        if h.head != base {
            return Err(Error::StaleBase { head: h.head });
        }
        if !h.deleted && read_body(&tx, h.head)? == body {
            return Ok(Saved { doc, rev: h.head, hlc: h.updated, changed: false, head: h.head });
        }
        let (rev, hlc) = (RevId::new(), self.clock.now());
        insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: Some(h.head), kind: RevKind::Content, ns: h.ns, title: &h.title, message, body })?;
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        tx.execute("UPDATE doc SET head = ?2, updated = ?3, deleted = 0 WHERE id = ?1", params![doc, rev, hlc])?;
        let (merged, head) = crate::sync::after_local_write(&tx, doc, device, rev)?;
        tx.commit()?;
        self.observe(merged);
        Ok(Saved { doc, rev, hlc, changed: true, head })
    }

    /// 충돌 중이던 문서를 고쳐 합친 판이 생겼으면 시계를 그 뒤로.
    fn observe(&mut self, merged: Option<Hlc>) {
        if let Some(h) = merged {
            self.clock.observe(h);
        }
    }

    /// 한꺼번에 들이기(측정기 · 새 기기) — `batch`편마다 트랜잭션 하나, `create`와 같은 규칙(제목 정규화 · 줄끝 · 첫 판).
    /// 쓸 수 없는 제목을 만나면 그 묶음은 넣지 않고 `BadTitle("<n>번째: …")`로 멈춘다. 앞 묶음은 남는다.
    pub fn import(&mut self, docs: &mut dyn Iterator<Item = NewDoc>, batch: usize) -> Result<Vec<Saved>> {
        let batch = batch.max(1);
        let device = self.info.device;
        // 제목 색인에 무작위로 넣으므로 들이는 동안만 캐시를 크게 둔다.
        self.db.execute_batch("PRAGMA cache_size = -262144;")?;
        let out = self.import_batches(docs, batch, device);
        self.db.execute_batch("PRAGMA cache_size = -16384;")?;
        out
    }

    fn import_batches(&mut self, docs: &mut dyn Iterator<Item = NewDoc>, batch: usize, device: DeviceId) -> Result<Vec<Saved>> {
        let mut out = Vec::new();
        let mut pending: Vec<(Namespace, String, String)> = Vec::with_capacity(batch);
        let mut seen = 0usize;
        loop {
            pending.clear();
            for d in (&mut *docs).take(batch) {
                seen += 1;
                let title = normalize_title(&d.title).map_err(|e| match e {
                    Error::BadTitle(why) => Error::BadTitle(format!("{seen}번째: {why}")),
                    e => e,
                })?;
                let body = match lf(&d.body) {
                    Cow::Borrowed(_) => d.body,
                    Cow::Owned(b) => b,
                };
                pending.push((d.ns, title, body));
            }
            if pending.is_empty() {
                return Ok(out);
            }
            let tx = self.db.transaction()?;
            for (ns, title, body) in &pending {
                let (doc, rev, hlc) = (DocId::new(), RevId::new(), self.clock.now());
                insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: None, kind: RevKind::Content, ns: *ns, title, message: "", body })?;
                insert_doc(&tx, doc, *ns, title, rev, hlc)?;
                out.push(Saved { doc, rev, hlc, changed: true, head: rev });
            }
            tx.commit()?;
        }
    }

    pub fn rename(&mut self, doc: DocId, base: RevId, title: &str, keep_redirect: bool, message: &str) -> Result<Renamed> {
        let new_title = normalize_title(title)?;
        let device = self.info.device;
        let tx = self.db.transaction()?;
        let h = load_head(&tx, doc)?;
        if h.head != base {
            return Err(Error::StaleBase { head: h.head });
        }
        if h.deleted {
            return Err(Error::Deleted(doc));
        }
        if h.title == new_title {
            return Err(Error::BadTitle("지금 제목과 같음".into()));
        }
        let body = read_body(&tx, h.head)?;
        let (rev, hlc) = (RevId::new(), self.clock.now());
        insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: Some(h.head), kind: RevKind::Content, ns: h.ns, title: &new_title, message, body: &body })?;
        tx.execute("UPDATE doc SET title = ?2, head = ?3, updated = ?4 WHERE id = ?1", params![doc, new_title, rev, hlc])?;
        let redirect = if keep_redirect {
            let (rdoc, rrev, rhlc) = (DocId::new(), RevId::new(), self.clock.now());
            let rbody = format!("#넘겨주기 {}\n", h.ns.link_title(&new_title));
            let r = NewRev { id: rrev, doc: rdoc, parent: None, kind: RevKind::Content, ns: h.ns, title: &h.title, message: "이름을 바꾸며 남긴 넘겨주기", body: &rbody };
            insert_rev(&tx, device, rhlc, &r)?;
            insert_doc(&tx, rdoc, h.ns, &h.title, rrev, rhlc)?;
            Some(rdoc)
        } else {
            None
        };
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        let (merged, head) = crate::sync::after_local_write(&tx, doc, device, rev)?;
        tx.commit()?;
        self.observe(merged);
        Ok(Renamed { rev, hlc, head, redirect })
    }

    pub fn delete(&mut self, doc: DocId, base: RevId, message: &str) -> Result<Saved> {
        let device = self.info.device;
        let tx = self.db.transaction()?;
        let h = load_head(&tx, doc)?;
        if h.head != base {
            return Err(Error::StaleBase { head: h.head });
        }
        if h.deleted {
            return Ok(Saved { doc, rev: h.head, hlc: h.updated, changed: false, head: h.head });
        }
        let (rev, hlc) = (RevId::new(), self.clock.now());
        insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: Some(h.head), kind: RevKind::Deleted, ns: h.ns, title: &h.title, message, body: "" })?;
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        tx.execute("UPDATE doc SET head = ?2, updated = ?3, deleted = 1 WHERE id = ?1", params![doc, rev, hlc])?;
        let (merged, head) = crate::sync::after_local_write(&tx, doc, device, rev)?;
        tx.commit()?;
        self.observe(merged);
        Ok(Saved { doc, rev, hlc, changed: true, head })
    }

    pub fn get(&self, doc: DocId) -> Result<Option<Doc>> {
        let h = match load_head(&self.db, doc) {
            Ok(h) => h,
            Err(Error::NotFound(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        let body = read_body(&self.db, h.head)?;
        Ok(Some(Doc { id: doc, ns: h.ns, title: h.title, head: h.head, created: h.created, updated: h.updated, deleted: h.deleted, body }))
    }

    /// 이름공간 · 제목이 같은 살아 있는 문서들(만든 차례). 쓸 수 없는 제목이면 빈 목록.
    pub fn by_title(&self, ns: Namespace, title: &str) -> Result<Vec<DocSummary>> {
        let Ok(title) = normalize_title(title) else {
            return Ok(Vec::new());
        };
        let mut st = self.db.prepare_cached(
            "SELECT id, ns, title, head, created, updated FROM doc WHERE ns = ?1 AND title = ?2 AND deleted = 0 ORDER BY created, id",
        )?;
        let rows = st.query_map(params![ns, title], |r| {
            Ok(DocSummary { id: r.get(0)?, ns: r.get(1)?, title: r.get(2)?, head: r.get(3)?, created: r.get(4)?, updated: r.get(5)? })
        })?;
        let out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(out)
    }

    /// 판 기록, 새 것부터. `before`가 있으면 그 시각보다 앞선 판만(이어 받기).
    pub fn history(&self, doc: DocId, limit: usize, before: Option<Hlc>) -> Result<Vec<RevInfo>> {
        let mut st = self.db.prepare_cached(&format!("SELECT {REV_COLUMNS} FROM rev WHERE doc = ?1 AND hlc < ?2 ORDER BY hlc DESC LIMIT ?3"))?;
        let rows = st.query_map(params![doc, before.unwrap_or(Hlc(i64::MAX as u64)), limit as i64], rev_info)?;
        let out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(out)
    }

    pub fn revision(&self, rev: RevId) -> Result<Option<Revision>> {
        let row = self
            .db
            .query_row(&format!("SELECT {REV_COLUMNS}, body FROM rev WHERE id = ?1"), params![rev], |r| Ok((rev_info(r)?, r.get::<_, Vec<u8>>(11)?)))
            .optional()?;
        match row {
            None => Ok(None),
            Some((info, blob)) => Ok(Some(Revision { info, body: decode(&blob)? })),
        }
    }

    /// 제목이 `prefix`로 시작하는 살아 있는 문서(제목 차례), 많아야 `limit`. 앞붙이는 NFC로 맞추고 앞 공백만 뗀다.
    pub fn titles_with_prefix(&self, ns: Namespace, prefix: &str, limit: usize) -> Result<Vec<DocSummary>> {
        use unicode_normalization::UnicodeNormalization;
        let prefix: String = prefix.trim_start().nfc().collect();
        if prefix.is_empty() {
            return Ok(Vec::new());
        }
        let upper = format!("{prefix}\u{10FFFF}");
        let mut st = self.db.prepare_cached(
            "SELECT id, ns, title, head, created, updated FROM doc WHERE ns = ?1 AND title >= ?2 AND title < ?3 AND deleted = 0 ORDER BY title, created LIMIT ?4",
        )?;
        let rows = st.query_map(params![ns, prefix, upper, limit as i64], |r| {
            Ok(DocSummary { id: r.get(0)?, ns: r.get(1)?, title: r.get(2)?, head: r.get(3)?, created: r.get(4)?, updated: r.get(5)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// 마지막 판 번호(`rev.seq`, 늘기만 한다) — 색인의 눈금과 견준다.
    pub fn last_seq(&self) -> Result<i64> {
        Ok(self.db.query_row("SELECT COALESCE(MAX(seq), 0) FROM rev", [], |r| r.get(0))?)
    }

    /// 판 번호 `after` 뒤의 판(번호 차례), 많아야 `limit` — (번호, 문서, 시각).
    pub fn revisions_after(&self, after: i64, limit: usize) -> Result<Vec<(i64, DocId, Hlc)>> {
        let mut st = self.db.prepare_cached("SELECT seq, doc, hlc FROM rev WHERE seq > ?1 ORDER BY seq LIMIT ?2")?;
        let rows = st.query_map(params![after, limit.min(i64::MAX as usize) as i64], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// 판 번호 `upto`까지 모든 판의 시각(통째 짓기가 날마다 고친 횟수를 센다).
    pub fn revision_times(&self, upto: i64) -> Result<Vec<Hlc>> {
        let mut st = self.db.prepare("SELECT hlc FROM rev WHERE seq <= ?1")?;
        let rows = st.query_map([upto], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// 모든 문서 ID(만든 차례, 지운 것 포함) — 통째 짓기.
    pub fn doc_ids(&self) -> Result<Vec<DocId>> {
        let mut st = self.db.prepare("SELECT id FROM doc ORDER BY created, id")?;
        let rows = st.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// 이 이름공간의 살아 있는 문서(만든 차례).
    pub fn live_docs(&self, ns: Namespace) -> Result<Vec<DocSummary>> {
        let mut st = self.db.prepare_cached("SELECT id, ns, title, head, created, updated FROM doc WHERE ns = ?1 AND deleted = 0 ORDER BY created, id")?;
        let rows = st.query_map([ns], |r| {
            Ok(DocSummary { id: r.get(0)?, ns: r.get(1)?, title: r.get(2)?, head: r.get(3)?, created: r.get(4)?, updated: r.get(5)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// 같은 서재 파일을 읽기 전용으로 따로 연다 — 다른 스레드에서 읽을 때(통째 짓기).
    pub fn reader(&self) -> Result<Reader> {
        let path = self.info.folder.join(crate::store::FILE);
        let db = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        Ok(Reader { db })
    }

    /// 판 번호 `after` 뒤에 판이 생긴 문서 수.
    pub fn docs_changed_after(&self, after: i64) -> Result<u64> {
        let n: i64 = self.db.prepare_cached("SELECT count(DISTINCT doc) FROM rev WHERE seq > ?1")?.query_row([after], |r| r.get(0))?;
        Ok(n as u64)
    }

    /// 이 문서에 판 번호 `after` 뒤의 판이 있는가.
    pub fn changed_after(&self, doc: DocId, after: i64) -> Result<bool> {
        Ok(self.db.prepare_cached("SELECT 1 FROM rev WHERE doc = ?1 AND seq > ?2 LIMIT 1")?.query_row(params![doc, after], |_| Ok(())).optional()?.is_some())
    }

    /// 본문을 풀지 않는 요약과 지움 표시. 없으면 None.
    pub fn summary(&self, doc: DocId) -> Result<Option<(DocSummary, bool)>> {
        match load_head(&self.db, doc) {
            Ok(h) => Ok(Some((DocSummary { id: doc, ns: h.ns, title: h.title, head: h.head, created: h.created, updated: h.updated }, h.deleted))),
            Err(Error::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn counts(&self) -> Result<Counts> {
        let mut counts = Counts::default();
        let mut st = self.db.prepare_cached(COUNTS_SQL)?;
        let mut rows = st.query([])?;
        while let Some(r) = rows.next()? {
            let (ns, deleted, n): (Namespace, bool, i64) = (r.get(0)?, r.get(1)?, r.get(2)?);
            if deleted {
                counts.deleted += n as u64;
            } else {
                counts.live[ns as usize] += n as u64;
            }
        }
        Ok(counts)
    }
}

/// 읽기 전용 연결 하나(`Library::reader`). 다른 스레드로 옮길 수 있다.
pub struct Reader {
    db: Connection,
}

impl Reader {
    /// 서재 파일(`<자료 폴더>/HistoryCore/library.db`)을 읽기 전용으로 연다.
    pub fn open(library_db: &std::path::Path) -> Result<Reader> {
        let db = Connection::open_with_flags(library_db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        Ok(Reader { db })
    }

    pub fn get(&self, doc: DocId) -> Result<Option<Doc>> {
        let h = match load_head(&self.db, doc) {
            Ok(h) => h,
            Err(Error::NotFound(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        let body = read_body(&self.db, h.head)?;
        Ok(Some(Doc { id: doc, ns: h.ns, title: h.title, head: h.head, created: h.created, updated: h.updated, deleted: h.deleted, body }))
    }

    /// 이름공간 · 제목이 같은 살아 있는 문서 중 먼저 만든 것.
    pub fn first_by_title(&self, ns: Namespace, title: &str) -> Result<Option<DocId>> {
        let Ok(title) = normalize_title(title) else {
            return Ok(None);
        };
        Ok(self
            .db
            .prepare_cached("SELECT id FROM doc WHERE ns = ?1 AND title = ?2 AND deleted = 0 ORDER BY created, id LIMIT 1")?
            .query_row(params![ns, title], |r| r.get(0))
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, Library, Namespace};
    use rusqlite::Connection;

    /// EXPLAIN QUERY PLAN의 설명 열을 한 줄로.
    fn plan(sql: &str) -> String {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(crate::store::SCHEMA).unwrap();
        let mut st = db.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
        let rows = st.query_map([], |r| r.get::<_, String>(3)).unwrap();
        rows.map(|r| r.unwrap()).collect::<Vec<_>>().join(" / ")
    }

    #[test]
    fn counting_docs_reads_only_an_index() {
        // 앱이 켤 때마다 센다 — 100만 편에서 문서마다 표를 찾아가면 10초가 넘는다(최종 검토 #2).
        let p = plan(super::COUNTS_SQL);
        assert!(p.contains("COVERING INDEX") && !p.contains("TEMP B-TREE"), "{p}");
    }

    #[test]
    fn the_last_clock_reading_comes_straight_from_an_index() {
        // 열 때마다 묻는다 — 판 기록 전체를 훑으면 기록이 쌓일수록 시작이 느려진다(최종 검토 #4).
        let p = plan(crate::store::LAST_HLC_SQL);
        assert!(p.contains("rev_hlc"), "{p}");
    }

    #[test]
    fn a_write_cut_after_the_revision_leaves_no_half_state() {
        // Review Focus 4 — 판을 넣은 뒤 문서 머리를 옮기기 전에 끊긴다.
        let app = std::env::temp_dir().join(format!("서재 시험 끊긴 저장 {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&app);
        let mut lib = Library::open(&app).unwrap();
        let s = lib.create(Namespace::Doc, "임진왜란", "첫 판", "").unwrap();
        lib.fail_after_rev = true;
        assert!(matches!(lib.save(s.doc, s.rev, "둘째 판", ""), Err(Error::Injected)));
        assert!(matches!(lib.delete(s.doc, s.rev, ""), Err(Error::Injected)));
        assert!(matches!(lib.rename(s.doc, s.rev, "임진 전쟁", true, ""), Err(Error::Injected)));
        assert!(matches!(lib.create(Namespace::Doc, "정유재란", "x", ""), Err(Error::Injected)));
        lib.fail_after_rev = false;
        let d = lib.get(s.doc).unwrap().unwrap();
        assert_eq!((d.head, d.body.as_str(), d.title.as_str()), (s.rev, "첫 판", "임진왜란"));
        let revs: i64 = lib.db.query_row("SELECT count(*) FROM rev", [], |r| r.get(0)).unwrap();
        let docs: i64 = lib.db.query_row("SELECT count(*) FROM doc", [], |r| r.get(0)).unwrap();
        assert_eq!((revs, docs), (1, 1));
        drop(lib);
        std::fs::remove_dir_all(&app).unwrap();
    }
}
