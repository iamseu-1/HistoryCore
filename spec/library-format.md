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
| `held` | `id` · `rev` | 앞 판이 아직 없어 보류한 받은 판(묶음 속 모양 JSON). spec/drive-format.md 4절 |
| `conflict` | `doc` | 충돌로 남은 문서. spec/drive-format.md 5절 |

- `held` · `conflict`는 P4-2에서 더했다 — 열 때 `CREATE TABLE IF NOT EXISTS`로 만들고 형식 판은 1 그대로(그 전 엔진에는 모르는 표일 뿐).
- 색인: `doc(ns, title, created)`(제목으로 찾기) · `doc(ns, deleted)`(문서 수 — 켤 때마다 센다) · `rev(doc, hlc)`(판 기록) · `rev(hlc)`(열 때 시계를 잇는 마지막 판).
- ID(`doc.id` · `doc.head` · `rev.id` · `rev.doc` · `rev.parent*` · `rev.device`)는 UUIDv7 16바이트 BLOB.
- `ns`: 0 문서 · 1 분류 · 2 틀 · 3 파일 · 4 대문.
- `hlc`: `(벽시계 ms << 16) | 카운터`. 한 기기 안에서 늘 커진다 — 열 때 `MAX(rev.hlc)`에서 잇는다. `doc.created` · `doc.updated`는 첫 판 · 머리 판의 `hlc`.
- `kind`: 0 내용 · 1 지움(본문 빈 표시 판). 지운 문서는 `doc.deleted = 1`이고 판은 남는다.
- `body`: UTF-8 나무마크를 zstd(레벨 3)로 누른 것. `len`은 누르기 전 바이트 수.

## 제목

NFC로 맞추고(호환 한자 → 통합 한자) 보이지 않는 글자(U+200B 폭 없는 공백 · U+2060 · U+FEFF BOM · U+00AD 무른 하이픈)를 지운 뒤 앞뒤 공백을 없앤다. ZWJ · ZWNJ는 남긴다. 받지 않는 것: 빈 제목 · 200자 넘음 · `[` `]` `|` `#` · 제어 문자 · 방향 바꿈 글자(U+200E · U+200F · U+202A–202E · U+2066–2069) · 줄 나눔 글자(U+2028 · U+2029) · `:`로 시작 · `문서` `분류` `틀` `파일` `대문` `연표` `字` `책` 뒤에 `:`로 시작.

## 이름 바꾸기 · 넘겨주기

이름을 바꾸면 같은 본문 · 새 제목의 판이 더해진다. 넘겨주기를 남기면(기본) 옛 제목에 새 문서가 생기고 본문은 `#넘겨주기 <새 제목>`(일반 문서가 아니면 `분류:새 제목`처럼 접두어를 붙인다) 한 줄이다. 넘겨주기를 따라가는 일은 해석기(P2-2) · 색인(P2-3)이 한다.

## 색인 (`HistoryCore/index.db`)

설계 5-5 · 5-7. 원본에서 언제든 다시 짓는 파생물이라 지워도 된다 — 동기화로 보내지 않는다.

- **정체**: `PRAGMA application_id` = `0x48434958`("HCIX"), `user_version` = 1, `meta.library` = 서재 ID.
  - 우리 것인데 판이 다르거나, 다른 서재의 색인이거나, SQLite가 아니면(깨짐) 그 파일만 지우고 새로 만든다.
  - application_id가 남의 것이면 건드리지 않고 `not_a_library`로 답한다.
  - 옛 앱이 자료 폴더 바로 아래 둔 `index.db`는 다른 파일이다 — 읽지도 쓰지도 않는다.
- **눈금**: `meta.seq` = 색인에 든 마지막 판 번호(`rev.seq`, 늘기만 한다). 그 뒤의 판(끊긴 저장 · 엔진 없이 들어온 판)과 `stale` 표(바뀐 틀을 쓰는 문서 · 틀을 읽다 실패한 문서)가 「낡음」이다.
  - 열 때 다시 짓지 않고, `index.step`이 조금씩 따라잡는다.
  - 눈금은 색인 트랜잭션 안에서 옮기므로 끊겨도 어긋나지 않는다(`synchronous=NORMAL`).
- **문서 번호**: `item.rowid` — 처음 색인할 때 정해지고 바뀌지 않는다. `model`(그리기 모델 JSON, zstd) · `plain`(찾기 확인 · 미리보기용 NFC 글자, zstd) · 조각 색인이 이 번호를 쓴다.
- **조각 색인**: `fts0`–`fts3`(문서 번호 % 4) — `fts5(t, content='', contentless_delete=1, detail=none, tokenize='ascii')`.
  - 넣는 글은 Rust가 자른 조각을 띄어 쓴 것이다: 한글 · 한자 덩이의 겹치는 2글자와 끝 글자, 그 밖의 낱말(소문자).
- **뽑기 표**: `link` · `target`(본문 링크 대상별 문서 수 · 있는지) · `member` · `note` · `source` · `entity` · `uses` · `years` · `folder`(분류마다 바로 든 일반 문서 수).
- **통계 표**: `stat_edit` · `stat_new`(UTC 시마다) · `source_count` · `decade` · `group_link`(뒤에서 다시 센다).
- **통째 짓기**: `index.db.building`과 조각 색인마다 `index.db.building-fts<k>`에 나란히 짓고, 조각 색인을 옮겨 붙인 뒤 `index.db`와 바꿔 끼운다. 남은 `.building*` 파일은 다음 열기에서 지운다.

