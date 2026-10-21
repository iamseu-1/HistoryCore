use history_bench::{naive::NaiveSqlite, Library};
use history_corpus::{Corpus, CorpusSpec, TextPool};
use std::path::PathBuf;

/// 한글 · 공백이 든 작업 폴더(Review Focus 1).
fn workdir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("측정 시험 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// 이어진 한글 n글자(본문에 실제로 있는 검색어).
fn hangul_run(text: &str, n: usize) -> String {
    for w in text.split(|ch: char| !('가'..='힣').contains(&ch)) {
        if w.chars().count() >= n {
            return w.chars().take(n).collect();
        }
    }
    panic!("한글 {n}글자 이어진 곳이 없음");
}

#[test]
fn answers_like_a_library() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 2_000, seed: 4 }, &pool);
    let dir = workdir("기본");
    let mut lib = NaiveSqlite::open(&dir).unwrap();
    lib.load(&mut c.docs()).unwrap();
    let first = c.len() - 2_000;
    let id = first + 10;
    let body = c.doc(id).body;

    // 제목
    let prefix: String = c.title(10).chars().take(2).collect();
    let hits = lib.find_title(&prefix, 50).unwrap();
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|h| c.doc(*h).title.starts_with(&prefix)));

    // 본문 — 3글자(trigram) · 2글자(훑기)
    let q3 = hangul_run(&body, 3);
    let found = lib.search_body(&q3, 20).unwrap();
    assert!(!found.is_empty());
    assert!(found.iter().all(|h| c.doc(*h).body.contains(&q3)));
    let q2 = hangul_run(&body, 2);
    assert!(!lib.search_body(&q2, 20).unwrap().is_empty());

    // 열기 · 저장 · 다시 찾기
    assert_eq!(lib.open(id).unwrap().unwrap(), body);
    let new = format!("{body}새로 넣은 문장 가나다라마바사.\n");
    lib.save(id, &new).unwrap();
    assert_eq!(lib.open(id).unwrap().unwrap(), new);
    assert!(lib.search_body("가나다라마바사", 20).unwrap().contains(&id));

    // 분류 · 모음
    if let Some(home) = &c.doc(id).home {
        assert!(lib.category_children(home).unwrap().contains(&id));
    }
    let agg = lib.home_aggregates().unwrap();
    assert_eq!(agg.groups.iter().map(|g| g.1).sum::<u64>(), 2_000);
    assert_eq!(agg.recent[0], id);

    // 다시 열기 · 크기
    lib.reopen().unwrap();
    assert_eq!(lib.open(id).unwrap().unwrap(), new);
    assert!(lib.bytes_on_disk().unwrap() > 0);
    drop(lib);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn loading_again_starts_clean() {
    // Review Focus 3 — 끊긴 측정이 남긴 DB 위에 다시 들인다
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 500, seed: 6 }, &pool);
    let dir = workdir("다시");
    let mut lib = NaiveSqlite::open(&dir).unwrap();
    lib.load(&mut c.docs().take(100)).unwrap();
    drop(lib);
    let mut lib = NaiveSqlite::open(&dir).unwrap();
    lib.load(&mut c.docs()).unwrap();
    let agg = lib.home_aggregates().unwrap();
    assert_eq!(agg.groups.iter().map(|g| g.1).sum::<u64>(), 500);
    drop(lib);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn saving_a_missing_doc_is_an_error_not_a_crash() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 100, seed: 1 }, &pool);
    let dir = workdir("없는 문서");
    let mut lib = NaiveSqlite::open(&dir).unwrap();
    lib.load(&mut c.docs()).unwrap();
    assert!(lib.save(9_999_999, "x").is_err());
    assert_eq!(lib.open(9_999_999).unwrap(), None);
    drop(lib);
    std::fs::remove_dir_all(&dir).unwrap();
}
