/// 서재 문서 그리기 — 그리기 모델(판 1, HistoryCore spec/namumark.md) JSON과 바깥 일(콜백)만으로
/// 문서 전문 · 한자 카드 · 문서 속 연표 띠를 그린다. 엔진(history_core)에 기대지 않는다.
///
/// - 문서 전문: [LibraryDocView] · [SliverLibraryDoc] (모델 + [DocContext] + 폭)
/// - 블록만: [SliverDocBlocks] · [BlockView] · [ParaView]
/// - 한자 카드: [HanjaCardView] · [HanjaCard] · [kangxiUri]
/// - 연표 띠 자료: [TimelineStripData] · [StripSpan] · [StripEvent]
///
/// 토큰(색 · 글자 · 누름 · 아이콘)은 `package:library_view/ui.dart`.
library;

export 'src/doc_view.dart';
export 'src/view/blocks.dart';
export 'src/view/context.dart';
export 'src/view/doc_head.dart';
export 'src/view/float_layout.dart';
export 'src/view/hanja_card.dart';
export 'src/view/info_box.dart';
export 'src/view/inlines.dart';
export 'src/view/model.dart';
export 'src/view/selection.dart';
export 'src/view/timeline_strip.dart';
export 'src/view/toc.dart';
