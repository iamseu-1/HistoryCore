use history_bench::{nearest_rank, run, Aggregates, Library, Op, Report, Result, RunPlan, Workload};
use history_corpus::{Corpus, CorpusSpec, GenDoc, TextPool};
use std::time::Duration;

/// 메모리 속 흉내 서재 — 판이 일을 다 돌리고 보고를 만드는지만 본다.
#[derive(Default)]
struct Fake {
    docs: Vec<GenDoc>,
}

impl Library for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }
    fn load(&mut self, docs: &mut dyn Iterator<Item = GenDoc>) -> Result<()> {
        self.docs = docs.collect();
        Ok(())
    }
    fn find_title(&self, p: &str, limit: usize) -> Result<Vec<u32>> {
        Ok(self.docs.iter().filter(|d| d.title.starts_with(p)).take(limit).map(|d| d.id).collect())
    }
    fn search_body(&self, q: &str, limit: usize) -> Result<Vec<u32>> {
        Ok(self.docs.iter().filter(|d| d.body.contains(q)).take(limit).map(|d| d.id).collect())
    }
    fn open(&self, id: u32) -> Result<Option<String>> {
        Ok(self.docs.get(id as usize).map(|d| d.body.clone()))
    }
    fn save(&mut self, id: u32, body: &str) -> Result<()> {
        self.docs[id as usize].body = body.into();
        Ok(())
    }
    fn category_children(&self, c: &str) -> Result<Vec<u32>> {
        Ok(self.docs.iter().filter(|d| d.home.as_deref() == Some(c)).map(|d| d.id).collect())
    }
    fn home_aggregates(&self) -> Result<Aggregates> {
        Ok(Aggregates::default())
    }
    fn reopen(&mut self) -> Result<()> {
        Ok(())
    }
    fn bytes_on_disk(&self) -> Result<u64> {
        Ok(0)
    }
}

#[test]
fn nearest_rank_percentiles() {
    let v: Vec<Duration> = (1..=100).map(Duration::from_millis).collect();
    assert_eq!(nearest_rank(&v, 50.0), Duration::from_millis(50));
    assert_eq!(nearest_rank(&v, 95.0), Duration::from_millis(95));
    assert_eq!(nearest_rank(&v, 99.0), Duration::from_millis(99));
    assert_eq!(nearest_rank(&[Duration::from_millis(7)], 99.0), Duration::from_millis(7));
}

#[test]
fn same_seed_same_workload() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 1_000, seed: 2 }, &pool);
    let a = Workload::new(&c, &RunPlan::quick(5));
    let b = Workload::new(&c, &RunPlan::quick(5));
    assert_eq!(a.title_prefixes, b.title_prefixes);
    assert_eq!(a.body_queries, b.body_queries);
    assert_eq!(a.open_ids, b.open_ids);
    assert!(a.body_queries.iter().all(|q| (2..=4).contains(&q.chars().count())));
}

#[test]
fn run_measures_every_op_and_judges_every_budget() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 1_000, seed: 2 }, &pool);
    let mut f = Fake::default();
    let r = run(&mut f, &c, &RunPlan::quick(2), "시험").unwrap();
    assert_eq!(r.ops.len(), 7);
    assert_eq!(r.verdicts.len(), 7);
    assert!(r.ops.iter().any(|o| o.op == Op::Startup));
    let md = r.to_markdown();
    assert!(md.contains("| 제목 찾기 p99 ≤ 10ms |"));
    assert!(md.contains("fake · 시험 · 문서 1000편"));
    let j: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
    assert_eq!(j["docs"], 1000);
    assert_eq!(j["verdicts"].as_array().unwrap().len(), 7);
}

#[test]
fn verdict_fails_when_over_budget() {
    let ops = vec![history_bench::OpStats { op: Op::FindTitle, count: 1, p50_us: 5, p95_us: 9_000, p99_us: 12_000, max_us: 12_000 }];
    let v = Report::judge(&ops);
    assert_eq!(v.len(), 1);
    assert!(!v[0].pass);
    assert_eq!(v[0].measured_us, 12_000);
}
