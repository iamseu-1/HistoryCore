## 0.1.0

처음 판(HistoryCore P5 걸음 4 — 가지 `remake-7-link`, 태그 전).

* 아카이브 `lib/src/view`(charts · preview_card 빼고) · `lib/src/ui`(context_menu · mark · window_title 빼고)를 옮겨 왔다 — 그리기 모델(판 1) → 위젯, 토큰. 이름은 그대로.
* 새 위젯 `LibraryDocView` · `SliverLibraryDoc` — 모델 JSON + `DocContext` + 폭으로 머리 · 정보 상자 · 본문 · 각주(HistoryTime 상세 칸 380).
* `HanjaCardView`는 자료가 없으면 글자와 「자전에서 열기」만(`missing`을 주면 그 한 줄), `kangxiUri`로 `historyarchive://kangxi/<글자>`.
* `DocContext.copyWith` · `DocContext.stripOpenLabel` · `stripMissing`(연표 띠의 두 글 — 기본은 아카이브 글) · `DocWidth`(칸 폭 — 표가 화면 대신 칸 폭으로 꼴을 고름) · `DocHeadView.updated`는 없어도 됨.
* 엔진(`history_core`)에 기대지 않는다. 글꼴은 싣지 않는다(README의 가족 이름을 받는 앱이 선언).
