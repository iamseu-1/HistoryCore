# P5 서재 창구 · `library_view` — 구현 계획(HistoryCore 몫)

> **가벼운 계획**(memory `workflow-autonomy`): 인터페이스 · 먼저 쓸 검사 · 까다로운 곳 · 판단만 적는다. 구현은 실행하면서 쓴다 — 검사를 먼저 쓰고 빨간불을 본 뒤 구현.
>
> **누가 · 어디서**: 감독 허락(2026-10-24 밤 — HistoryTime 감독 결정 G10 "지금 시작" · G11 서명 키는 나중)으로 HistoryTime 리메이크 7단계의 '다른 저장소 몫' 갈래가 한다. 가지 `remake-7-link`(main `8c7a7ba`에서) · 작업 폴더 `.claude/worktrees/remake-7-link`. 아카이브 쪽은 같은 이름의 가지(아카이브 계획 `docs/superpowers/plans/2026-10-25-p5-2-timeline-window.md`).
> **하지 않는 것**: main 합치기 · 밀기 · 태그(감독) · Android provider(G11 뒤) · HistoryTime 저장소 고치기.

**Goal:** 다른 앱(HistoryTime)이 같은 기기의 아카이브 서재를 **읽기 전용으로** 묻는 길(서재 창구)과, 그리기 모델만으로 문서 전문 · 한자 카드를 그리는 패키지(`library_view`)를 짓는다. 약속은 `spec/windows-api.md`(이 가지에서 새로 씀)가 원본이다. 통과: 아카이브가 켜진 채로 · 꺼진 채로 다른 프로세스가 `window.*`를 물어도 서재 폴더의 파일이 바뀌지 않고(곁 파일 포함), HistoryTime이 받을 꼴(`docs_for_entity` · `doc`)이 약속대로 나오며, `library_view`가 엔진 없이 표본 모델을 그린다.

**Architecture:**
- 엔진(Rust): 새 형 `history_index::Window`(`crates/index/src/window.rs`) — 서재 읽기 연결(`history_library::Reader`와 같은 꼴) + 색인 읽기 연결(있고 판이 맞을 때만). 쓰는 `Engine`에 깃발을 더하지 않고 **형으로 쓰는 길을 막는다**(R2).
- 다리(`crates/ffi`): `library.open {read_only: true}`는 손잡이 표에 `Window`를 따로 넣는다. 그 손잡이로 `window.*` · `library.close` 밖의 op를 부르면 `read_only`, `window.*`는 보통 손잡이도 받는다(R1).
- Dart(`packages/history_core`): `LibraryWindow.open(core, dir)` — 쓰는 메서드가 없는 따로 된 형. 값 형은 공개 `fromJson`.
- `packages/library_view`(새, Flutter): 아카이브 `lib/src/view`의 「그리기 모델 → 위젯」 몫과 그것이 쓰는 토큰(`lib/src/ui`의 일부)을 옮겨 온다. 엔진(FFI)에 기대지 않고 모델 JSON(`Map`) · 콜백(`DocContext`)만 받는다(R6 · R8).

**Tech Stack:** 지금 것 그대로 — Rust 1.94.1 · rusqlite 0.37 · unicode-normalization(이미 `history_namumark`에 있음) · Flutter 3.47 / Dart 3.13 · `path_drawing`(아카이브가 이미 씀).

**Spec:** `spec/windows-api.md` 3-1 · 4 · 8절 · `spec/engine-api.md` '예정' · `spec/namumark.md`(그리기 모델 판 1) · 설계 4-1 · 4-3 · 7-1 · 7-3.

## 판단(Ruling)

