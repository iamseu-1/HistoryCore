//! P2-3 Task 2 — 검색 조각 색인만 따로 100만 편에 지어 재 본다(짓는 시간 · 크기 · 본문 찾기 첫 20건).
//! 짓기는 통째 짓기와 같은 모양이다: 일꾼이 해석 · plain · 조각내기, 쓰는 쪽 하나가 큰 트랜잭션으로 넣는다.
use crate::{nearest_rank, Result, RunPlan, Workload};
use history_corpus::Corpus;
use history_index::pipeline::ordered;
use history_index::search::{fts_match, index_tokens, parse_query, stored_plain, verify, FTS_SCHEMA};
use history_namumark::{parse, plain_document, Options};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::{Duration, Instant};

pub struct SpikeReport {
    pub docs: u32,
    pub build: Duration,
    pub optimize: Duration,
    pub bytes: u64,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub max: Duration,
    /// 질의 하나가 확인하려고 푼 후보 수의 평균.
    pub checked: f64,
    pub empty: usize,
}

pub fn spike(corpus: &Corpus, dir: &Path, workers: usize, seed: u64) -> Result<SpikeReport> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("search-spike.db");
    let reuse = std::env::var_os("HC_SPIKE_REUSE").is_some() && path.exists();
    if !reuse {
        let _ = std::fs::remove_file(&path);
    }
    let db = Connection::open(&path)?;
    let (mut build, mut optimize) = (Duration::ZERO, Duration::ZERO);
    if !reuse {
    db.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF; PRAGMA cache_size = -524288;")?;
    db.execute_batch(FTS_SCHEMA)?;
    db.execute_batch("CREATE TABLE plain(rowid INTEGER PRIMARY KEY, z BLOB NOT NULL);")?;
    // 짓는 동안: 메모리 해시를 크게(작은 조각 파일을 덜 만든다) · 자동 병합 끔 — 끝에 optimize 한 번.
    let hash: i64 = std::env::var("HC_SPIKE_HASH_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(256);
    db.execute("INSERT INTO fts(fts, rank) VALUES ('hashsize', ?1)", [hash * 1024 * 1024])?;
    db.execute("INSERT INTO fts(fts, rank) VALUES ('automerge', 0)", [])?;

    let t = Instant::now();
    db.execute_batch("BEGIN")?;
    let mut n = 0u32;
    ordered(
        corpus.docs(),
        workers,
        |d| {
            let plain = stored_plain(&plain_document(&parse(&d.body, &Options::default())));
            let z = zstd::bulk::compress(plain.as_bytes(), 3).expect("zstd");
            (d.id as i64 + 1, z, index_tokens(&plain).join(" "))
        },
        |(rowid, z, tokens)| -> Result<()> {
            db.prepare_cached("INSERT INTO plain(rowid, z) VALUES (?1, ?2)")?.execute(params![rowid, z])?;
            db.prepare_cached("INSERT INTO fts(rowid, t) VALUES (?1, ?2)")?.execute(params![rowid, tokens])?;
            n += 1;
            if n % 50_000 == 0 {
                db.execute_batch("COMMIT; BEGIN")?;
                eprintln!("  {n}편 · {:?}", t.elapsed());
            }
            Ok(())
        },
    )?;
    db.execute_batch("COMMIT")?;
    build = t.elapsed();
    let t = Instant::now();
    db.execute("INSERT INTO fts(fts) VALUES ('optimize')", [])?;
    optimize = t.elapsed();
    }
    let bytes = std::fs::metadata(&path)?.len();

    let work = Workload::new(corpus, &RunPlan::standard(seed));
    let mut samples = Vec::with_capacity(work.body_queries.len());
    let (mut checked, mut empty) = (0usize, 0usize);
    let mut unz = zstd::bulk::Decompressor::new()?;
    for q in &work.body_queries {
        let t = Instant::now();
        let Some(query) = parse_query(q) else { continue };
        let mut st = db.prepare_cached("SELECT rowid FROM fts WHERE fts MATCH ?1")?;
        let mut rows = st.query([fts_match(&query)])?;
        let mut hits = 0;
        let before = checked;
        while let Some(r) = rows.next()? {
            let rowid: i64 = r.get(0)?;
            if !query.verify.is_empty() {
                checked += 1;
                let z: Option<Vec<u8>> = db.prepare_cached("SELECT z FROM plain WHERE rowid = ?1")?.query_row([rowid], |r| r.get(0)).optional()?;
                let plain = String::from_utf8(unz.decompress(&z.unwrap_or_default(), 1 << 24)?)?;
                if !verify(&plain, &query) {
                    continue;
                }
            }
            hits += 1;
            if hits == 20 {
                break;
            }
        }
        if hits == 0 {
            empty += 1;
        }
        samples.push(t.elapsed());
        if std::env::var_os("HC_SPIKE_DIAG").is_some() && t.elapsed() > Duration::from_millis(10) {
            eprintln!("느림 {:>8.2}ms  {q:?}  must={:?} verify={:?} 확인={}", t.elapsed().as_secs_f64() * 1e3, query.must, query.verify, checked - before);
        }
    }
    samples.sort();
    let count = samples.len().max(1) as f64;
    Ok(SpikeReport {
        docs: corpus.len(),
        build,
        optimize,
        bytes,
        p50: nearest_rank(&samples, 50.0),
        p95: nearest_rank(&samples, 95.0),
        p99: nearest_rank(&samples, 99.0),
        max: *samples.last().unwrap_or(&Duration::ZERO),
        checked: checked as f64 / count,
        empty,
    })
}

impl SpikeReport {
    pub fn to_markdown(&self) -> String {
        format!(
            "검색 조각 측정 — {}편\n\n| 것 | 값 |\n| --- | --- |\n| 짓기 | {:.1}초 |\n| optimize | {:.1}초 |\n| 파일 크기 | {:.2}GB |\n| 본문 찾기 첫 20건 p50 · p95 · p99 · 최대 | {:.2} · {:.2} · {:.2} · {:.2}ms |\n| 질의당 확인한 후보 | {:.1} |\n| 0건 질의 | {} |\n",
            self.docs,
            self.build.as_secs_f64(),
            self.optimize.as_secs_f64(),
            self.bytes as f64 / 1e9,
            self.p50.as_secs_f64() * 1e3,
            self.p95.as_secs_f64() * 1e3,
            self.p99.as_secs_f64() * 1e3,
            self.max.as_secs_f64() * 1e3,
            self.checked,
            self.empty
        )
    }
}
