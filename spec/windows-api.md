# 창구 약속 — 같은 기기의 두 앱이 서로 읽는 길 · 딥링크 (`api_version` 1)

설계 7-2(딥링크) · 7-3(같은 기기 창구). 아카이브(서재)와 HistoryTime(연표)이 **같은 기기에서** 상대의 자료를 **읽기만** 하는 길과, 서로를 여는 링크의 약속이다. **이 문서가 원본이다** — 두 앱의 구현과 초안은 이 문서를 따르고, 바꿀 때는 이 문서를 먼저 고친다(설계 4-4).

씨앗이 된 글(이 문서로 합쳤다):

- 아카이브 초안 `HistoryArchive/docs/superpowers/specs/2026-10-23-windows-api-draft.md`(아카이브 P5-1) — 판 · Windows 자리 · 서재 창구 질의 · Android 이름 · 딥링크.
- HistoryTime 계약 `docs/계약-아카이브-3단계.md` 1-2절과 계획 `docs/계획-리메이크-7단계.md`의 '넘길 제안 묶음' 3 · 4 · 5(HistoryTime 가지 `remake-7` · 커밋 `de5f1f9`) — 연표 창구 · 항목ID · `HKCU\Software\HistoryTime` · 다른 앱이 읽는 길 · Android 이름.

## 0. 한 줄과 지금 상태

읽기 전용 · 같은 기기 · 판이 맞을 때만. 창구는 상대의 자료를 바꾸지 않는다. 상대 앱이 없거나, 판이 맞지 않거나, 읽다 실패하면 **그 기능만 조용히 빠지고** 앱은 지금 화면 그대로다.

| 쪽 | 상태 |
| --- | --- |
| 서재 창구의 자리(아카이브 레지스트리 · `HistoryArchive.api.json` · `historyarchive` 등록) | **됨** — 아카이브 P5-1 |
| 서재 창구의 질의(HistoryCore 읽기 전용 열기 + `window.*`) | **됨** — HistoryCore 가지 `remake-7-link`(엔진 · 다리 op · Dart `LibraryWindow`, main 합치기 전 — 계획 `docs/superpowers/plans/2026-10-25-p5-window-library-view.md`) |
| 서재 문서 그리기(`packages/library_view`) | **예정** — 같은 계획 |
| 연표 창구(HistoryTime 엔진 `window.*` · `HKCU\Software\HistoryTime` · `historytime` 등록) | **됨** — HistoryTime 가지 `remake-7`(`de5f1f9`, 그 저장소 main 합치기 전) |
| 연표 창구를 읽는 쪽(아카이브) | **예정** — 아카이브 P5-2(계획 `docs/superpowers/plans/2026-10-25-p5-2-timeline-window.md`) |
| 딥링크 받기 | **됨** — 아카이브(P5-1) · HistoryTime(Windows · 안드로이드는 코드만) |
| Android 창구(ContentProvider) | **예정** — 서명 키(G11) 뒤. 아카이브는 `version`만 답하는 뼈대(P5-1) |

## 1. 판 — `api_version`

- 정수, 1부터. 이 문서가 1이다. **창구마다 제 판**을 가진다 — 서재 창구는 아카이브의 `ApiVersion`, 연표 창구는 HistoryTime의 `ApiVersion`. 지금 둘 다 1.
- 판이 실리는 곳: 레지스트리 `ApiVersion` · `*.api.json`의 `api_version` · **창구 답마다**의 `api_version` · Android provider의 `version`.
- 읽는 쪽은 제가 아는 판보다 **크면 그 기능만 조용히 끈다**(죽지 않는다 · 안내 한 줄은 앱 몫). 같거나 작으면 아는 질의만 쓴다. 자리(레지스트리 · `*.api.json`)에서 한 번, 연 뒤 답의 `api_version`에서 한 번 더 본다.
- 판을 올리는 때: 칸의 뜻 · 꼴을 바꾸거나 칸 · 질의를 뺄 때. **칸 · 질의를 더하는 것은 판을 올리지 않는다**(읽는 쪽은 모르는 칸을 무시한다).
- 연표 창구는 자료 판 `data_version`(5절)도 싣는다 — 같으면 받아 둔 답을 다시 묻지 않아도 된다.

## 2. Windows 자리

두 앱은 같은 모양의 자리를 쓴다. 모두 HKCU(관리자 권한 없음). **켤 때마다** 지금 값으로 덮어쓴다(포터블 ZIP을 옮겨도 다음에 켤 때 따라온다). 쓰기에 실패해도 앱은 그대로 뜬다.

### 2-1. 아카이브(서재)