- **R1 읽기 전용 손잡이는 `window.*` · `library.close`만 받는다** — 다른 읽기 op도 막는다. `doc.open`은 낡은 모델을 그 자리에서 그려 색인에 굳혀 넣는 쓰기가 있고, 바깥 앱이 약속 밖 op(`search.titles` 등)에 기대면 그 op를 바꿀 때 바깥이 깨진다. 틀리면: 바깥 앱이 더 필요한 것이 생기면 `window.*`에 질의를 더한다(판 그대로 — spec 1절). **거꾸로 `window.*`는 보통 손잡이도 받는다**(걸음 3 지시 — 처음 계획의 「보통 손잡이로 `window.doc` → `bad_handle`」을 바꿈): 그 서재 자리에 따로 연 `Window`로 답해 두 손잡이의 답이 바이트까지 같고 쓰는 엔진의 색인에 굳혀 넣지 않는다. 아카이브 자신(G11 뒤 Android provider — 앱이 이미 쓰는 손잡이를 쥐고 있음)과 검사가 쓴다.
- **R2 읽기 전용은 새 형 `Window`** — `Engine`에 `read_only` 깃발을 두면 쓰는 메서드를 하나라도 빠뜨렸을 때 쓴다. `Window`는 읽기 연결만 갖는다(`SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only`).
- **R3 색인이 없거나 판이 다르면** `docs_for_entity` · `titles_exist` · `search`는 `no_index`, `doc` · `doc_by_title`은 서재의 머리 판을 그 자리에서 그려 답한다(틀은 같은 서재에서 읽어 펼침 — `LibraryTemplates`와 같은 셈, 읽기 연결로). 읽기 전용은 색인을 짓지 못하므로.
- **R4 곁 파일 치우기** — 아카이브가 꺼진 서재를 읽으면 SQLite가 빈 `-wal` · `-shm`을 만든다. 열기 전에 없던 것만, `-wal`은 비었을 때만, `-shm`은 `-wal`이 없어졌을 때만, Windows만 지운다(HistoryTime `prototype/engine/src/shelf.rs`의 `tidy`). `library.db` · `index.db` 둘 다.
- **R5 항목ID는 NFC** — `window.docs_for_entity`는 받은 ID를 앞뒤 공백을 떼고 NFC로 맞춰 찾는다. 그리고 **연표 띠 `[연표(<항목ID>, …)]`의 항목ID도 해석기가 NFC로 맞춘다**(A, 추천 — 지금은 링크 `[[연표:…]]`만 `ObjectRef::parse`로 NFC이고 띠는 `split_args` 그대로라, NFD로 쓴 띠는 색인 `entity` 표에서 못 찾는다). 그리기 모델의 `timeline_strip.entity`도 NFC가 된다 — NFD로 적은 띠만 바뀌고 모델 판은 1 그대로. B: 뽑기(`timeline_entities`)에서만 NFC — 모델과 색인이 달라진다.
- **R6 `library_view`로 옮기는 몫** — 아카이브 `lib/src/view`의 `model.dart` · `context.dart` · `blocks.dart` · `inlines.dart` · `toc.dart` · `info_box.dart` · `float_layout.dart` · `selection.dart` · `hanja_card.dart` · `timeline_strip.dart`와, 그것들이 쓰는 `lib/src/ui`의 `tokens.dart` · `type.dart` · `dom.dart` · `press.dart` · `svg.dart` · `icons.dart`(필요한 아이콘만이 아니라 파일째). 앱 몫(`doc_head.dart` · `preview_card.dart` · `charts/` · 화면)은 아카이브에 남는다. **아카이브 화면은 바뀌지 않는다**(걸음 5의 검사기 FAIL 0 · 위젯 검사 그대로).
- **R7 판** — `library_view` 0.1.0. `history_core`도 새 API(읽기 전용 · `window.*`)가 드니 0.0.1 → **0.1.0**으로 함께 올린다(A, 추천 — 저장소 태그 하나가 두 패키지를 같은 판으로 가리킴). 태그 이름과 다는 것은 감독. 아카이브는 path 의존이라 `pubspec.lock`의 판 줄만 바뀐다(아카이브 걸음 5에서 함께 커밋).
- **R8 `library_view`는 엔진에 기대지 않는다** — 의존은 `flutter` · `path_drawing`뿐. 글꼴은 앱이 싣고 패키지는 가족 이름(`Pretendard` · `NotoSerifKR` · `D2Coding` · `HanaMinA` · `HanaMinB`)만 쓴다. 바깥 일(링크 열기 · 문서 있는지 · 연표 띠 자료 · 한자 카드 자료)은 모두 `DocContext`의 콜백.
- **R9 엔진 dll** — 이 PC에서는 `tools/build-native.ps1`(Windows x64)만 짓는다. Android `.so`는 이 PC에 NDK가 없어 CI 몫(합칠 때).

## 인터페이스

