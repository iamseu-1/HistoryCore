use crate::run::RunPlan;
use history_corpus::{Corpus, Rng};

/// 정해진 씨앗에서 뽑은 일감 — 어떤 구현이든 같은 일감을 받는다.
pub struct Workload {
    pub title_prefixes: Vec<String>,
    pub body_queries: Vec<String>,
    pub open_ids: Vec<u32>,
    pub saves: Vec<(u32, String)>,
    pub categories: Vec<String>,
}

impl Workload {
    pub fn new(c: &Corpus, plan: &RunPlan) -> Self {
        let mut r = Rng::for_item(plan.seed, 21, 0);
        let docs = c.spec().docs;
        let first_article = c.len() - docs; // 분류 문서가 앞 번호를 쓴다
        let title_prefixes = (0..plan.find_title)
            .map(|_| {
                let t = c.title(r.below(docs as u64) as u32);
                t.chars().take(1 + r.below(3) as usize).collect()
            })
            .collect();
        let body_queries = (0..plan.search_body).map(|_| query_from(&mut r, c)).collect();
        let open_ids = (0..plan.open).map(|_| first_article + r.below(docs as u64) as u32).collect();
        let saves = (0..plan.save)
            .map(|_| {
                let id = first_article + r.below(docs as u64) as u32;
                let mut body = c.doc(id).body;
                body.push_str(*r.pick(&c.pool().sentences));
                body.push('\n');
                (id, body)
            })
            .collect();
        let categories = (0..plan.categories).map(|_| c.tree().get(*r.pick(c.tree().leaves())).name.clone()).collect();
        Workload { title_prefixes, body_queries, open_ids, saves, categories }
    }
}

/// 문장 속 한글 2~4글자 덩이 — 사람이 칠 만한 검색어.
fn query_from(r: &mut Rng, c: &Corpus) -> String {
    loop {
        let s = r.pick(&c.pool().sentences);
        let words: Vec<&str> = s.split(|ch: char| !('가'..='힣').contains(&ch)).filter(|w| (2..=4).contains(&w.chars().count())).collect();
        if !words.is_empty() {
            return r.pick(&words).to_string();
        }
    }
}
