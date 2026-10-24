## 0.1.0

처음 판(HistoryCore P5 걸음 4 — 가지 `remake-7-link`, 태그 전).

* 아카이브 `lib/src/view`(charts · preview_card 빼고) · `lib/src/ui`(context_menu · mark · window_title 빼고)를 옮겨 왔다 — 그리기 모델(판 1) → 위젯, 토큰. 이름은 그대로.
* 새 위젯 `LibraryDocView` · `SliverLibraryDoc` — 모델 JSON + `DocContext` + 폭으로 머리 · 정보 상자 · 본문 · 각주(HistoryTime 상세 칸 380).
* `HanjaCardView`는 자료가 없으면 글자와 「자전에서 열기」만(`missing`을 주면 그 한 줄), `kangxiUri`로 `historyarchive://kangxi/<글자>`.
* `DocContext.copyWith` · `DocContext.stripOpenLabel` · `stripMissing`(연표 띠의 두 글 — 기본은 아카이브 글) · `DocWidth`(칸 폭 — 표가 화면 대신 칸 폭으로 꼴을 고름) · `DocHeadView.updated`는 없어도 됨.
* `TimelineStripData.yearLabel` — 띠의 눈금 글을 받는 앱이 짓는다(서기 0년이 없는 셈의 자료를 천문 해로 그릴 때 — 아카이브 P5-2 걸음 6).
* 띠 머리의 범위 글 `stripRangeText` — 범위의 해는 서기 0년이 없는 셈이라 음수는 「기원전 n」(`연표 · 기원전 57–기원전 18`, 아카이브 P5-2 걸음 7).
* 띠의 가는 칸은 `stripYearLinesMost`(40)해까지 해마다, 넘으면 눈금 글 자리에만 — 수백 해 띠가 회색 덩어리가 되지 않게(표본 띠 그림은 그대로).
* 엔진(`history_core`)에 기대지 않는다. 글꼴은 싣지 않는다(README의 가족 이름을 받는 앱이 선언).