```rust
// crates/index/src/window.rs — 걸음 2에서 지은 그대로
pub struct Window { /* 자리(앱 자료 폴더)와 압축 풀기 문맥만 — 파일은 질의 동안만 연다(까다로운 곳 8) */ }
impl Window {
    pub fn open(app_dir: &Path) -> Result<Window>;                       // no_library · not_a_library · newer_format(보고 곧 놓음)
    pub fn version(&mut self) -> Result<LibraryId>;                      // 질의마다 읽음 — 합류하면 바뀐다
    pub fn info(&mut self) -> Result<LibraryInfo>;                       // 걸음 3 — 읽기 전용 열기의 답(서재 · 기기 · 형식 · 자리)
    pub fn folder(&self) -> &Path;                                       // 걸음 3 — 손잡이 표의 같은 폴더 찾기
    pub fn doc(&mut self, id: DocId) -> Result<Option<WindowDoc>>;       // 지운 문서 · 없음 = None. 넘겨주기 안 따라감
    pub fn doc_by_title(&mut self, ns: Namespace, title: &str) -> Result<Option<WindowDoc>>;
    pub fn docs_for_entity(&mut self, entity: &str) -> Result<(String, Vec<DocSummary>)>; // (맞춘 ID, 문서) · no_index
    pub fn titles_exist(&mut self, titles: &[(Namespace, String)]) -> Result<Vec<Option<DocId>>>; // no_index
    pub fn search(&mut self, q: &str, limit: usize) -> Result<Vec<Hit>>; // 제목 줄(snippet "") 먼저 · 본문 줄 · no_index
}
pub struct WindowDoc { pub summary: DocSummary, pub model: String }    // 모델은 글 그대로(다리가 풀지 않고 끼움)
// 찾기 한 줄은 색인의 `Hit { doc, snippet }` 그대로(따로 된 WindowHit를 두지 않음). 곁 파일 치우기는 연결마다의 `SideFiles`가 한다.
// history_library: Reader::open_library(app_dir) · Reader::read(질의 하나 = 읽기 거래 하나) · library · device · format · last_seq · summary · body_by_title
//                  open_read_only(path) → (Connection, SideFiles) · in_read_tx · READ_ONLY_BUSY(2초)
//                  Hlc::utc_iso() — `updated`의 UTC ISO 글(다리가 씀) · WINDOW_API_VERSION = 1
// history_library::Error에 NoLibrary · ReadOnly · NoIndex — code "no_library" · "read_only" · "no_index"
```

다리 op(spec/windows-api.md 4절 그대로): `library.open {dir, read_only: true}` → `library.open`의 칸 + `read_only` · `window.version` · `window.doc` · `window.doc_by_title` · `window.docs_for_entity` · `window.titles_exist` · `window.search`. 답마다 `api_version: 1`, `updated`는 UTC ISO(`…T03:15:00.123Z`).

```dart
// packages/history_core/lib/src/window.dart — 걸음 3에서 지은 그대로
const windowApiVersion = 1;
class LibraryWindow {
  static Future<LibraryWindow> open(HistoryCore core, String appDataDir); // 실패는 HistoryCoreError.code(no_library 등)
  final LibraryInfo info;                                                 // LibraryInfo.fromJson도 공개
  Future<WindowVersion> version();                                        // {apiVersion, library, engine}
  Future<WindowDoc?> doc(String id);
  Future<WindowDoc?> docByTitle(String title, {Namespace ns = Namespace.doc});
  Future<List<WindowDocSummary>> docsForEntity(String entity);
  Future<List<String?>> titlesExist(List<(Namespace, String)> titles);
  Future<List<WindowHit>> search(String q, {int limit = 20});
  Future<void> close();
}
// WindowDocSummary {id, ns, title, updated: DateTime(UTC)} · WindowDoc extends WindowDocSummary {model: Map<String, dynamic>}
// · WindowHit {id, ns, title, snippet} · WindowVersion — 공개 fromJson. doc · docByTitle은 found가 거짓이면 null
```

