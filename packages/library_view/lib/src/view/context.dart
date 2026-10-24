import 'package:flutter/widgets.dart';

import 'model.dart';
import 'timeline_strip.dart';

/// 문서를 그리는 데 필요한 바깥 것 — 링크가 있는지, 누르면 할 일, 연표 띠 자료, 오늘.
/// 그리기는 서가 · 엔진을 모른다(P5 R8): 받는 앱(아카이브 · HistoryTime)이 이것을 채워 넘긴다.
class DocContext {
  DocContext({
    this.exists = _always,
    this.footnotes = const [],
    this.sections = const [],
    this.onLink,
    this.onUrl,
    this.onFootnote,
    this.onSection,
    this.onEditSection,
    this.onTimeline,
    this.onHanja,
    this.onHanjaExit,
    this.onHanjaTap,
    this.strip,
    this.onMenu,
    this.onLinkHover,
    this.onFootnoteHover,
    this.now = DateTime.now,
    this.stripOpenLabel = 'HistoryTime에서 열기',
    this.stripMissing = 'HistoryTime이 이 기기에 있으면 이 자리에 사건이 보입니다.',
    DocAnchors? anchors,
  }) : anchors = anchors ?? DocAnchors();

  /// 우클릭 · 길게 누름(P3-6 R3) — 문단 위라면 어느 링크 위인지와 함께. 화면이 메뉴를 띄운다.
  final void Function(MenuRequest request)? onMenu;

  /// 링크 · 각주 표시에 마우스를 올리거나 떠났다(P3-6 R4) — [rect]는 그 조각의 화면 자리. 화면이 미리보기 카드를 띄운다.
  final void Function(ObjectAddress target, Rect rect, bool enter)? onLinkHover;
  final void Function(int n, Rect rect, bool enter)? onFootnoteHover;

  /// 링크 대상이 서재에 있는가(볼 때 titles.exist로 한 번에 물은 답, 설계 5-6).
  final bool Function(ObjectAddress target) exists;
  final List<Footnote> footnotes;
  final List<Section> sections;
  final void Function(ObjectAddress target)? onLink;
  final void Function(String href)? onUrl;
  final void Function(int n)? onFootnote;

  /// 목차에서 고른 문단(sections의 index).
  final void Function(int section)? onSection;
  final void Function(int section)? onEditSection;
  final void Function(String entity)? onTimeline;

  /// 한자 한 글자에 올리거나 눌렀다 — [rect]는 그 글자의 화면 자리.
  final void Function(String char, Rect rect)? onHanja;
  final VoidCallback? onHanjaExit;

  /// 한자를 눌렀다 — 있으면 누름은 이것만 부른다(올림에는 반응하지 않는 곳, 예: 대문).
  final void Function(String char)? onHanjaTap;
  final Future<TimelineStripData?> Function(String entity, (int, int)? range)? strip;
  final DateTime Function() now;
  final DocAnchors anchors;

  /// 문서 속 연표 띠의 여는 단추 글(누르면 [onTimeline]) — 아카이브는 「HistoryTime에서 열기」. HistoryTime이 그릴 때는 제 말로 바꾼다.
  final String stripOpenLabel;

  /// 연표 띠 자료가 없을 때(`strip`이 없거나 null · 실패) 「제목 · 범위」 아래의 한 줄.
  final String stripMissing;

  static bool _always(ObjectAddress _) => true;

  /// 몇 칸만 바꾼 사본 — 뛰는 자리([anchors])는 같은 것을 쓴다. 콜백을 null로 되돌리지는 못한다(새로 만든다).
  DocContext copyWith({
    bool Function(ObjectAddress target)? exists,
    List<Footnote>? footnotes,
    List<Section>? sections,
    void Function(ObjectAddress target)? onLink,
    void Function(String href)? onUrl,
    void Function(int n)? onFootnote,
    void Function(int section)? onSection,
    void Function(int section)? onEditSection,
    void Function(String entity)? onTimeline,
    void Function(String char, Rect rect)? onHanja,
    VoidCallback? onHanjaExit,
    void Function(String char)? onHanjaTap,
    Future<TimelineStripData?> Function(String entity, (int, int)? range)? strip,
    void Function(MenuRequest request)? onMenu,
    void Function(ObjectAddress target, Rect rect, bool enter)? onLinkHover,
    void Function(int n, Rect rect, bool enter)? onFootnoteHover,
    DateTime Function()? now,
    String? stripOpenLabel,
    String? stripMissing,
  }) => DocContext(
    exists: exists ?? this.exists,
    footnotes: footnotes ?? this.footnotes,
    sections: sections ?? this.sections,
    onLink: onLink ?? this.onLink,
    onUrl: onUrl ?? this.onUrl,
    onFootnote: onFootnote ?? this.onFootnote,
    onSection: onSection ?? this.onSection,
    onEditSection: onEditSection ?? this.onEditSection,
    onTimeline: onTimeline ?? this.onTimeline,
    onHanja: onHanja ?? this.onHanja,
    onHanjaExit: onHanjaExit ?? this.onHanjaExit,
    onHanjaTap: onHanjaTap ?? this.onHanjaTap,
    strip: strip ?? this.strip,
    onMenu: onMenu ?? this.onMenu,
    onLinkHover: onLinkHover ?? this.onLinkHover,
    onFootnoteHover: onFootnoteHover ?? this.onFootnoteHover,
    now: now ?? this.now,
    stripOpenLabel: stripOpenLabel ?? this.stripOpenLabel,
    stripMissing: stripMissing ?? this.stripMissing,
    anchors: anchors,
  );
}

/// 메뉴를 부른 자리 — 화면 좌표와, 링크 위였으면 그 링크.
class MenuRequest {
  const MenuRequest(this.at, {this.link});
  final Offset at;
  final ObjectAddress? link;
}

/// 문단 · 각주로 뛰는 자리.
class DocAnchors {
  final _sections = <int, GlobalKey>{};
  final _footnotes = <int, GlobalKey>{};
  GlobalKey section(int index) => _sections.putIfAbsent(index, () => GlobalKey(debugLabel: '문단 $index'));
  GlobalKey footnote(int n) => _footnotes.putIfAbsent(n, () => GlobalKey(debugLabel: '각주 $n'));
}

/// 문서를 그리는 칸의 폭 — 화면보다 좁은 칸(HistoryTime 상세 칸 380)에 그릴 때 [LibraryDocView]가 둔다.
/// 표처럼 폭으로 꼴을 고르는 위젯은 이것이 있으면 이것을, 없으면 화면 폭(MediaQuery)을 본다 — 아카이브 화면은 두지 않아 그대로다.
class DocWidth extends InheritedWidget {
  const DocWidth({super.key, required this.width, required super.child});
  final double width;

  /// 칸의 폭, 없으면 화면 폭.
  static double of(BuildContext context) => context.dependOnInheritedWidgetOfExactType<DocWidth>()?.width ?? MediaQuery.sizeOf(context).width;

  @override
  bool updateShouldNotify(DocWidth old) => old.width != width;
}
