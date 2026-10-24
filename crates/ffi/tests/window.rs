//! 다리의 서재 창구(spec/windows-api.md 3-1 · 4절 · spec/engine-api.md 「서재 창구 op」) — 읽기 전용 손잡이와 `window.*` 여섯.
use history_core::{hc_call, hc_free};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// 답 글 그대로(모델을 끼운 자리를 보려고).
fn call_text(req: &Value) -> String {
    unsafe {
        let c = CString::new(req.to_string()).unwrap();
        let out = hc_call(c.as_ptr());
        let s = CStr::from_ptr(out).to_str().unwrap().to_string();
        hc_free(out);
        s
    }
}

fn call(req: Value) -> Value {
    serde_json::from_str(&call_text(&req)).unwrap()
}

fn ok(r: Value) -> Value {
    assert_eq!(r["ok"], true, "{r}");
    r
}

/// 창구 답 — `ok`와 `api_version` 1을 함께 본다.
fn answered(r: Value) -> Value {
    assert_eq!((r["ok"].clone(), r["api_version"].clone()), (json!(true), json!(1)), "{r}");
    r
}

/// 한글 · 공백이 든 시험용 앱 자료 폴더(아직 만들지 않음).
fn place(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("다리 창구 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn temp(name: &str) -> PathBuf {
    let d = place(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn dir_of(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// 폴더 안 파일마다 (이름, 크기, 고친 때, 바이트) — 파일마다 따로 묻는다(Windows의 디렉터리 항목 값은 늦게 고쳐진다).
fn snapshot(dir: &Path) -> Vec<(String, u64, SystemTime, Vec<u8>)> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            let m = std::fs::metadata(&p).unwrap();
            (p.file_name().unwrap().to_string_lossy().into_owned(), m.len(), m.modified().unwrap(), std::fs::read(&p).unwrap())
        })
        .collect();
    out.sort();
    out
}

const LANTERN: &str = "lantern-chronicle/p-lantern";

struct Seeded {
    war: Value,
    redirect: Value,
    gone: Value,
}

/// 보통 손잡이로 씨를 심는다 — 연표 링크 · 띠 · 넘겨주기 · 지운 문서 · 틀.
fn seed(h: &Value) -> Seeded {
    let create = |ns: &str, title: &str, body: &str| ok(call(json!({"op": "doc.create", "handle": h, "ns": ns, "title": title, "body": body})));
    create("template", "정보", "'''@이름@''' — 틀 글");
    let war = create("doc", "임진왜란", &format!("[include(틀:정보, 이름=임진왜란)]\n왜란 이야기 [[연표:{LANTERN}|등불]]"));
    let renamed = ok(call(json!({"op": "doc.rename", "handle": h, "doc": war["doc"], "base": war["rev"], "title": "임진 전쟁"})));
    create("doc", "가", &format!("[연표({LANTERN}, 범위=1592..1598)]\n[[연표:{LANTERN}]]"));
    create("doc", "임진강", "강 이름");
    create("doc", "정유재란", "임진왜란 뒤의 전쟁");
    let gone = create("doc", "라", &format!("[[연표:{LANTERN}]] 지울 문서"));
    ok(call(json!({"op": "doc.delete", "handle": h, "doc": gone["doc"], "base": gone["rev"]})));
    Seeded { war: war["doc"].clone(), redirect: renamed["redirect"].clone(), gone: gone["doc"].clone() }
}

fn open_read_only(dir: &Path) -> Value {
    call(json!({"op": "library.open", "dir": dir_of(dir), "read_only": true}))
}

fn titles(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|d| d["title"].as_str().unwrap().to_string()).collect()
}

/// `2026-10-24T03:15:00.123Z` 꼴인가.
fn is_utc_iso(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 24 && b[4] == b'-' && b[7] == b'-' && b[10] == b'T' && b[13] == b':' && b[16] == b':' && b[19] == b'.' && b[23] == b'Z'
        && s.chars().enumerate().all(|(i, c)| [4, 7, 10, 13, 16, 19, 23].contains(&i) || c.is_ascii_digit())
}

/// 답 글에서 `"model":` 뒤의 글(끝 괄호 하나 빼고) — 다리가 풀지 않고 끼웠는지 보려고.
fn raw_model(reply: &str) -> &str {
    let at = reply.find(",\"model\":").expect("모델 칸");
    &reply[at + 9..reply.len() - 1]
}

