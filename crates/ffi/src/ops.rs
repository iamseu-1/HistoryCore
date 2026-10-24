//! 서재 일의 다리 — `library.*` · `doc.*` · `rev.*` · 색인 op · `sync.*`(spec/engine-api.md). 열린 서재(엔진 = 서재 + 색인)는 손잡이 표에 든다.
use history_index::{Children, Engine, Hit, HomeOptions, LibraryTemplates, Opened, Wanted};
use base64::Engine as _;
use history_library::{Applied, Counts, Doc, DocId, DocSummary, Error, Hlc, Library, LibraryId, Namespace, RevId, RevInfo, Saved};
use history_namumark::{render, NoTemplates, RenderOptions};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

struct Open {
    next: u64,
    libraries: HashMap<u64, Engine>,
}

/// 답 — 보통은 JSON 값, `doc.open`은 그리기 모델 글을 풀지 않고 끼운다.
pub enum Reply {
    Json(Value),
    WithModel(Value, String),
}

fn table() -> MutexGuard<'static, Open> {
    static TABLE: OnceLock<Mutex<Open>> = OnceLock::new();
    TABLE
        .get_or_init(|| Mutex::new(Open { next: 1, libraries: HashMap::new() }))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

type Answer = Result<Value, Value>;

const LIBRARY_OPS: [&str; 9] = ["library.counts", "doc.create", "doc.save", "doc.rename", "doc.delete", "doc.get", "doc.by_title", "doc.history", "rev.get"];
const SYNC_OPS: [&str; 5] = ["sync.export", "sync.apply", "sync.status", "sync.join", "library.backup"];
const INDEX_OPS: [&str; 11] =
    ["titles.exist", "search.titles", "search.body", "tree.children", "links.back", "links.wanted", "home.aggregates", "stats.library", "timeline.docs", "index.status", "index.step"];

/// 서재 op면 답을, 아니면 None(모르는 op).
pub fn handle_op(op: &str, req: &Value) -> Option<Reply> {
    let answer = match op {
        "library.open" => open(req),
        "library.close" => close(req),
        "namumark.parse" => parse_op(req),
        "doc.open" => return Some(with_engine(req, |e| open_doc(e, req)).map_or_else(Reply::Json, |(v, model)| Reply::WithModel(v, model))),
        op if LIBRARY_OPS.contains(&op) => with_engine(req, |e| run(op, e, req)),
        op if SYNC_OPS.contains(&op) => with_engine(req, |e| run_sync(op, e, req)),
        op if INDEX_OPS.contains(&op) => with_engine(req, |e| run_index(op, e, req)),
        _ => return None,
    };
    Some(Reply::Json(answer.unwrap_or_else(|e| e)))
}

fn bad(message: impl Into<String>) -> Value {
    json!({"ok": false, "error": message.into(), "code": "bad_request"})
}

fn fail(e: Error) -> Value {
    let mut v = json!({"ok": false, "error": e.to_string(), "code": e.code()});
    if let Error::StaleBase { head } = &e {
        v["head"] = json!(head.to_string());
    }
    if let Error::WrongLibrary(found) = &e {
        v["found"] = json!(found.to_string());
    }
    v
}

fn text<'a>(req: &'a Value, key: &str) -> Result<&'a str, Value> {
    req.get(key).and_then(Value::as_str).ok_or_else(|| bad(format!("`{key}`(글)가 없음")))
}

fn optional_text<'a>(req: &'a Value, key: &str) -> &'a str {
    req.get(key).and_then(Value::as_str).unwrap_or("")
}

fn doc_id(req: &Value, key: &str) -> Result<DocId, Value> {
    DocId::parse(text(req, key)?).ok_or_else(|| bad(format!("`{key}`가 UUID가 아님")))
}

fn rev_id(req: &Value, key: &str) -> Result<RevId, Value> {
    RevId::parse(text(req, key)?).ok_or_else(|| bad(format!("`{key}`가 UUID가 아님")))
}

fn namespace(req: &Value) -> Result<Namespace, Value> {
    let code = text(req, "ns")?;
    Namespace::from_code(code).ok_or_else(|| bad(format!("모르는 ns: {code}")))
}

