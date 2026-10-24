//! 동기화 — 묶음 내보내기 · 적용 · 머리 정하기 · 3-방향 합치기 · 합류 · 내보내기(spec/drive-format.md). 네트워크는 모른다.
use crate::docs::{decode, encode, load_head};
use crate::error::{Error, Result};
use crate::hlc::Hlc;
use crate::ids::{DeviceId, DocId, LibraryId, RevId};
use crate::ns::Namespace;
use crate::store::{Library, RevKind, Saved};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::Write;
use std::path::Path;

/// 열 때마다 — 0.0.1 서재에도 표를 더한다(형식 판은 1 그대로, 옛 엔진에는 모르는 표일 뿐).
pub(crate) const SYNC_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS held(id BLOB PRIMARY KEY, rev TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS conflict(doc BLOB PRIMARY KEY) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS adopted(device BLOB PRIMARY KEY, upto INTEGER NOT NULL) WITHOUT ROWID;
";

/// 내 판 — 이 기기의 판과, 물려받은 기기의 그때까지 판(합류 · 새 기기 ID, spec 3절).
const MINE: &str = "(device = ?2 OR EXISTS (SELECT 1 FROM adopted a WHERE a.device = rev.device AND rev.seq <= a.upto))";

/// 합친 판 ID의 이름에 넣는 합치기 판 — diff3 · 공통 조상 규칙이 바뀌면 올린다(다른 판끼리 같은 ID로 다른 본문을 만들지 않게).
const MERGE_VERSION: &[u8] = b"merge-v1";

const PACK_FORMAT: i64 = 1;
/// 한 묶음의 상한 — 판 수 · 본문 바이트, 먼저 닿는 것.
const MAX_REVS: usize = 500;
const MAX_BYTES: u64 = 4 << 20;
const ZSTD_LEVEL: i32 = 3;
const MERGE_MESSAGE: &str = "자동 합침";

/// 묶음 속 판 하나(spec/drive-format.md 2절). 보류 판도 이 모양으로 `held`에 남는다.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PackRev {
    id: String,
    doc: String,
    parent: Option<String>,
    parent2: Option<String>,
    device: String,
    hlc: u64,
    kind: i64,
    ns: String,
    title: String,
    #[serde(default)]
    message: String,
    body: String,
}

#[derive(Serialize, Deserialize)]
struct Pack {
    format: i64,
    library: String,
    device: String,
    no: u64,
    revs: Vec<PackRev>,
}

/// 저장소에 넣을 판 한 줄 — 받은 판 · 합친 판 · 견주어 합친 판.
struct Row<'a> {
    id: RevId,
    doc: DocId,
    parent: Option<RevId>,
    parent2: Option<RevId>,
    device: DeviceId,
    hlc: Hlc,
    kind: RevKind,
    ns: Namespace,
    title: &'a str,
    message: &'a str,
    body: &'a str,
}

/// 내보낸 묶음.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exported {
    /// zstd로 누른 묶음 바이트.
    pub pack: Vec<u8>,
    /// 묶음에 든 마지막 판의 `rev.seq`.
    pub seq: i64,
    pub count: usize,
    /// 상한에 닿아 남은 판이 있다 — 다음 번호로 이어서 낸다.
    pub more: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncStatus {
    pub seq: i64,
    /// `after_seq` 뒤에 이 기기가 만든 판 수.
    pub pending: u64,
    pub held: u64,
    /// 충돌로 남은 문서 수.
    pub conflicts: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub added: u64,
    pub skipped: u64,
    /// 지금 보류 중인 판 수.
    pub held: u64,
    pub forked: u64,
    pub merged: u64,
    pub conflicts: Vec<DocId>,
    pub revived: Vec<DocId>,
    /// 판이 든 문서 — 색인이 다시 본다.
    pub touched: Vec<DocId>,
}

fn bad_pack(why: impl Into<String>) -> Error {
    Error::BadPack(why.into())
}

