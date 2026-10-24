# 드라이브 형식 1

설계 6절(동기화). 이 문서가 `crates/library/src/sync.rs`의 약속이다. 앱(HistoryArchive)은 이 모양대로 올리고 받고, 엔진은 묶음을 만들고 적용하고 합친다. 엔진은 네트워크를 모른다. 처음 초안: HistoryArchive `docs/superpowers/specs/2026-10-23-drive-format-draft.md`(P4-1).

## 1. 드라이브 속 모양(앱이 쓴다)

```
역사 서재/                         appProperties.historyArchive = "library"
  library.json                     {"format": 1, "library": "<서재 UUID>", "created": "<ISO>"}
  devices/<기기 UUID>.json           {"name": "<기기 이름>", "last": <올린 마지막 묶음 번호>, "seq": <그 묶음의 마지막 rev.seq>, "at": "<ISO>"}
  log/<기기 UUID>/<번호 8자리>.pack   묶음 — 그 기기가 만든 판만, 번호는 기기마다 1부터 하나씩
```

- 기기마다 자기 폴더와 자기 파일에만 쓴다. 묶음 파일은 한 번 쓰고 바꾸지 않는다.
- 통째 사본(`snapshot/`) · 첨부(`blobs/`)는 다음 판(P4-3).

## 2. 묶음(.pack)

UTF-8 JSON을 zstd(레벨 3)로 누른 것.

```json
{
  "format": 1,
  "library": "<서재 UUID>",
  "device": "<기기 UUID>",
  "no": 12,
  "revs": [
    {"id": "<판>", "doc": "<문서>", "parent": "<판>|null", "parent2": "<판>|null",
     "device": "<기기>", "hlc": 123456789012345, "kind": 0, "ns": "doc",
     "title": "임진왜란", "message": "덧붙임", "body": "<나무마크 원문>"}
  ]
}
```

- `revs`는 `rev.seq` 차례이고 모두 `device == 묶음의 device`(합친 판도 합친 기기의 것).
- `kind`: 0 내용 · 1 지움(`body`는 빈 글). `ns`: `doc` · `category` · `template` · `file` · `front`.
- 한 묶음은 많아야 **500판 또는 본문 4MB**(먼저 닿는 것, 판 하나가 4MB를 넘으면 그 판 하나로). 남으면 답의 `more`가 참이고 앱은 다음 번호로 이어서 낸다.

## 3. 내보내기(`sync.export`)

- `after_seq` 뒤에 이 기기가 만든 판(`rev.device == meta.device` · `rev.seq > after_seq`)을 `seq` 차례로. 없으면 `pack: null`.
- 답의 `seq`는 묶음에 든 마지막 판의 `rev.seq`. 번호 `no`는 앱이 준다(엔진은 번호를 모른다).
- `sync.status`의 `pending`은 `after_seq` 뒤에 이 기기가 만든 판 수다. 받은 판도 `seq`를 올리므로 `seq - after_seq`가 아니다.

## 4. 적용(`sync.apply`)

적용은 트랜잭션 하나다 — 도중에 끊기면 아무것도 들지 않는다.

1. 묶음의 `library`가 이 서재가 아니면 `wrong_library`로 답하고 아무것도 하지 않는다. 형식이 1이 아니거나 풀 수 없으면 `bad_pack`.
2. 판 ID로 거른다 — 이미 있는 판은 건너뛴다(`skipped`).
3. 앞 판(`parent` · `parent2`)이 아직 없는 판은 **보류**한다(`held`). 보류 판은 `library.db`의 `held` 표에 남고(앱을 다시 켜도), 다음 적용 때 먼저 다시 본다.
4. 받은 판의 `hlc`가 이 기기 시계보다 앞서면 시계를 그 뒤로 옮긴다 — 받은 뒤 고친 판은 늘 받은 판보다 늦다.
5. 판이 든 문서마다 머리를 다시 정한다(5절).
6. 답: `added` · `skipped` · `held`(지금 보류 중인 판 수) · `forked`(잎이 둘 이상이 된 문서 수) · `merged`(자동 합친 판 수) · `conflicts`: [충돌로 남은 문서 ID] · `revived`: [지움을 고침이 이긴 문서 ID].

