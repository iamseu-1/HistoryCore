# 서재 형식 1

설계 5-1 · 5-2 · 5-7. 이 문서가 `crates/library/src/store.rs`의 약속이다.

## 자리

- 엔진은 앱이 주는 **앱 자료 폴더** 아래 `HistoryCore/`에만 쓴다: `HistoryCore/library.db`(원본). 나중 계획에서 같은 폴더에 `index.db`(파생) · `blobs/`(첨부) · `local.db`(기기 상태)가 든다.
- 옛 앱(같은 앱 정체)은 앱 자료 폴더 **바로 아래**에 `library.db` · `index.db` · `local.db`를 두었다. 그 자리는 읽지도 고치지도 않는다.
- Windows 앱 자료 폴더: `%APPDATA%\HistoryArchive\WikiArchive`. Android: 패키지의 `files/`.

## 정체 확인

열기 전에 아무것도 쓰지 않고 본다.

| 파일 상태 | 엔진 |
| --- | --- |
| 없음 · 표가 없는 빈 파일 | 새 서재를 만든다 |
| `application_id = 0x48434C42`("HCLB") · `user_version ≤ 1` | 연다 |
| `application_id`가 다른 SQLite · SQLite가 아닌 파일 | `not_a_library` — 건드리지 않는다 |
| `user_version > 1` | `newer_format` — 건드리지 않는다 |

열면 `journal_mode = WAL`, `synchronous = FULL`.

## 표

| 표 | 열 | 뜻 |
| --- | --- | --- |
| `meta` | `key` · `value` | `library`(서재 UUID) · `device`(이 기기 UUID) · `format`(`1`) |
| `doc` | `id` · `ns` · `title` · `head` · `created` · `updated` · `deleted` | 문서마다 한 줄 — 지금 제목과 머리 판. 같은 이름공간 · 같은 제목이 둘일 수 있다(설계 6-4) |
| `rev` | `seq` · `id` · `doc` · `parent` · `parent2` · `device` · `hlc` · `kind` · `ns` · `title` · `message` · `len` · `body` | 판마다 한 줄. 지우지 않는다. `body`는 맨 끝 열(긴 값이 다른 열 읽기를 막지 않게) |

- 색인: `doc(ns, title, created)`(제목으로 찾기) · `doc(ns, deleted)`(문서 수 — 켤 때마다 센다) · `rev(doc, hlc)`(판 기록) · `rev(hlc)`(열 때 시계를 잇는 마지막 판).
- ID(`doc.id` · `doc.head` · `rev.id` · `rev.doc` · `rev.parent*` · `rev.device`)는 UUIDv7 16바이트 BLOB.
- `ns`: 0 문서 · 1 분류 · 2 틀 · 3 파일 · 4 대문.
- `hlc`: `(벽시계 ms << 16) | 카운터`. 한 기기 안에서 늘 커진다 — 열 때 `MAX(rev.hlc)`에서 잇는다. `doc.created` · `doc.updated`는 첫 판 · 머리 판의 `hlc`.
- `kind`: 0 내용 · 1 지움(본문 빈 표시 판). 지운 문서는 `doc.deleted = 1`이고 판은 남는다.
- `body`: UTF-8 나무마크를 zstd(레벨 3)로 누른 것. `len`은 누르기 전 바이트 수.

## 제목

NFC로 맞추고(호환 한자 → 통합 한자) 앞뒤 공백을 없앤다. 받지 않는 것: 빈 제목 · 200자 넘음 · `[` `]` `|` `#` · 제어 문자 · `:`로 시작 · `문서` `분류` `틀` `파일` `대문` `연표` `字` `책` 뒤에 `:`로 시작.

## 이름 바꾸기 · 넘겨주기

이름을 바꾸면 같은 본문 · 새 제목의 판이 더해진다. 넘겨주기를 남기면(기본) 옛 제목에 새 문서가 생기고 본문은 `#넘겨주기 <새 제목>`(일반 문서가 아니면 `분류:새 제목`처럼 접두어를 붙인다) 한 줄이다. 넘겨주기를 따라가는 일은 해석기(P2-2) · 색인(P2-3)이 한다.