fn insert_row(db: &Connection, r: &Row) -> Result<()> {
    db.prepare_cached(
        "INSERT INTO rev(id, doc, parent, parent2, device, hlc, kind, ns, title, message, len, body)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?
    .execute(params![r.id, r.doc, r.parent, r.parent2, r.device, r.hlc, r.kind, r.ns, r.title, r.message, r.body.len() as i64, encode(r.body)?])?;
    Ok(())
}

fn rev_known(db: &Connection, id: RevId) -> Result<bool> {
    Ok(db.prepare_cached("SELECT 1 FROM rev WHERE id = ?1")?.query_row(params![id], |_| Ok(())).optional()?.is_some())
}

fn in_conflict(db: &Connection, doc: DocId) -> Result<bool> {
    Ok(db.prepare_cached("SELECT 1 FROM conflict WHERE doc = ?1")?.query_row(params![doc], |_| Ok(())).optional()?.is_some())
}

/// 머리를 정할 때 보는 판 하나(본문 없이).
#[derive(Clone)]
struct Node {
    parent: Option<RevId>,
    parent2: Option<RevId>,
    hlc: Hlc,
    kind: RevKind,
    ns: Namespace,
    title: String,
}

fn nodes(db: &Connection, doc: DocId) -> Result<BTreeMap<RevId, Node>> {
    let mut st = db.prepare_cached("SELECT id, parent, parent2, hlc, kind, ns, title FROM rev WHERE doc = ?1")?;
    let rows = st.query_map(params![doc], |r| {
        Ok((r.get::<_, RevId>(0)?, Node { parent: r.get(1)?, parent2: r.get(2)?, hlc: r.get(3)?, kind: r.get(4)?, ns: r.get(5)?, title: r.get(6)? }))
    })?;
    Ok(rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()?)
}

/// 잎 — 다른 판의 앞 판이 아닌 판, (hlc, id) 차례.
fn leaves(nodes: &BTreeMap<RevId, Node>) -> Vec<RevId> {
    let parents: HashSet<RevId> = nodes.values().flat_map(|n| [n.parent, n.parent2]).flatten().collect();
    let mut out: Vec<RevId> = nodes.keys().copied().filter(|id| !parents.contains(id)).collect();
    out.sort_by_key(|id| (nodes[id].hlc, *id));
    out
}

fn ancestors(nodes: &BTreeMap<RevId, Node>, of: RevId) -> HashSet<RevId> {
    let mut seen = HashSet::new();
    let mut stack: Vec<RevId> = nodes.get(&of).map(|n| [n.parent, n.parent2].into_iter().flatten().collect()).unwrap_or_default();
    while let Some(id) = stack.pop() {
        if seen.insert(id) {
            if let Some(n) = nodes.get(&id) {
                stack.extend([n.parent, n.parent2].into_iter().flatten());
            }
        }
    }
    seen
}

/// 5-1 — 가장 가까운 공통 조상들 가운데 (hlc, id)가 가장 큰 것.
fn common_ancestor(nodes: &BTreeMap<RevId, Node>, x: RevId, y: RevId) -> Option<RevId> {
    let (ax, ay) = (ancestors(nodes, x), ancestors(nodes, y));
    let common: HashSet<RevId> = ax.intersection(&ay).copied().collect();
    let shadowed: HashSet<RevId> = common.iter().flat_map(|&c| ancestors(nodes, c)).collect();
    common.into_iter().filter(|c| !shadowed.contains(c)).max_by_key(|c| (nodes[c].hlc, *c))
}

fn body_of(db: &Connection, rev: RevId) -> Result<String> {
    let blob: Vec<u8> = db.prepare_cached("SELECT body FROM rev WHERE id = ?1")?.query_row(params![rev], |r| r.get(0))?;
    decode(&blob)
}

/// 조상의 각 줄이 다른 쪽 몇째 줄과 짝지어지는지(가장 긴 공통 부분열).
fn matches(base: &[&str], other: &[&str]) -> Vec<Option<usize>> {
    let mut m = vec![None; base.len()];
    for op in similar::capture_diff_slices(similar::Algorithm::Myers, base, other) {
        if let similar::DiffOp::Equal { old_index, new_index, len } = op {
            for k in 0..len {
                m[old_index + k] = Some(new_index + k);
            }
        }
    }
    m
}

/// 줄 단위 diff3(5-2). 겹치면 None.
pub(crate) fn merge_text(base: &str, a: &str, b: &str) -> Option<String> {
    let (base, a, b): (Vec<&str>, Vec<&str>, Vec<&str>) = (base.split_inclusive('\n').collect(), a.split_inclusive('\n').collect(), b.split_inclusive('\n').collect());
    let (ma, mb) = (matches(&base, &a), matches(&base, &b));
    let (mut i, mut ja, mut jb) = (0, 0, 0);
    let mut out = String::new();
    loop {
        let mut k = i;
        while k < base.len() && !(ma[k].is_some() && mb[k].is_some()) {
            k += 1;
        }
        let (ea, eb) = if k < base.len() { (ma[k].unwrap(), mb[k].unwrap()) } else { (a.len(), b.len()) };
        let (bc, ac, cc) = (&base[i..k], &a[ja..ea], &b[jb..eb]);
        let take = if ac == bc {
            cc
        } else if cc == bc || ac == cc {
            ac
        } else {
            return None;
        };
        out.extend(take.iter().copied());
        if k >= base.len() {
            return Some(out);
        }
        out.push_str(base[k]);
        (i, ja, jb) = (k + 1, ea + 1, eb + 1);
    }
}

/// 제목은 값 하나로 — 한쪽만 바꿨으면 그쪽, 같으면 그것, 다르게 바꿨으면 None.
fn merge_value<'a>(base: Option<&str>, a: &'a str, b: &'a str) -> Option<&'a str> {
    if a == b {
        Some(a)
    } else if base == Some(a) {
        Some(b)
    } else if base == Some(b) {
        Some(a)
    } else {
        None
    }
}

/// 합친 판의 ID — 같은 두 앞 판이면 어느 기기에서든 같다(spec 5-2).
fn merge_id(p1: RevId, p2: RevId, hlc: Hlc) -> RevId {
    let mut name = MERGE_VERSION.to_vec();
    name.extend_from_slice(p1.as_bytes());
    name.extend_from_slice(p2.as_bytes());
    let mut bytes = *uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, &name).as_bytes();
    bytes[..6].copy_from_slice(&hlc.wall_ms().to_be_bytes()[2..]);
    RevId(uuid::Builder::from_custom_bytes(bytes).into_uuid())
}

