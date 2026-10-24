# 엔진 질의 약속

다리 `hc_call(요청 JSON) → 답 JSON`(UTF-8). 앱은 이 문서만 보고 엔진을 부른다. P2-1 · P2-2 · P2-3이 정한 것은 **됨**, 뒤 계획이 채울 것은 **예정**(모양은 그 계획에서 굳히고 이 문서를 고친다 — 아직 출시 전이라 ABI 판은 1 그대로).

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
| `wrong_library` | 다른 서재의 묶음 — 들지 않았다. 답에 `found`(묶음의 서재) |
| `bad_pack` | 묶음을 풀 수 없거나 모양이 틀림 |
| `not_a_leaf` | 견주어 합칠 판이 그 문서의 다른 내용 잎이 아님 |
| `no_library` | 읽기 전용으로 열 서재가 없음(폴더 · `library.db`가 없거나 비었음) — 아무것도 만들지 않았다 |
| `read_only` | 읽기 전용 손잡이로는 할 수 없는 op — 그 손잡이는 `window.*` · `library.close`만 받는다 |
| `no_index` | 쓸 수 있는 색인이 없음(없거나 판 · 서재가 다름) — 읽기 전용 열기는 색인을 짓지 않으므로 색인을 쓰는 창구 질의가 답하지 못함 |
| `corrupt` · `sqlite` · `io` | 저장소 · 파일 문제 |

## 됨 (P2-1 · P2-2 · P2-3)

| op | 요청 | 답 |
| --- | --- | --- |
| `version` | — | `engine` · `abi` |
| `library.open` | `dir`(앱 자료 폴더) · `read_only`?(기본 거짓 — 참이면 아래 「서재 창구 op」의 읽기 전용 열기) | `handle` · `library` · `device` · `format` · `folder`(`<dir>/HistoryCore`). 같은 폴더를 다시 열면 같은 `handle` |
| `library.close` | `handle` | — |
| `library.counts` | `handle` | `doc` · `category` · `template` · `file` · `front`(살아 있는 수) · `deleted` · `total` |
| `doc.create` | `handle` · `ns` · `title` · `body` · `message`? | `doc` · `rev` · `hlc` · `changed`(참) |
| `doc.save` | `handle` · `doc` · `base` · `body` · `message`? · `merge`?(충돌의 다른 잎 판 — 주면 앞 판이 `base`와 이것 둘인 판, spec/drive-format.md 5-3) | `doc` · `rev` · `hlc` · `changed`(본문이 같으면 거짓, 판을 더하지 않음. `merge`면 늘 참). `merge`가 다른 잎이 아니면 `not_a_leaf` |
| `doc.rename` | `handle` · `doc` · `base` · `title` · `keep_redirect`?(기본 참) · `message`? | `rev` · `hlc` · `redirect`(옛 제목의 넘겨주기 문서 ID 또는 null) |
| `doc.delete` | `handle` · `doc` · `base` · `message`? | `doc` · `rev` · `hlc` · `changed` |
| `doc.get` | `handle` · `doc` | `doc`: `id` · `ns` · `title` · `head` · `created` · `updated` · `deleted` · `body` · `conflict`: [머리가 아닌 내용 잎 판 ID, 새 것부터 — 충돌이 아니면 빈 목록] |
| `doc.by_title` | `handle` · `ns` · `title` | `docs`: [`id` · `ns` · `title` · `head` · `created` · `updated`] — 만든 차례, 지운 것 뺌 |
| `doc.history` | `handle` · `doc` · `limit`?(50, 많아야 1000) · `before`?(hlc) | `revisions`: [`id` · `doc` · `parent` · `parent2` · `device` · `hlc` · `kind`(`content`/`deleted`) · `ns` · `title` · `message` · `len`] — 새 것부터 |
| `rev.get` | `handle` · `rev` | `revision`: 위 판 정보 + `body` |
| `namumark.parse` | `text` · `title`? · `ns`?(`template`이면 틀 문서로 봄) · `handle`?(있으면 그 서재의 틀을 펼침) | `model`: 그리기 모델(spec/namumark.md). 틀을 읽다 오류가 나면 「없는 틀」로 그리지 않고 오류로 답한다 |

`doc.create` · `doc.save` · `doc.delete`의 답과 `doc.rename`의 답에는 `head`(쓴 뒤 지금 머리)도 있다 — 보통 `rev`와 같고, 충돌 중인 문서를 고쳐 저절로 합쳤으면 합친 판이다(spec/drive-format.md 5-3).

`doc.*`의 쓰는 op는 서재에 커밋한 뒤 같은 호출 안에서 그 문서를 색인한다(색인이 실패해도 저장은 그대로 — 그 문서는 낡음으로 남아 `index.step`이 다시 한다).

