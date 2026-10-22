// 측정용 버릴 도구 — P2-3 과제 9가 끝나면 지운다(인수인계 2026-10-22).
// 문서 하나를 색인할 때 단계별 CPU(버릴 도구).
use history_corpus::{Corpus, CorpusSpec, TextPool};
use history_namumark::{plain_document, NoTemplates, RenderOptions};
use std::time::{Duration, Instant};
fn main() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 3000, seed: 20261021 }, &pool);
    let docs: Vec<_> = c.docs().skip(500).take(2000).collect();
    let mut t = [Duration::ZERO; 7];
    let mut bytes = 0usize;
    for d in &docs {
        bytes += d.body.len();
        let a = Instant::now();
        let (m, tree) = history_namumark::render_document(&d.body, &RenderOptions { title: Some(d.title.clone()), template: false }, &NoTemplates);
        t[0] += a.elapsed();
        let a = Instant::now();
        let _ = 0;
        t[1] += a.elapsed();
        let a = Instant::now();
        let plain = history_index::search::stored_plain(&plain_document(&tree));
        t[2] += a.elapsed();
        let a = Instant::now();
        let tokens = history_index::search::index_text(&plain);
        t[3] += a.elapsed();
        let a = Instant::now();
        let json = serde_json::to_vec(&m).unwrap();
        t[4] += a.elapsed();
        let a = Instant::now();
        let z1 = zstd::bulk::compress(&json, 1).unwrap();
        let z2 = zstd::bulk::compress(plain.as_bytes(), 1).unwrap();
        t[5] += a.elapsed();
        let a = Instant::now();
        let _ = history_namumark::out_links(&tree);
        let _ = history_namumark::timeline_entities(&tree);
        t[6] += a.elapsed();
        std::hint::black_box((z1, z2, tokens));
    }
    let n = docs.len() as f64;
    for (name, d) in ["render", "parse(둘째)", "plain", "tokens", "json", "zstd 둘", "links · entities"].iter().zip(t) {
        println!("{name:<16} {:>7.1}µs", d.as_secs_f64() * 1e6 / n);
    }
    println!("본문 평균 {:.0}B", bytes as f64 / n);
}