```dart
// packages/library_view/lib/library_view.dart — 아카이브 lib/src/view · lib/src/ui의 지금 이름 그대로 옮김
class DocModel { static DocModel parse(Map<String, dynamic> j); final ObjectAddress? redirect; final List<Block> blocks; ... } // 그리기 모델 판 1
sealed class Block / Inline; class ObjectAddress; class Footnote; class Section;
class DocContext { exists · onLink · onUrl · onTimeline · onHanja · onHanjaTap · strip · footnotes · sections · onMenu · ... } // 바깥 일은 모두 콜백
class SliverDocBlocks extends StatefulWidget { SliverDocBlocks({required List<Block> blocks, required DocContext context}); }
class BlockView · HeadingView · TableView · FootnotesView;  class InlineSpans;
class HanjaCard { char · readings · radical · strokes · ... }  class HanjaCardView extends StatelessWidget;
class TimelineStripData / StripSpan / StripEvent;            // 문서 속 연표 띠 그림 자료
abstract final class Ink / Vermilion / Radii / Metrics / TextStyles; // 설계 3절 토큰

// 걸음 4에서 지은 그대로 — 새로 더한 것
class LibraryDocView extends StatelessWidget {   // 제 굴림. 머리 · 정보 상자(칸 720 아래면 본문 위) · 본문 · 각주
  LibraryDocView({required Map<String, dynamic> model, required String title, DateTime? updated, DocContext? ctx,
      double? width, EdgeInsets padding = const EdgeInsets.fromLTRB(16, 20, 16, 40), ScrollController? controller,
      bool head = true, String? redirectedFrom});
}
class SliverLibraryDoc extends StatefulWidget { /* 같은 칸 — 받는 앱의 CustomScrollView 속 슬리버(padding · controller 없음) */ }
class HanjaCardView { HanjaCardView({required String char, HanjaCard? card, required VoidCallback onOpen, double? width = 320, bool docked = false, String? missing}); }
Uri kangxiUri(String char);                       // historyarchive://kangxi/<글자> (퍼센트로 쌈)
class DocWidth extends InheritedWidget { static double of(BuildContext c); } // 칸 폭 → 없으면 화면 폭(표가 씀)
DocContext DocContext.copyWith({...});            // 뛰는 자리(anchors)는 같은 것
// 내는 곳: package:library_view/library_view.dart(그리기) · package:library_view/ui.dart(토큰 — Material의 Ink와 이름이 겹침)
```

(이름은 아카이브의 지금 이름을 따른다 — 아카이브는 import만 바꾸면 된다. 공개 범위는 걸음 4에서 정리하되 아카이브 화면이 쓰는 것은 모두 낸다. → 걸음 4: 옮긴 파일의 공개 이름을 모두 낸다 — 알게 된 것 17 · 18.)

## 먼저 쓸 검사

- **엔진 `crates/index/tests/window.rs`**
  - 없는 폴더 · 빈 폴더 → `no_library`, 폴더 · 파일이 생기지 않음. 남의 파일 → `not_a_library`.
  - 아카이브가 꺼진 서재: 열고 · 묻고 · 닫은 뒤 `HistoryCore/`의 파일 목록 · 크기 · 고친 때가 그대로(곁 파일 치움, R4).
  - 아카이브가 켜진 서재(같은 시험 안의 쓰는 `Engine`): 읽는 동안 쓰는 쪽이 저장 → 다음 질의에 보임 · 읽기가 쓰기를 막지 않음.
  - `docs_for_entity`: 링크 · 띠 둘 다 · NFD로 물어도 · NFD로 적은 띠도(R5) · 지운 문서 뺌 · 같은 문서 한 번 · 제목 차례 · `timeline.docs`와 같은 답.
  - `doc`: 없음 · 지움 → None · 넘겨주기 문서는 따라가지 않고 `model.redirect` · 색인이 낡은 문서는 머리 판으로 그려 답하고 `index.db`는 바이트 그대로.
  - 색인 없음 · 판 다름 → `docs_for_entity` · `titles_exist` · `search`는 `no_index`, `doc`은 됨.
  - `titles_exist` · `search`(제목 먼저 · 본문 뒤 · 한 번 · `limit`).
- **다리 `crates/ffi/tests/window.rs`**: `read_only` 답 칸 · 보통 손잡이와 따로 · 같은 폴더 두 번 같은 손잡이 · 읽기 전용 손잡이로 `doc.save` · `doc.open` · `index.step` → `read_only` · 보통 손잡이로 `window.*` → 읽기 전용 손잡이와 바이트까지 같은 답(R1 — 걸음 3에서 바꿈) · 답마다 `api_version` 1 · `updated` ISO 꼴 · `model`이 글 그대로.
- **Dart `packages/history_core/test/window_test.dart`**: 진짜 엔진으로 열고 읽고 닫음 · 값 형 `fromJson` · 없는 서재 오류 code.
- **`packages/library_view/test/`**: 나무마크 견본(spec/namumark.md 표의 블록 · 인라인 전부)의 모델 → 그려짐(예외 없음) · 링크 · 연표 링크 · 한자 누름이 콜백으로 · 없는 문서 링크는 `exists`로 가름 · 연표 띠는 `strip`이 null이면 「제목 · 범위」 자리 · 한자 카드 · 380 폭에서 넘침 없음.

## 까다로운 곳