fn handle_of(req: &Value) -> Result<u64, Value> {
    req.get("handle").and_then(Value::as_u64).ok_or_else(|| bad("`handle`(수)이 없음"))
}

fn bad_handle(h: u64) -> Value {
    json!({"ok": false, "error": format!("열린 서재가 아님: {h}"), "code": "bad_handle"})
}

fn same_folder(a: &Path, b: &Path) -> bool {
    matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
}

fn opened(h: u64, lib: &Library) -> Value {
    let i = lib.info();
    json!({"ok": true, "handle": h, "library": i.library.to_string(), "device": i.device.to_string(), "format": i.format, "folder": i.folder.to_string_lossy()})
}

fn open(req: &Value) -> Answer {
    let dir = PathBuf::from(text(req, "dir")?);
    let folder = dir.join(history_library::FOLDER);
    let mut t = table();
    if let Some((&h, e)) = t.libraries.iter().find(|(_, e)| same_folder(&e.lib.info().folder, &folder)) {
        return Ok(opened(h, &e.lib));
    }
    let e = Engine::open(&dir).map_err(fail)?;
    let h = t.next;
    t.next += 1;
    let answer = opened(h, &e.lib);
    t.libraries.insert(h, e);
    Ok(answer)
}

fn close(req: &Value) -> Answer {
    let h = handle_of(req)?;
    table().libraries.remove(&h).map(|_| json!({"ok": true})).ok_or_else(|| bad_handle(h))
}

fn with_engine<T>(req: &Value, f: impl FnOnce(&mut Engine) -> Result<T, Value>) -> Result<T, Value> {
    let h = handle_of(req)?;
    let mut t = table();
    let e = t.libraries.get_mut(&h).ok_or_else(|| bad_handle(h))?;
    f(e)
}

fn saved(s: &Saved) -> Value {
    json!({"ok": true, "doc": s.doc.to_string(), "rev": s.rev.to_string(), "hlc": s.hlc.0, "changed": s.changed})
}

fn doc_json(d: &Doc) -> Value {
    json!({"id": d.id.to_string(), "ns": d.ns.code(), "title": d.title, "head": d.head.to_string(), "created": d.created.0, "updated": d.updated.0, "deleted": d.deleted, "body": d.body})
}

fn summary_json(d: &DocSummary) -> Value {
    json!({"id": d.id.to_string(), "ns": d.ns.code(), "title": d.title, "head": d.head.to_string(), "created": d.created.0, "updated": d.updated.0})
}

fn rev_json(r: &RevInfo) -> Value {
    json!({
        "id": r.id.to_string(), "doc": r.doc.to_string(),
        "parent": r.parent.map(|p| p.to_string()), "parent2": r.parent2.map(|p| p.to_string()),
        "device": r.device.to_string(), "hlc": r.hlc.0, "kind": r.kind.code(), "ns": r.ns.code(),
        "title": r.title, "message": r.message, "len": r.len,
    })
}

fn not_found(what: &str) -> Value {
    json!({"ok": false, "error": format!("없음: {what}"), "code": "not_found"})
}

fn counts_json(c: &Counts) -> Value {
    let mut v = json!({"deleted": c.deleted, "total": c.total()});
    for ns in Namespace::ALL {
        v[ns.code()] = json!(c.of(ns));
    }
    v
}

