// 문서 전문 한 장(P5 걸음 4 · HistoryTime 제안 2) — 그리기 모델 JSON과 바깥 일(DocContext)만으로 머리 · 정보 상자 · 본문 · 각주를 그린다.
// 엔진 없이 그린다: 받는 앱(HistoryTime)이 서재 창구 `window.doc`의 `model`을 그대로 넘긴다(spec/windows-api.md 4절).
// 아카이브 문서 화면(lib/src/screens/doc_screen.dart)의 종이 몫과 같은 꼴 — 머리줄 · 판 기록 · 편집 · 미리보기 카드는 앱 몫이라 뺐다.
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

import 'ui/press.dart';
import 'ui/tokens.dart';
import 'ui/type.dart';
import 'view/blocks.dart';
import 'view/doc_head.dart';
import 'view/float_layout.dart';
import 'view/info_box.dart';
import 'view/model.dart';

/// 정보 상자를 오른쪽에 띄우는 가장 좁은 칸 — 곁의 본문이 388 이상. 이보다 좁으면(HistoryTime 상세 칸 380) 본문 위에 둔다.
const floatInfoMinWidth = 720.0;

/// 정보 상자 곁으로 흐르게 둘 앞쪽 블록 수 — 나머지는 게으른 목록(아카이브 문서 화면과 같음).
const _floatBlocks = 6;

/// 문서 전문 — 제 굴림을 가진 위젯. 다른 굴림 안에 넣으려면 [SliverLibraryDoc].
///
/// [width]를 주면 그 폭으로 그린다(HistoryTime 상세 칸 380). 없으면 받은 폭.
/// 링크 · 연표 · 바깥 주소 · 한자 누름은 모두 [ctx]의 콜백으로 나간다 — 패키지는 아무것도 열지 않는다.
/// 끌어 선택 · 복사는 받는 앱이 `SelectionArea` 따위로 감싸 켠다.
class LibraryDocView extends StatelessWidget {
  const LibraryDocView({
    super.key,
    required this.model,
    required this.title,
    this.updated,
    this.ctx,
    this.width,
    this.padding = const EdgeInsets.fromLTRB(16, 20, 16, 40),
    this.controller,
    this.head = true,
    this.redirectedFrom,
  });

  /// 그리기 모델(판 1) JSON — `window.doc`의 `model` 그대로.
  final Map<String, dynamic> model;
  final String title;

  /// 고친 날(`window.doc`의 `updated`) — 그 고장 날로 보인다. null이면 그 줄을 뺀다.
  final DateTime? updated;
  final DocContext? ctx;
  final double? width;
  final EdgeInsets padding;
  final ScrollController? controller;

  /// 머리(제목 · 원어 · 고친 날 · 분류 줄)를 그릴지 — 받는 앱이 제 머리를 둘 때 끈다.
  final bool head;

  /// 받는 앱이 넘겨주기를 따라왔으면 그 문서 제목 — 「…에서 넘어옴」.
  final String? redirectedFrom;

  @override
  Widget build(BuildContext context) {
    Widget view(double w) => CustomScrollView(
      controller: controller,
      scrollCacheExtent: ScrollCacheExtent.pixels(ArchiveScroll.cacheExtent),
      slivers: [
        SliverPadding(
          padding: padding,
          sliver: SliverLibraryDoc(
            model: model,
            title: title,
            updated: updated,
            ctx: ctx,
            width: w - padding.horizontal < 0 ? 0 : w - padding.horizontal,
            head: head,
            redirectedFrom: redirectedFrom,
          ),
        ),
      ],
    );
    if (width case final w?) {
      return Align(alignment: Alignment.topLeft, child: SizedBox(width: w, child: view(w)));
    }
    return LayoutBuilder(builder: (context, c) => view(c.maxWidth));
  }
}

/// 문서 전문의 슬리버 — 받는 앱의 굴림(CustomScrollView) 속에 머리 · 정보 상자 · 블록 · 각주를 둔다.
/// 각주 위첨자 · 목차를 누르면, [ctx]에 `onFootnote` · `onSection`이 없을 때 이 굴림 안에서 그 자리로 뛴다.
class SliverLibraryDoc extends StatefulWidget {
  const SliverLibraryDoc({
    super.key,
    required this.model,
    required this.title,
    this.updated,
    this.ctx,
    this.width,
    this.head = true,
    this.redirectedFrom,
  });
  final Map<String, dynamic> model;
  final String title;
  final DateTime? updated;
  final DocContext? ctx;

  /// 글 칸의 폭(여백 뺌). 없으면 슬리버가 받은 폭.
  final double? width;
  final bool head;
  final String? redirectedFrom;

  @override
  State<SliverLibraryDoc> createState() => _SliverLibraryDocState();
}

class _SliverLibraryDocState extends State<SliverLibraryDoc> {
  late DocModel _doc;
  InfoBoxData? _info;

  /// 본문 흐름의 블록 — 한 번만 지어 같은 목록을 쓴다(다시 그릴 때마다 새 목록이면 펼친 접힌 문단이 닫힌다).
  late List<Block> _blocks;
  late List<Block> _flow;
  late List<Block> _rest;
  late DocContext _ctx;

  @override
  void initState() {
    super.initState();
    _parse();
    _context();
  }

  @override
  void didUpdateWidget(SliverLibraryDoc old) {
    super.didUpdateWidget(old);
    if (!identical(old.model, widget.model)) _parse();
    if (!identical(old.model, widget.model) || !identical(old.ctx, widget.ctx)) _context();
  }

