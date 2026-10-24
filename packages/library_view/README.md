# library_view

서재 문서 그리기 — HistoryCore 그리기 모델(판 1, [`spec/namumark.md`](../../spec/namumark.md)) JSON과 바깥 일(콜백)만으로 **문서 전문 · 한자 카드 · 문서 속 연표 띠**를 그리는 Flutter 패키지. 아카이브와 HistoryTime이 함께 쓴다.

- **엔진에 기대지 않는다.** `history_core`(FFI) · `dart:io`를 import하지 않는다 — 의존은 `flutter` · `path_drawing`뿐. 모델은 받는 앱이 엔진(아카이브)이나 서재 창구 `window.doc`(HistoryTime — [`spec/windows-api.md`](../../spec/windows-api.md) 4절)에서 받아 넘긴다.
- **아무것도 열지 않는다.** 링크 · 연표 · 바깥 주소 · 한자 · 분류를 누르면 `DocContext`의 콜백만 부른다. 여는 것(다른 문서 · `historytime://` · `historyarchive://` · 브라우저)은 받는 앱 몫이다.
- 판 **0.1.0**(태그 전 — 태그 이름과 다는 것은 감독). 아카이브 `lib/src/view` · `lib/src/ui`에서 옮겨 왔고 이름은 그대로다.

## 가져오기

```yaml
dependencies:
  library_view:
    git:
      url: https://github.com/iamseu-1/HistoryCore
      path: packages/library_view
      ref: <태그>        # 감독이 단 판 태그
```

같은 기기에서 HistoryCore를 곁에 둔 아카이브는 `path: ../HistoryCore/packages/library_view`.

## 글꼴 — 받는 앱이 선언한다

패키지는 글꼴을 싣지 않는다(花園明朝가 수십 MB). 그리기는 아래 **가족 이름**만 쓴다 — 받는 앱의 `pubspec.yaml` `flutter: fonts:`에 같은 이름으로 선언한다. 없으면 기기 글꼴로 떨어진다(그림은 되지만 글자 폭 · 한자 꼴이 달라진다).

| 가족 이름 | 쓰는 곳 | 굵기 |
| --- | --- | --- |
| `Pretendard` | 본문 · 제목 · 표 · 그래프 글(`TextStyles.sans`) | 400 · 600 · 700 |
| `NotoSerifKR` | 한자 구간 · 원어 · 한자 카드(`TextStyles.serif`), 한글 글꼴의 한자 대체 | 400 · 700 |
| `D2Coding` | 코드(`TextStyles.mono`) | 400 |
| `HanaMinA` | 한자 대체 — 확장 A · 호환 한자(BMP) | 400 |
| `HanaMinB` | 한자 대체 — 확장 B 이후(4바이트 한자) | 400 |

아카이브의 선언(글꼴 파일과 라이선스는 아카이브 `assets/fonts/`)을 그대로 따라 하면 된다:

```yaml
flutter:
  fonts:
    - family: Pretendard
      fonts:
        - asset: assets/fonts/Pretendard-Regular.otf
          weight: 400
        - asset: assets/fonts/Pretendard-SemiBold.otf
          weight: 600
        - asset: assets/fonts/Pretendard-Bold.otf
          weight: 700
    - family: NotoSerifKR
      fonts:
        - asset: assets/fonts/NotoSerifKR-Regular.ttf
          weight: 400
        - asset: assets/fonts/NotoSerifKR-Bold.ttf
          weight: 700
    - family: D2Coding
      fonts:
        - asset: assets/fonts/D2Coding-Regular.ttf
    - family: HanaMinA
      fonts:
        - asset: assets/fonts/HanaMinA.ttf
    - family: HanaMinB
      fonts:
        - asset: assets/fonts/HanaMinB.ttf
```

## 쓰기

