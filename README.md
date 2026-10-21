# HistoryCore

역사 앱 생태계(아카이브 · HistoryTime · 히스토리챗)가 함께 쓰는 셈과 그림. 설계: [docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md](docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md).

| 자리 | 하는 일 |
| --- | --- |
| `crates/library` | 서재 엔진 |
| `crates/ffi` | Flutter 다리 — C 함수 `hc_call`(UTF-8 JSON 요청 → 답) · `hc_free` |
| `crates/corpus` | 100만 건 측정용 더미 서재 |
| `crates/bench` | 측정 판 `hc-bench` |
| `packages/history_core` | Flutter FFI 플러그인(미리 지은 엔진을 앱에 싣는다) |
| `spec/` | 앱 사이의 약속 문서 |

    cargo test --locked --workspace
    ./tools/build-native.ps1 -Android          # 엔진을 packages/history_core/native에 놓는다