1. **WAL과 읽기 전용** — 아카이브가 꺼져 있으면 SQLite가 곁 파일을 만들고(R4), 아카이브가 쓰다 죽어 거두지 않은 WAL이 남았으면 읽기 전용 연결은 되살리지 못할 수 있다 — 그때는 실패(`sqlite`)로 답하고 읽는 쪽은 칸을 뺀다. 읽기 연결이 남의 WAL을 합쳐 넣는(체크포인트) 일이 없게 `query_only`와 닫을 때 체크포인트 없음을 검사로 묶는다.
2. **같은 프로세스의 두 열기** — 아카이브 앱 자신이 `window.*`를 부를 일은 없지만, 검사와 작은 앱은 쓰는 손잡이와 읽기 전용 손잡이를 한 프로세스에서 함께 연다. 손잡이 표는 둘을 다른 칸에 두고 `same_folder` 재사용은 같은 갈래끼리만.
3. **색인 판 문서가 낡음** — `spec/library-format.md` 「색인」은 `user_version` = 1이라 적었지만 코드(`crates/index/src/store.rs`의 `FORMAT`)는 2다. 걸음 2에서 문서를 고친다(창구가 판으로 거절하는 기준이라). → **고침**(걸음 2 — 2로 적고, 창구는 맞지 않으면 「색인 없음」으로 다룬다는 줄과 「읽기 전용 열기」 절을 더함).
4. **모델 글 그대로** — `window.doc`의 `model`은 `doc.open`처럼 풀지 않고 끼운다(`Reply::WithModel`) — 깊게 중첩된 모델(P2-2 미룬 것).
5. **`library_view` 옮기기와 아카이브 화면** — 토큰 · 아이콘 · 글꼴을 옮기면서 그림이 한 픽셀도 바뀌지 않아야 한다(검사기 · 위젯 검사 · 스크린숏). 옮기는 동안 아카이브는 `lib/src/view`의 옮긴 파일을 지우고 패키지를 import(걸음 5).
6. **두 가지가 함께 간다** — 아카이브 `remake-7-link`는 이 가지의 엔진 · 패키지를 기대한다. 아카이브 CI는 `HISTORYCORE_REF`(= `main`)의 HistoryCore로 지으므로, **합칠 때는 HistoryCore를 먼저** main에 넣어야 아카이브 CI가 초록이다(감독). 다만 이 가지를 넣으면 아카이브 main은 깨진다 — 알게 된 것 24.
7. **작업 폴더의 정션** — 아카이브의 `path: ../HistoryCore/...`와 `peek_test`의 `Directory.current.parent\HistoryCore`가 작업 폴더 안에서는 `C:/claude/HistoryArchive/.claude/worktrees/HistoryCore`를 가리킨다 — 그 자리에 이 작업 폴더로 가는 디렉터리 정션을 두었다(`mklink /J`). `pubspec_overrides`는 쓰지 않는다(절대 경로가 `pubspec.lock`에 들어감). 치울 때는 `cmd /c rmdir`(`/s` 없이 — 정션만 지움).

### 걸음 2에서 알게 된 것

8. **창구는 질의마다 열고 닫는다** — 손잡이를 쥔 채 파일을 쥐면 (1) 아카이브가 꺼진 서재에서는 곁 파일이 질의 사이에 남고, (2) Windows에서는 아카이브의 통째 짓기가 `index.db`를 지우고 바꿔 끼우지 못한다. 그래서 `Window`는 자리만 들고, 질의마다 `library.db` · `index.db`를 읽기 전용으로 열어 읽기 거래 하나로 읽고 닫는다(곁 파일은 그때 치움). 남는 틈: 창구가 질의하는 몇 ms 사이에 아카이브가 바꿔 끼우면 그 통째 짓기는 지우기에서 실패한다(드묾 — HistoryTime의 지금 임시 길 `shelf.rs`도 같다). 아카이브 엔진(`rebuild.rs`의 바꿔 끼우기)이 몇 번 다시 해 보게 하는 것은 넘길 것.
9. **낡음 판정이 쓰는 쪽 `doc.open`보다 넓다** — 색인 모델을 그대로 주는 것은 ①머리 판으로 색인했고 ②`stale` 표에 없고 ③색인이 서재보다 뒤처졌다면(눈금 < 마지막 판) 틀을 쓰지 않는 문서일 때뿐이다. ③이 없으면 틀 저장이 색인되기 전에 끊겼을 때(그 틀을 쓰는 문서가 아직 `stale`에 없음) 옛 틀로 그린 모델을 준다(검사 `a_lagging_index_redraws_docs_that_use_templates_but_not_the_others` — ③을 빼면 빨갛다). 쓰는 쪽 `doc.open`은 ①만 보지만, `index.step`이 따라잡은 뒤의 `doc.open` 모델과 창구의 답이 같다.
10. **Windows 파일 크기 · 고친 때는 파일마다 묻는다** — `DirEntry::metadata()`는 디렉터리 항목의 값이라, 다른 손잡이가 열어 둔 `-wal`은 크기가 늦게 고쳐진다(켜진 아카이브 곁 검사에서 32바이트로 보이다가 창구가 연 뒤 실제 크기로 바뀜). 두 검사 파일의 `snapshot`은 `std::fs::metadata(path)`로 잰다.
11. **연표 띠는 한 줄을 다 차지해야 띠다** — `[연표(X)] 글`은 문단이다(spec/namumark.md 블록 매크로 규칙). 검사 견본을 띠 한 줄 · 글 한 줄로 고쳤다.
12. **함께 쓰는 셈** — 같은 답을 내야 하는 찾기는 `engine.rs`의 함수 하나를 쓰는 엔진과 창구가 함께 부른다: `draw`(그리기 선택) · `timeline_docs_in` · `titles_exist_in` · `search_titles_in` · `search_body_in`(머리 판 견주기는 `Heads` — `Library` · `Reader`). 창구의 틀 읽기 `ReaderTemplates`는 `LibraryTemplates`와 같은 셈(맞춘 제목의 살아 있는 문서 중 먼저 만든 것)을 같은 읽기 거래에서 한다.

