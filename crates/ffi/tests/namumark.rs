use history_core::{hc_call, hc_free};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};

fn call(req: Value) -> Value {
    unsafe {
        let c = CString::new(req.to_string()).unwrap();
        let out = hc_call(c.as_ptr());
        let v = serde_json::from_str(CStr::from_ptr(out).to_str().unwrap()).unwrap();
        hc_free(out);
        v
    }
}

#[test]
fn the_namespace_prefixes_of_parser_and_library_agree() {
    assert_eq!(history_namumark::NAMESPACES, history_library::RESERVED_PREFIXES);
}

#[test]
fn parse_without_a_library() {
    let r = call(json!({"op": "namumark.parse", "text": "'''鶴'''은 새"}));
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["model"]["version"], 1);
    let first = &r["model"]["blocks"][0]["inl"][0];
    assert_eq!((first["t"].clone(), first["style"].clone()), (json!("style"), json!("bold")));
    assert_eq!(first["inl"][0], json!({"t": "han", "s": "鶴"}));
    assert_eq!(call(json!({"op": "namumark.parse"}))["code"], "bad_request");
}

#[test]
fn a_template_page_keeps_its_parameters() {
    let r = call(json!({"op": "namumark.parse", "text": "@이름=미상@", "ns": "template", "title": "해전 정보"}));
    assert_eq!(r["model"]["blocks"][0]["inl"][0], json!({"t": "param", "name": "이름", "fallback": "미상"}));
}

#[test]
fn templates_come_from_the_open_library_and_follow_a_rename() {
    let dir = std::env::temp_dir().join(format!("다리 나무마크 {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let h = call(json!({"op": "library.open", "dir": dir.to_str().unwrap()}))["handle"].clone();
    let t = call(json!({"op": "doc.create", "handle": h, "ns": "template", "title": "해전 정보", "body": "'''@이름@'''"}));
    let text = "[include(틀:해전 정보, 이름=한산도)]";
    let r = call(json!({"op": "namumark.parse", "handle": h, "text": text}));
    let block = &r["model"]["blocks"][0];
    assert_eq!((block["t"].clone(), block["status"].clone()), (json!("template"), json!("ok")), "{r}");
    assert_eq!(block["blocks"][0]["inl"][0]["inl"][0]["s"], "한산도");
    assert_eq!(r["model"]["templates"], json!(["틀:해전 정보"]));
    // 틀 이름을 바꾸면 옛 이름에 넘겨주기가 남는다 — 옛 이름으로 부른 문서도 그대로 펼쳐진다.
    call(json!({"op": "doc.rename", "handle": h, "doc": t["doc"], "base": t["rev"], "title": "해전 요약"}));
    let again = call(json!({"op": "namumark.parse", "handle": h, "text": text}));
    assert_eq!(again["model"]["blocks"][0]["status"], "ok");
    assert_eq!(again["model"]["templates"], json!(["틀:해전 정보", "틀:해전 요약"]));
    let missing = call(json!({"op": "namumark.parse", "handle": h, "text": "[include(틀:없는 틀)]"}));
    assert_eq!(missing["model"]["blocks"][0]["status"], "missing");
    call(json!({"op": "library.close", "handle": h}));
    std::fs::remove_dir_all(&dir).unwrap();
}
