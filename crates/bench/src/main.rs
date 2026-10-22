//! hc-bench — 더미 서재를 들이고 일감을 재어 예산표(설계 9-2)와 견준다.
//!   hc-bench run --docs 100000 --dir <작업 폴더> [--seed 20261021] [--device pc] [--out bench/results] [--quick] [--library engine|naive] [--workers <수>]
//!   hc-bench search --docs 1000000 --dir <작업 폴더> [--seed 20261021] [--workers <수>] — 검색 조각 색인만 지어 잰다(P2-3)
use history_bench::{engine::EngineLibrary, naive::NaiveSqlite, run, Library, Result, RunPlan};
use history_corpus::{Corpus, CorpusSpec, TextPool};
use std::path::PathBuf;

const USAGE: &str = "쓰는 법: hc-bench run --docs <수> --dir <작업 폴더> [--seed <수>] [--device <이름>] [--out <결과 폴더>] [--quick]";

fn main() {
    if let Err(e) = real_main() {
        eprintln!("hc-bench: {e}");
        std::process::exit(1);
    }
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn real_main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("search") {
        return search(&args);
    }
    if args.first().map(String::as_str) != Some("run") {
        return Err(USAGE.into());
    }
    let docs: u32 = arg(&args, "--docs").ok_or(USAGE)?.parse()?;
    let dir = PathBuf::from(arg(&args, "--dir").ok_or(USAGE)?);
    let seed: u64 = arg(&args, "--seed").map(|s| s.parse()).transpose()?.unwrap_or(20_261_021);
    let device = arg(&args, "--device").unwrap_or_else(|| "pc".into());
    let out = PathBuf::from(arg(&args, "--out").unwrap_or_else(|| "bench/results".into()));
    let plan = if args.iter().any(|a| a == "--quick") { RunPlan::quick(seed) } else { RunPlan::standard(seed) };

    let pool = TextPool::embedded();
    eprintln!("더미 서재 {docs}편을 준비한다(씨앗 {seed})…");
    let corpus = Corpus::new(CorpusSpec { docs, seed }, &pool);
    let mut lib: Box<dyn Library> = match arg(&args, "--library").as_deref().unwrap_or("engine") {
        "naive" => Box::new(NaiveSqlite::open(&dir)?),
        "engine" => Box::new(EngineLibrary::open(&dir, workers(&args)?)?),
        other => return Err(format!("모르는 --library: {other}").into()),
    };
    eprintln!("들이고 재는 중 — {} · {}", lib.name(), dir.display());
    let report = run(lib.as_mut(), &corpus, &plan, &device)?;
    std::fs::create_dir_all(&out)?;
    let stem = format!("{}-{}-{}", device, report.library, docs);
    std::fs::write(out.join(format!("{stem}.json")), report.to_json())?;
    std::fs::write(out.join(format!("{stem}.md")), report.to_markdown())?;
    println!("{}", report.to_markdown());
    Ok(())
}

/// 통째 짓기의 해석 일꾼 수 — 쓰는 스레드 다섯(본 표 하나 · 조각 색인 넷)과 원천 하나를 빼고 남는 코어.
fn workers(args: &[String]) -> Result<usize> {
    Ok(match arg(args, "--workers") {
        Some(w) => w.parse()?,
        None => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8).saturating_sub(6).max(2),
    })
}

fn search(args: &[String]) -> Result<()> {
    let docs: u32 = arg(args, "--docs").ok_or(USAGE)?.parse()?;
    let dir = PathBuf::from(arg(args, "--dir").ok_or(USAGE)?);
    let seed: u64 = arg(args, "--seed").map(|s| s.parse()).transpose()?.unwrap_or(20_261_021);
    let workers = match arg(args, "--workers") {
        Some(w) => w.parse()?,
        None => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).saturating_sub(1).max(1),
    };
    let pool = TextPool::embedded();
    let corpus = Corpus::new(CorpusSpec { docs, seed }, &pool);
    eprintln!("검색 조각 색인을 짓는다 — {docs}편, 일꾼 {workers}");
    let r = history_bench::search_spike::spike(&corpus, &dir, workers, seed)?;
    println!("{}", r.to_markdown());
    Ok(())
}
