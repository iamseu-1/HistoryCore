# 인수인계 — P5 서재 창구 · `library_view` (2026-10-25)

이 문서만 읽고 이어 갈 수 있게 적었다. 사용자(감독)에게 하는 말은 **한국어**로 한다. 계획(구속력 — 판단 R1 ~ R9 · 알게 된 것 1 ~ 39): [plans/2026-10-25-p5-window-library-view.md](../plans/2026-10-25-p5-window-library-view.md). 약속의 원본: [spec/windows-api.md](../../../spec/windows-api.md).

## 1. 지금 어디까지

| 단계 | 상태 | 어디 |
| --- | --- | --- |
| P5 서재 창구(읽기 전용 열기 · `window.*` 여섯 · Dart `LibraryWindow`) · `packages/library_view` 0.1.0 | **걸음 1 ~ 9 끝 — 가지 `remake-7-link`(main `8c7a7ba`에서), main 합치기 · 밀기 · 태그 전** | 계획 위 · 아카이브 쪽 보고 `HistoryArchive/docs/superpowers/reports/2026-10-25-p5-2.md` |

- 누가: HistoryTime 리메이크 7단계의 '다른 저장소 몫' 갈래(감독 허락 2026-10-24 밤 — G10 「지금 시작」). 작업 폴더 `C:/claude/HistoryCore/.claude/worktrees/remake-7-link`.
- 커밋(`git log --oneline main..remake-7-link`): 약속 문서 · 계획(`8c70f61`) → 엔진(`6dec2fb`) → 다리 · Dart · dll(`12595ff`) → `library_view`(`7000131`) → 아카이브에서 알게 된 것(`99673b2` · `171620e` · `c561608` · `9395dcf`) → 마무리(이 문서의 커밋).
- 마지막 검사(걸음 9): `cargo test --locked --workspace` 238 통과 · 릴리스 `hardening` 10 · `model` 13 · clippy 새 경고 0 · `library_view` 47 · `history_core` 22 · 두 패키지 `flutter analyze` 문제 없음.
- 가지 검토 뒤(알게 된 것 36 ~ 39 — 긴 띠를 눈금 수만큼만 · 옛 엔진 dll 가르기 `engine_too_old`): `cargo test` 238 그대로(Rust 그대로) · `library_view` 49 · `history_core` 23 · 1 건너뜀(옛 dll 손잡이 `HISTORY_CORE_OLD_LIBRARY`를 주면 24) · 두 패키지 `flutter analyze` 문제 없음.

## 2. 무엇이 생겼나(한눈에)

- **엔진** — `history_library::Reader::open_library`(만들지 않음 · `query_only` · 곁 파일 치우기 `SideFiles`) · `history_index::Window`(질의마다 열고 읽기 거래 하나로 읽고 곧 닫음) · 오류 코드 `no_library` · `read_only` · `no_index` · `Hlc::utc_iso` · `WINDOW_API_VERSION` 1 · 연표 띠 항목ID NFC(R5 = A).
- **다리** — `library.open {read_only:true}`(따로 된 손잡이 칸 `windows`) · `window.version` · `window.doc` · `window.doc_by_title` · `window.docs_for_entity` · `window.titles_exist` · `window.search`. 읽기 전용 손잡이로 그 밖의 op는 `read_only`. 보통 손잡이도 `window.*`를 받는다(칸 `views` — 두 손잡이의 답이 바이트까지 같음).
- **Dart** — `LibraryWindow.open(core, appDataDir)`(창구를 모르는 옛 엔진 dll이면 열기 전에 가려 `engine_too_old`) · 값 형 `fromJson` 공개 · `windowApiVersion`.
- **`packages/library_view` 0.1.0** — 엔진 없이 그리기 모델 JSON + `DocContext` 콜백으로 문서 전문(`LibraryDocView` · `SliverLibraryDoc`) · 한자 카드(`HanjaCardView`) · 연표 띠(`TimelineStripData.yearLabel` · `stripRangeText`). 글꼴은 싣지 않음(README의 가족 이름을 받는 앱이 선언). CI 일 `library_view`.
- **spec** — `windows-api.md`(새 — 두 창구 · 자리 · 딥링크 · Android 이름), `engine-api.md`(창구 op 「됨」), `library-format.md`(색인 판 2 · 읽기 전용 열기), `namumark.md`(띠 NFC), `README.md`.

## 3. 합칠 때 확인 (감독)

