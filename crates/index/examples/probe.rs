// 측정용 버릴 도구 — P2-3 과제 9가 끝나면 지운다(인수인계 2026-10-22).
// 측정 폴더(10만 편)에서 통째 짓기 단계별 시간 · 분류 폴더 · 저장을 잰다(버릴 도구).
use history_index::Engine;
use std::time::Instant;
fn main() {
    let dir = std::path::Path::new(r"D:\hc-bench-engine");
    let mut e = Engine::open(dir).unwrap();
    let workers: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(6);
    let t = Instant::now();
    let r = e.rebuild_index(workers).unwrap();
    println!("rebuild {:?} (일꾼 {workers}) {r:?}", t.elapsed());
    let home: String = e.debug_index().query_row("SELECT home FROM item WHERE ns = 0 AND home IS NOT NULL LIMIT 1", [], |r| r.get(0)).unwrap();
    let t = Instant::now();
    let c = e.children(Some(&home), 200, None).unwrap();
    println!("children {} docs {:?}", c.docs.len(), t.elapsed());
}
