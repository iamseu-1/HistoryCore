//! 검색 조각 · 질의 · 확인(설계 5-5 「검색」, Review Focus 4).
use history_index::search::{fts_match, index_tokens, normalize, parse_query, snippet, stored_plain, verify, FTS_SCHEMA};
use history_namumark::{parse, plain_document, Options};
use rusqlite::{params, Connection};
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

fn set(v: Vec<String>) -> BTreeSet<String> {
    v.into_iter().collect()
}

fn strs(v: &[&str]) -> BTreeSet<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn tokens_of_hangul_runs() {
    assert_eq!(set(index_tokens("임진왜란")), strs(&["임진", "진왜", "왜란", "란"]));
    // 문서 안 같은 조각은 한 번
    let t = index_tokens("임진 임진 임진");
    assert_eq!(t.len(), 2, "{t:?}");
}

#[test]
fn tokens_mix_scripts() {
    assert_eq!(set(index_tokens("鶴𪄃 Joseon 1592년")), strs(&["鶴𪄃", "𪄃", "joseon", "1592", "년"]));
    assert!(index_tokens("😀 , . !").is_empty());
}

#[test]
fn nfd_hangul_is_found() {
    let nfd: String = "임진".nfd().collect();
    assert_ne!(nfd, "임진");
    assert_eq!(normalize(&nfd), "임진");
    assert_eq!(set(index_tokens(&nfd)), strs(&["임진", "진"]));
    assert_eq!(parse_query(&nfd).unwrap().must, ["임진"]);
}

#[test]
fn query_shapes() {
    let q = parse_query("임진왜란").unwrap();
    assert_eq!((q.must.clone(), q.verify.clone(), q.prefix.clone()), (vec!["임진".into(), "진왜".into(), "왜란".into()], vec!["임진왜란".to_string()], vec![]));
    let q = parse_query("왜란 조선").unwrap();
    assert_eq!((q.must.clone(), q.verify.is_empty()), (vec!["왜란".to_string(), "조선".to_string()], true));
    let q = parse_query("鶴").unwrap();
    assert_eq!(q.prefix, ["鶴"]);
    assert_eq!(fts_match(&q), "\"鶴\"*");
    assert_eq!(fts_match(&parse_query("JOSEON 왜란").unwrap()), "\"joseon\" AND \"왜란\"");
    assert!(parse_query("   ").is_none());
    assert!(parse_query("😀!").is_none());
}

#[test]
fn markup_between_letters_still_matches() {
    let plain = stored_plain(&plain_document(&parse("'''임진'''왜란은 [[조선]]의 전쟁", &Options::default())));
    let q = parse_query("임진왜란").unwrap();
    assert!(verify(&plain, &q), "{plain}");
    let s = snippet(&plain, &q, 40);
    assert!(s.contains("임진왜란") && !s.contains("'''") && !s.contains("[["), "{s}");
}

#[test]
fn snippets_cut_long_text_around_the_match() {
    let plain = format!("{}임진왜란{}", "앞".repeat(200), "뒤".repeat(200));
    let s = snippet(&plain, &parse_query("임진왜란").unwrap(), 10);
    assert_eq!(s, format!("…{}임진왜란{}…", "앞".repeat(10), "뒤".repeat(10)));
}

#[test]
fn sqlite_round_trip() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(FTS_SCHEMA).unwrap();
    let docs = ["임진왜란이 일어났다", "임진년에 진왜가 왜란을", "조선의 학"];
    for (i, d) in docs.iter().enumerate() {
        db.execute("INSERT INTO fts(rowid, t) VALUES (?1, ?2)", params![i as i64 + 1, index_tokens(d).join(" ")]).unwrap();
    }
    let find = |q: &str| -> Vec<i64> {
        let q = parse_query(q).unwrap();
        let mut st = db.prepare("SELECT rowid FROM fts WHERE fts MATCH ?1").unwrap();
        let rows: Vec<i64> = st.query_map([fts_match(&q)], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect();
        rows.into_iter().filter(|id| verify(docs[*id as usize - 1], &q)).collect()
    };
    assert_eq!(find("임진왜란"), [1]); // 2번은 조각은 다 있지만 확인에서 빠진다
    assert_eq!(find("왜란"), [1, 2]);
    assert_eq!(find("학"), [3]);
    db.execute("DELETE FROM fts WHERE rowid = 1", []).unwrap();
    assert!(find("임진왜란").is_empty());
}

#[test]
fn nfd_documents_are_stored_as_nfc_so_verify_needs_no_normalizing() {
    let nfd: String = "임진왜란".nfd().collect();
    let q = parse_query("임진왜란").unwrap();
    assert!(!verify(&nfd, &q));
    assert!(verify(&stored_plain(&nfd), &q));
}