#[test]
fn a_read_only_handle_is_separate_and_takes_only_window_ops() {
    let dir = temp("따로 된 손잡이");
    let w = ok(call(json!({"op": "library.open", "dir": dir_of(&dir)})));
    let s = seed(&w["handle"]);
    let r = ok(open_read_only(&dir));
    assert_eq!(r["read_only"], true);
    assert_ne!(r["handle"], w["handle"], "보통 손잡이를 돌려주었다");
    for key in ["library", "device", "format", "folder"] {
        assert_eq!(r[key], w[key], "{key}");
    }
    assert!(w.get("read_only").is_none_or(|v| v == false), "{w}");
    // 같은 폴더를 다시 읽기 전용으로 열면 같은 손잡이 · 보통으로 다시 열면 보통 손잡이.
    assert_eq!(ok(open_read_only(&dir))["handle"], r["handle"]);
    assert_eq!(ok(call(json!({"op": "library.open", "dir": dir_of(&dir)})))["handle"], w["handle"]);
    let h = r["handle"].clone();
    for req in [
        json!({"op": "doc.create", "handle": h, "ns": "doc", "title": "새 문서", "body": ""}),
        json!({"op": "doc.save", "handle": h, "doc": s.war, "base": s.war, "body": "덮어씀"}),
        json!({"op": "doc.delete", "handle": h, "doc": s.war, "base": s.war}),
        json!({"op": "doc.open", "handle": h, "doc": s.war}),
        json!({"op": "doc.get", "handle": h, "doc": s.war}),
        json!({"op": "library.counts", "handle": h}),
        json!({"op": "titles.exist", "handle": h, "titles": []}),
        json!({"op": "timeline.docs", "handle": h, "entity": LANTERN}),
        json!({"op": "index.step", "handle": h}),
        json!({"op": "sync.export", "handle": h, "after_seq": 0, "no": 1}),
        json!({"op": "sync.join", "handle": h, "library": r["library"]}),
        json!({"op": "library.backup", "handle": h, "path": dir_of(&dir.join("x.zip"))}),
        json!({"op": "namumark.parse", "handle": h, "text": "[include(틀:정보)]"}),
    ] {
        let a = call(req.clone());
        assert_eq!((a["ok"].clone(), a["code"].clone()), (json!(false), json!("read_only")), "{req} → {a}");
    }
    assert!(!dir.join("x.zip").exists());
    // `window.*`는 두 손잡이 모두 받고 같은 답이다(보통 손잡이는 쓰는 엔진 곁에서 따로 읽기 전용으로 읽는다).
    for req in [
        json!({"op": "window.version"}),
        json!({"op": "window.doc", "id": s.war}),
        json!({"op": "window.doc_by_title", "title": "임진왜란"}),
        json!({"op": "window.docs_for_entity", "entity": LANTERN}),
        json!({"op": "window.titles_exist", "titles": [{"ns": "doc", "title": "가"}, {"ns": "doc", "title": "없음"}]}),
        json!({"op": "window.search", "q": "임진"}),
    ] {
        let mut on_read = req.clone();
        on_read["handle"] = h.clone();
        let mut on_write = req.clone();
        on_write["handle"] = w["handle"].clone();
        let (a, b) = (call_text(&on_read), call_text(&on_write));
        answered(serde_json::from_str(&a).unwrap());
        assert_eq!(a, b, "{req}");
    }
    // 읽기 전용 손잡이를 닫아도 보통 손잡이는 그대로.
    ok(call(json!({"op": "library.close", "handle": h})));
    assert_eq!(call(json!({"op": "window.version", "handle": h}))["code"], "bad_handle");
    assert_eq!(call(json!({"op": "library.close", "handle": h}))["code"], "bad_handle");
    answered(call(json!({"op": "window.version", "handle": w["handle"]})));
    ok(call(json!({"op": "doc.create", "handle": w["handle"], "ns": "doc", "title": "닫은 뒤", "body": ""})));
    ok(call(json!({"op": "library.close", "handle": w["handle"]})));
    assert_eq!(call(json!({"op": "window.version", "handle": w["handle"]}))["code"], "bad_handle");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn window_answers_have_the_promised_shape() {
    let dir = temp("답 꼴");
    let w = ok(call(json!({"op": "library.open", "dir": dir_of(&dir)})));
    let s = seed(&w["handle"]);
    let h = ok(open_read_only(&dir))["handle"].clone();

    let v = answered(call(json!({"op": "window.version", "handle": h})));
    assert_eq!(v["library"], w["library"]);
    assert_eq!(v["engine"], call(json!({"op": "version"}))["engine"]);

    // 문서 — 요약 칸 · UTC ISO · 모델은 글 그대로 끼움(`doc.open`의 모델 글과 바이트까지 같다).
    let text = call_text(&json!({"op": "window.doc", "handle": h, "id": s.war}));
    let d = answered(serde_json::from_str(&text).unwrap());
    assert_eq!((d["found"].clone(), d["id"].clone(), d["ns"].clone(), d["title"].clone()), (json!(true), s.war.clone(), json!("doc"), json!("임진 전쟁")));
    assert!(is_utc_iso(d["updated"].as_str().unwrap()), "{}", d["updated"]);
    assert_eq!(d["model"]["version"], 1);
    let opened = call_text(&json!({"op": "doc.open", "handle": w["handle"], "doc": s.war, "follow": false}));
    assert_eq!(raw_model(&text), raw_model(&opened));
    // 넘겨주기 문서는 따라가지 않는다 · 지운 문서 · 없는 문서는 `found: false`만.
    let r = answered(call(json!({"op": "window.doc", "handle": h, "id": s.redirect})));
    assert_eq!((r["title"].clone(), r["model"]["redirect"].is_object()), (json!("임진왜란"), true), "{r}");
    for id in [s.gone.clone(), json!("0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b")] {
        let a = call(json!({"op": "window.doc", "handle": h, "id": id}));
        assert_eq!(a, json!({"ok": true, "api_version": 1, "found": false}));
    }
    // 제목으로 — `ns` 기본은 `doc` · 쓸 수 없는 제목 · 없는 제목은 `found: false`.
    let by = answered(call(json!({"op": "window.doc_by_title", "handle": h, "title": "임진왜란"})));
    assert_eq!((by["id"].clone(), by["found"].clone()), (s.redirect.clone(), json!(true)));
    let by = answered(call(json!({"op": "window.doc_by_title", "handle": h, "ns": "template", "title": "정보"})));
    assert_eq!(by["ns"], "template");
    for title in ["없는 문서", "가|나", "라"] {
        assert_eq!(call(json!({"op": "window.doc_by_title", "handle": h, "title": title})), json!({"ok": true, "api_version": 1, "found": false}), "{title}");
    }

    // 항목 문서 — 맞춘 ID · `{id, ns, title, updated}`만 · 지운 문서 뺌 · 제목 차례.
    let padded = format!("  {LANTERN} ");
    let e = answered(call(json!({"op": "window.docs_for_entity", "handle": h, "entity": padded})));
    assert_eq!(e["entity"], LANTERN);
    assert_eq!(titles(&e["docs"]), ["가", "임진 전쟁"]);
    let first = e["docs"][0].as_object().unwrap();
    let mut keys: Vec<&str> = first.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, ["id", "ns", "title", "updated"]);
    assert!(is_utc_iso(first["updated"].as_str().unwrap()));
    let decomposed: String = "가/나".chars().flat_map(unicode_nfd).collect();
    assert_eq!(answered(call(json!({"op": "window.docs_for_entity", "handle": h, "entity": decomposed})))["entity"], "가/나");

    // 있는 제목 — 같은 차례 · ID 또는 null.
    let t = answered(call(json!({"op": "window.titles_exist", "handle": h, "titles": [{"ns": "doc", "title": "가"}, {"ns": "doc", "title": "없음"}, {"ns": "template", "title": "정보"}, {"ns": "doc", "title": "라"}]})));
    let docs = t["docs"].as_array().unwrap();
    assert_eq!((docs.len(), docs[0].is_string(), docs[1].is_null(), docs[2].is_string(), docs[3].is_null()), (4, true, true, true, true), "{t}");

    // 찾기 — 제목 앞부분 먼저(`snippet` 빈 글), 이어 본문 · 같은 문서는 한 번 · `{id, ns, title, snippet}`.
    // 제목 줄은 「임진 전쟁」 · 「임진강」 · 「임진왜란」(넘겨주기 문서), 본문 줄은 그 셋을 빼고 「정유재란」.
    let f = answered(call(json!({"op": "window.search", "handle": h, "q": "임진"})));
    let hits = f["docs"].as_array().unwrap();
    assert_eq!(titles(&f["docs"]), ["임진 전쟁", "임진강", "임진왜란", "정유재란"]);
    assert!(hits[..3].iter().all(|x| x["snippet"] == ""), "{f}");
    assert!(hits[3]["snippet"].as_str().unwrap().contains("임진왜란"), "{f}");
    for hit in hits {
        let mut keys: Vec<&str> = hit.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, ["id", "ns", "snippet", "title"]);
    }
    // `limit`은 1 ~ 100으로 자른다.
    assert_eq!(titles(&answered(call(json!({"op": "window.search", "handle": h, "q": "임진", "limit": 0})))["docs"]), ["임진 전쟁"]);
    assert_eq!(answered(call(json!({"op": "window.search", "handle": h, "q": "임진", "limit": 1_000_000})))["docs"].as_array().unwrap().len(), 4);
    assert!(answered(call(json!({"op": "window.search", "handle": h, "q": "  "})))["docs"].as_array().unwrap().is_empty());

    ok(call(json!({"op": "library.close", "handle": h})));
    ok(call(json!({"op": "library.close", "handle": w["handle"]})));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 한글 음절 하나를 풀어 쓴(NFD) 자모로 — 시험 글을 짓는 데만 쓴다(한글이 아니면 그대로).