## 5. 머리 정하기

문서의 **잎** = 그 문서의 판 가운데 다른 판의 `parent` · `parent2`가 아닌 판. 내용 잎(`kind 0`)과 지움 잎(`kind 1`)을 나눈다.

- 내용 잎이 없으면: 머리 = (hlc, id)가 가장 큰 지움 잎, 문서는 지움.
- 내용 잎이 하나면: 그것이 머리, 문서는 살아 있음. 지움 잎은 머리 후보가 아니다 — 고친 것이 지운 것을 이긴다. 앞서 지움이던 문서면 `revived`.
- 내용 잎이 둘 이상이면 (hlc, id) 차례로 앞의 둘을 **3-방향 합치기**하고, 합쳐지면 남은 잎과 다시 — 하나가 될 때까지. 어느 쌍에서든 겹치면 거기서 멈추고 **충돌**이다: 머리 = (hlc, id)가 가장 큰 내용 잎, 문서는 충돌 목록에 든다.

### 5-1. 공통 조상

두 잎의 공통 조상 가운데 다른 공통 조상의 조상이 아닌 것들(가장 가까운 것들) 중 (hlc, id)가 가장 큰 판. 공통 조상이 없으면(뿌리가 둘) 빈 본문 · 같은 제목을 조상으로 친다.

### 5-2. 3-방향 합치기

- 제목은 값 하나로 본다: 한쪽만 바꿨으면 그쪽, 둘이 같으면 그것, 둘이 다르게 바꿨으면 충돌.
- 본문은 줄 단위(줄끝 `\n` 포함, 마지막 줄은 줄끝 없이도 한 줄) diff3. 조상과 두 쪽 모두에서 짝지어진 줄 사이의 덩이마다: 한쪽만 바뀌었으면 그쪽, 둘이 같게 바뀌었으면 그것, 다르게 바뀌었으면 충돌. 짝짓기는 조상 ↔ 한쪽의 가장 긴 공통 부분열(Myers, `similar` 크레이트 — 판 고정).
- 합친 판: `parent` · `parent2` = 두 잎을 ID 차례로, `id` = 두 앞 판 ID(차례대로 32바이트)의 UUIDv5(이름공간 `6ba7b811-9dad-11d1-80b4-00c04fd430c8`)를 바탕으로 앞 6바이트에 큰 앞 판 hlc의 ms를 넣은 UUIDv8, `hlc` = 큰 앞 판 hlc + 1, `title` = 합친 제목, `message` = `자동 합침`, `kind` 0, `device` = 합친 기기.
- **같은 두 잎이면 어느 기기에서든 `id` · `hlc` · `title` · `body`가 같다.** `device`만 다를 수 있다 — 먼저 받은 쪽은 ID로 건너뛰므로 기기마다 그 판의 `device`가 다를 수 있다(수렴은 `device`를 뺀 열로 본다).

### 5-3. 충돌 풀기

`doc.get`의 `conflict`는 머리가 아닌 내용 잎들의 ID다. 앱이 나란히 견주어 저장할 때 `doc.save`에 `merge`(다른 잎 ID)를 주면 `parent` = 머리 · `parent2` = 그 잎인 판이 생긴다. 잎이 하나가 되면 충돌 목록에서 빠진다.

## 6. 합류(`sync.join`)

서재 ID는 기기마다 처음 열 때 생긴다. 드라이브에 다른 서재가 있으면 앱은 사용자에게 묻고 `sync.join(library)`로 이 서재의 `meta.library`를 그 ID로 바꾼다(색인 `index.db`의 `meta.library`도 함께, 보류 판은 비운다). 이 기기의 판은 그대로 남아 다음 내보내기에서 모두 나간다(`after_seq` 0부터).

## 7. 보내지 않는 것 · 내보내기

`index.db` · 기기 상태 파일 · 자전 · 연표 자료집은 보내지 않는다. `library.backup(path)`은 드라이브와 따로, `library.db`의 한 때 사본(`VACUUM INTO`)과 `읽어보기.txt`를 ZIP 하나로 쓴다.