### 걸음 3에서 알게 된 것

13. **손잡이 표는 칸 셋** — `libraries`(쓰는 `Engine`) · `windows`(읽기 전용 `Window`) · `views`(보통 손잡이의 `window.*`용 `Window` — 처음 물을 때 `<folder>/..`로 연다). 번호는 함께 세고, `library.close`는 셋에서 모두 지운다. `with_engine`은 읽기 전용 번호면 `read_only`로 거절하므로 `handle`을 준 `namumark.parse`도 막힌다. 읽기 전용 열기는 손잡이 표를 잡기 전에 `Window::open` · `info`를 한다(정체를 보고 곧 놓음).
14. **읽기 전용 열기의 답 칸** — `device` · `format`은 파일에서 읽는다(`Reader::device` · `Reader::format` — `user_version`, `Window::info`). 같은 폴더를 다시 열면 같은 손잡이지만 답은 그때 다시 읽은 값이다(합류하면 `library`가 바뀜).
15. **`limit`은 자른다** — 걸음 2의 넘길 것은 「`limit` 100 넘음 `bad_request`」였지만 spec 4절은 「많아야 100」이고 HistoryTime 창구(`limit_arg`)도 1 ~ 가장 많이로 자른다. 같은 규칙으로 갔다: 정수면 1 ~ 100, 정수가 아니면(글 · 소수) `bad_request`. `entity` 길이는 앞뒤 공백을 뗀 뒤 코드 포인트로 센다(HistoryTime `ID_MAX`와 같은 셈).
16. **Dart 기록(record)은 `Uint8List`를 같은 객체인지로 견준다** — 파일이 바이트까지 그대로인지 보는 검사는 바이트 대신 지문(FNV-1a)을 기록에 넣는다(`window_test.dart`의 `snapshot`).

### 걸음 4에서 알게 된 것

17. **옮긴 몫(R6을 넓힘)** — 걸음 4 지시대로 `doc_head.dart`(`DocHeadView` · `CrumbView` · `HeadAction` · `PrimaryAction`)와 `ui/format.dart`도 옮겼다 — `LibraryDocView`의 머리가 `DocHeadView`를 쓰고, 파일째 옮겨 아카이브는 import만 바꾼다. 앱에 남는 것: `view/charts/` · `view/preview_card.dart`(서가 `DocPreview`에 기댐) · `ui/context_menu.dart` · `ui/mark.dart` · `ui/window_title.dart`. 이름은 모두 그대로(`ArchiveScroll`도).
18. **옮기며 바꾼 셋 — 모두 아카이브와 맞물림(걸음 5가 지킬 것)**
    - `HanjaCardView`: 자료가 없으면 **글자 칸과 「자전에서 열기」만**(지시 그대로 — 받는 앱 HistoryTime은 `window.hanja`가 예정이라 늘 이 꼴). 아카이브의 「표본 자전에 없는 글자」 줄은 새 칸 `missing`으로 준다 — **걸음 5에서 `doc_screen.dart`의 `HanjaCardView(…, missing: '표본 자전에 없는 글자')`를 넣어야 화면이 그대로다**(넣지 않으면 자료 없는 글자의 카드에서 그 줄과 음훈 자리 글자가 빠진다).
    - `DocHeadView.updated`는 없어도 된다(null이면 「고침」 줄을 뺌) — 아카이브는 늘 주므로 그대로.
    - `TableView`의 좁은 칸 여백(16 · 32)을 화면 폭 대신 **`DocWidth`(칸 폭) → 없으면 화면 폭**으로 고른다. 넓은 화면 속 380 칸에서 넓은 칸 여백을 고르던 것을 고침. 아카이브는 `DocWidth`를 두지 않아 그대로(검사 「표는 화면이 아니라 칸 폭으로」 — 되돌리면 빨갛다).
    - 더한 것(아카이브는 쓰지 않음): `DocContext.copyWith` · `DocWidth` · `kangxiUri` · `LibraryDocView` · `SliverLibraryDoc` · `floatInfoMinWidth`, 그리고 `DocContext.stripOpenLabel` · `stripMissing` — 연표 띠의 「HistoryTime에서 열기」 · 「HistoryTime이 이 기기에 있으면 …」은 아카이브 말이라, HistoryTime이 그릴 때 제 말로 바꾼다(기본값은 아카이브 글이라 아카이브는 그대로).
