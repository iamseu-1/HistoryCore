//! 서재 일의 다리 — `library.*` · `doc.*` · `rev.*`(spec/engine-api.md). 열린 서재는 손잡이 표에 든다.
use history_library::{Doc, DocId, DocSummary, Error, Hlc, Library, Namespace, RevId, RevInfo, Saved};
use history_namumark::{render, NoTemplates, ObjectRef, RenderOptions, Templates};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

struct Open {
    next: u64,
    libraries: HashMap<u64, Library>,
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

/// 서재 op면 답을, 아니면 None(모르는 op).
pub fn handle_op(op: &str, req: &Value) -> Option<Value> {
    let answer = match op {
        "library.open" => open(req),
        "library.close" => close(req),
        "namumark.parse" => parse_op(req),
        op if LIBRARY_OPS.contains(&op) => with_library(req, |lib| run(op, lib, req)),
        _ => return None,
    };
    Some(answer.unwrap_or_else(|e| e))
}

fn bad(message: impl Into<String>) -> Value {
    json!({"ok": false, "error": message.into(), "code": "bad_request"})
}

fn fail(e: Error) -> Value {
    let mut v = json!({"ok": false, "error": e.to_string(), "code": e.code()});
    if let Error::StaleBase { head } = &e {
        v["head"] = json!(head.to_string());
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
    if let Some((&h, lib)) = t.libraries.iter().find(|(_, lib)| same_folder(&lib.info().folder, &folder)) {
        return Ok(opened(h, lib));
    }
    let lib = Library::open(&dir).map_err(fail)?;
    let h = t.next;
    t.next += 1;
    let answer = opened(h, &lib);
    t.libraries.insert(h, lib);
    Ok(answer)
}

fn close(req: &Value) -> Answer {
    let h = handle_of(req)?;
    table().libraries.remove(&h).map(|_| json!({"ok": true})).ok_or_else(|| bad_handle(h))
}

fn with_library(req: &Value, f: impl FnOnce(&mut Library) -> Answer) -> Answer {
    let h = handle_of(req)?;
    let mut t = table();
    let lib = t.libraries.get_mut(&h).ok_or_else(|| bad_handle(h))?;
    f(lib)
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

fn run(op: &str, lib: &mut Library, req: &Value) -> Answer {
    match op {
        "library.counts" => {
            let c = lib.counts().map_err(fail)?;
            let mut v = json!({"ok": true, "deleted": c.deleted, "total": c.total()});
            for ns in Namespace::ALL {
                v[ns.code()] = json!(c.of(ns));
            }
            Ok(v)
        }
        "doc.create" => {
            let s = lib.create(namespace(req)?, text(req, "title")?, text(req, "body")?, optional_text(req, "message")).map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.save" => {
            let s = lib.save(doc_id(req, "doc")?, rev_id(req, "base")?, text(req, "body")?, optional_text(req, "message")).map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.delete" => {
            let s = lib.delete(doc_id(req, "doc")?, rev_id(req, "base")?, optional_text(req, "message")).map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.rename" => {
            let keep = req.get("keep_redirect").and_then(Value::as_bool).unwrap_or(true);
            let r = lib.rename(doc_id(req, "doc")?, rev_id(req, "base")?, text(req, "title")?, keep, optional_text(req, "message")).map_err(fail)?;
            Ok(json!({"ok": true, "rev": r.rev.to_string(), "hlc": r.hlc.0, "redirect": r.redirect.map(|d| d.to_string())}))
        }
        "doc.get" => {
            let id = doc_id(req, "doc")?;
            match lib.get(id).map_err(fail)? {
                Some(d) => Ok(json!({"ok": true, "doc": doc_json(&d)})),
                None => Err(not_found(&format!("문서 {id}"))),
            }
        }
        "doc.by_title" => {
            let docs = lib.by_title(namespace(req)?, text(req, "title")?).map_err(fail)?;
            Ok(json!({"ok": true, "docs": docs.iter().map(summary_json).collect::<Vec<_>>()}))
        }
        "doc.history" => {
            let limit = req.get("limit").and_then(Value::as_u64).unwrap_or(50).min(1000) as usize;
            let before = req.get("before").and_then(Value::as_u64).map(Hlc);
            let revs = lib.history(doc_id(req, "doc")?, limit, before).map_err(fail)?;
            Ok(json!({"ok": true, "revisions": revs.iter().map(rev_json).collect::<Vec<_>>()}))
        }
        "rev.get" => {
            let id = rev_id(req, "rev")?;
            match lib.revision(id).map_err(fail)? {
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

/// 열린 서재에서 틀 원문을 읽는다 — 서재에 담을 수 있는 이름공간(문서 · 분류 · 틀 · 파일 · 대문)만. 같은 제목이 둘이면 먼저 만든 것.
struct LibraryTemplates<'a>(&'a Library);

impl Templates for LibraryTemplates<'_> {
    fn source(&self, target: &ObjectRef) -> Option<String> {
        let ns = Namespace::ALL.into_iter().find(|n| n.prefix() == target.ns)?;
        let first = self.0.by_title(ns, &target.key).ok()?.into_iter().next()?;
        self.0.get(first.id).ok()?.map(|d| d.body)
    }
}

/// 저장하지 않은 글의 그리기 모델(편집기 미리보기 · spec/namumark.md). `handle`이 있으면 그 서재의 틀을 펼친다.
fn parse_op(req: &Value) -> Answer {
    let text = text(req, "text")?.to_string();
    let options = RenderOptions {
        title: req.get("title").and_then(Value::as_str).map(str::to_string),
        template: req.get("ns").and_then(Value::as_str) == Some(Namespace::Template.code()),
    };
    let model = if req.get("handle").is_some() {
        with_library(req, |lib| Ok(json!(render(&text, &options, &LibraryTemplates(lib)))))?
    } else {
        json!(render(&text, &options, &NoTemplates))
    };
    Ok(json!({"ok": true, "model": model}))
}
