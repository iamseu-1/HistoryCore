use history_corpus::{Corpus, CorpusSpec, Ns, TextPool};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

fn digest(c: &Corpus) -> Vec<u8> {
    let mut h = Sha256::new();
    for d in c.docs() {
        h.update(d.title.as_bytes());
        h.update(d.body.as_bytes());
    }
    h.finalize().to_vec()
}

#[test]
fn embedded_pool_is_large_enough() {
    let p = TextPool::embedded();
    assert!(p.sentences.len() >= 10_000, "문장 {}", p.sentences.len());
    assert!(p.words.len() >= 2_000, "낱말 {}", p.words.len());
}

#[test]
fn same_seed_same_library_other_seed_other_library() {
    let pool = TextPool::embedded();
    let a = Corpus::new(CorpusSpec { docs: 2_000, seed: 7 }, &pool);
    let b = Corpus::new(CorpusSpec { docs: 2_000, seed: 7 }, &pool);
    let c = Corpus::new(CorpusSpec { docs: 2_000, seed: 8 }, &pool);
    assert_eq!(digest(&a), digest(&b));
    assert_ne!(digest(&a), digest(&c));
}

#[test]
fn doc_i_does_not_depend_on_earlier_docs() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 3_000, seed: 1 }, &pool);
    let from_stream: Vec<_> = c.docs().skip(2_500).take(3).collect();
    let direct: Vec<_> = (2_500..2_503).map(|i| c.doc(i)).collect();
    assert_eq!(from_stream, direct);
}

#[test]
fn counts_match_and_titles_are_unique_per_namespace() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 5_000, seed: 3 }, &pool);
    let docs: Vec<_> = c.docs().collect();
    assert_eq!(docs.len() as u32, c.len());
    assert_eq!(docs.iter().filter(|d| d.ns == Ns::Doc).count(), 5_000);
    let mut seen = HashSet::new();
    for d in &docs {
        assert!(seen.insert((d.ns == Ns::Doc, d.title.clone())), "제목 겹침: {}", d.title);
    }
}

#[test]
fn first_category_is_home_and_every_category_reaches_a_root() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 5_000, seed: 3 }, &pool);
    let tree = c.tree();
    let index: HashMap<&str, u32> = tree.categories().iter().enumerate().map(|(i, k)| (k.name.as_str(), i as u32)).collect();
    let mut unfiled = 0;
    for d in c.docs().filter(|d| d.ns == Ns::Doc) {
        match &d.home {
            None => {
                unfiled += 1;
                assert!(!d.body.starts_with("[[분류:"));
                assert!(d.group.is_none());
            }
            Some(h) => {
                assert!(d.body.starts_with(&format!("[[분류:{h}]]")), "{}", d.title);
                let path = tree.path(index[h.as_str()]);
                assert!(path.len() <= 4 && tree.get(*path.last().unwrap()).parent.is_none());
                assert!(d.group.is_some());
            }
        }
    }
    assert!((25..=80).contains(&unfiled), "분류 안 됨 {unfiled}편 — 1% 안팎이어야 함");
}

#[test]
fn brackets_and_footnotes_close() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 2_000, seed: 5 }, &pool);
    for d in c.docs() {
        assert_eq!(d.body.matches("[[").count(), d.body.matches("]]").count(), "{}", d.title);
        assert_eq!(d.body.matches("[*").count(), d.body.matches("권.]").count(), "{}", d.title);
    }
}

#[test]
fn bodies_carry_hanja_including_beyond_bmp() {
    // 설계 9-1 P1: "실제 같은 … 한자". 재료(康熙字典 번역층)의 문단 열에 아홉에 한자, 일곱에 하나꼴로 확장 B 이후 한자가 있다.
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 2_000, seed: 8 }, &pool);
    let is_han = |ch: char| ('\u{4E00}'..='\u{9FFF}').contains(&ch) || ('\u{3400}'..='\u{4DBF}').contains(&ch) || ch >= '\u{20000}';
    let (mut han, mut beyond) = (0, 0);
    for d in c.docs().filter(|d| d.ns == Ns::Doc) {
        if d.body.chars().any(is_han) {
            han += 1;
        }
        if d.body.chars().any(|ch| ch >= '\u{20000}') {
            beyond += 1;
        }
    }
    assert!(han >= 1_000, "한자가 든 문서 {han}/2000");
    assert!(beyond >= 1, "확장 B 이후 한자가 든 문서가 없음");
}

#[test]
fn a_few_links_point_to_missing_docs() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 5_000, seed: 9 }, &pool);
    let titles: HashSet<&str> = (0..5_000).map(|i| c.title(i)).collect();
    let (mut all, mut missing) = (0usize, 0usize);
    for d in c.docs().filter(|d| d.ns == Ns::Doc) {
        for part in d.body.split("[[").skip(1) {
            let target = part.split("]]").next().unwrap();
            if target.starts_with("분류:") {
                continue;
            }
            all += 1;
            if !titles.contains(target) {
                missing += 1;
            }
        }
    }
    let share = missing as f64 / all as f64;
    assert!((0.02..0.09).contains(&share), "없는 문서 링크 비율 {share:.3}");
}
