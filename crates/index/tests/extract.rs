//! 한 문서 색인하기 — 뽑기 여덟 · 틀 바뀜 · 읽기 오류(Review Focus 3).
use history_index::Engine;
use history_library::{DocId, Namespace};
use std::path::PathBuf;
use std::time::Duration;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("색인 뽑기 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn rowid(e: &Engine, doc: DocId) -> i64 {
    e.debug_index().query_row("SELECT rowid FROM item WHERE doc = ?1", [doc], |r| r.get(0)).unwrap()
}

/// 이 문서의 표 `table`에서 `col`(글)을 값 차례로(문서 번호가 앞인 기본 키 표라 넣은 차례는 없다).
fn texts(e: &Engine, table: &str, col: &str, doc: DocId) -> Vec<String> {
    let id = rowid(e, doc);
    let mut st = e.debug_index().prepare(&format!("SELECT {col} FROM {table} WHERE doc = ?1 ORDER BY {col}")).unwrap();
    st.query_map([id], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
}

fn link_keys(e: &Engine, doc: DocId) -> Vec<String> {
    let id = rowid(e, doc);
    let mut st = e.debug_index().prepare("SELECT ns || ':' || key FROM link WHERE src = ?1 ORDER BY ns, key").unwrap();
    st.query_map([id], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
}

fn model(e: &mut Engine, doc: DocId) -> String {
    e.open_doc(doc, false).unwrap().unwrap().model
}

fn settle(e: &mut Engine) {
    for _ in 0..20 {
        if e.status().unwrap().stale == 0 {
            return;
        }
        e.step(Duration::from_millis(200)).unwrap();
    }
    panic!("따라잡지 못함: {:?}", e.status().unwrap());
}

#[test]
fn links_categories_footnotes_sources_entities() {
    let app = temp("뽑기");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "가", "[[임진왜란]] [[분류:조선]][[분류:전쟁]][* 『선조실록』 1권, 『난중일기』] [[연표:01A]] [[:분류:조선]]", "").unwrap();
    assert_eq!(link_keys(&e, a.doc), ["문서:임진왜란", "분류:조선"]);
    assert_eq!(texts(&e, "member", "category", a.doc), ["전쟁", "조선"]);
    let first: String = e.debug_index().query_row("SELECT category FROM member WHERE doc = ?1 AND first = 1", [rowid(&e, a.doc)], |r| r.get(0)).unwrap();
    assert_eq!(first, "조선");
    assert_eq!(texts(&e, "source", "name", a.doc), ["난중일기", "선조실록"]);
    assert_eq!(texts(&e, "entity", "entity", a.doc), ["01A"]);
    let home: Option<String> = e.debug_index().query_row("SELECT home FROM item WHERE doc = ?1", [a.doc], |r| r.get(0)).unwrap();
    assert_eq!(home.as_deref(), Some("조선"));
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn years_from_an_info_box() {
    let app = temp("해");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Template, "전쟁 정보", "기간: @기간@ @연도@", "").unwrap();
    let years = |e: &Engine, doc: DocId| -> Option<(i64, i64)> {
        e.debug_index().query_row("SELECT y0, y1 FROM years WHERE doc = ?1", [rowid(e, doc)], |r| Ok((r.get(0)?, r.get(1)?))).ok()
    };
    let a = e.create(Namespace::Doc, "임진왜란", "[include(틀:전쟁 정보, 기간=1592~1598)]", "").unwrap();
    assert_eq!(years(&e, a.doc), Some((1592, 1598)));
    let b = e.create(Namespace::Doc, "고구려", "[include(틀:전쟁 정보, 연도=기원전 57년)]", "").unwrap();
    assert_eq!(years(&e, b.doc), Some((-57, -57)));
    let c = e.create(Namespace::Doc, "해 없음", "그냥 글 1592", "").unwrap();
    assert_eq!(years(&e, c.doc), None);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn changing_a_template_redraws_only_its_users() {
    let app = temp("틀 바뀜");
    let mut e = Engine::open(&app).unwrap();
    let t = e.create(Namespace::Template, "T", "옛 틀 글", "").unwrap();
    let a = e.create(Namespace::Doc, "A", "[include(틀:T)]", "").unwrap();
    e.create(Namespace::Doc, "B", "틀 없음", "").unwrap();
    assert!(model(&mut e, a.doc).contains("옛 틀 글"));
    assert_eq!(texts(&e, "uses", "template", a.doc), ["틀:T"]);
    e.save(t.doc, t.rev, "새 틀 글", "").unwrap();
    assert_eq!(e.status().unwrap().stale, 1); // A만
    settle(&mut e);
    assert!(model(&mut e, a.doc).contains("새 틀 글"));
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_template_created_later_fills_its_users() {
    let app = temp("나중 틀");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "A", "[include(틀:나중)]", "").unwrap();
    assert!(model(&mut e, a.doc).contains("\"missing\""));
    e.create(Namespace::Template, "나중", "나중에 생긴 틀", "").unwrap();
    assert_eq!(e.status().unwrap().stale, 1);
    settle(&mut e);
    let m = model(&mut e, a.doc);
    assert!(m.contains("나중에 생긴 틀") && !m.contains("\"missing\""), "{m}");
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn renaming_a_template_marks_users_of_both_names() {
    let app = temp("틀 이름 바꿈");
    let mut e = Engine::open(&app).unwrap();
    let t = e.create(Namespace::Template, "옛", "틀 글", "").unwrap();
    let a = e.create(Namespace::Doc, "A", "[include(틀:옛)]", "").unwrap();
    let b = e.create(Namespace::Doc, "B", "[include(틀:새)]", "").unwrap();
    e.rename(t.doc, t.rev, "새", true, "").unwrap();
    assert_eq!(e.status().unwrap().stale, 2);
    settle(&mut e);
    assert!(model(&mut e, a.doc).contains("틀 글")); // 옛 이름은 넘겨주기를 한 번 따라간다
    assert!(model(&mut e, b.doc).contains("틀 글"));
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_template_read_error_is_not_frozen_as_missing() {
    let app = temp("틀 읽기 오류");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Template, "T", "틀 글", "").unwrap();
    let a = e.create(Namespace::Doc, "A", "첫 판 [include(틀:T)]", "").unwrap();
    e.fail_template_reads = true;
    e.save(a.doc, a.rev, "둘째 판\n[include(틀:T)]", "").unwrap();
    assert_eq!(e.status().unwrap().stale, 1);
    let stored: Vec<u8> = e.debug_index().query_row("SELECT z FROM model WHERE rowid = ?1", [rowid(&e, a.doc)], |r| r.get(0)).unwrap();
    let stored = String::from_utf8(zstd::stream::decode_all(&stored[..]).unwrap()).unwrap();
    assert!(!stored.contains("\"missing\"") && stored.contains("첫 판"), "{stored}");
    assert!(e.open_doc(a.doc, false).is_err());
    e.fail_template_reads = false;
    settle(&mut e);
    let m = model(&mut e, a.doc);
    assert!(m.contains("둘째 판") && m.contains("틀 글") && !m.contains("\"missing\""), "{m}");
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn reindexing_replaces_all_rows() {
    let app = temp("다시 색인");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "A", "[[가]] [[분류:옛]][* 『옛 사료』] [[연표:01A]]", "").unwrap();
    e.save(a.doc, a.rev, "[[나]] [[분류:새]][* 『새 사료』] [[연표:01B]]", "").unwrap();
    assert_eq!(link_keys(&e, a.doc), ["문서:나"]);
    assert_eq!(texts(&e, "member", "category", a.doc), ["새"]);
    assert_eq!(texts(&e, "source", "name", a.doc), ["새 사료"]);
    assert_eq!(texts(&e, "entity", "entity", a.doc), ["01B"]);
    let refs: Vec<(String, i64)> = {
        let mut st = e.debug_index().prepare("SELECT key, refs FROM target ORDER BY key").unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(|r| r.unwrap()).collect()
    };
    assert_eq!(refs, [("나".to_string(), 1)]);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn deleted_docs_leave_the_indexes() {
    let app = temp("지운 문서");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "A", "[[가]] 한산도 [[분류:조선]]", "").unwrap();
    e.delete(a.doc, a.rev, "").unwrap();
    assert!(link_keys(&e, a.doc).is_empty());
    assert!(texts(&e, "member", "category", a.doc).is_empty());
    let (deleted, fts): (i64, i64) = (
        e.debug_index().query_row("SELECT deleted FROM item WHERE doc = ?1", [a.doc], |r| r.get(0)).unwrap(),
        e.debug_index().query_row("SELECT count(*) FROM fts0 WHERE fts0 MATCH '\"한산\"'", [], |r| r.get(0)).unwrap(),
    );
    let others: i64 = ["fts1", "fts2", "fts3"]
        .iter()
        .map(|t| e.debug_index().query_row(&format!("SELECT count(*) FROM {t} WHERE {t} MATCH '\"한산\"'"), [], |r| r.get::<_, i64>(0)).unwrap())
        .sum();
    assert_eq!((deleted, fts + others), (1, 0));
    let refs: i64 = e.debug_index().query_row("SELECT count(*) FROM target", [], |r| r.get(0)).unwrap();
    assert_eq!(refs, 0);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}
