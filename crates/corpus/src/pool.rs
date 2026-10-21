/// 더미 문서의 재료 — 《康熙字典》 한국어 번역층에서 뽑은 문장과 낱말(`assets/`, `tools/make-pool.py`).
pub struct TextPool {
    pub sentences: Vec<&'static str>,
    pub words: Vec<&'static str>,
}

impl TextPool {
    pub fn embedded() -> Self {
        fn lines(s: &'static str) -> Vec<&'static str> {
            s.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
        }
        TextPool { sentences: lines(include_str!("../assets/pool.txt")), words: lines(include_str!("../assets/words.txt")) }
    }
}