/// 머리를 다시 정한 결과.
#[derive(Default)]
pub(crate) struct Reheaded {
    forked: bool,
    merged: u64,
    conflict: bool,
    revived: bool,
    /// 합친 판 가운데 가장 늦은 시각(시계를 그 뒤로).
    last: Option<Hlc>,
}

/// 5절 — 잎으로 머리 · 지움 · 충돌을 정하고, 내용 잎이 여럿이면 합친다. 합친 판은 `device`의 것.
pub(crate) fn rehead(db: &Connection, doc: DocId, device: DeviceId) -> Result<Reheaded> {
    let mut nodes = nodes(db, doc)?;
    let mut out = Reheaded::default();
    let all = leaves(&nodes);
    let mut content: Vec<RevId> = all.iter().copied().filter(|id| nodes[id].kind == RevKind::Content).collect();
    out.forked = content.len() > 1;
    while content.len() > 1 {
        let (x, y) = (content[0], content[1]);
        let base = common_ancestor(&nodes, x, y);
        let (nx, ny) = (nodes[&x].clone(), nodes[&y].clone());
        let base_title = base.map(|b| nodes[&b].title.clone());
        let base_body = match base {
            Some(b) => body_of(db, b)?,
            None => String::new(),
        };
        let (bx, by) = (body_of(db, x)?, body_of(db, y)?);
        let (Some(title), Some(body)) = (merge_value(base_title.as_deref(), &nx.title, &ny.title), merge_text(&base_body, &bx, &by)) else {
            out.conflict = true;
            break;
        };
        let (p1, p2) = if x < y { (x, y) } else { (y, x) };
        let top = nx.hlc.max(ny.hlc);
        let (id, hlc) = (merge_id(p1, p2, top), Hlc(top.0 + 1));
        let title = title.to_string();
        insert_row(
            db,
            &Row { id, doc, parent: Some(p1), parent2: Some(p2), device, hlc, kind: RevKind::Content, ns: nx.ns, title: &title, message: MERGE_MESSAGE, body: &body },
        )?;
        nodes.insert(id, Node { parent: Some(p1), parent2: Some(p2), hlc, kind: RevKind::Content, ns: nx.ns, title });
        out.merged += 1;
        out.last = out.last.max(Some(hlc));
        content.drain(..2);
        content.push(id);
        content.sort_by_key(|id| (nodes[id].hlc, *id));
    }
    let head = match content.last() {
        Some(&h) => h,
        None => *all.last().ok_or_else(|| Error::Corrupt(format!("문서 {doc}에 판이 없음")))?,
    };
    let n = &nodes[&head];
    let deleted = n.kind == RevKind::Deleted;
    let was_deleted: Option<bool> = db.prepare_cached("SELECT deleted FROM doc WHERE id = ?1")?.query_row(params![doc], |r| r.get(0)).optional()?;
    out.revived = was_deleted == Some(true) && !deleted;
    db.prepare_cached("UPDATE doc SET title = ?2, head = ?3, updated = ?4, deleted = ?5 WHERE id = ?1")?.execute(params![doc, n.title, head, n.hlc, deleted])?;
    if out.conflict {
        db.prepare_cached("INSERT OR IGNORE INTO conflict(doc) VALUES (?1)")?.execute(params![doc])?;
    } else {
        db.prepare_cached("DELETE FROM conflict WHERE doc = ?1")?.execute(params![doc])?;
    }
    Ok(out)
}

