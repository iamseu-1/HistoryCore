//! 질의 — 제목 · 본문 · 링크 · 만들 문서 · 연표(Review Focus 4).
use history_index::Engine;
use history_library::Namespace;
use std::path::PathBuf;
use unicode_normalization::UnicodeNormalization;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("색인 질의 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn titles(e: &mut Engine, q: &str) -> Vec<String> {
    e.search_body(q, 20).unwrap().into_iter().map(|h| h.doc.title).collect()
}

#[test]
fn titles_exist_in_order_with_nulls_and_normalization() {
    let app = temp("있는 제목");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "임진왜란", "", "").unwrap();
    e.create(Namespace::Doc, "임진왜란", "같은 제목 둘째", "").unwrap();
    let c = e.create(Namespace::Category, "조선", "", "").unwrap();
    let got = e
        .titles_exist(&[(Namespace::Doc, "임진왜란".into()), (Namespace::Doc, "없음".into()), (Namespace::Doc, " 임진왜란 ".into()), (Namespace::Category, "조선".into()), (Namespace::Doc, "분류:조선".into())])
        .unwrap();
    assert_eq!(got, [Some(a.doc), None, Some(a.doc), Some(c.doc), None]);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn title_prefix() {
    let app = temp("제목 앞");
    let mut e = Engine::open(&app).unwrap();
    for t in ["임진왜란", "임진강", "임꺽정", "정유재란", "임진년"] {
        e.create(Namespace::Doc, t, "", "").unwrap();
    }
    let d = e.create(Namespace::Doc, "임진각", "", "").unwrap();
    e.delete(d.doc, d.rev, "").unwrap();
    let got: Vec<String> = e.search_titles(Namespace::Doc, "임진", 2).unwrap().into_iter().map(|d| d.title).collect();
    assert_eq!(got, ["임진강", "임진년"]);
    let all: Vec<String> = e.search_titles(Namespace::Doc, "임진", 20).unwrap().into_iter().map(|d| d.title).collect();
    assert_eq!(all, ["임진강", "임진년", "임진왜란"]);
    assert!(e.search_titles(Namespace::Category, "임진", 20).unwrap().is_empty());
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn body_search_cases() {
    let app = temp("본문 찾기");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Doc, "가", "'''임진'''왜란은 [[조선]]의 큰 전쟁이다.", "").unwrap();
    e.create(Namespace::Doc, "나", "학(鶴)은 물새다. 𪄃도 있다.", "").unwrap();
    e.create(Namespace::Doc, "다", "Joseon dynasty 왜란 이야기", "").unwrap();
    e.create(Namespace::Doc, "라", "임진년에 진왜가 왜란을", "").unwrap(); // 조각은 다 있지만 「임진왜란」은 아니다
    assert_eq!(titles(&mut e, "임진왜란"), ["가"]);
    assert_eq!(titles(&mut e, "왜란"), ["가", "다", "라"]);
    assert_eq!(titles(&mut e, "鶴"), ["나"]);
    assert_eq!(titles(&mut e, "𪄃"), ["나"]);
    assert_eq!(titles(&mut e, "왜란 조선"), ["가"]);
    assert_eq!(titles(&mut e, "JOSEON"), ["다"]);
    assert_eq!(titles(&mut e, "joseon 왜란"), ["다"]);
    let nfd: String = "임진왜란".nfd().collect();
    assert_eq!(titles(&mut e, &nfd), ["가"]);
    assert!(titles(&mut e, "없는말").is_empty());
    assert!(titles(&mut e, "  ").is_empty());
    let hit = &e.search_body("임진왜란", 20).unwrap()[0];
    assert!(hit.snippet.contains("임진왜란") && !hit.snippet.contains("'''") && !hit.snippet.contains("[["), "{}", hit.snippet);
    assert_eq!(e.search_body("왜란", 2).unwrap().len(), 2);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn body_search_forgets_old_text_and_deleted_docs() {
    let app = temp("옛 글 잊기");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "가", "한산도 대첩", "").unwrap();
    let b = e.create(Namespace::Doc, "나", "한산도 앞바다", "").unwrap();
    e.save(a.doc, a.rev, "명량 해전", "").unwrap();
    e.delete(b.doc, b.rev, "").unwrap();
    assert!(titles(&mut e, "한산도").is_empty());
    assert_eq!(titles(&mut e, "명량"), ["가"]);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn back_links_and_wanted() {
    let app = temp("링크");
    let mut e = Engine::open(&app).unwrap();
    let c = e.create(Namespace::Doc, "C", "", "").unwrap();
    let a = e.create(Namespace::Doc, "A", "[[없는 문서]] [[C]] [[C|다시]]", "").unwrap();
    e.create(Namespace::Doc, "B", "[[ 없는 문서 ]] [[또 없는 문서]]", "").unwrap();
    e.create(Namespace::Doc, "C 자신", "[[C 자신]]", "").unwrap();
    let back: Vec<String> = e.links_back(c.doc, 200).unwrap().into_iter().map(|d| d.title).collect();
    assert_eq!(back, ["A"]);
    let wanted = e.links_wanted(50).unwrap();
    let w: Vec<(Namespace, &str, u64)> = wanted.iter().map(|w| (w.ns, w.title.as_str(), w.refs)).collect();
    assert_eq!(w, [(Namespace::Doc, "없는 문서", 2), (Namespace::Doc, "또 없는 문서", 1)]);
    e.create(Namespace::Doc, "없는 문서", "이제 있다", "").unwrap();
    let w: Vec<String> = e.links_wanted(50).unwrap().into_iter().map(|w| w.title).collect();
    assert_eq!(w, ["또 없는 문서"]);
    let _ = a;
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn deleting_the_linked_doc_makes_it_wanted_again() {
    let app = temp("다시 만들 문서");
    let mut e = Engine::open(&app).unwrap();
    let t = e.create(Namespace::Doc, "대상", "", "").unwrap();
    e.create(Namespace::Doc, "가리킴", "[[대상]]", "").unwrap();
    assert!(e.links_wanted(50).unwrap().is_empty());
    e.delete(t.doc, t.rev, "").unwrap();
    let w: Vec<String> = e.links_wanted(50).unwrap().into_iter().map(|w| w.title).collect();
    assert_eq!(w, ["대상"]);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn timeline_links_and_strips() {
    let app = temp("연표");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Doc, "나", "[[연표:01A|임진왜란]]", "").unwrap();
    e.create(Namespace::Doc, "가", "[연표(01A, 범위=1592..1598)]", "").unwrap();
    e.create(Namespace::Doc, "다", "[[연표:01B]]", "").unwrap();
    let got: Vec<String> = e.timeline_docs("01A").unwrap().into_iter().map(|d| d.title).collect();
    assert_eq!(got, ["가", "나"]);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}
