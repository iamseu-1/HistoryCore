//! hc-bench — 더미 서재를 들이고 일감을 재어 예산표(설계 9-2)와 견준다.
//!   hc-bench run --docs 100000 --dir <작업 폴더> [--seed 20261021] [--device pc] [--out bench/results] [--quick]
use history_bench::{naive::NaiveSqlite, run, Result, RunPlan};
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
    let mut lib = NaiveSqlite::open(&dir)?;
    eprintln!("들이고 재는 중 — {}", dir.display());
    let report = run(&mut lib, &corpus, &plan, &device)?;
    std::fs::create_dir_all(&out)?;
    let stem = format!("{}-{}-{}", device, report.library, docs);
    std::fs::write(out.join(format!("{stem}.json")), report.to_json())?;
    std::fs::write(out.join(format!("{stem}.md")), report.to_markdown())?;
    println!("{}", report.to_markdown());
    Ok(())
}