/// 이 기기에서 고친 문서가 충돌 중이었으면 머리를 다시 정한다 — 받는 기기와 같은 규칙이어야 모인다.
/// (합친 판의 마지막 시각, 지금 머리) — 충돌이 아니었으면 머리는 방금 더한 `rev`.
pub(crate) fn after_local_write(db: &Connection, doc: DocId, device: DeviceId, rev: RevId) -> Result<(Option<Hlc>, RevId)> {
    if !in_conflict(db, doc)? {
        return Ok((None, rev));
    }
    let last = rehead(db, doc, device)?.last;
    Ok((last, load_head(db, doc)?.head))
}

fn parse_rev(p: &PackRev) -> Result<Row<'_>> {
    let id = |s: &str| uuid::Uuid::parse_str(s).map_err(|_| bad_pack(format!("UUID 아님: {s}")));
    let opt = |s: &Option<String>| -> Result<Option<RevId>> { s.as_deref().map(|s| id(s).map(RevId)).transpose() };
    Ok(Row {
        id: RevId(id(&p.id)?),
        doc: DocId(id(&p.doc)?),
        parent: opt(&p.parent)?,
        parent2: opt(&p.parent2)?,
        device: DeviceId(id(&p.device)?),
        hlc: Hlc(p.hlc),
        kind: match p.kind {
            0 => RevKind::Content,
            1 => RevKind::Deleted,
            k => return Err(bad_pack(format!("모르는 kind: {k}"))),
        },
        ns: Namespace::from_code(&p.ns).ok_or_else(|| bad_pack(format!("모르는 ns: {}", p.ns)))?,
        title: &p.title,
        message: &p.message,
        body: &p.body,
    })
}

