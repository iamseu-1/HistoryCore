//! 측정 판에 엔진을 꽂아 작은 서재로 한 바퀴 — 모든 일을 돌고 예산표를 다 낸다.
use history_bench::{engine::EngineLibrary, run, Library, RunPlan};
use history_corpus::{Corpus, CorpusSpec, TextPool};

#[test]
fn engine_quick_run_produces_a_report() {
    let dir = std::env::temp_dir().join(format!("측정 엔진 {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pool = TextPool::embedded();
    let corpus = Corpus::new(CorpusSpec { docs: 2_000, seed: 7 }, &pool);
    let mut lib = EngineLibrary::open(&dir, 2).unwrap();
    let r = run(&mut lib, &corpus, &RunPlan::quick(3), "시험").unwrap();
    assert_eq!((r.library.as_str(), r.ops.len(), r.verdicts.len()), ("engine", 7, 7));
    assert_eq!(r.load_parts.iter().take(2).map(|p| p.label.as_str()).collect::<Vec<_>>(), ["서재 들이기", "색인 짓기"]);
    assert_eq!(r.disk_parts.len(), 2);
    assert!(r.bytes_on_disk > 0);
    // 찾기 · 폴더 · 대문이 비지 않는다 — 엔진이 진짜로 색인했다.
    assert!(!lib.search_body(&corpus.pool().sentences[0].chars().filter(|c| ('가'..='힣').contains(c)).take(2).collect::<String>(), 20).unwrap().is_empty());
    assert!(!lib.home_aggregates().unwrap().groups.is_empty());
    let md = r.to_markdown();
    assert!(md.contains("들이기 나눔: 서재 들이기") && md.contains("저장 공간 나눔: 원본 library.db"), "{md}");
    drop(lib);
    std::fs::remove_dir_all(&dir).unwrap();
}