/// 쓰는 일은 엔진을 거친다(저장 뒤 그 문서를 색인), 읽는 일은 서재에서 바로.
fn run(op: &str, e: &mut Engine, req: &Value) -> Answer {
    match op {
        "library.counts" => {
            let mut v = counts_json(&e.lib.counts().map_err(fail)?);
            v["ok"] = json!(true);
            Ok(v)
        }
        "doc.create" => {
            let s = e.create(namespace(req)?, text(req, "title")?, text(req, "body")?, optional_text(req, "message")).map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.save" => {
            let (doc, base, body, message) = (doc_id(req, "doc")?, rev_id(req, "base")?, text(req, "body")?, optional_text(req, "message"));
            let s = match req.get("merge") {
                Some(Value::Null) | None => e.save(doc, base, body, message),
                Some(_) => e.save_merge(doc, base, rev_id(req, "merge")?, body, message),
            }
            .map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.delete" => {
            let s = e.delete(doc_id(req, "doc")?, rev_id(req, "base")?, optional_text(req, "message")).map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.rename" => {
            let keep = req.get("keep_redirect").and_then(Value::as_bool).unwrap_or(true);
            let r = e.rename(doc_id(req, "doc")?, rev_id(req, "base")?, text(req, "title")?, keep, optional_text(req, "message")).map_err(fail)?;
            Ok(json!({"ok": true, "rev": r.rev.to_string(), "hlc": r.hlc.0, "redirect": r.redirect.map(|d| d.to_string())}))
        }
        "doc.get" => {
            let id = doc_id(req, "doc")?;
            match e.lib.get(id).map_err(fail)? {
                Some(d) => {
                    let mut v = doc_json(&d);
                    v["conflict"] = json!(e.lib.conflict_leaves(id).map_err(fail)?.iter().map(|r| r.to_string()).collect::<Vec<_>>());
                    Ok(json!({"ok": true, "doc": v}))
                }
                None => Err(not_found(&format!("문서 {id}"))),
            }
        }
        "doc.by_title" => {
            let docs = e.lib.by_title(namespace(req)?, text(req, "title")?).map_err(fail)?;
            Ok(json!({"ok": true, "docs": docs.iter().map(summary_json).collect::<Vec<_>>()}))
        }
        "doc.history" => {
            let limit = req.get("limit").and_then(Value::as_u64).unwrap_or(50).min(1000) as usize;
            let before = req.get("before").and_then(Value::as_u64).map(Hlc);
            let revs = e.lib.history(doc_id(req, "doc")?, limit, before).map_err(fail)?;
            Ok(json!({"ok": true, "revisions": revs.iter().map(rev_json).collect::<Vec<_>>()}))
        }
        "rev.get" => {
            let id = rev_id(req, "rev")?;
            match e.lib.revision(id).map_err(fail)? {
                Some(r) => {
                    let mut v = rev_json(&r.info);
                    v["body"] = json!(r.body);
                    Ok(json!({"ok": true, "revision": v}))
                }
                None => Err(not_found(&format!("판 {id}"))),
            }
        }
        other => Err(bad(format!("모르는 서재 op: {other}"))),
    }
}

fn ids(docs: &[DocId]) -> Value {
    json!(docs.iter().map(|d| d.to_string()).collect::<Vec<_>>())
}

fn applied_json(r: &Applied) -> Value {
    json!({
        "ok": true, "added": r.added, "skipped": r.skipped, "held": r.held, "forked": r.forked, "merged": r.merged,
        "conflicts": ids(&r.conflicts), "revived": ids(&r.revived),
    })
}

fn seq_of(req: &Value) -> Result<i64, Value> {
    req.get("after_seq").and_then(Value::as_i64).ok_or_else(|| bad("`after_seq`(수)가 없음"))
}

/// 동기화 op(spec/drive-format.md) — 묶음 바이트는 base64 글로 오간다.
fn run_sync(op: &str, e: &mut Engine, req: &Value) -> Answer {
    match op {
        "sync.export" => {
            let no = req.get("no").and_then(Value::as_u64).ok_or_else(|| bad("`no`(수)가 없음"))?;
            Ok(match e.lib.export(seq_of(req)?, no).map_err(fail)? {
                Some(x) => json!({"ok": true, "pack": base64::engine::general_purpose::STANDARD.encode(&x.pack), "seq": x.seq, "count": x.count, "more": x.more}),
                None => json!({"ok": true, "pack": null, "seq": seq_of(req)?, "count": 0, "more": false}),
            })
        }
        "sync.apply" => {
            let pack = base64::engine::general_purpose::STANDARD.decode(text(req, "pack")?).map_err(|_| bad("`pack`이 base64가 아님"))?;
            Ok(applied_json(&e.apply(&pack).map_err(fail)?))
        }
        "sync.status" => {
            let s = e.lib.sync_status(seq_of(req)?).map_err(fail)?;
            let i = e.lib.info();
            Ok(json!({"ok": true, "library": i.library.to_string(), "device": i.device.to_string(), "seq": s.seq, "pending": s.pending, "held": s.held, "conflicts": s.conflicts}))
        }
        "sync.join" => {
            let library = LibraryId::parse(text(req, "library")?).ok_or_else(|| bad("`library`가 UUID가 아님"))?;
            e.join(library).map_err(fail)?;
            Ok(json!({"ok": true, "library": library.to_string()}))
        }
        "library.backup" => {
            e.lib.backup(Path::new(text(req, "path")?)).map_err(fail)?;
            Ok(json!({"ok": true}))
        }
        other => Err(bad(format!("모르는 동기화 op: {other}"))),
    }
}