  void _parse() {
    final m = _doc = DocModel.parse(widget.model);
    final info = _info = infoBoxOf(m);
    // 정보 상자의 틀 블록은 본문에서 빼고, `[각주]`가 없으면 끝에 각주를 둔다(아카이브와 같음 — 나무위키처럼).
    _blocks = [
      for (final (i, b) in m.blocks.indexed)
        if (i != info?.blockIndex) b,
      if (m.footnotes.isNotEmpty && !m.blocks.any((b) => b is FootnotesBlock)) const FootnotesBlock(),
    ];
    _flow = _blocks.take(_floatBlocks).toList();
    _rest = _blocks.skip(_floatBlocks).toList();
  }

  /// 받은 바깥 일에 이 문서의 각주 · 문단을 채우고, 뛰기가 없으면 이 굴림 안에서 뛰게 한다.
  void _context() {
    final given = widget.ctx ?? DocContext();
    final anchors = given.anchors;
    _ctx = given.copyWith(
      footnotes: _doc.footnotes,
      sections: _doc.sections,
      onFootnote: given.onFootnote ?? (n) => _jump(anchors.footnote(n)),
      onSection: given.onSection ?? (i) => _jump(anchors.section(i)),
    );
  }

  /// 그 자리가 지어질 때까지 조금씩 내려 가며 찾는다(게으른 목록 속 문단 · 각주 — 아카이브 문서 화면과 같은 셈).
  Future<void> _jump(GlobalKey key) async {
    for (var i = 0; i < 60 && mounted; i++) {
      final target = key.currentContext;
      if (target != null) {
        await Scrollable.ensureVisible(target, alignment: 0.05, duration: const Duration(milliseconds: 200));
        return;
      }
      final pos = Scrollable.maybeOf(context)?.position;
      if (pos == null || pos.pixels >= pos.maxScrollExtent) return;
      pos.jumpTo((pos.pixels + pos.viewportDimension * 0.8).clamp(0, pos.maxScrollExtent));
      await SchedulerBinding.instance.endOfFrame;
    }
  }

  @override
  Widget build(BuildContext context) {
    final w = widget.width;
    if (w != null) return _sliver(w);
    return SliverLayoutBuilder(builder: (context, c) => _sliver(c.crossAxisExtent));
  }

  Widget _sliver(double width) {
    final ctx = _ctx;
    final m = _doc;
    final phone = breakpointOf(width) == Breakpoint.phone;
    final head = !widget.head
        ? null
        // 머리는 끌어 선택에서 뺀다 — 본문만 읽는 차례대로 복사된다(P3-6 R5).
        : SelectionContainer.disabled(
            child: DocHeadView(
              title: widget.title,
              original: originalTitle(m, widget.title),
              updated: widget.updated?.toLocal(),
              redirectedFrom: widget.redirectedFrom,
              phone: phone,
              categories: [
                for (final c in m.categories) (c, ctx.exists(ObjectAddress('분류', c)), () => ctx.onLink?.call(ObjectAddress('분류', c))),
              ],
            ),
          );
    final List<Widget> slivers;
    if (m.redirect case final to?) {
      // 창구는 넘겨주기를 따라가지 않는다(spec/windows-api.md 4절) — 대상을 보이고, 누르면 받는 앱이 다시 묻는다.
      slivers = [
        if (head != null) SliverToBoxAdapter(child: head),
        SliverToBoxAdapter(
          child: Padding(padding: EdgeInsets.only(top: head == null ? 0 : 18), child: _RedirectNote(to, ctx: ctx)),
        ),
      ];
    } else {
      final info = _info == null ? null : SelectionContainer.disabled(child: InfoBoxView(_info!, ctx: ctx));
      final float = info != null && width >= floatInfoMinWidth;
      slivers = [
        if (head != null) SliverToBoxAdapter(child: head),
        if (float) ...[
          SliverToBoxAdapter(
            child: FloatLayout(
              float: info,
              floatWidth: Metrics.infoBox,
              gap: Metrics.infoGap,
              floatTop: head == null ? 0 : 28,
              flow: [for (final (i, b) in _flow.indexed) BlockView(b, ctx: ctx, first: head == null && i == 0, key: ValueKey('float-$i'))],
            ),
          ),
          SliverDocBlocks(blocks: _rest, context: ctx),
        ] else ...[
          if (info != null)
            SliverToBoxAdapter(
              child: Padding(padding: EdgeInsets.only(top: head == null ? 0 : 20), child: info),
            ),
          SliverPadding(
            padding: EdgeInsets.only(top: head == null && info == null ? 0 : 18),
            sliver: SliverDocBlocks(blocks: _blocks, context: ctx),
          ),
        ],
      ];
    }
    return DocWidth(
      width: width,
      child: SliverMainAxisGroup(slivers: slivers),
    );
  }
}

/// 넘겨주기 문서 — 대상 이름과 「열기」. 누르면 [DocContext.onLink].
class _RedirectNote extends StatelessWidget {
  const _RedirectNote(this.to, {required this.ctx});
  final ObjectAddress to;
  final DocContext ctx;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    mainAxisSize: MainAxisSize.min,
    children: [
      Text('넘겨주기 문서입니다.', style: TextStyles.meta),
      const SizedBox(height: 4),
      KeyedSubtree(
        key: const ValueKey('redirect'),
        child: Press(
          link: true,
          label: '${to.title} 열기',
          onTap: ctx.onLink == null ? null : () => ctx.onLink!(to),
          radius: Radii.chip,
          child: Text(
            '「${to.title}」로',
            style: TextStyles.body.copyWith(decoration: TextDecoration.underline, decorationColor: Ink.linkLine),
          ),
        ),
      ),
    ],
  );
}
