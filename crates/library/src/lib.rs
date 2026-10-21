//! 서재 엔진. P0에서는 판 번호만 낸다(설계 9-1 P0).

/// 엔진 판 — 작업공간 판과 같다.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// 다리 · 창구 약속의 판. 요청 · 답의 모양이 바뀌면 올린다.
pub const ABI_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_three_numbers() {
        assert_eq!(super::ENGINE_VERSION.split('.').count(), 3);
        assert_eq!(super::ABI_VERSION, 1);
    }
}
