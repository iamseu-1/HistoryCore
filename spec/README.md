# 약속 문서

설계 4-1의 `spec/`. P2부터 채운다: 서재 형식(`library-format.md`) · 드라이브 형식(`drive-format.md`) · 창구 질의(`windows-api.md`) · 나무마크 문법(`namumark.md`).

지금 정해진 것 — 엔진 다리: `hc_call(요청 JSON) → 답 JSON`. 답은 늘 `ok`를 가진다. `{"op":"version"}` → `{"ok":true,"engine":"0.0.1","abi":1}`.