fn unicode_nfd(c: char) -> Vec<char> {
    let s = c as u32;
    if !(0xAC00..=0xD7A3).contains(&s) {
        return vec![c];
    }
    let i = s - 0xAC00;
    let (l, v, t) = (0x1100 + i / 588, 0x1161 + (i % 588) / 28, i % 28);
    let mut out = vec![char::from_u32(l).unwrap(), char::from_u32(v).unwrap()];
    if t > 0 {
        out.push(char::from_u32(0x11A7 + t).unwrap());
    }
    out
}

#[test]
fn bad_window_requests_are_bad_requests() {
    let dir = temp("나쁜 물음");
    let w = ok(call(json!({"op": "library.open", "dir": dir_of(&dir)})));
    seed(&w["handle"]);
    let h = ok(open_read_only(&dir))["handle"].clone();
    let long = "가".repeat(1025);
    let many: Vec<Value> = (0..1001).map(|i| json!({"ns": "doc", "title": format!("제목 {i}")})).collect();
    for req in [
        json!({"op": "window.doc", "handle": h}),
        json!({"op": "window.doc", "handle": h, "id": "UUID 아님"}),
        json!({"op": "window.doc_by_title", "handle": h}),
        json!({"op": "window.doc_by_title", "handle": h, "ns": "연표", "title": "가"}),
        json!({"op": "window.docs_for_entity", "handle": h}),
        json!({"op": "window.docs_for_entity", "handle": h, "entity": ""}),
        json!({"op": "window.docs_for_entity", "handle": h, "entity": "   "}),
        json!({"op": "window.docs_for_entity", "handle": h, "entity": long}),
        json!({"op": "window.titles_exist", "handle": h}),
        json!({"op": "window.titles_exist", "handle": h, "titles": many}),
        json!({"op": "window.titles_exist", "handle": h, "titles": [{"ns": "연표", "title": "가"}]}),
        json!({"op": "window.titles_exist", "handle": h, "titles": [{"title": "가"}]}),
        json!({"op": "window.search", "handle": h}),
        json!({"op": "window.search", "handle": h, "q": "임진", "limit": "많이"}),
        json!({"op": "window.search", "handle": h, "q": "임진", "limit": 2.5}),
        json!({"op": "library.open", "dir": dir_of(&dir), "read_only": "예"}),
    ] {
        let a = call(req.clone());
        assert_eq!((a["ok"].clone(), a["code"].clone()), (json!(false), json!("bad_request")), "{req} → {a}");
    }
    // 1,024자까지는 받는다 · 1,000개까지는 받는다.
    answered(call(json!({"op": "window.docs_for_entity", "handle": h, "entity": "가".repeat(1024)})));
    answered(call(json!({"op": "window.titles_exist", "handle": h, "titles": many[..1000]})));
    assert_eq!(call(json!({"op": "window.doc", "handle": 987_654_321u64, "id": "0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b"}))["code"], "bad_handle");
    assert_eq!(call(json!({"op": "window.doc", "id": "0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b"}))["code"], "bad_request");
    ok(call(json!({"op": "library.close", "handle": h})));
    ok(call(json!({"op": "library.close", "handle": w["handle"]})));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn opening_read_only_where_there_is_no_library_makes_nothing() {
    let missing = place("없는 폴더");
    assert_eq!(open_read_only(&missing)["code"], "no_library");
    assert!(!missing.exists(), "폴더를 만들었다");
    let empty = temp("빈 폴더");
    assert_eq!(open_read_only(&empty)["code"], "no_library");
    assert!(std::fs::read_dir(&empty).unwrap().next().is_none(), "파일을 만들었다");
    std::fs::create_dir_all(empty.join("HistoryCore")).unwrap();
    std::fs::write(empty.join("HistoryCore").join("library.db"), "SQLite 아님").unwrap();
    assert_eq!(open_read_only(&empty)["code"], "not_a_library");
    assert_eq!(std::fs::read_to_string(empty.join("HistoryCore").join("library.db")).unwrap(), "SQLite 아님");
    assert_eq!(call(json!({"op": "library.open", "read_only": true}))["code"], "bad_request");
    std::fs::remove_dir_all(&empty).unwrap();
}

#[test]
fn reading_a_closed_library_leaves_its_files_alone() {
    let dir = temp("꺼진 서재");
    let w = ok(call(json!({"op": "library.open", "dir": dir_of(&dir)})));
    let s = seed(&w["handle"]);
    ok(call(json!({"op": "library.close", "handle": w["handle"]})));
    let folder = dir.join("HistoryCore");
    let before = snapshot(&folder);
    let h = ok(open_read_only(&dir))["handle"].clone();
    for req in [
        json!({"op": "window.version", "handle": h}),
        json!({"op": "window.doc", "handle": h, "id": s.war}),
        json!({"op": "window.doc_by_title", "handle": h, "title": "가"}),
        json!({"op": "window.docs_for_entity", "handle": h, "entity": LANTERN}),
        json!({"op": "window.titles_exist", "handle": h, "titles": [{"ns": "doc", "title": "가"}]}),
        json!({"op": "window.search", "handle": h, "q": "이야기"}),
        json!({"op": "doc.save", "handle": h, "doc": s.war, "base": s.war, "body": "덮어씀"}),
    ] {
        call(req);
        // 질의 사이에도 곁 파일이 남지 않는다(질의마다 열고 닫는다).
        assert_eq!(snapshot(&folder), before, "질의 사이");
    }
    ok(call(json!({"op": "library.close", "handle": h})));
    assert_eq!(snapshot(&folder), before);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn without_an_index_only_the_index_questions_fail() {
    let dir = temp("색인 없음");
    let w = ok(call(json!({"op": "library.open", "dir": dir_of(&dir)})));
    let s = seed(&w["handle"]);
    ok(call(json!({"op": "library.close", "handle": w["handle"]})));
    std::fs::remove_file(dir.join("HistoryCore").join("index.db")).unwrap();
    let h = ok(open_read_only(&dir))["handle"].clone();
    for req in [
        json!({"op": "window.docs_for_entity", "handle": h, "entity": LANTERN}),
        json!({"op": "window.titles_exist", "handle": h, "titles": [{"ns": "doc", "title": "가"}]}),
        json!({"op": "window.search", "handle": h, "q": "임진"}),
    ] {
        assert_eq!(call(req.clone())["code"], "no_index", "{req}");
    }
    let d = answered(call(json!({"op": "window.doc", "handle": h, "id": s.war})));
    assert!(d["model"]["blocks"].is_array(), "{d}");
    answered(call(json!({"op": "window.version", "handle": h})));
    assert!(!dir.join("HistoryCore").join("index.db").exists(), "색인을 지었다");
    ok(call(json!({"op": "library.close", "handle": h})));
    std::fs::remove_dir_all(&dir).unwrap();
}