/// 저장하지 않은 글의 그리기 모델(편집기 미리보기 · spec/namumark.md). `handle`이 있으면 그 서재의 틀을 펼친다.
/// 틀을 읽다 오류가 나면 「없는 틀」로 그리지 않고 오류로 답한다(P2-2 미룬 것).
fn parse_op(req: &Value) -> Answer {
    let text = text(req, "text")?.to_string();
    let options = RenderOptions {
        title: req.get("title").and_then(Value::as_str).map(str::to_string),
        template: req.get("ns").and_then(Value::as_str) == Some(Namespace::Template.code()),
    };
    let model = if req.get("handle").is_some() {
        with_engine(req, |e| {
            let templates = LibraryTemplates::new(&e.lib);
            let model = render(&text, &options, &templates);
            match templates.take_error() {
                Some(err) => Err(fail(err)),
                None => Ok(json!(model)),
            }
        })?
    } else {
        json!(render(&text, &options, &NoTemplates))
    };
    Ok(json!({"ok": true, "model": model}))
}

fn limit(req: &Value, default: u64, most: u64) -> usize {
    req.get("limit").and_then(Value::as_u64).unwrap_or(default).clamp(1, most) as usize
}

fn number(req: &Value, key: &str, default: u64, most: u64) -> usize {
    req.get(key).and_then(Value::as_u64).unwrap_or(default).min(most) as usize
}

fn summaries(docs: &[DocSummary]) -> Value {
    json!(docs.iter().map(summary_json).collect::<Vec<_>>())
}

fn wanted_json(w: &[Wanted]) -> Value {
    json!(w.iter().map(|w| json!({"ns": w.ns.code(), "title": w.title, "refs": w.refs})).collect::<Vec<_>>())
}

fn tz(req: &Value) -> i32 {
    req.get("tz").and_then(Value::as_i64).unwrap_or(0).clamp(-14 * 60, 14 * 60) as i32
}

/// `doc.open` — 그리기 모델 글은 따로 돌려준다(답에 풀지 않고 끼운다).
fn open_doc(e: &mut Engine, req: &Value) -> Result<(Value, String), Value> {
    let id = doc_id(req, "doc")?;
    let follow = req.get("follow").and_then(Value::as_bool).unwrap_or(true);
    let Some(Opened { doc, deleted, model, redirected_from, stale }) = e.open_doc(id, follow).map_err(fail)? else {
        return Err(not_found(&format!("문서 {id}")));
    };
    let mut d = summary_json(&doc);
    d["deleted"] = json!(deleted);
    let conflict: Vec<String> = e.lib.conflict_leaves(doc.id).map_err(fail)?.iter().map(|r| r.to_string()).collect();
    Ok((json!({"ok": true, "doc": d, "redirected_from": redirected_from.as_ref().map(summary_json), "stale": stale, "conflict": conflict}), model))
}