19. **`LibraryDocView`의 꼴** — 아카이브 문서 화면의 종이 몫과 같은 셈: 정보 상자의 틀 블록은 본문에서 빼고, `[각주]`가 없으면 끝에 각주, 본문 블록 목록은 한 번만 지어 펼친 접힌 문단이 다시 그려도 닫히지 않는다. 다른 것: 정보 상자를 띄우는 기준은 화면 갈래(1,100) 대신 **칸 폭 720**(곁 본문 388 이상), 넘겨주기 문서는 따라가지 않고 대상과 「열기」(`onLink`) — 창구 약속(4절)이 넘겨주기를 따라가지 않으므로. 각주 · 목차 뛰기는 받는 앱이 `onFootnote` · `onSection`을 주지 않으면 패키지가 제 굴림 안에서 뛴다(`Scrollable.maybeOf` — 슬리버로 남의 굴림에 넣어도 됨). `updated`는 UTC로 받아 `toLocal()`로 보인다.
20. **이름 겹침** — `ui.dart`의 `Ink`는 Material의 `Ink` 위젯과 같은 이름이다. 그래서 토큰은 `package:library_view/ui.dart`로 따로 두고, `library_view.dart`가 내는 이름은 Flutter(material · widgets · rendering · painting · services · gestures)의 이름과 겹치지 않음을 확인했다. Material 앱은 `ui.dart`를 접두(`as lv`)나 `hide Ink`로 가져온다(README).
21. **위젯 검사의 화면 밖** — `find.*`는 기본이 `skipOffstage: true`라, 게으른 목록이 미리 지은(화면 밖 · `cacheExtent` 안) 블록을 찾지 못한다. 문서 전부를 보는 검사는 `ArchiveScroll.cacheExtent`를 크게 하고 `skipOffstage: false`(그리고 `find.descendant`의 `matching` 찾기에도)를 준다. 아직 지어지지 않은 각주 위첨자는 `scrollUntilVisible`로 찾는다.
22. **CI는 제 일(job)** — `library_view`는 엔진을 짓지 않는 따로 된 일 `library_view`(windows-2025 · pwsh)에서 `pub get` · `analyze` · `test`를 돈다. `ci_workflow_test`가 그 일이 있고 엔진 짓기 · cargo가 없음을 본다(일을 빼면 빨갛다).
23. **R7은 하지 않았다** — `history_core` 판은 0.0.1 그대로(감독 결정을 기다림 — 올리면 아카이브 `pubspec.lock`의 판 줄이 함께 바뀐다). `library_view`만 0.1.0 · CHANGELOG. 태그는 감독.

### 걸음 5(아카이브)에서 알게 된 것

24. **새 공개 이름이 받는 앱과 겹쳤다** — 걸음 3이 `history_core`에 낸 `windowApiVersion`이 아카이브 `window_place.dart`의 같은 이름 상수와 부딪혀, 아카이브의 `boot.dart` · `peek.dart`(둘 다 `history_core`를 통째로 가져옴)가 지어지지 않았다(검사 파일 넷이 못 돎). 아카이브 `remake-7-link`가 제 상수를 지우고 `history_core`의 것을 가져와 다시 내어 풀었다(아카이브 `9ae7700`). **그래서 이 가지를 main에 넣으면 아카이브 main(`9ae7700` 전)은 `HISTORYCORE_REF` = `main`으로 지어지지 않는다** — 곧이어 아카이브 `remake-7-link`를 합치거나, 그 사이 아카이브 main의 `HISTORYCORE_REF`를 `8c7a7ba`에 묶는다(감독 — 아카이브 계획 「합칠 때 확인」). 앞으로 `history_core` · `library_view`에 공개 이름을 더할 때는 받는 앱의 같은 이름을 먼저 찾아본다.
25. **아카이브가 패키지로 옮겨 감** — 아카이브 걸음 5(`900e529`)가 옮긴 view 11 · ui 7을 지우고 `package:library_view`를 쓴다. 스크린숏 46장과 쌍둥이 HTML이 바꾸기 전과 바이트까지 같고(검사기 view FAIL 0), `HanjaCardView(missing: '표본 자전에 없는 글자')`를 넣었다(알게 된 것 18).

