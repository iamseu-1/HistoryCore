//! 다리의 동기화 op — `sync.*` · `library.backup` · `doc.save`의 `merge` · `doc.get`의 `conflict`(spec/engine-api.md).
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
    let d = std::env::temp_dir().join(format!("다리 동기화 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ok(v: Value) -> Value {
    assert_eq!(v["ok"], true, "{v}");
    v
}

fn open(name: &str) -> (Value, Value) {
    let o = ok(call(json!({"op": "library.open", "dir": temp(name).to_str().unwrap()})));
    (o["handle"].clone(), o["library"].clone())
}

#[test]
fn two_libraries_meet_through_packs_and_a_conflict_is_merged_by_hand() {
    let (a, library) = open("가");
    let (b, _) = open("나");
    ok(call(json!({"op": "sync.join", "handle": b, "library": library})));
    let created = ok(call(json!({"op": "doc.create", "handle": a, "ns": "doc", "title": "임진왜란", "body": "첫 줄\n둘째 줄\n"})));
    let (doc, root) = (created["doc"].clone(), created["rev"].clone());

    let st = ok(call(json!({"op": "sync.status", "handle": a, "after_seq": 0})));
    assert_eq!((st["pending"].as_u64(), st["held"].as_u64(), st["conflicts"].as_u64(), st["seq"].as_i64()), (Some(1), Some(0), Some(0), Some(1)));
    assert_eq!(st["device"], ok(call(json!({"op": "library.open", "dir": temp("가").to_str().unwrap()})))["device"]);
    let e = ok(call(json!({"op": "sync.export", "handle": a, "after_seq": 0, "no": 1})));
    assert_eq!((e["count"].as_u64(), e["seq"].as_i64(), e["more"].as_bool()), (Some(1), Some(1), Some(false)));
    let r = ok(call(json!({"op": "sync.apply", "handle": b, "pack": e["pack"]})));
    assert_eq!((r["added"].as_u64(), r["skipped"].as_u64(), r["merged"].as_u64()), (Some(1), Some(0), Some(0)));
    assert_eq!(r["conflicts"], json!([]));
    assert!(ok(call(json!({"op": "sync.export", "handle": b, "after_seq": 0, "no": 1})))["pack"].is_null(), "받은 판은 내보내지 않는다");

    // 같은 줄을 둘이 다르게 — 충돌.
    ok(call(json!({"op": "doc.save", "handle": a, "doc": doc, "base": root, "body": "첫 줄\n가\n"})));
    ok(call(json!({"op": "doc.save", "handle": b, "doc": doc, "base": root, "body": "첫 줄\n나\n"})));
    let from_b = ok(call(json!({"op": "sync.export", "handle": b, "after_seq": 0, "no": 1})));
    let r = ok(call(json!({"op": "sync.apply", "handle": a, "pack": from_b["pack"]})));
    assert_eq!(r["conflicts"], json!([doc]));
    let got = ok(call(json!({"op": "doc.get", "handle": a, "doc": doc})));
    let other = got["doc"]["conflict"][0].clone();
    assert!(other.is_string(), "{got}");
    assert_eq!(ok(call(json!({"op": "doc.open", "handle": a, "doc": doc})))["conflict"], json!([other]), "열 때도 충돌을 안다");
    assert_eq!(ok(call(json!({"op": "sync.status", "handle": a, "after_seq": 0})))["conflicts"], 1);
    let merged = ok(call(json!({"op": "doc.save", "handle": a, "doc": doc, "base": got["doc"]["head"], "merge": other, "body": "첫 줄\n가나\n"})));
    let history = ok(call(json!({"op": "doc.history", "handle": a, "doc": doc, "limit": 1})));
    assert_eq!(history["revisions"][0]["id"], merged["rev"]);
    assert_eq!(history["revisions"][0]["parent2"], other);
    assert_eq!(ok(call(json!({"op": "doc.get", "handle": a, "doc": doc})))["doc"]["conflict"], json!([]));
    let bad = call(json!({"op": "doc.save", "handle": a, "doc": doc, "base": merged["rev"], "merge": root, "body": "x"}));
    assert_eq!(bad["code"], "not_a_leaf");
}

#[test]
fn bad_packs_and_other_libraries_are_refused() {
    let (a, _) = open("거절 가");
    let (b, _) = open("거절 나");
    ok(call(json!({"op": "doc.create", "handle": a, "ns": "doc", "title": "임진왜란", "body": "x"})));
    let e = ok(call(json!({"op": "sync.export", "handle": a, "after_seq": 0, "no": 1})));
    assert_eq!(call(json!({"op": "sync.apply", "handle": b, "pack": e["pack"]}))["code"], "wrong_library");
    assert_eq!(call(json!({"op": "sync.apply", "handle": b, "pack": "QUJD"}))["code"], "bad_pack");
    assert_eq!(call(json!({"op": "sync.apply", "handle": b, "pack": "base64 아님!"}))["code"], "bad_request");
    assert_eq!(call(json!({"op": "sync.join", "handle": b, "library": "uuid 아님"}))["code"], "bad_request");
}

#[test]
fn a_backup_lands_where_asked() {
    let (a, _) = open("내보내기");
    ok(call(json!({"op": "doc.create", "handle": a, "ns": "doc", "title": "임진왜란", "body": "x"})));
    let out = temp("내보내기 파일").join("서재.zip");
    ok(call(json!({"op": "library.backup", "handle": a, "path": out.to_str().unwrap()})));
    assert!(std::fs::metadata(&out).unwrap().len() > 100);
}

#[test]
fn rotate_gives_a_new_device_and_saves_report_the_head() {
    let (a, _) = open("새 기기");
    let created = ok(call(json!({"op": "doc.create", "handle": a, "ns": "doc", "title": "임진왜란", "body": "x"})));
    assert_eq!(created["head"], created["rev"]);
    let before = ok(call(json!({"op": "sync.status", "handle": a, "after_seq": 0})))["device"].clone();
    let r = ok(call(json!({"op": "sync.rotate", "handle": a})));
    assert_ne!(r["device"], before);
    let st = ok(call(json!({"op": "sync.status", "handle": a, "after_seq": 0})));
    assert_eq!((st["device"].clone(), st["pending"].as_u64()), (r["device"].clone(), Some(1)), "옛 ID의 판을 물려받는다");
}
