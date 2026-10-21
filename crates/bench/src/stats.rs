use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Op {
    FindTitle,
    SearchBody,
    Open,
    Save,
    CategoryChildren,
    HomeAggregates,
    Startup,
}

impl Op {
    pub fn label(self) -> &'static str {
        match self {
            Op::FindTitle => "제목 찾기",
            Op::SearchBody => "본문 찾기 첫 20건",
            Op::Open => "문서 열기",
            Op::Save => "저장 한 편",
            Op::CategoryChildren => "분류 폴더 펼치기",
            Op::HomeAggregates => "대문 · 통계 모음",
            Op::Startup => "다시 열기 → 대문 모음",
        }
    }
}

/// 일 하나를 여러 번 잰 값(마이크로초).
#[derive(Clone, Debug, Serialize)]
pub struct OpStats {
    pub op: Op,
    pub count: usize,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub max_us: u64,
}

impl OpStats {
    pub fn from_samples(op: Op, mut samples: Vec<Duration>) -> Self {
        assert!(!samples.is_empty(), "잰 값이 없음: {op:?}");
        samples.sort();
        let us = |d: Duration| d.as_micros() as u64;
        OpStats {
            op,
            count: samples.len(),
            p50_us: us(nearest_rank(&samples, 50.0)),
            p95_us: us(nearest_rank(&samples, 95.0)),
            p99_us: us(nearest_rank(&samples, 99.0)),
            max_us: us(*samples.last().expect("비지 않음")),
        }
    }
}

/// 정렬된 값의 nearest-rank 백분위(0 < p ≤ 100).
pub fn nearest_rank(sorted: &[Duration], p: f64) -> Duration {
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}