impl Library {
    /// `after_seq` 뒤에 이 기기가 만든 판을 묶음 `no`로(3절). 없으면 None.
    pub fn export(&self, after_seq: i64, no: u64) -> Result<Option<Exported>> {
        let device = self.info.device;
        let mut st = self.db.prepare_cached(
            &format!("SELECT seq, id, doc, parent, parent2, hlc, kind, ns, title, message, len, body, device FROM rev WHERE seq > ?1 AND {MINE} ORDER BY seq"),
        )?;
        let mut rows = st.query(params![after_seq, device])?;
        let (mut revs, mut bytes, mut seq, mut more) = (Vec::new(), 0u64, after_seq, false);
        while let Some(r) = rows.next()? {
            let len = r.get::<_, i64>(10)? as u64;
            if revs.len() == MAX_REVS || (!revs.is_empty() && bytes + len > MAX_BYTES) {
                more = true;
                break;
            }
            let kind: RevKind = r.get(6)?;
            let ns: Namespace = r.get(7)?;
            revs.push(PackRev {
                id: r.get::<_, RevId>(1)?.to_string(),
                doc: r.get::<_, DocId>(2)?.to_string(),
                parent: r.get::<_, Option<RevId>>(3)?.map(|p| p.to_string()),
                parent2: r.get::<_, Option<RevId>>(4)?.map(|p| p.to_string()),
                device: r.get::<_, DeviceId>(12)?.to_string(),
                hlc: r.get::<_, Hlc>(5)?.0,
                kind: if kind == RevKind::Deleted { 1 } else { 0 },
                ns: ns.code().to_string(),
                title: r.get(8)?,
                message: r.get(9)?,
                body: decode(&r.get::<_, Vec<u8>>(11)?)?,
            });
            bytes += len;
            seq = r.get(0)?;
        }
        if revs.is_empty() {
            return Ok(None);
        }
        let count = revs.len();
        let pack = Pack { format: PACK_FORMAT, library: self.info.library.to_string(), device: device.to_string(), no, revs };
        let json = serde_json::to_vec(&pack).map_err(|e| Error::Corrupt(format!("묶음 쓰기: {e}")))?;
        Ok(Some(Exported { pack: zstd::bulk::compress(&json, ZSTD_LEVEL)?, seq, count, more }))
    }

    pub fn sync_status(&self, after_seq: i64) -> Result<SyncStatus> {
        let count = |sql: &str, p: &[&dyn rusqlite::ToSql]| -> Result<u64> { Ok(self.db.prepare_cached(sql)?.query_row(p, |r| r.get::<_, i64>(0))? as u64) };
        Ok(SyncStatus {
            seq: self.last_seq()?,
            pending: count(&format!("SELECT count(*) FROM rev WHERE seq > ?1 AND {MINE}"), &[&after_seq, &self.info.device])?,
            held: count("SELECT count(*) FROM held", &[])?,
            conflicts: count("SELECT count(*) FROM conflict", &[])?,
        })
    }

    /// 묶음을 든다(4절) — 트랜잭션 하나.
    pub fn apply(&mut self, pack: &[u8]) -> Result<Applied> {
        let json = zstd::stream::decode_all(pack).map_err(|e| bad_pack(format!("풀 수 없음: {e}")))?;
        let pack: Pack = serde_json::from_slice(&json).map_err(|e| bad_pack(format!("모양이 틀림: {e}")))?;
        if pack.format != PACK_FORMAT {
            return Err(bad_pack(format!("모르는 묶음 형식: {}", pack.format)));
        }
        let library = LibraryId::parse(&pack.library).ok_or_else(|| bad_pack("library가 UUID가 아님"))?;
        if library != self.info.library {
            return Err(Error::WrongLibrary(library));
        }
        let device = self.info.device;
        let tx = self.db.transaction()?;
        let held: Vec<PackRev> = {
            let mut st = tx.prepare("SELECT rev FROM held")?;
            let rows = st.query_map([], |r| r.get::<_, String>(0))?;
            let texts = rows.collect::<rusqlite::Result<Vec<_>>>()?;
            texts.iter().map(|t| serde_json::from_str(t).map_err(|e| Error::Corrupt(format!("보류 판: {e}")))).collect::<Result<_>>()?
        };
        tx.execute("DELETE FROM held", [])?;
        let from_pack: HashSet<&str> = pack.revs.iter().map(|r| r.id.as_str()).collect();
        let mut seen = HashSet::new();
        let mut queue: Vec<&PackRev> = held.iter().chain(&pack.revs).filter(|r| seen.insert(r.id.as_str())).collect();
        let mut out = Applied::default();
        let mut touched = BTreeSet::new();
        let mut last = Hlc(0);
        let mut progress = true;
        while progress && !queue.is_empty() {
            progress = false;
            let mut wait = Vec::new();
            for p in queue {
                let r = parse_rev(p)?;
                if rev_known(&tx, r.id)? {
                    if from_pack.contains(p.id.as_str()) {
                        out.skipped += 1;
                    }
                    continue;
                }
                let ready = [r.parent, r.parent2].into_iter().flatten().try_fold(true, |ok, id| rev_known(&tx, id).map(|k| ok && k))?;
                if !ready {
                    wait.push(p);
                    continue;
                }
                insert_row(&tx, &r)?;
                tx.prepare_cached("INSERT OR IGNORE INTO doc(id, ns, title, head, created, updated, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)")?
                    .execute(params![r.doc, r.ns, r.title, r.id, r.hlc, r.kind == RevKind::Deleted])?;
                touched.insert(r.doc);
                last = last.max(r.hlc);
                out.added += 1;
                progress = true;
            }
            queue = wait;
        }
        for p in &queue {
            tx.prepare_cached("INSERT OR IGNORE INTO held(id, rev) VALUES (?1, ?2)")?.execute(params![
                parse_rev(p)?.id,
                serde_json::to_string(p).map_err(|e| Error::Corrupt(format!("보류 판: {e}")))?
            ])?;
        }
        out.held = tx.query_row("SELECT count(*) FROM held", [], |r| r.get::<_, i64>(0))? as u64;
        for &doc in &touched {
            let h = rehead(&tx, doc, device)?;
            out.forked += h.forked as u64;
            out.merged += h.merged;
            if h.conflict {
                out.conflicts.push(doc);
            }
            if h.revived {
                out.revived.push(doc);
            }
            last = last.max(h.last.unwrap_or(Hlc(0)));
        }
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        tx.commit()?;
        self.clock.observe(last);
        out.touched = touched.into_iter().collect();
        Ok(out)
    }

