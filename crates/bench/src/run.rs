use crate::{
    report::Report,
    stats::{Op, OpStats},
    workload::Workload,
    Library, Result,
};
use history_corpus::Corpus;
use std::time::Instant;

/// 일마다 몇 번 잴지.
#[derive(Clone, Debug)]
pub struct RunPlan {
    pub seed: u64,
    pub find_title: usize,
    pub search_body: usize,
    pub open: usize,
    pub save: usize,
    pub categories: usize,
    pub aggregates: usize,
    pub startups: usize,
}

impl RunPlan {
    pub fn standard(seed: u64) -> Self {
        RunPlan { seed, find_title: 2_000, search_body: 1_000, open: 2_000, save: 300, categories: 500, aggregates: 50, startups: 5 }
    }
    pub fn quick(seed: u64) -> Self {
        RunPlan { seed, find_title: 50, search_body: 50, open: 50, save: 20, categories: 20, aggregates: 5, startups: 2 }
    }
}

fn measure(ops: &mut Vec<OpStats>, op: Op, n: usize, mut f: impl FnMut(usize) -> Result<()>) -> Result<()> {
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let t = Instant::now();
        f(i)?;
        samples.push(t.elapsed());
    }
    ops.push(OpStats::from_samples(op, samples));
    Ok(())
}

/// 들이고, 일감을 돌리고, 예산표와 견준 보고를 만든다.
pub fn run(lib: &mut dyn Library, corpus: &Corpus, plan: &RunPlan, device: &str) -> Result<Report> {
    let work = Workload::new(corpus, plan);
    let mut docs = corpus.docs();
    let t = Instant::now();
    lib.load(&mut docs)?;
    let load = t.elapsed();
    let mut ops = Vec::new();
    measure(&mut ops, Op::FindTitle, work.title_prefixes.len(), |i| lib.find_title(&work.title_prefixes[i], 20).map(drop))?;
    measure(&mut ops, Op::SearchBody, work.body_queries.len(), |i| lib.search_body(&work.body_queries[i], 20).map(drop))?;
    measure(&mut ops, Op::Open, work.open_ids.len(), |i| lib.open(work.open_ids[i]).map(drop))?;
    measure(&mut ops, Op::CategoryChildren, work.categories.len(), |i| lib.category_children(&work.categories[i]).map(drop))?;
    measure(&mut ops, Op::HomeAggregates, plan.aggregates, |_| lib.home_aggregates().map(drop))?;
    measure(&mut ops, Op::Save, work.saves.len(), |i| {
        let (id, body) = &work.saves[i];
        lib.save(*id, body)
    })?;
    measure(&mut ops, Op::Startup, plan.startups, |_| {
        lib.reopen()?;
        lib.home_aggregates().map(drop)
    })?;
    let verdicts = Report::judge(&ops);
    let part = |v: Vec<(&'static str, u64)>| v.into_iter().map(|(label, value)| crate::report::Part { label: label.into(), value }).collect();
    Ok(Report {
        library: lib.name().into(),
        device: device.into(),
        docs: corpus.spec().docs,
        seed: corpus.spec().seed,
        load_ms: load.as_millis() as u64,
        load_parts: part(lib.load_parts()),
        bytes_on_disk: lib.bytes_on_disk()?,
        disk_parts: part(lib.disk_parts()),
        ops,
        verdicts,
    })
}