### 색인 op (P2-3)

「요약」은 `doc.by_title`의 문서 모양(`id` · `ns` · `title` · `head` · `created` · `updated`)이다. 색인은 `HistoryCore/index.db`(spec/library-format.md 「색인」)에 있고, 서재를 열 때 통째로 다시 짓지 않는다 — 앱은 `index.status`의 `stale`이 0이 될 때까지 `index.step`을 뒤에서 부른다(색인이 없던 서재를 처음 열면 모든 문서가 낡음이다).

| op | 요청 | 답 |
| --- | --- | --- |
| `doc.open` | `handle` · `doc` · `follow`?(기본 참 — 넘겨주기면 한 번 따라감) | `doc`: 요약 + `deleted` · `model`: 그리기 모델(spec/namumark.md, 글 그대로 끼움) · `redirected_from`: 요약 \| null · `stale`: 색인이 낡아 이 호출에서 새로 그렸으면 참 · `conflict`: `doc.get`과 같음. 없는 문서면 `not_found` |
| `titles.exist` | `handle` · `titles`: [`{ns, title}`] | `docs`: [ID \| null] — 같은 차례, 제목은 정규화해서, 같은 제목이 둘이면 먼저 만든 것 |
| `search.titles` | `handle` · `prefix` · `ns`?(기본 `doc`) · `limit`?(20, 많아야 200) | `docs`: [요약] — 제목 차례 |
| `search.body` | `handle` · `query` · `limit`?(20, 많아야 200) | `hits`: [`{doc: 요약, snippet}`] — 문서 만든 차례. `snippet`은 첫 맞은 곳 앞뒤 약 40자의 글자(표식 없음). 한글 · 한자 2글자 조각으로 찾고 3글자 넘는 덩이는 그대로 들었는지 확인한다. 1글자 한글 · 한자는 그 글자로 시작하는 조각으로 찾는다. 지운 문서 · 색인이 낡은 문서는 나오지 않는다 |
| `tree.children` | `handle` · `category`?(없거나 null이면 맨 위, 빈 글이면 「분류 안 됨」) · `limit`?(200) · `after`?(문서 제목, 이어 받기) | `categories`: [`{name, total, has_doc, cycle}`](이름 차례) · `docs`: [요약](제목 차례, 그 분류가 집인 일반 문서) · 맨 위에서만 `uncategorized`: 수 · `cycles`: [끊은 분류 이름] |
| `links.back` | `handle` · `doc` · `limit`?(200) | `docs`: [요약] — 이 문서 제목으로 링크(본문 · 넘겨주기)한 살아 있는 문서, 제목 차례 |
| `links.wanted` | `handle` · `limit`?(50) | `wanted`: [`{ns, title, refs}`] — 없는 문서를 가리킨 본문 링크, 가리킨 문서 수 많은 차례 |
| `home.aggregates` | `handle` · `recent`?(10) · `wanted`?(10) · `days`?(84) · `tz`?(기기 시간대, UTC에서 분 — 기본 0) | `groups`: [`{name, total}`](맨 위 분류) · `uncategorized` · `recent`: [요약](바뀐 차례) · `wanted`: 위와 같음 · `rhythm`: [`{day: "YYYY-MM-DD", edits}`](오늘까지 `days`일, 빈 날은 0) · `counts`: `library.counts`와 같은 모양 |
| `stats.library` | `handle` · `tz`? | `monthly_new`: [`{month: "YYYY-MM", docs}`](살아 있는 일반 문서가 만들어진 달) · `daily_edits`: [`{day, edits}`](오늘까지 365일) · `group_links`: [`{from, to, links}`](맨 위 분류 사이 본문 링크) · `top_sources`: [`{source, refs}`](각주 속 『…』, 20) · `top_linked`: [`{doc: 요약, refs}`](20) · `decades`: [`{decade, docs}`](다루는 해) · `as_of`: 갈래 사이 링크를 마지막으로 다 센 때(ms, 0이면 아직) |
| `timeline.docs` | `handle` · `entity` | `docs`: [요약] — 그 연표 항목을 잇는 문서(연표 링크 · 연표 띠), 제목 차례 |
| `index.status` | `handle` | `stale`: 다시 색인해야 하는 문서 수 · `total`: 색인한 문서 수 · `building`: 통째 짓는 중 |
| `index.step` | `handle` · `max_ms`?(200, 많아야 1000) | `done`: 이번에 고친 문서 수 · `stale`: 남은 수. 남는 시간에 갈래 사이 링크를 센다 |

### 동기화 op (P4-2, spec/drive-format.md)

묶음 바이트는 base64 글(표준, 덧붙임 있음)로 오간다. 앱은 묶음 속을 해석하지 않는다.