    /// 머리가 아닌 내용 잎들(새 것부터) — 충돌이 아니면 빈 목록(5-3).
    pub fn conflict_leaves(&self, doc: DocId) -> Result<Vec<RevId>> {
        if !in_conflict(&self.db, doc)? {
            return Ok(Vec::new());
        }
        let head = load_head(&self.db, doc)?.head;
        let nodes = nodes(&self.db, doc)?;
        let mut out: Vec<RevId> = leaves(&nodes).into_iter().filter(|id| *id != head && nodes[id].kind == RevKind::Content).collect();
        out.reverse();
        Ok(out)
    }

    /// 견주어 합쳐 저장한다 — `parent` = 머리(`base`), `parent2` = 다른 잎 `merge`(5-3).
    pub fn save_merge(&mut self, doc: DocId, base: RevId, merge: RevId, body: &str, message: &str) -> Result<Saved> {
        let body = &*crate::docs::lf(body);
        let device = self.info.device;
        let tx = self.db.transaction()?;
        let h = load_head(&tx, doc)?;
        if h.head != base {
            return Err(Error::StaleBase { head: h.head });
        }
        let nodes = nodes(&tx, doc)?;
        if merge == base || !leaves(&nodes).contains(&merge) || nodes[&merge].kind != RevKind::Content {
            return Err(Error::NotALeaf(merge));
        }
        let (rev, hlc) = (RevId::new(), self.clock.now());
        insert_row(&tx, &Row { id: rev, doc, parent: Some(base), parent2: Some(merge), device, hlc, kind: RevKind::Content, ns: h.ns, title: &h.title, message, body })?;
        let last = rehead(&tx, doc, device)?.last;
        let head = load_head(&tx, doc)?.head;
        tx.commit()?;
        self.clock.observe(last.unwrap_or(hlc));
        Ok(Saved { doc, rev, hlc, changed: true, head })
    }

    /// 드라이브의 서재에 합류한다(6절) — 서재 ID를 바꾸고 보류 판을 비운다. 이 기기에 있는 다른 기기의 판도 물려받아
    /// 다음 내보내기에 함께 싣는다 — 받아 둔 문서도 새 서재로 가고, 그 위에 고친 판이 보류되지 않게(최종 검토 I1).
    pub fn join(&mut self, library: LibraryId) -> Result<()> {
        let tx = self.db.transaction()?;
        tx.execute("UPDATE meta SET value = ?1 WHERE key = 'library'", params![library.to_string()])?;
        tx.execute("DELETE FROM held", [])?;
        adopt(&tx, "SELECT DISTINCT device FROM rev WHERE device != ?1", self.info.device)?;
        tx.commit()?;
        self.info.library = library;
        Ok(())
    }

