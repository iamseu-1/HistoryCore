# HistoryCore

역사 앱 생태계(아카이브 · HistoryTime · 히스토리챗)가 함께 쓰는 셈과 그림. 설계: [docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md](docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md).

| 자리 | 하는 일 |
| --- | --- |
| `crates/library` | 서재 엔진 |
| `crates/ffi` | Flutter 다리 — C 함수 `hc_call`(UTF-8 JSON 요청 → 답) · `hc_free` |
| `crates/corpus` | 100만 건 측정용 더미 서재 |
| `crates/bench` | 측정 판 `hc-bench` |
| `packages/history_core` | Flutter FFI 플러그인(미리 지은 엔진을 앱에 싣는다) |
| `packages/library_view` | 그리기 모델 JSON → 문서 전문 · 한자 카드 위젯. 엔진 없이 모델과 콜백만 받는다(아카이브 · HistoryTime이 함께 씀) — **짓는 중**(가지 `remake-7-link`, 계획 `docs/superpowers/plans/2026-10-25-p5-window-library-view.md`) |
| `spec/` | 앱 사이의 약속 문서 — 서재 형식 · 엔진 질의 · 나무마크 · 드라이브 형식 · 창구(`spec/windows-api.md` — 두 앱이 같은 기기에서 서로 읽는 길 · 딥링크) |

    cargo test --locked --workspace
    ./tools/build-native.ps1 -Android          # 엔진을 packages/history_core/native에 놓는다