| 어디 | 값 |
| --- | --- |
| `HKCU\Software\HistoryArchive` | `LibraryDir`(REG_SZ — 앱 자료 폴더. 서재는 그 아래 `HistoryCore\`) · `ApiVersion`(REG_DWORD — 1) · `Exe`(REG_SZ — 지금 실행 파일) |
| `<LibraryDir>\HistoryArchive.api.json` | `{"format": 1, "api_version": 1, "library_dir": "<LibraryDir>", "exe": "<EXE>", "written": "<ISO>"}` — 창 기억 파일 `HistoryArchive.window.json`과 다른 파일 |
| `HKCU\Software\Classes\historyarchive` | 프로토콜 등록 — 기본값 `URL:HistoryArchive` · `URL Protocol`="" · `DefaultIcon` = `"<EXE>",0` · `shell\open\command` = `"<EXE>" "%1"` |

- 쓰는 곳: 러너(`windows/runner/registry.cpp`) · 설치기(`tools/windows/installer.iss`의 `[Registry]` — 프로토콜과 `Exe`만, 제거 때 지움. `LibraryDir` · `ApiVersion`은 앱이 켤 때 쓴다).

### 2-2. HistoryTime(연표)

| 어디 | 값 |
| --- | --- |
| `HKCU\Software\HistoryTime` | `Store`(REG_SZ — 지금 연 저장소 `archive.db`의 경로) · `ApiVersion`(REG_DWORD — 1) · `Exe`(REG_SZ — 지금 실행 파일) · `Engine`(REG_SZ — 실행 파일 옆 `archive_engine.dll`) |
| `<기억 폴더>\HistoryTime.api.json` | `{"format": 1, "api_version": 1, "store": "<Store>", "exe": "<EXE>", "engine": "<Engine>", "written": "<UTC ISO>"}` — 옆 임시 파일에 다 쓴 뒤 이름을 바꾼다. 기억 폴더는 앱 자료 폴더 아래 `archive-prototype-v2`(`--memory-dir=<폴더>`면 그 폴더) |
| `HKCU\Software\Classes\historytime` | 프로토콜 등록 — 기본값 `URL:HistoryTime` · `URL Protocol`="" · `DefaultIcon` = `"<EXE>",0` · `shell\open\command` = `"<EXE>" "%1"` |

- `Engine`은 연표만의 값이다 — 읽는 쪽이 실을 dll(3-2). 아카이브는 HistoryCore 엔진을 제가 가지므로 이 값이 없다.
- `Store`는 **마지막으로 다 뜬 보통 실행 창의 저장소**다(저장소 바로가기 여럿을 켜면 마지막 것이 덮는다). 그 창이 닫힌 뒤에도 남는다.
- 쓰는 곳(HistoryTime 저장소 기준): 러너(`prototype/app/windows/runner/main.cpp` · `link.cpp` — 프로토콜) · Dart(`prototype/app/lib/ui/deep_link.dart`의 `LinkGlue` — 창구 자리) · 설치기(`prototype/scripts/release/installer.iss`의 `[Registry]` — 설치 때 프로토콜을 적고 `Software\HistoryTime` 열쇠를 만들어 두며, 제거 때 둘 다 지움).
- 앞 초안의 `Software\HistoryTimeline\Gateway`(`store` · `api_version`)는 쓰지 않는다.

### 2-3. 시험 실행은 레지스트리를 쓰지 않는다

- 아카이브: 환경 변수 `HISTORY_ARCHIVE_DATA_DIR`가 있으면 레지스트리를 건드리지 않고 `HistoryArchive.api.json`만 쓴다.
- HistoryTime: 명령줄에 `--memory-dir=` · `--measure=` · `--capture-only=`가 있거나 환경 변수 `HISTORYTIME_TEST_RUN`이 비어 있지 않으면 레지스트리를 쓰지 않고 `HistoryTime.api.json`만 쓴다. 기억을 쓰지 않는 실행(재기 · 그림만 · 메이커 미리보기)은 둘 다 쓰지 않는다.
- 읽는 쪽의 검사도 상대 자리를 **손잡이로만** 받는다(아래 2-4) — 감독 PC의 레지스트리 · 서재 · 저장소를 검사가 읽거나 바꾸지 않는다.

### 2-4. 읽는 쪽이 자리를 찾는 차례

| 읽는 쪽 → 상대 | 차례 |
| --- | --- |
| HistoryTime → 아카이브 서재 | `--library-dir=<폴더>` → `HKCU\Software\HistoryArchive`의 `LibraryDir` · `ApiVersion` → 판이 없으면 `<LibraryDir>\HistoryArchive.api.json`의 `api_version`. 시험 실행 · 안드로이드는 손잡이 없이 찾지 않는다 |
| 아카이브 → HistoryTime 저장소 | 시험 손잡이(아카이브 계획 P5-2가 정함) → `HKCU\Software\HistoryTime`의 `Store` · `ApiVersion` · `Engine`. `HistoryTime.api.json`은 그 기억 폴더를 모르므로 보지 않는다. 시험 실행은 손잡이 없이 찾지 않는다 |
| 시험용 작은 앱(아카이브 `tools/window/peek.dart`) → 서재 | 명령줄 폴더 → `LibraryDir`. 판은 그 폴더의 `HistoryArchive.api.json` |

- 레지스트리는 `RegGetValueW`로 읽기를 권한다(프로세스를 띄우지 않고 · 글자 인코딩이 깨지지 않고 · REG_SZ 끝 NUL을 맞춰 준다). 값이 없거나 꼴이 다르면(REG_SZ가 아님 등) 자리가 없는 것으로 본다.

## 3. 읽는 길 둘

### 3-1. 서재 — HistoryCore 읽기 전용 열기 + `window.*` (됨 — HistoryCore 가지 `remake-7-link`)

읽는 앱(HistoryTime)은 HistoryCore 엔진(`packages/history_core` — 제 앱에 싣는 엔진)으로 아카이브의 서재를 **읽기 전용**으로 연다. Dart는 `LibraryWindow.open(core, <LibraryDir>)` → `version` · `doc` · `docByTitle` · `docsForEntity` · `titlesExist` · `search` · `close`(실패는 `HistoryCoreError.code`).

1. 자리(2-4)에서 `LibraryDir` · `ApiVersion`을 얻는다. 판이 크면 끈다.
2. `{"op": "library.open", "dir": <LibraryDir>, "read_only": true}` → `handle`(답은 `library.open`과 같은 칸 — `library` · `device` · `format` · `folder` — + `"read_only": true`. `read_only`가 참 · 거짓이 아니면 `bad_request`).
   - **만들지 않는다** — 폴더 · `library.db` · `index.db` 어느 것도. 서재가 없으면 `no_library`로 실패한다.
   - **쓰지 않는다** — `library.db` · `index.db`를 `SQLITE_OPEN_READ_ONLY` · `query_only`로 연다. 정체 확인(`not_a_library` · `newer_format`)은 보통 열기와 같다.
   - **색인을 고치지 않는다** — 색인 isolate · `index.step` · 통째 짓기가 없다. 낡은 문서의 모델도 굳혀 넣지 않는다.
   - 아카이브가 켜져 있어도 된다 — WAL 읽기라 커밋된 판만 본다. 질의마다 읽기 거래로 감싸 반쯤 쓴 장면을 보지 않는다.
   - 아카이브가 꺼져 있을 때 SQLite는 읽기 전용 연결에도 빈 `-wal` · `-shm`을 만든다. **이 열기가 만든 것**(열기 전에 없던 것)은 닫을 때 치운다: `-wal`은 비었을 때만, `-shm`은 `-wal`이 없어졌을 때만, Windows만(그새 아카이브가 연 곁 파일은 지워지지 않고 실패한다 — 그대로 둔다). HistoryTime 엔진 `prototype/engine/src/shelf.rs`의 `tidy`(지금 연표가 아카이브 색인을 직접 읽는 임시 길)와 같은 규칙이다.
   - 같은 프로세스에서 같은 폴더를 다시 읽기 전용으로 열면 같은 손잡이다. 보통 열기 손잡이와는 따로다.
3. 4절의 `window.*`를 묻는다. **읽기 전용 손잡이는 `window.*` · `library.close`만 받는다** — 다른 op(`doc.open`처럼 낡은 모델을 굳혀 넣는 읽기 op · `handle`을 준 `namumark.parse` 포함)는 `read_only`로 실패한다. 거꾸로 `window.*`는 **보통 손잡이도 받는다** — 그 서재 자리에 따로 연 창구로 답해 두 손잡이의 답이 바이트까지 같다(쓰는 엔진의 색인에 굳혀 넣지 않는다). 아카이브 자신(G11 뒤의 Android provider)과 검사가 쓴다.
4. `library.close`로 닫는다. 열어 둔 채 여러 번 물어도 되지만, 읽기 거래는 질의 하나 동안만 연다.

Android는 7절의 provider로 읽는다(예정).

### 3-2. 연표 — `Engine` dll의 `archive_call` (됨 — HistoryTime 엔진)

읽는 앱(아카이브)은 HistoryTime의 엔진 dll을 실어 그 저장소를 묻는다. HistoryTime 엔진이 저장소를 **읽기 전용**(`SQLITE_OPEN_READ_ONLY` · `query_only`)으로 연다 — 연표 앱이 켜져 있어도 된다.

1. 자리(2-4)에서 `Store` · `ApiVersion` · `Engine`을 얻는다. 판이 크거나, 값 하나라도 없거나, `Store` · `Engine` 파일이 없으면 끈다.
2. **일지가 남은 저장소는 열지 않는다** — `<Store>-journal`이 있으면(연표 앱이 쓰는 중이거나 쓰다가 죽음) 이번에는 끈다. 열면 HistoryTime 엔진(`Store::open`의 일지 되돌림)이 쓰기 연결로 되돌림을 해 저장소 파일이 바뀔 수 있다.
3. **dll은 사본을 싣는다** — `Engine` 파일을 읽는 앱의 임시 폴더로 복사해 그 사본을 싣는다(사본 이름에 원본의 크기 · 고친 때 같은 지문을 넣어, 원본이 바뀌면 새 사본). 실린 dll은 Windows가 잠가, 원본을 실으면 HistoryTime 업데이트(설치기 · 포터블 덮어쓰기)가 막힌다.
4. 부르는 함수는 `archive_call`(UTF-8 JSON 요청 → 답 글)과 `archive_free`(답을 놓음) 둘뿐이다. 다른 내보낸 함수(`archive_import` · `archive_tile` 등)는 부르지 않는다.
5. **허락된 op만 보낸다** — `open` · `close` · `window.version` · `window.entity` · `window.entities_on_date` · `window.search_entities` · `window.strip` · `window.spans`. 그 밖(`apply` · `add_pack` · `remove_pack` · `window.locate` · `window.ids` · 다른 조회)은 읽는 쪽이 보내기 전에 거른다.
6. **물을 때 열고 곧 놓는다** — `{"op": "open", "path": <Store>}` → `window.*` 몇 → `{"op": "close"}`. 연 채로 두지 않는다 — 연표 앱의 쓰기(DLC 넣고 빼기)는 열린 읽기 거래가 끝나기를 5초까지 기다린다.
7. **한 번에 하나씩** — dll 하나에 열린 저장소는 하나(전역)다. 질의를 겹쳐 보내지 않는다.
8. 요청 글은 **16KB**(16,384바이트)까지 — 긴 `window.spans`는 수백 개씩 나눠 묻는다.
9. **`data_version`이 같으면 받아 둔 답을 쓴다** — 열고 `window.version`만 물어 같으면 무거운 질의(`window.strip` · `window.spans`)를 건너뛴다.
10. 답은 `{"ok": true, "data": <답>, "db_ms": …}`, 실패는 `{"ok": false, "error": "<까닭>"}`. `open`의 `data`(저장소 정보)는 약속 밖이다 — 쓰지 않는다.
11. 저장소의 **SQLite 표를 직접 읽는 것은 약속 밖**이다(표 꼴은 HistoryTime 저장소 스키마 판마다 바뀐다).
12. 손으로 볼 때와 검사는 HistoryTime 명령줄 도구 `archive-tool query <저장소> '<JSON>'`이 같은 답을 낸다.

## 4. 서재 창구 질의 (아카이브의 서재 — HistoryCore `window.*`, 됨 — 가지 `remake-7-link`)

공통:

- 요청은 `{"op": "window.<이름>", "handle": <읽기 전용 손잡이>, …}`(보통 손잡이도 받는다 — 3-1의 3). 답은 늘 `{"ok": true, "api_version": 1, …}`이고, 실패는 spec/engine-api.md의 실패 꼴(`code`)이다. 요청 모양이 틀리면(빠진 칸 · 글이 아님 · 모르는 `ns` · 한도를 넘음) `bad_request`, 닫았거나 없는 손잡이는 `bad_handle`.
- **지운 문서는 없는 것으로.** **넘겨주기를 따라가지 않는다** — 넘겨주기 문서면 `model.redirect`를 보고 묻는 쪽이 다시 묻는다.
- `id`는 소문자 UUID, `ns`는 이름공간 글자(`doc` · `category` · `template` · `file` · `front`), `updated`는 머리 판 HLC의 ms(위 48비트)를 UTC ISO 글로 쓴 것(`2026-10-24T03:15:00.123Z` — 밀리초 세 자리 · `Z`).
- 색인을 쓰는 질의(`docs_for_entity` · `titles_exist` · `search`)는 색인에 든 대로 답한다 — 아카이브가 쓰다 끊겼거나 동기화로 판만 들어온 문서는 다음에 아카이브가 색인할 때까지 옛 값일 수 있다. 색인 파일이 없거나 판이 다르면(`HCIX` · `user_version` — spec/library-format.md 「색인」) 이 셋은 `no_index`로 실패하고, `doc` · `doc_by_title`은 서재에서 그려 답한다.

| op | 묻는 꼴 | 답(`ok` · `api_version` 빼고) |
| --- | --- | --- |
| `window.version` | `handle` | `library`(서재 UUID) · `engine`(엔진 판) |
| `window.doc` | `handle` · `id` | `found` · `id` · `ns` · `title` · `updated` · `model` |
| `window.doc_by_title` | `handle` · `ns`?(기본 `doc`) · `title` | `window.doc`과 같음 |
| `window.docs_for_entity` | `handle` · `entity` | `entity`(맞춘 항목ID) · `docs`: [`{id, ns, title, updated}`] |
| `window.titles_exist` | `handle` · `titles`: [`{ns, title}`](1,000개까지) | `docs`: [ID \| null] |
| `window.search` | `handle` · `q` · `limit`?(20, 1 ~ 100으로 자름) | `docs`: [`{id, ns, title, snippet}`] |
| `window.hanja` — **예정**(엔진 `hanja.*` P2-4 뒤) | `handle` · `char` | `found` · `char` · `reading` · `radical` · `strokes` · `meaning` · `entry` |

- `window.doc`: 없는 ID · 지운 문서면 `{"found": false}`만 싣는다(값 칸 없음 — `null`이 아니다. 연표 창구와 같은 꼴). `id`가 UUID 꼴이 아니면 `bad_request`. `model`은 spec/namumark.md의 그리기 모델(판 1)이고 글 그대로 끼운다. 색인의 모델이 낡았거나 없으면 머리 판을 그 자리에서 그려 답한다(굳혀 넣지 않는다).
- `window.doc_by_title`: 제목은 spec/library-format.md 「제목」 규칙으로 맞춘 뒤 찾는다. 같은 제목이 둘이면 먼저 만든 것(`titles.exist`와 같음). 제목으로 쓸 수 없는 글이면 `found: false`, 모르는 `ns`는 `bad_request`(`ns`가 없거나 null이면 `doc`).
- `window.docs_for_entity`: `entity`를 8절대로 맞춰(앞뒤 공백 떼기 · NFC), 문서 속 연표 링크 `[[연표:…]]` · 연표 띠 `[연표(…)]`로 그 항목을 잇는 문서를 낸다 — 지운 문서 뺌 · 같은 문서는 한 번 · 제목 차례(UTF-8 바이트 차례). 색인 op `timeline.docs`와 같은 찾기다. 자리 꼬리(`.mv<해>` · `#1` · `~seg<수>`)는 떼지 않는다 — 원 ID로 묻는 것은 묻는 쪽 몫(HistoryTime `api::entity_id`). 앞뒤 공백을 뗀 `entity`가 비었거나 1,024자(코드 포인트)를 넘으면 `bad_request`.
- `window.titles_exist`: 같은 차례 · 제목은 맞춰서 · 같은 제목이 둘이면 먼저 만든 것 · 제목으로 쓸 수 없는 글은 null(색인 op `titles.exist`와 같음). 줄마다 `ns`와 `title`이 있어야 하고(없거나 모르는 `ns`는 `bad_request`), 1,000개를 넘으면 `bad_request`.
- `window.search`: 제목 앞부분(`search.titles`, `ns` = `doc`)으로 찾은 것을 먼저, 이어 본문(`search.body`)으로 찾은 것 — 같은 문서는 한 번, `limit`까지. 제목으로 찾은 줄의 `snippet`은 빈 글, 본문 줄은 `search.body`의 `snippet`. `limit`은 정수면 1 ~ 100으로 자르고(HistoryTime 창구의 `limit`과 같은 규칙), 정수가 아니면 `bad_request`. 빈 `q`(공백뿐 포함)는 빈 `docs`.
- `window.hanja`: 자전 카드(아카이브 초안 4절의 꼴). 엔진 `hanja.*`가 오면 그 꼴로 굳힌다 — 그때까지 이 질의는 없고(모르는 op), HistoryTime은 한자 카드 칸을 뺀다.

## 5. 연표 창구 질의 (HistoryTime — 됨)

출처: HistoryTime 계약 `docs/계약-아카이브-3단계.md` 1-2절(가지 `remake-7` · 커밋 `de5f1f9`). 아래는 그 절의 항목ID · 판 · 연도 · 켠 자료집 · 답의 꼴과 질의 표를 그대로 옮긴 것이다. 그 절이 바뀌면 이 절도 함께 고친다.

**항목ID**: 모델의 uid에서 자리 꼬리를 뗀 **원 uid** — `<DLC id>/<객체 id>`(사건 · 인물 · 문서도 같은 꼴 · 예 `lantern-chronicle/p-lantern`). 창구는 무엇을 받든 원 ID로 고쳐 찾고 원 ID로 답한다. 떼는 셈은 HistoryTime 엔진 `prototype/engine/src/api.rs`의 `entity_id` 하나이고, 차례는 다음과 같다.

1. `#` 뒤를 뗀다 — 공동 사건의 둘째 자리 `#1` · 여러 줄에 깔린 바탕 문화 `#u3`(`#` 앞이 비면 그대로).
2. 단체의 띠 꼬리 `~band` · `~seg<수>`(ASCII 숫자 하나 이상)를 뗀다.
3. 끝의 옮겨 앉기 토막 `.mv<해>`를 뗀다 — 오른쪽 `.mv`부터 보아 그 뒤가 `-?\d+(\.\d+)?`로 끝까지 맞을 때만, 줄 끝 글자(`\n` `\r` U+2028 U+2029)가 있으면 떼지 않는다. 뗐으면 단체 띠의 토막(`~seg2.mv1600`)일 수 있어 2를 한 번 더 한다.
4. 그 밖은 그대로 둔다(`p/a.mvx` 등). 어느 걸음도 앞부분을 비우지 않는다(비게 되면 떼지 않음).

같은 자료를 어떤 길로 넣어도 항목ID 집합이 같다. 바뀌는 것은 DLC가 id 없이 이름 해시로 짓는 가문(`house:` · `kin:h-<해시>`)뿐이다 — 이름을 고치면 새 ID가 된다.

**판**: 창구 답마다 `api_version`(지금 1 — HistoryTime 엔진 `API_VERSION` · 레지스트리 `ApiVersion`과 같은 값)과 `data_version`을 싣는다. `data_version`은 다음 넷을 줄바꿈으로 이은 글의 SHA-256 16진 앞 16자다 — 같으면 받아 둔 답(다루는 해 등)을 다시 묻지 않아도 된다.

1. `meta.pack_order` 2. `meta.rules` 3. `meta.now`(없으면 빈 글자)
4. **자료집 원본의 지문** — `packs.sha256`을 자료집 번호 차례로 `,`로 이은 것(원본 없는 자료집은 빈 글자). 같은 id의 DLC를 높은 판으로 바꿔 넣으면 앞 셋이 그대로라서 이 줄이 있어야 자료가 바뀐 것을 안다.

**연도**: 답의 `start` · `end`는 연도 실수다 — 정수 부분이 해, 소수 부분이 그해 안에서 지난 몫. 해는 DLC 형식 · 모델의 해 좌표와 같은 셈이라 **서기 0년이 없다**(−1 = 기원전 1년, 그다음이 1). DLC에 적힌 해(−57 · 935)가 그대로 돌아온다. 정수 해(`window.strip`의 `lo` · `hi` · 줄의 `year`, `window.entities_on_date`의 `year`)도 같은 셈이다. 앱 안 전용 `window.locate`만 분으로 답한다.

**켠 자료집 · 거르개**: 항목 · 목록을 찾는 질의 여섯(`window.entity` · `spans` · `search_entities` · `strip` · `entities_on_date` · `locate`)은 선택 칸 `packs`(자료집 번호 목록)를 받는다 — 목록이면 그 자료집의 자리 · 기록 · 사건만 보고(정수가 아닌 번호는 까닭 글 · 빈 목록이면 아무것도 없음), 주지 않으면 저장소의 모든 자료집. 거르개(`filter`)는 `window.entities_on_date`만 받는다. **바깥 창구(아카이브가 읽는 것)는 `packs` · `filter` 없이 저장소 전체를 묻는다** — 화면에서만 끈 DLC의 목록(저장소 옆 `DLC/hidden.json`)은 엔진이 모르므로 그 DLC의 항목도 답한다.

**답의 꼴**: 늘 객체이고 `api_version` · `data_version`이 붙는다. 하나를 찾는 질의(`window.entity` · `window.locate`)는 찾은 값을 그 객체에 펴서 `found: true`와 함께 싣고, 못 찾으면 `{found: false, api_version, data_version}`만 싣는다(값 칸 없음 — `null`이 아니다). `window.strip`도 `found`를 싣는다. 한 답은 **읽기 거래 하나**에서 짓는다. `archive_call`로 부르면 3-2의 10처럼 싸인다. 물음의 소수 꼴 수는 정수로 반올림된다(`month` · `day` · `lo` · `hi`는 정수로 보낸다). 항목ID 하나는 1,024자까지(`ID_MAX`).

| op | 묻는 꼴 | 답(판 두 칸 빼고) | 누가 |
| --- | --- | --- | --- |
| `window.version` | — | (판 두 칸만) | 창구 |
| `window.entity` | `{id, packs?}` | `{found, id, kind, name, start, end, track, pack, summary}` | 창구 |
| `window.entities_on_date` | `{month, day, limit? 20 ≤ 200, packs?, filter?}` | `{items:[{id, kind, name, year, when}]}` | 창구 · 앱 '이 날의 역사' |
| `window.search_entities` | `{text, kinds?, limit? 20 ≤ 100, packs?}` | `{items:[{id, kind, name, when, track}]}` | 창구 |
| `window.strip` | `{id, lo, hi, limit? 200 ≤ 1000, packs?}` | `{found, name, items:[{id, name, when, year, category}]}` | 창구(문서 속 연표 띠) |
| `window.spans` | `{ids ≤ 2000, packs?}` | `{items:[{id, start, end}]}` | 창구 |
| `window.locate` | `{id, packs?}` | `{found, id, uid, kind, band, event, row, start, end}` | **앱 안 전용**(딥링크 · '이 날의 역사') |
| `window.ids` | — | `{count, ids}` | **검사 전용**(entity-id-check) |

- `window.locate` · `window.ids`는 **약속 밖**이다 — 바깥 창구는 보내지 않는다(3-2의 5).
- `window.entity`: `kind` = 원 ID의 항목 기록 종류(기록 없이 자리만 있는 사건은 `event`) · `name` = 기록의 `name` · `title`(없으면 띠 이름 · 사건 제목) · `start` · `end` = 그 원 ID로 돌아오는 모든 자리를 합친 기간(없으면 기록의 `x_span`, 그것도 없으면 기록 본문의 해 좌표) · `track` = 트랙 이름 · `pack` = DLC 글자 id · `summary` = 설명 첫 문장(300자 넘으면 잘라 '…').
- `window.entities_on_date`: **날짜를 일까지 적은 사건만**. 월일은 날짜 글이 적는 **서력** 월일이다 — 1582-10-15 앞은 율리우스력. 이른 시작 차례 · 같은 원 ID는 처음 한 번 · 이름 없는 사건은 `id: null` · 2월 29일은 받고 없는 월일은 까닭 글로 실패.
- `window.search_entities`: 이름 검색을 원 ID로 묶는다. 항목ID가 없는 줄은 뺀다. `kind`는 원 ID의 기록 종류(단체 띠의 토막도 `group`, 조약은 `treaty`)이고 `kinds`는 이 `kind`로 거른다 — 주지 않으면 `cell` · `doc` · `house`를 뺀 것, `cell` · `doc`을 주면 까닭 글로 거절. 사건이 아닌 항목의 `when`은 합친 기간의 때 글.
- `window.strip`: `lo` ~ `hi`(해)를 반열린 구간 [`lo`년 1월 1일, `hi` 다음 해 1월 1일)로 바꾼다(`hi`가 −1이면 다음 해는 1년 · `0`은 거절). 그 원 ID로 돌아오는 모든 띠의 사건을 모은다. 중요도 → 시작 → 사건 번호 차례 · 같은 사건 · 같은 원 ID는 한 번 · `category`는 사건의 분류 이름 · 이름 없는 사건은 `id: null` · 없는 항목은 `{found: false, items: []}`.
- `window.spans`: 입력 차례 · 없는 것은 뺌 · 같은 원 ID는 처음 한 번. 기간은 `window.entity`와 같은 셈.

## 6. 딥링크

공통: 스킴 · host는 대소문자를 가리지 않는다. 받은 값은 믿지 않는다 — 자리로 바꾸기 전에 거르고, 모르는 꼴 · 깨진 꼴(퍼센트가 깨짐 포함)은 안내 한 줄로 끝낸다(앱이 죽지 않는다). 받을 앱이 없으면 **보내는 쪽**이 안내한다(Windows: `HKCU\Software\Classes\<스킴>` · HKCR에 없음, Android: 받을 activity 없음).

Windows에서 링크를 받으면: 둘째 실행(명령줄에 링크)은 같은 앱의 첫 창에 `WM_COPYDATA`(UTF-16 글)로 넘기고 앞으로 가져온 뒤 곧 끝난다. 첫 창이 아직 뜨는 중이면 마지막 링크 하나를 들고 있다가 준비되면 넘긴다. Android는 `intent-filter`(VIEW · DEFAULT · BROWSABLE · 그 스킴)와 `onNewIntent`.

### 6-1. `historyarchive://` — 아카이브가 받는다

| 링크 | 뜻 | 거르기 |
| --- | --- | --- |
| `historyarchive://doc/<문서ID>` | 문서 보기 | ID는 UUID 꼴(`8-4-4-4-12`, 16진수 — 대문자는 소문자로 맞춤)만, 마디 하나 |
| `historyarchive://edit/<문서ID>` | 편집 | 같음 |
| `historyarchive://new?title=<제목>&draft=<파일>` | 새 문서 편집 — `draft` 파일의 글을 원문으로(저장은 사용자가) | 제목은 서재의 제목 규칙(빈 것 · 금지 글자 · 200자 넘음은 버림, 이름공간 접두는 그대로) · `draft`는 임시 폴더(Windows `%TEMP%` · Android 캐시) 안(정션 · 링크를 푼 길로 봄) · 1MB 이하 · UTF-8만, 아니면 제목만 |
| `historyarchive://kangxi/<글자>` | 자전 | 글자(코드 포인트) 하나 — 글자 · 그림 글자만 |
| 그 밖 | 대문 + 아래 줄 「열 수 없는 링크」 | |

- **HistoryTime이 보내는 꼴**: `historyarchive://doc/<id>` · `historyarchive://edit/<id>` — `id`는 서재 창구가 준 소문자 UUID 그대로. (예정) 자료집 문서의 「서재로 가져오기」 `historyarchive://new?title=…&draft=<파일>`(설계 7-4). 보내기 전에 큰따옴표 · 제어 글자 · 2,048자 넘음은 거른다.

### 6-2. `historytime://` — HistoryTime이 받는다

| 링크 | 뜻 | 거르기 |
| --- | --- | --- |
| `historytime://entity/<항목ID>` | 그 항목으로 날아가 고른다. 없으면 알림 「이 연표에 없는 항목입니다 — DLC가 꺼져 있거나 없을 수 있습니다」 | 항목ID는 경로 마디를 하나씩 퍼센트 풀어 `/`로 잇는다 — `/`가 `%2F`로 와도, 그대로 와도 같은 ID. 끝의 `/` 하나는 지나치고, 빈 마디 · 제어 문자 · 512자 넘음은 버린다 |
| `historytime://date/<월>-<일>` | '이 날의 역사 — <월>월 <일>일' 창(`window.entities_on_date`) | 1 ~ 12월 · 그 달의 날(2월 29일 받음 · 앞의 0 받음) |
| 그 밖 | 알림 「읽을 수 없는 연표 링크입니다」 | 링크 글은 2,048자까지 |

- **아카이브가 보내는 꼴**: 항목은 `Uri(scheme: 'historytime', host: 'entity', pathSegments: [<항목ID>])`(`/`는 `%2F`) — 문서 속 연표 링크 · 「연표에서 보기」. 날짜는 `historytime://date/<월>-<일>`(앞의 0 없이, 예 `historytime://date/10-21`) — 대문 '오늘의 역사'. 받을 앱이 없으면 누른 자리에 「HistoryTime이 없습니다」.
- 늦게 온 링크가 이긴다. 여러 저장소 창이 켜져 있으면 창구 자리 `Store`를 가장 늦게 적은 보통 창이 받는다 — 그래서 아카이브가 읽은 저장소와 링크가 여는 저장소가 같다(HistoryTime 계약 1-2 '여러 저장소 창일 때의 규칙').

## 7. Android 이름 (예정 — 서명 키 G11 뒤)

두 앱은 **ContentProvider + 서명 수준 권한**으로 창구를 연다. 그래서 두 APK는 **같은 서명 키**로 서명한다(설계 7-3 · 감독 결정 G11).

| 것 | 아카이브(서재 창구) | HistoryTime(연표 창구) |
| --- | --- | --- |
| authority | `io.github.seugg.archive.window` | `io.github.seugg.historytime.window` |
| 권한 | `io.github.seugg.archive.permission.WINDOW` | `io.github.seugg.historytime.permission.WINDOW` |
| 경로 | `version` · `doc/<문서ID>` · `title/<이름공간>/<제목>` · `entity/<항목ID>` · `exist?t=<제목>&t=…` · `search?q=<검색어>` · `hanja/<글자>` | `version` · `entity/<항목ID>` · `entities_on_date?month=&day=` · `search_entities?text=&kinds=` · `strip/<항목ID>?lo=&hi=` · `spans?id=…` |
| 매니페스트 `queries` | `historytime` 스킴 | `historyarchive` 스킴 |

- 권한은 둘 다 `protectionLevel="signature"`. authority · 권한 이름은 패키지 이름과 따로라, G11 때 패키지 이름을 바꿔도 그대로 둔다.
- 답은 `Cursor` 한 줄 또는 여러 줄. 열은 4 · 5절의 칸 이름 그대로(JSON 값은 문자열 열에 JSON). `version`은 `api_version` 한 열(연표 창구는 `data_version` 열도).
- 지금: 아카이브 provider는 `version`만 답하는 뼈대(P5-1), HistoryTime은 provider가 없다. 속(엔진을 Flutter 없이 부르는 길)은 G11 뒤의 계획에서 정한다.

## 8. 항목ID 견주기

- 연표 항목ID는 HistoryTime 형식 4의 원 uid다(5절). 서재는 문서 속 `[[연표:<항목ID>|…]]` · `[연표(<항목ID>, 범위=…)]`에서 항목ID를 뽑는다(`history_namumark::timeline_entities`, spec/namumark.md).
- 링크의 항목ID는 주소 해석(나무마크 `ObjectRef::parse` — 앞뒤 공백을 떼고 **NFC로 맞춤** · `#` 뒤는 앵커로 뗌)을 거친다. 그래서 서재 창구는 항목ID를 **NFC로 견준다** — `window.docs_for_entity`는 받은 항목ID도 앞뒤 공백을 떼고 NFC로 맞춰 찾는다.
- 연표 창구는 받은 항목ID를 그대로(자리 꼬리만 떼어) 견준다. 보내는 쪽(아카이브)은 문서에서 뽑은 ID — 이미 NFC — 를 그대로 보낸다.

## 9. 고칠 때

- 이 문서를 먼저 고치고, 두 앱의 구현이 따른다. 연표 창구(5절)는 HistoryTime 계약 1-2와 함께 고친다.
- 칸 · 질의를 더하는 것은 판 그대로, 바꾸거나 빼면 그 창구의 `api_version`을 올린다(1절).
- 엔진 op(`library.open`의 `read_only` · `window.*`)는 spec/engine-api.md에도 적는다.