    /// 새 기기 ID(6절) — 자료 폴더를 복사했거나 내보낸 ZIP을 되살려 두 곳이 한 ID를 쓰게 됐을 때(최종 검토 C1). 옛 ID의 지금까지 판은
    /// 물려받아 다음 내보내기에 함께 싣고, 그 뒤 옛 ID로 오는 판(복제된 다른 쪽)은 남의 판으로 받는다.
    pub fn rotate_device(&mut self) -> Result<DeviceId> {
        let (old, new) = (self.info.device, DeviceId::new());
        let tx = self.db.transaction()?;
        rotate(&tx, old, new)?;
        tx.commit()?;
        self.info.device = new;
        Ok(new)
    }

    /// `library.db`의 한 때 사본과 읽어보기를 ZIP 하나로(7절). 다 쓴 뒤에 이름을 바꿔 반쯤 쓴 파일이 남지 않는다.
    pub fn backup(&self, path: &Path) -> Result<()> {
        let copy = self.info.folder.join("library.backup.tmp");
        let _ = std::fs::remove_file(&copy);
        self.db.execute("VACUUM INTO ?1", params![copy.to_string_lossy()])?;
        // 되살린 서재는 새 기기다 — 같은 기기 ID를 쓰면 이 기기가 그 뒤 올린 판을 「내 것」이라 받지 않는다(최종 검토 C1).
        {
            let mut c = Connection::open(&copy)?;
            let tx = c.transaction()?;
            rotate(&tx, self.info.device, DeviceId::new())?;
            tx.commit()?;
        }
        let partial = path.with_extension("zip.tmp");
        let written = (|| -> Result<()> {
            let counts = self.counts()?;
            let readme = format!(
                "역사 서재 내보내기\n\n만든 때(UTC): {}\n서재: {}\n문서: {}편(지운 것 {}편)\n판: {}개\n\n\
                 library.db는 서재 원본입니다. 앱 자료 폴더의 HistoryCore\\library.db 자리에 두면 그 서재로 열립니다.\n",
                utc_text(crate::hlc::Clock::system()()),
                self.info.library,
                counts.total(),
                counts.deleted,
                self.last_seq()?
            );
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&partial)?);
            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("library.db", options).map_err(zip_error)?;
            std::io::copy(&mut std::fs::File::open(&copy)?, &mut zip)?;
            zip.start_file("읽어보기.txt", options).map_err(zip_error)?;
            zip.write_all(readme.as_bytes())?;
            zip.finish().map_err(zip_error)?.sync_all()?;
            Ok(())
        })();
        let _ = std::fs::remove_file(&copy);
        if let Err(e) = written {
            let _ = std::fs::remove_file(&partial);
            return Err(e);
        }
        std::fs::rename(&partial, path)?;
        Ok(())
    }
}

/// 지금까지의 판 번호까지 그 기기들을 물려받는다(이미 물려받았으면 더 늦은 번호로). [devices]는 ?1 = 이 기기를 받는 질의.
fn adopt(db: &Connection, devices: &str, me: DeviceId) -> Result<()> {
    let upto: i64 = db.query_row("SELECT COALESCE(MAX(seq), 0) FROM rev", [], |r| r.get(0))?;
    db.execute(
        &format!("INSERT INTO adopted(device, upto) SELECT device, ?2 FROM ({devices}) WHERE true ON CONFLICT(device) DO UPDATE SET upto = max(upto, excluded.upto)"),
        params![me, upto],
    )?;
    Ok(())
}

fn rotate(db: &Connection, old: DeviceId, new: DeviceId) -> Result<()> {
    adopt(db, "SELECT ?1 AS device", old)?;
    db.execute("UPDATE meta SET value = ?1 WHERE key = 'device'", params![new.to_string()])?;
    Ok(())
}

