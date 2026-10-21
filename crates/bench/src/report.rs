use crate::stats::{Op, OpStats};
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub enum Stat {
    P95,
    P99,
}

/// 설계 9-2의 예산 한 줄.
pub struct Budget {
    pub label: &'static str,
    pub op: Op,
    pub stat: Stat,
    pub limit_us: u64,
}

/// 설계 9-2 가운데 이 판이 재는 일곱. 묶음 적용은 P4에서 재고, 통째 사본 들이기 · 저장 공간은 보고만 한다(Report의 load_ms · bytes_on_disk).
pub const BUDGETS: [Budget; 7] = [
    Budget { label: "제목 찾기 p99 ≤ 10ms", op: Op::FindTitle, stat: Stat::P99, limit_us: 10_000 },
    Budget { label: "본문 찾기 첫 20건 p95 ≤ 10ms", op: Op::SearchBody, stat: Stat::P95, limit_us: 10_000 },
    Budget { label: "문서 열기 p95 ≤ 16ms", op: Op::Open, stat: Stat::P95, limit_us: 16_000 },
    Budget { label: "저장 한 편 p95 ≤ 50ms", op: Op::Save, stat: Stat::P95, limit_us: 50_000 },
    Budget { label: "분류 폴더 하나 펼치기 ≤ 16ms", op: Op::CategoryChildren, stat: Stat::P95, limit_us: 16_000 },
    Budget { label: "대문 · 통계 모음 ≤ 50ms", op: Op::HomeAggregates, stat: Stat::P95, limit_us: 50_000 },
    Budget { label: "앱 시작 → 대문 ≤ 1초(엔진 몫)", op: Op::Startup, stat: Stat::P95, limit_us: 1_000_000 },
];

#[derive(Clone, Debug, Serialize)]
pub struct Verdict {
    pub label: String,
    pub limit_us: u64,
    pub measured_us: u64,
    pub pass: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub library: String,
    pub device: String,
    pub docs: u32,
    pub seed: u64,
    pub load_ms: u64,
    pub bytes_on_disk: u64,
    pub ops: Vec<OpStats>,
    pub verdicts: Vec<Verdict>,
}

impl Report {
    pub fn judge(ops: &[OpStats]) -> Vec<Verdict> {
        BUDGETS
            .iter()
            .filter_map(|b| {
                let s = ops.iter().find(|o| o.op == b.op)?;
                let measured = match b.stat {
                    Stat::P95 => s.p95_us,
                    Stat::P99 => s.p99_us,
                };
                Some(Verdict { label: b.label.into(), limit_us: b.limit_us, measured_us: measured, pass: measured <= b.limit_us })
            })
            .collect()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("보고는 늘 JSON이 된다")
    }

    pub fn to_markdown(&self) -> String {
        let ms = |us: u64| format!("{:.2}ms", us as f64 / 1000.0);
        let mut s = format!(
            "# 측정 — {} · {} · 문서 {}편\n\n들이기 {:.1}초 · 저장 공간 {:.1}MB · 씨앗 {}\n\n## 예산표(설계 9-2)\n\n| 예산 | 잰 값 | 통과 |\n| --- | --- | --- |\n",
            self.library,
            self.device,
            self.docs,
            self.load_ms as f64 / 1000.0,
            self.bytes_on_disk as f64 / 1_048_576.0,
            self.seed
        );
        for v in &self.verdicts {
            s += &format!("| {} | {} | {} |\n", v.label, ms(v.measured_us), if v.pass { "통과" } else { "못 미침" });
        }
        s += "\n## 잰 값\n\n| 일 | 횟수 | p50 | p95 | p99 | 최대 |\n| --- | --- | --- | --- | --- | --- |\n";
        for o in &self.ops {
            s += &format!("| {} | {} | {} | {} | {} | {} |\n", o.op.label(), o.count, ms(o.p50_us), ms(o.p95_us), ms(o.p99_us), ms(o.max_us));
        }
        s
    }
}