### 걸음 6(아카이브)에서 알게 된 것

26. **띠 눈금 글을 받는 앱이 짓는다** — 연표 창구의 해는 서기 0년이 없는 셈(−1 다음이 1)이라, 아카이브는 띠 축을 고르게 그리려 천문 해(0 = 기원전 1년)로 바꿔 `TimelineStripData`를 짓는다. 그러면 눈금 글(`'$y'`)이 `0` · `-1`로 어긋나므로 `TimelineStripData.yearLabel`(정수 해 → 글, 없으면 그 수 그대로)을 더했다 — 칸 하나를 더한 것이라 판(0.1.0, 태그 전) · 다른 그림은 그대로, 패키지 검사 45(새 `test/timeline_strip_test.dart` 1). 아카이브는 「기원전 n」으로 준다. 공개 이름은 늘지 않았다(알게 된 것 24).
27. **긴 범위의 띠는 해마다 가는 칸이 빽빽하다** — `stripScene`은 해마다 세로 줄 하나를 긋는다. 실제 연표 자료는 수백 ~ 수천 해 띠(고조선 −2333 ~ −108)가 있어 줄이 회색 덩어리가 된다. 아카이브 걸음 7(화면 잇기)에서 그림을 보고 줄 간격을 눈금 간격에 맞출지 정한다(그리기 바꿈 — 스크린숏 표본 띠 7해는 그대로여야 함).

## 진행(걸음 표시)

전체 걸음은 두 저장소에 걸친 아홉이다. 이 저장소가 맡는 걸음은 1 · 2 · 3 · 4 · 8 · 9(5 · 6 · 7은 아카이브).

| 걸음 | 무엇 | 상태 |
| --- | --- | --- |
| 1 | 작업 폴더 · 정션 · `spec/windows-api.md` · `spec/README.md` · `spec/engine-api.md` 예정 줄 · `README.md` · 이 계획 | **끝**(2026-10-25) |
| 2 | 엔진: 읽기 전용 열기(`Reader::open_library` · `Window`) · `window.*` 여섯의 Rust 몫 · 오류 코드 셋 · 띠 항목ID NFC(R5 — A로 감) · HLC → UTC ISO · `WINDOW_API_VERSION` · `spec/library-format.md` 색인 판(1 → 2)과 「읽기 전용 열기」 · `spec/namumark.md` 띠 NFC | **끝**(2026-10-25 — 새 검사 `crates/library/tests/read_only.rs` 9 · `crates/index/tests/window.rs` 12, 작업공간 232 통과) |
| 3 | 다리 op · Dart `LibraryWindow` · 엔진 dll 짓기(`tools/build-native.ps1`) · `spec/engine-api.md` 예정 → 됨 | **끝**(2026-10-25 — 새 검사 `crates/ffi/tests/window.rs` 6 · `packages/history_core/test/window_test.dart` 5, dll `packages/history_core/native/windows-x64/history_core.dll`(git 밖) — 아카이브 걸음 5 ~ 8이 정션으로 씀) |
| 4 | `packages/library_view` 0.1.0(R6 · R7 · R8) · `history_core` 0.1.0 | **끝**(2026-10-25 — `library_view` 0.1.0 · CHANGELOG · README(글꼴 이름) · CI 일 `library_view`, 패키지 검사 44(옮긴 `blocks_test` 18 · `model_test` 5 + 새 `doc_view_test` 13 · `hanja_card_test` 5 · `package_test` 3) · `ci_workflow_test` 3 통과. `history_core` 0.1.0은 감독 결정 기다림(알게 된 것 23)) |
| 8 | 함께 시험: 아카이브 작은 앱이 읽기 전용으로 peek · HistoryTime 임시 길(`shelf.docs`)과 같은 답인지 교차 확인 · 릴리스 빌드 | 기다림 |
| 9 | 마무리: 새 검토자 · 넓은 검사(`cargo test --locked --workspace` · `clippy` · `flutter test` 두 패키지) · 보고 · 인수인계 · 넘길 것 | 기다림 |