| op | 요청 | 답 |
| --- | --- | --- |
| `sync.export` | `handle` · `after_seq` · `no` | `pack`: base64 | null(내보낼 판 없음) · `seq`: 묶음의 마지막 판 번호(없으면 `after_seq`) · `count` · `more`: 상한(500판 · 본문 4MB)에 닿아 남은 판이 있음 |
| `sync.apply` | `handle` · `pack` | `added` · `skipped` · `held`(지금 보류 중인 판 수) · `forked` · `merged` · `conflicts`: [문서 ID] · `revived`: [문서 ID]. 받은 문서는 같은 호출에서 색인한다 |
| `sync.status` | `handle` · `after_seq` | `library` · `device` · `seq` · `pending`(`after_seq` 뒤 이 기기가 만든 판 수) · `held` · `conflicts`(충돌 문서 수) |
| `sync.join` | `handle` · `library` | `library` — 이 서재를 드라이브의 서재로 합류(색인 그대로, 보류 판 비움, 다른 기기의 판을 물려받음) |
| `sync.rotate` | `handle` | `device` — 새 기기 ID(옛 ID의 판은 물려받음, spec/drive-format.md 6-1) |
| `library.backup` | `handle` · `path` | — `path`에 ZIP(`library.db` 사본 · `읽어보기.txt`). 다 쓴 뒤 이름을 바꾼다 |

### 서재 창구 op (P5 — 가지 `remake-7-link`, spec/windows-api.md 3-1 · 4절)

다른 앱(HistoryTime)이 같은 기기의 아카이브 서재를 **읽기 전용으로** 묻는 길이다. 답의 꼴 · 규칙의 원본은 spec/windows-api.md 4절이고, 아래는 다리 쪽 요약이다. Dart는 `LibraryWindow.open(core, dir)`(`packages/history_core/lib/src/window.dart`).

| op | 요청 | 답 |
| --- | --- | --- |
| `library.open`(`read_only: true`) | `dir` · `read_only` | `library.open`의 칸 + `read_only: true`. 폴더 · 파일 · 색인을 만들지 않고(없으면 `no_library`) 쓰지 않으며 색인을 고치지 않는다. 같은 폴더를 다시 읽기 전용으로 열면 같은 `handle`, 보통 손잡이와는 따로다. `read_only`가 참 · 거짓이 아니면 `bad_request` |
| `window.version` | `handle` | `library` · `engine` |
| `window.doc` | `handle` · `id` | `found` · `id` · `ns` · `title` · `updated`(UTC ISO) · `model`(그리기 모델, 글 그대로 끼움). 없거나 지운 문서면 `found: false`만. 넘겨주기를 따라가지 않는다 |
| `window.doc_by_title` | `handle` · `ns`?(기본 `doc`) · `title` | `window.doc`과 같음 |
| `window.docs_for_entity` | `handle` · `entity`(앞뒤 공백을 떼고 1 ~ 1,024자) | `entity`(맞춘 항목ID — NFC) · `docs`: [`{id, ns, title, updated}`] — 제목 차례 |
| `window.titles_exist` | `handle` · `titles`: [`{ns, title}`](1,000개까지) | `docs`: [ID \| null] |
| `window.search` | `handle` · `q` · `limit`?(20, 1 ~ 100으로 자름 · 정수가 아니면 `bad_request`) | `docs`: [`{id, ns, title, snippet}`] — 제목 앞부분으로 찾은 것(`snippet` 빈 글) 먼저, 이어 본문 |

- 창구 답은 모두 `api_version`(지금 1)을 싣는다.
- **읽기 전용 손잡이는 `window.*` · `library.close`만 받는다** — 그 밖의 op(쓰기 · 동기화 · `index.step` · 낡은 모델을 굳혀 넣는 `doc.open` · `handle`을 준 `namumark.parse` 등)는 `read_only`로 실패한다.
- **`window.*`는 보통 손잡이도 받는다** — 그 서재 자리에 따로 연 창구로 답하므로 두 손잡이의 답이 바이트까지 같다(쓰는 엔진의 색인에 굳혀 넣지 않는다). 아카이브 자신(G11 뒤의 Android provider)과 검사가 쓴다.
- 색인이 없거나 판 · 서재가 다르면 `window.docs_for_entity` · `window.titles_exist` · `window.search`는 `no_index`, `window.doc` · `window.doc_by_title`은 서재에서 그려 답한다.

## 예정

| op | 계획 | 하는 일 |
| --- | --- | --- |
| `hanja.get` · `hanja.search` | P2-4 | 자전 조목 · 찾기 |
| `window.hanja` | P2-4 뒤 | 서재 창구의 자전 카드(spec/windows-api.md 4절) |