fn zip_error(e: zip::result::ZipError) -> Error {
    Error::Io(std::io::Error::other(e))
}

/// 1970년부터 ms → `YYYY-MM-DD HH:MM`(UTC).
fn utc_text(ms: u64) -> String {
    let (days, secs) = ((ms / 86_400_000) as i64, (ms / 1000) % 86_400);
    // 날 수 → 그레고리력 날짜(Howard Hinnant의 civil_from_days).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", secs / 3600, (secs / 60) % 60)
}

#[cfg(test)]
mod tests {
    use super::merge_text;
    use crate::{Error, Library, Namespace};

    #[test]
    fn diff3_takes_each_sides_change_and_stops_at_overlap() {
        let base = "가\n나\n다\n라\n";
        assert_eq!(merge_text(base, "가가\n나\n다\n라\n", "가\n나\n다\n라라\n").as_deref(), Some("가가\n나\n다\n라라\n"));
        assert_eq!(merge_text(base, "가\n다\n라\n", "가\n나\n다\n라\n마\n").as_deref(), Some("가\n다\n라\n마\n"));
        assert_eq!(merge_text(base, "가\n나나\n다\n라\n", "가\n나나\n다\n라\n").as_deref(), Some("가\n나나\n다\n라\n"), "같게 바꿈");
        assert_eq!(merge_text(base, "가\n하나\n다\n라\n", "가\n둘\n다\n라\n"), None);
        assert_eq!(merge_text("", "가\n", "나\n"), None, "같은 자리에 다른 글");
        assert_eq!(merge_text("가", "가\n나", "가").as_deref(), Some("가\n나"), "줄끝 없는 마지막 줄");
    }

    #[test]
    fn a_merge_id_is_pinned() {
        // 정답지(최종 검토 I5) — 합친 판 ID가 판마다 바뀌면 기기끼리 같은 ID로 다른 본문을 만든다. 바꿀 때는 MERGE_VERSION을 올린다.
        let p = |s: &str| crate::RevId::parse(s).unwrap();
        let id = super::merge_id(p("0192f0a0-0000-7000-8000-000000000001"), p("0192f0a0-0000-7000-8000-000000000002"), crate::Hlc::new(1_792_800_000_000, 3));
        assert_eq!(id.to_string(), "01a16b35-0800-8649-8284-18c97a7eea5c");
        let body = merge_text("하나\n둘\n셋\n", "하나!\n둘\n셋\n", "하나\n둘\n셋!\n").unwrap();
        assert_eq!(body.as_bytes(), "하나!\n둘\n셋!\n".as_bytes());
    }

    #[test]
    fn utc_text_reads_like_a_date() {
        assert_eq!(super::utc_text(0), "1970-01-01 00:00");
        assert_eq!(super::utc_text(1_792_800_000_000), "2026-10-24 00:00");
    }

    #[test]
    fn an_apply_cut_before_commit_leaves_nothing() {
        // 설계 9-3 실패 주입 — 적용 도중 끊어도 저장소는 온전하다.
        let dir = |n: &str| {
            let d = std::env::temp_dir().join(format!("서재 동기화 끊긴 적용 {n} {}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            d
        };
        let mut a = Library::open(&dir("가")).unwrap();
        let mut b = Library::open(&dir("나")).unwrap();
        b.join(a.info().library).unwrap();
        let s = a.create(Namespace::Doc, "임진왜란", "본문", "").unwrap();
        let first = a.export(0, 1).unwrap().unwrap();
        a.save(s.doc, s.rev, "둘째", "").unwrap();
        let second = a.export(first.seq, 2).unwrap().unwrap();
        b.apply(&second.pack).unwrap(); // 보류 하나
        b.fail_after_rev = true;
        assert!(matches!(b.apply(&first.pack), Err(Error::Injected)));
        b.fail_after_rev = false;
        assert!(b.get(s.doc).unwrap().is_none());
        let st = b.sync_status(0).unwrap();
        assert_eq!((st.seq, st.held), (0, 1), "판도 머리도 없고 보류 판은 그대로");
        assert_eq!(b.apply(&first.pack).unwrap().added, 2);
    }
}
