//! 받은 판도 색인된다 · 합류해도 색인을 다시 짓지 않는다(spec/drive-format.md 4 · 6절).
use history_index::Engine;
use std::path::PathBuf;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("색인 동기화 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn received_documents_are_found_and_joining_keeps_the_index() {
    let mut a = Engine::open(&temp("가")).unwrap();
    let b_dir = temp("나");
    let mut b = Engine::open(&b_dir).unwrap();
    let mine = b.create(history_library::Namespace::Doc, "병자호란", "나의 문서 남한산성", "").unwrap();
    b.join(a.lib.info().library).unwrap();
    drop(b);
    let mut b = Engine::open(&b_dir).unwrap();
    assert_eq!(b.status().unwrap().stale, 0, "합류 뒤 다시 열어도 색인은 그대로");
    assert_eq!(b.search_body("남한산성", 10).unwrap().len(), 1);
    let s = a.create(history_library::Namespace::Doc, "임진왜란", "1592년 한산도 대첩", "").unwrap();
    let r = b.apply(&a.lib.export(0, 1).unwrap().unwrap().pack).unwrap();
    assert_eq!(r.touched, vec![s.doc]);
    let hits = b.search_body("한산도", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].doc.id, s.doc);
    assert_eq!(b.status().unwrap().stale, 0);
    assert_eq!(b.search_titles(history_library::Namespace::Doc, "병자", 10).unwrap()[0].id, mine.doc);
}
