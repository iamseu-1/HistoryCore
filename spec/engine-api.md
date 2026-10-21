# 엔진 질의 약속

다리 `hc_call(요청 JSON) → 답 JSON`(UTF-8). 앱은 이 문서만 보고 엔진을 부른다. P2-1 · P2-2가 정한 것은 **됨**, 뒤 계획이 채울 것은 **예정**(모양은 그 계획에서 굳히고 이 문서를 고친다 — 아직 출시 전이라 ABI 판은 1 그대로).

## 약속

- 요청은 `{"op": "<이름>", …}`. 답은 늘 `ok`를 갖는다.
- 실패: `{"ok": false, "error": "<사람이 읽는 글>", "code": "<짧은 이름>"}`.
- ID는 UUID 글(소문자 · 하이픈). 시각(`hlc`)은 수 — `(ms << 16) | 카운터`.
- 이름공간 `ns`: `doc`(문서) · `category`(분류) · `template`(틀) · `file`(파일) · `front`(대문).
- 서재 op는 `library.open`이 준 `handle`을 받는다.

| code | 뜻 |
| --- | --- |
| `bad_request` | 요청 모양이 틀림(빠진 값 · UUID 아님 · 모르는 ns) |
| `bad_handle` | 열린 서재가 아님(닫았거나 없음) |
| `not_found` | 그런 문서 · 판이 없음 |
| `bad_title` | 제목으로 쓸 수 없음(spec/library-format.md「제목」) |
| `stale_base` | 고치기 시작한 판이 지금 머리가 아님 — 덮어쓰지 않았다. 답에 `head`(지금 머리 판) |
| `deleted` | 지운 문서에는 할 수 없음 |
| `not_a_library` | 서재 자리에 다른 파일 — 건드리지 않았다 |
| `newer_format` | 더 새 형식의 서재 — 건드리지 않았다 |
| `corrupt` · `sqlite` · `io` | 저장소 · 파일 문제 |

## 됨 (P2-1 · P2-2)

| op | 요청 | 답 |
| --- | --- | --- |
| `version` | — | `engine` · `abi` |
| `library.open` | `dir`(앱 자료 폴더) | `handle` · `library` · `device` · `format` · `folder`(`<dir>/HistoryCore`). 같은 폴더를 다시 열면 같은 `handle` |
| `library.close` | `handle` | — |
| `library.counts` | `handle` | `doc` · `category` · `template` · `file` · `front`(살아 있는 수) · `deleted` · `total` |
| `doc.create` | `handle` · `ns` · `title` · `body` · `message`? | `doc` · `rev` · `hlc` · `changed`(참) |
| `doc.save` | `handle` · `doc` · `base` · `body` · `message`? | `doc` · `rev` · `hlc` · `changed`(본문이 같으면 거짓, 판을 더하지 않음) |
| `doc.rename` | `handle` · `doc` · `base` · `title` · `keep_redirect`?(기본 참) · `message`? | `rev` · `hlc` · `redirect`(옛 제목의 넘겨주기 문서 ID 또는 null) |
| `doc.delete` | `handle` · `doc` · `base` · `message`? | `doc` · `rev` · `hlc` · `changed` |
| `doc.get` | `handle` · `doc` | `doc`: `id` · `ns` · `title` · `head` · `created` · `updated` · `deleted` · `body` |
| `doc.by_title` | `handle` · `ns` · `title` | `docs`: [`id` · `ns` · `title` · `head` · `created` · `updated`] — 만든 차례, 지운 것 뺌 |
| `doc.history` | `handle` · `doc` · `limit`?(50, 많아야 1000) · `before`?(hlc) | `revisions`: [`id` · `doc` · `parent` · `parent2` · `device` · `hlc` · `kind`(`content`/`deleted`) · `ns` · `title` · `message` · `len`] — 새 것부터 |
| `rev.get` | `handle` · `rev` | `revision`: 위 판 정보 + `body` |
| `namumark.parse` | `text` · `title`? · `ns`?(`template`이면 틀 문서로 봄) · `handle`?(있으면 그 서재의 틀을 펼침) | `model`: 그리기 모델(spec/namumark.md) |

## 예정

| op | 계획 | 하는 일 |
| --- | --- | --- |
| `doc.open` | P2-3 | 저장 때 굳힌 그리기 모델 · 분류 · 각주 · 넘겨주기 따라가기(한 번) |
| `titles.exist` | P2-3 | 제목 목록 → 있는 문서 ID(설계 5-6, 링크 색) |
| `search.titles` · `search.body` | P2-3 | 제목 앞부분 · 본문 찾기(한글 · 한자 2글자 조각), 첫 n건 |
| `tree.children` | P2-3 | 분류 폴더 하나: 하위 분류(아래 문서 합) · 문서 |
| `links.back` · `links.wanted` | P2-3 | 가리키는 문서 · 만들 문서 |
| `home.aggregates` · `stats.*` | P2-3 | 대문 · 서재 통계 모음(설계 5-5) |
| `timeline.docs` | P2-3 | 연표 항목ID → 이어진 문서(창구 `docs_for_entity`) |
| `hanja.get` · `hanja.search` | P2-4 | 자전 조목 · 찾기 |