1. **HistoryCore를 먼저** main에 넣고 태그를 단다 — 아카이브 CI는 `HISTORYCORE_REF`(= `main`)의 HistoryCore로 지으므로, 그래야 `library_view`를 찾는다.
2. **곧이어 아카이브 `remake-7-link`를 합친다** — 이 가지의 `history_core`가 낸 `windowApiVersion`이 아카이브 main(`9ae7700` 전)의 같은 이름과 겹쳐 아카이브 main이 지어지지 않는다(알게 된 것 24). 그 사이가 길면 아카이브 main의 `HISTORYCORE_REF`를 `8c7a7ba`에 묶는다.
3. **R7** — `history_core`를 0.1.0으로 함께 올릴지(추천 A) · 태그 이름은 감독. 올리면 아카이브 `pubspec.lock`의 판 줄이 함께 바뀐다.
4. **정션 치우기** — `C:/claude/HistoryArchive/.claude/worktrees/HistoryCore`(이 작업 폴더로 가는 디렉터리 정션)는 `cmd /c rmdir`(`/s` 없이)로 정션만 지운다. 아카이브 작업 폴더의 `windows/flutter/ephemeral/.plugin_symlinks/` 정션 넷도 **작업 폴더를 지우기 전에** 하나씩 `cmd /c rmdir`로 걷은 뒤 `git worktree remove`.
5. 두 main 작업 폴더의 `git status`에 `?? .claude/`가 보인다(작업 폴더 자리 — 숨기지 않고 알림).
6. HistoryTime `remake-7`이 계약 1-2를 고치면 `spec/windows-api.md` 5절과 아카이브 클라이언트가 따라가야 한다.
7. **아카이브 CI의 Android 일(Linux) `flutter test`가 초록인지 본다** — 두 가지의 새 검사는 이 PC(Windows)에서만 돌았다(WSL · 도커 없음 — 알게 된 것 38). 가지 CI를 `HISTORYCORE_REF=remake-7-link`로 한 번 돌려 보거나 합친 뒤 첫 CI에서 본다.
8. 합치는 차례(2)를 코드로 덜 수도 있다 — 이 패키지가 `windowApiVersion` 대신 겹치지 않는 이름을 내면 HistoryCore를 먼저 넣어도 아카이브 main이 그대로다(아카이브 가지 `9ae7700`을 함께 고침). 고르는 것은 감독(알게 된 것 39).

## 4. 넘길 것 · 미룬 것

- HistoryCore: ① `rebuild.rs`의 바꿔 끼우기 다시 해 보기(창구 질의와 겹치면 Windows에서 지우기 실패 — 계획 까다로운 곳 8). ② 틀로 들인 연표 링크를 항목 색인에(알게 된 것 32 — 펼친 모델에서 뽑고 틀 저장 때 다시 색인). ③ `window.hanja`(P2-4 뒤). ④ Android `.so`는 CI. ⑤ 검토 Minor: Dart 값 형의 모르는 `ns` 줄은 건너뛰기 · `with_window`의 손잡이 표 잠금(알게 된 것 33).
- HistoryTime에(아카이브 보고 「HistoryTime에 넘길 것」): `src/shelf.rs`를 이 창구로 바꾸는 두 길 · `library_view`를 git 의존(`ref: <태그>`)으로 받고 글꼴 이름 선언 · 엄격 열기 · `window.strip` 줄에 소수 해 · 날짜 링크 확인.
- Android provider의 속 · 두 APK 같은 서명 키 — G11 뒤.

## 5. 이 PC에서 걸리는 것

- 저장소 자리는 `C:\claude\{HistoryCore,HistoryArchive,HistoryTime}`(옛 인수인계의 `D:\…`는 옛 PC). Flutter `C:\flutter\bin\flutter.bat` · cargo `%USERPROFILE%\.cargo\bin`(PATH에 넣고) · 엔진 dll `tools\build-native.ps1`(`$env:CL='/utf-8'`).
- 개발자 모드가 꺼져 있어 아카이브 Windows 빌드 전에 `tools\windows\plugin-junction.ps1`(아카이브 저장소).
- 같은 PC에서 HistoryTime 갈래 여럿이 돈다 — `cargo test`는 `CARGO_BUILD_JOBS=4`로.
- 커밋: `git -c user.name=Claude -c user.email=noreply@anthropic.com commit -F <파일>`(한국어 제목 · 바뀐 것 `-` 세 줄 이내 · `— 검사:` 줄 · Co-Authored-By, 줄 끝 LF).
