/// SplitMix64 — 외부 상자 없이 플랫폼 · 판에 무관하게 늘 같은 수열을 낸다.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    /// 씨앗 · 갈래(salt) · 번호에서 갈라 낸 독립 수열 — 문서 i를 앞 문서 없이 만들 때 쓴다.
    pub fn for_item(seed: u64, salt: u64, i: u64) -> Self {
        let mut r = Rng(seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ i.wrapping_mul(0xD1B5_4A32_D192_ED03));
        r.next_u64();
        r
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0..n (n > 0). 이 쓰임에서 나머지 치우침은 무시할 만하다.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// [0, 1)
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    /// 앞쪽으로 쏠린 0..n — 앞 번호(인기 문서)가 자주 뽑힌다.
    pub fn skewed(&mut self, n: u64) -> u64 {
        let u = self.unit();
        ((u * u * u * n as f64) as u64).min(n.saturating_sub(1))
    }

    pub fn pick<'b, T>(&mut self, items: &'b [T]) -> &'b T {
        &items[self.below(items.len() as u64) as usize]
    }
}