fn run_index(op: &str, e: &mut Engine, req: &Value) -> Answer {
    match op {
        "titles.exist" => {
            let list = req.get("titles").and_then(Value::as_array).ok_or_else(|| bad("`titles`(목록)가 없음"))?;
            let mut titles = Vec::with_capacity(list.len());
            for t in list {
                titles.push((namespace(t)?, text(t, "title")?.to_string()));
            }
            let docs = e.titles_exist(&titles).map_err(fail)?;
            Ok(json!({"ok": true, "docs": docs.iter().map(|d| d.map(|d| d.to_string())).collect::<Vec<_>>()}))
        }
        "search.titles" => {
            let ns = if req.get("ns").is_some() { namespace(req)? } else { Namespace::Doc };
            let docs = e.search_titles(ns, text(req, "prefix")?, limit(req, 20, 200)).map_err(fail)?;
            Ok(json!({"ok": true, "docs": summaries(&docs)}))
        }
        "search.body" => {
            let hits: Vec<Hit> = e.search_body(text(req, "query")?, limit(req, 20, 200)).map_err(fail)?;
            Ok(json!({"ok": true, "hits": hits.iter().map(|h| json!({"doc": summary_json(&h.doc), "snippet": h.snippet})).collect::<Vec<_>>()}))
        }
        "tree.children" => {
            let category = req.get("category").and_then(Value::as_str);
            let after = req.get("after").and_then(Value::as_str);
            let c: Children = e.children(category, limit(req, 200, 10_000), after).map_err(fail)?;
            let mut v = json!({
                "ok": true,
                "categories": c.categories.iter().map(|f| json!({"name": f.name, "total": f.total, "has_doc": f.has_doc, "cycle": f.cycle})).collect::<Vec<_>>(),
                "docs": summaries(&c.docs),
                "cycles": c.cycles,
            });
            if let Some(n) = c.uncategorized {
                v["uncategorized"] = json!(n);
            }
            Ok(v)
        }
        "links.back" => {
            let docs = e.links_back(doc_id(req, "doc")?, limit(req, 200, 10_000)).map_err(fail)?;
            Ok(json!({"ok": true, "docs": summaries(&docs)}))
        }
        "links.wanted" => {
            let w = e.links_wanted(limit(req, 50, 1_000)).map_err(fail)?;
            Ok(json!({"ok": true, "wanted": wanted_json(&w)}))
        }
        "home.aggregates" => {
            let o = HomeOptions {
                recent: number(req, "recent", 10, 200),
                wanted: number(req, "wanted", 10, 200),
                days: number(req, "days", 84, 3_660).max(1),
                tz_minutes: tz(req),
                today_ms: None,
            };
            let h = e.home(&o).map_err(fail)?;
            Ok(json!({
                "ok": true,
                "groups": h.groups.iter().map(|g| json!({"name": g.name, "total": g.total})).collect::<Vec<_>>(),
                "uncategorized": h.uncategorized,
                "recent": summaries(&h.recent),
                "wanted": wanted_json(&h.wanted),
                "rhythm": h.rhythm.iter().map(|d| json!({"day": d.day, "edits": d.edits})).collect::<Vec<_>>(),
                "counts": counts_json(&h.counts),
            }))
        }
        "stats.library" => {
            let s = e.stats(tz(req), None).map_err(fail)?;
            Ok(json!({
                "ok": true,
                "monthly_new": s.monthly_new.iter().map(|m| json!({"month": m.month, "docs": m.docs})).collect::<Vec<_>>(),
                "daily_edits": s.daily_edits.iter().map(|d| json!({"day": d.day, "edits": d.edits})).collect::<Vec<_>>(),
                "group_links": s.group_links.iter().map(|g| json!({"from": g.from, "to": g.to, "links": g.links})).collect::<Vec<_>>(),
                "top_sources": s.top_sources.iter().map(|x| json!({"source": x.source, "refs": x.refs})).collect::<Vec<_>>(),
                "top_linked": s.top_linked.iter().map(|x| json!({"doc": summary_json(&x.doc), "refs": x.refs})).collect::<Vec<_>>(),
                "decades": s.decades.iter().map(|d| json!({"decade": d.decade, "docs": d.docs})).collect::<Vec<_>>(),
                "as_of": s.as_of,
            }))
        }
        "timeline.docs" => {
            let docs = e.timeline_docs(text(req, "entity")?).map_err(fail)?;
            Ok(json!({"ok": true, "docs": summaries(&docs)}))
        }
        "index.status" => {
            let s = e.status().map_err(fail)?;
            Ok(json!({"ok": true, "stale": s.stale, "total": s.total, "building": s.building}))
        }
        "index.step" => {
            let ms = req.get("max_ms").and_then(Value::as_u64).unwrap_or(200).clamp(1, 1_000);
            let r = e.step(std::time::Duration::from_millis(ms)).map_err(fail)?;
            Ok(json!({"ok": true, "done": r.done, "stale": r.stale}))
        }
        other => Err(bad(format!("모르는 색인 op: {other}"))),
    }
}
