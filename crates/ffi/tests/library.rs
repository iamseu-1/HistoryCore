use history_core::{hc_call, hc_free};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::path::PathBuf;

fn call(req: Value) -> Value {
    unsafe {
        let c = CString::new(req.to_string()).unwrap();
        let out = hc_call(c.as_ptr());
        let v = serde_json::from_str(CStr::from_ptr(out).to_str().unwrap()).unwrap();
        hc_free(out);
        v
    }
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("다리 서재 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn open(dir: &PathBuf) -> Value {
    let o = call(json!({"op": "library.open", "dir": dir.to_str().unwrap()}));
    assert_eq!(o["ok"], true, "{o}");
    o
}

#[test]
fn the_library_works_through_the_bridge() {
    let dir = temp("기본");
    let o = open(&dir);
    let h = o["handle"].clone();
    assert!(o["folder"].as_str().unwrap().ends_with("HistoryCore"));
    let c = call(json!({"op": "doc.create", "handle": h, "ns": "doc", "title": "鶴", "body": "𪄃 😀 학", "message": "처음"}));
    assert_eq!((c["ok"].clone(), c["changed"].clone()), (json!(true), json!(true)), "{c}");
    let g = call(json!({"op": "doc.get", "handle": h, "doc": c["doc"]}));
    assert_eq!(g["doc"]["body"], "𪄃 😀 학");
    assert_eq!(g["doc"]["head"], c["rev"]);
    assert_eq!(g["doc"]["ns"], "doc");
    assert_eq!(g["doc"]["deleted"], false);
    let s = call(json!({"op": "doc.save", "handle": h, "doc": c["doc"], "base": c["rev"], "body": "둘째"}));
    assert_eq!(s["changed"], true);
    let hist = call(json!({"op": "doc.history", "handle": h, "doc": c["doc"]}));
    let revs = hist["revisions"].as_array().unwrap();
    assert_eq!(revs.len(), 2);
    assert_eq!(revs[0]["id"], s["rev"]);
    assert_eq!(revs[0]["parent"], c["rev"]);
    assert_eq!(revs[1]["kind"], "content");
    let r = call(json!({"op": "rev.get", "handle": h, "rev": c["rev"]}));
    assert_eq!(r["revision"]["body"], "𪄃 😀 학");
    let rn = call(json!({"op": "doc.rename", "handle": h, "doc": c["doc"], "base": s["rev"], "title": "두루미"}));
    assert!(rn["redirect"].is_string(), "{rn}");
    let by = call(json!({"op": "doc.by_title", "handle": h, "ns": "doc", "title": "鶴"}));
    assert_eq!(by["docs"][0]["id"], rn["redirect"]);
    let d = call(json!({"op": "doc.delete", "handle": h, "doc": c["doc"], "base": rn["rev"]}));
    assert_eq!(d["changed"], true);
    let counts = call(json!({"op": "library.counts", "handle": h}));
    assert_eq!((counts["doc"].clone(), counts["deleted"].clone(), counts["total"].clone()), (json!(1), json!(1), json!(1)), "{counts}");
    assert_eq!(call(json!({"op": "library.close", "handle": h}))["ok"], true);
    assert_eq!(call(json!({"op": "doc.get", "handle": h, "doc": c["doc"]}))["code"], "bad_handle");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_stale_save_answers_with_the_current_head() {
    // Review Focus 3
    let dir = temp("낡은 판");
    let h = open(&dir)["handle"].clone();
    let c = call(json!({"op": "doc.create", "handle": h, "ns": "doc", "title": "정유재란", "body": "첫"}));
    let s = call(json!({"op": "doc.save", "handle": h, "doc": c["doc"], "base": c["rev"], "body": "다른 창"}));
    let late = call(json!({"op": "doc.save", "handle": h, "doc": c["doc"], "base": c["rev"], "body": "늦은 저장"}));
    assert_eq!(late["ok"], false);
    assert_eq!(late["code"], "stale_base");
    assert_eq!(late["head"], s["rev"]);
    assert_eq!(call(json!({"op": "doc.get", "handle": h, "doc": c["doc"]}))["doc"]["body"], "다른 창");
    call(json!({"op": "library.close", "handle": h}));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bad_requests_are_error_answers_not_crashes() {
    let dir = temp("나쁜 요청");
    let h = open(&dir)["handle"].clone();
    let codes = [
        call(json!({"op": "doc.get", "doc": "x"}))["code"].clone(),
        call(json!({"op": "doc.get", "handle": 987654, "doc": "x"}))["code"].clone(),
        call(json!({"op": "doc.get", "handle": h, "doc": "UUID 아님"}))["code"].clone(),
        call(json!({"op": "doc.create", "handle": h, "ns": "연표", "title": "가", "body": ""}))["code"].clone(),
        call(json!({"op": "doc.create", "handle": h, "ns": "doc", "title": "가|나", "body": ""}))["code"].clone(),
        call(json!({"op": "doc.get", "handle": h, "doc": "00000000-0000-7000-8000-000000000000"}))["code"].clone(),
        call(json!({"op": "library.open"}))["code"].clone(),
    ];
    assert_eq!(codes.to_vec(), vec![json!("bad_request"), json!("bad_handle"), json!("bad_request"), json!("bad_request"), json!("bad_title"), json!("not_found"), json!("bad_request")]);
    call(json!({"op": "library.close", "handle": h}));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn opening_twice_gives_the_same_handle_and_a_foreign_file_is_refused() {
    // Review Focus 2
    let dir = temp("두 번");
    let a = open(&dir)["handle"].clone();
    let b = open(&dir)["handle"].clone();
    assert_eq!(a, b);
    call(json!({"op": "library.close", "handle": a}));
    let other = temp("남의 파일");
    std::fs::create_dir_all(other.join("HistoryCore")).unwrap();
    std::fs::write(other.join("HistoryCore").join("library.db"), "SQLite 아님").unwrap();
    let r = call(json!({"op": "library.open", "dir": other.to_str().unwrap()}));
    assert_eq!(r["code"], "not_a_library");
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&other).unwrap();
}