```dart
import 'package:library_view/library_view.dart';

// HistoryTime 상세 칸 — 서재 창구로 받은 문서 하나를 380 폭에.
final doc = await window.doc(id);              // history_core LibraryWindow (받는 앱이 부른다)
LibraryDocView(
  model: doc!.model,                           // 그리기 모델 JSON 그대로
  title: doc.title,
  updated: doc.updated,                        // UTC — 그 고장 날로 보인다
  width: 380,
  ctx: DocContext(
    exists: (a) => known[a] ?? true,           // 링크 대상이 있는가(window.titles_exist로 한 번에 물은 답)
    onLink: openInArchive,                     // 문서 · 분류 링크, 넘겨주기 대상
    onTimeline: flyToEntity,                   // [[연표:…]] · 연표 띠의 「HistoryTime에서 열기」
    onUrl: launch,                             // 바깥 주소
    onHanjaTap: showHanjaCard,                 // 한자 한 글자
    strip: (entity, range) async => null,      // 문서 속 연표 띠 자료 — null이면 「제목 · 범위」 자리
    stripOpenLabel: '연표에서 보기',            // 띠 단추 글(기본은 아카이브의 「HistoryTime에서 열기」)
    stripMissing: '이 연표에 없는 항목입니다.',  // 띠 자료가 없을 때의 한 줄
  ),
);
```

| 위젯 · 함수 | 하는 일 |
| --- | --- |
| `LibraryDocView` | 머리(제목 · 원어 · 고친 날 · 분류) · 정보 상자 · 본문 · 각주를 제 굴림에. 칸이 720보다 좁으면 정보 상자를 본문 위에, 넓으면 오른쪽에 띄운다. 각주 위첨자 · 목차는 받는 앱이 `onFootnote` · `onSection`을 주지 않으면 이 굴림 안에서 그 자리로 뛴다. 넘겨주기 문서는 따라가지 않고 대상을 보인다(누르면 `onLink`). |
| `SliverLibraryDoc` | 같은 것을 받는 앱의 `CustomScrollView` 속 슬리버로. |
| `SliverDocBlocks` · `BlockView` · `ParaView` | 블록 · 글 덩이만(머리 · 정보 상자 없이). |
| `DocContext` | 바깥 일 — `exists` · `onLink` · `onUrl` · `onTimeline` · `onHanja`(올림 · 자리) · `onHanjaTap`(누름) · `strip` · `onFootnote` · `onSection` · `onEditSection` · `onMenu` · `onLinkHover` · `onFootnoteHover` · `now`, 연표 띠의 글 `stripOpenLabel`(기본 「HistoryTime에서 열기」) · `stripMissing`(자료가 없을 때의 한 줄). |
| `HanjaCardView` · `HanjaCard` | 한자 카드. 자료(`card`)가 없으면 글자와 「자전에서 열기」만. |
| `kangxiUri(글자)` | `historyarchive://kangxi/<글자>` — 「자전에서 열기」를 누르면 받는 앱이 이 링크를 연다. |
| `TimelineStripData` · `StripSpan` · `StripEvent` | `strip` 콜백이 돌려줄 띠 자료(해는 소수 — 1592.5 = 1592년 한가운데). 해는 그림 축의 해(천문 해 — 0 = 기원전 1년)이고, 서기 0년이 없는 셈의 자료는 주는 쪽이 바꾼 뒤 눈금 글을 `yearLabel`로 되돌린다. |
| `package:library_view/ui.dart` | 토큰 — `Ink` · `Vermilion` · `Radii` · `Metrics` · `TextStyles` · `Press` · `Glyphs` · `AppIcon` · `SvgPainter` · `DomTag`. |

끌어 선택 · 복사는 받는 앱이 `SelectionArea`(또는 `SelectableRegion`)로 감싸 켠다 — 블록마다 줄바꿈, 표는 칸을 탭으로 잇는다.

`ui.dart`의 `Ink`는 Material의 `Ink` 위젯과 이름이 같다 — Material 앱(HistoryTime)은 `import 'package:library_view/ui.dart' as lv;`처럼 접두를 붙이거나 `hide Ink`로 가져온다. `library_view.dart`만 쓰면 부딪히지 않는다.

## 검사

    flutter pub get
    flutter analyze
    flutter test

표본 모델(`test/fixtures/models`)은 아카이브 표본 서재를 엔진으로 그린 정답지(아카이브 `test/fixtures/sample/models`)의 사본과, 명세 표의 블록 · 인라인을 모두 담은 `all_kinds.json`이다.
