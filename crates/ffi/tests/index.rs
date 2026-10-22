//! 다리의 색인 op 열둘(spec/engine-api.md · P2-3 계획 「다리 op 약속」).
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
    let d = std::env::temp_dir().join(format!("다리 색인 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ok(r: Value) -> Value {
    assert_eq!(r["ok"], true, "{r}");
    r
}

fn titles(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|d| d["title"].as_str().unwrap().to_string()).collect()
}

#[test]
fn index_ops_shapes() {
    let dir = temp("모양");
    let h = ok(call(json!({"op": "library.open", "dir": dir.to_str().unwrap()})))["handle"].clone();
    let create = |ns: &str, title: &str, body: &str| ok(call(json!({"op": "doc.create", "handle": h, "ns": ns, "title": title, "body": body})));
    create("category", "조선", "");
    let ga = create("doc", "가", "[[분류:조선]] 임진왜란 이야기 [[나]] [[없는 문서]] [[연표:01A]][* 『선조실록』]");
    create("doc", "나", "[[가]] 둘째");
    let redirect = create("doc", "옛 가", "#넘겨주기 가");

    let o = ok(call(json!({"op": "doc.open", "handle": h, "doc": ga["doc"]})));
    assert!(o["model"].is_object() && o["model"]["blocks"].is_array(), "{o}");
    assert_eq!((o["doc"]["title"].clone(), o["doc"]["deleted"].clone(), o["stale"].clone(), o["redirected_from"].clone()), (json!("가"), json!(false), json!(false), Value::Null));
    let r = ok(call(json!({"op": "doc.open", "handle": h, "doc": redirect["doc"]})));
    assert_eq!((r["doc"]["title"].clone(), r["redirected_from"]["title"].clone()), (json!("가"), json!("옛 가")));
    let r = ok(call(json!({"op": "doc.open", "handle": h, "doc": redirect["doc"], "follow": false})));
    assert_eq!(r["doc"]["title"], "옛 가");
    assert!(r["model"]["redirect"].is_object());

    let t = ok(call(json!({"op": "titles.exist", "handle": h, "titles": [{"ns": "doc", "title": "가"}, {"ns": "doc", "title": "없음"}, {"ns": "category", "title": "조선"}]})));
    assert_eq!(t["docs"][0], ga["doc"]);
    assert_eq!(t["docs"][1], Value::Null);
    assert!(t["docs"][2].is_string());

    assert_eq!(titles(&ok(call(json!({"op": "search.titles", "handle": h, "prefix": "옛"})))["docs"]), ["옛 가"]);
    let s = ok(call(json!({"op": "search.body", "handle": h, "query": "임진왜란"})));
    assert_eq!(s["hits"][0]["doc"]["title"], "가");
    assert!(s["hits"][0]["snippet"].as_str().unwrap().contains("임진왜란"));

    let root = ok(call(json!({"op": "tree.children", "handle": h})));
    assert_eq!(root["categories"][0], json!({"name": "조선", "total": 1, "has_doc": true, "cycle": false}));
    assert_eq!((root["uncategorized"].clone(), root["cycles"].clone()), (json!(2), json!([])));
    let inner = ok(call(json!({"op": "tree.children", "handle": h, "category": "조선"})));
    assert_eq!(titles(&inner["docs"]), ["가"]);
    assert!(inner.get("uncategorized").is_none_or(Value::is_null));

    assert_eq!(titles(&ok(call(json!({"op": "links.back", "handle": h, "doc": ga["doc"]})))["docs"]), ["나", "옛 가"]);
    let w = ok(call(json!({"op": "links.wanted", "handle": h})));
    assert_eq!(w["wanted"], json!([{"ns": "doc", "title": "없는 문서", "refs": 1}]));

    let home = ok(call(json!({"op": "home.aggregates", "handle": h, "tz": 540})));
    assert_eq!(home["groups"], json!([{"name": "조선", "total": 1}]));
    assert_eq!(home["uncategorized"], 2);
    assert_eq!(home["recent"].as_array().unwrap().len(), 4);
    assert_eq!(home["rhythm"].as_array().unwrap().len(), 84);
    assert_eq!(home["rhythm"].as_array().unwrap().last().unwrap()["edits"], 4);
    assert_eq!(home["wanted"][0]["title"], "없는 문서");
    assert_eq!(home["counts"]["doc"], 3);

    let st = ok(call(json!({"op": "stats.library", "handle": h})));
    assert_eq!(st["daily_edits"].as_array().unwrap().len(), 365);
    assert_eq!(st["top_sources"], json!([{"source": "선조실록", "refs": 1}]));
    assert_eq!(st["top_linked"][0]["doc"]["title"], "가");
    for key in ["monthly_new", "group_links", "decades", "as_of"] {
        assert!(st.get(key).is_some(), "{key}");
    }

    assert_eq!(titles(&ok(call(json!({"op": "timeline.docs", "handle": h, "entity": "01A"})))["docs"]), ["가"]);
    let status = ok(call(json!({"op": "index.status", "handle": h})));
    assert_eq!((status["stale"].clone(), status["total"].clone(), status["building"].clone()), (json!(0), json!(4), json!(false)));
    let step = ok(call(json!({"op": "index.step", "handle": h, "max_ms": 50})));
    assert_eq!(step["stale"], 0);
    assert!(step["done"].is_u64());

    ok(call(json!({"op": "library.close", "handle": h})));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn saving_through_the_bridge_indexes_at_once() {
    let dir = temp("저장하면 색인");
    let h = ok(call(json!({"op": "library.open", "dir": dir.to_str().unwrap()})))["handle"].clone();
    let a = ok(call(json!({"op": "doc.create", "handle": h, "ns": "doc", "title": "가", "body": "한산도 대첩"})));
    assert_eq!(ok(call(json!({"op": "search.body", "handle": h, "query": "한산도"})))["hits"].as_array().unwrap().len(), 1);
    ok(call(json!({"op": "doc.save", "handle": h, "doc": a["doc"], "base": a["rev"], "body": "명량 해전"})));
    assert!(ok(call(json!({"op": "search.body", "handle": h, "query": "한산도"})))["hits"].as_array().unwrap().is_empty());
    ok(call(json!({"op": "library.close", "handle": h})));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bad_requests_and_handles() {
    let dir = temp("잘못된 요청");
    let h = ok(call(json!({"op": "library.open", "dir": dir.to_str().unwrap()})))["handle"].clone();
    for req in [
        json!({"op": "doc.open", "handle": h}),
        json!({"op": "search.body", "handle": h}),
        json!({"op": "titles.exist", "handle": h, "titles": [{"ns": "없는 ns", "title": "가"}]}),
        json!({"op": "titles.exist", "handle": h}),
        json!({"op": "links.back", "handle": h, "doc": "uuid 아님"}),
        json!({"op": "timeline.docs", "handle": h}),
    ] {
        assert_eq!(call(req.clone())["code"], "bad_request", "{req}");
    }
    let missing = call(json!({"op": "doc.open", "handle": h, "doc": "0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b"}));
    assert_eq!(missing["code"], "not_found");
    ok(call(json!({"op": "library.close", "handle": h})));
    assert_eq!(call(json!({"op": "index.status", "handle": h}))["code"], "bad_handle");
    std::fs::remove_dir_all(&dir).unwrap();
}
