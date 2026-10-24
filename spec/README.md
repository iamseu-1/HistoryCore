# 약속 문서

설계 4-1의 `spec/`. P2부터 채운다: 서재 형식(`library-format.md`) · 드라이브 형식(`drive-format.md`) · 창구 질의(`windows-api.md`) · 나무마크 문법(`namumark.md`).
- [서재 형식](library-format.md) — `library.db`(P2-1)
- [엔진 질의](engine-api.md) — 다리 op 전체(P2-1에서 정함, 몇은 뒤 계획에서 채움)
- [나무마크 문법과 그리기 모델](namumark.md) — 해석기 · 모델 JSON(P2-2)
- [드라이브 형식](drive-format.md) — 동기화 묶음 · 자동 합치기 · 충돌(P4-2)
- [창구 약속](windows-api.md) — 같은 기기의 아카이브 · HistoryTime이 서로 읽는 길(서재 창구 · 연표 창구) · Windows 자리 · 딥링크 · Android 이름(P5, `api_version` 1)

지금 정해진 것 — 엔진 다리: `hc_call(요청 JSON) → 답 JSON`. 답은 늘 `ok`를 가진다. `{"op":"version"}` → `{"ok":true,"engine":"0.0.1","abi":1}`.
