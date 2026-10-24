import 'package:flutter/widgets.dart';

import '../ui/dom.dart';
import '../ui/icons.dart';
import '../ui/press.dart';
import '../ui/tokens.dart';
import '../ui/type.dart';
import 'context.dart';
import 'inlines.dart';
import 'model.dart';
import 'selection.dart';
import 'timeline_strip.dart';
import 'toc.dart';

export 'context.dart';
export 'inlines.dart' show ParaView, plainTextOf;

/// 문서의 블록들 — 게으르게 짓는다(보이는 것과 [ArchiveScroll.cacheExtent] 안의 것만, Review Focus 1).
/// 접힌 문단 제목(`==# … #==`)은 눌러 펼칠 때까지 그 아래 블록을 숨긴다.
class SliverDocBlocks extends StatefulWidget {
  const SliverDocBlocks({super.key, required this.blocks, required this.context});
  final List<Block> blocks;
  final DocContext context;

  @override
  State<SliverDocBlocks> createState() => _SliverDocBlocksState();
}

class _SliverDocBlocksState extends State<SliverDocBlocks> {
  final _opened = <int>{};
  late List<int?> _owner;

  @override
  void initState() {
    super.initState();
    _owner = _folding(widget.blocks);
  }

  @override
  void didUpdateWidget(SliverDocBlocks old) {
    super.didUpdateWidget(old);
    if (!identical(old.blocks, widget.blocks)) {
      _owner = _folding(widget.blocks);
      _opened.clear();
    }
  }

  /// 블록마다 그것을 숨기는 접힌 제목의 자리(없으면 null).
  static List<int?> _folding(List<Block> blocks) {
    final owner = List<int?>.filled(blocks.length, null);
    int? fold;
    var level = 0;
    for (final (i, b) in blocks.indexed) {
      if (b is HeadingBlock) {
        if (fold != null && b.level <= level) fold = null;
        if (fold == null && b.folded) {
          fold = i;
          level = b.level;
          continue;
        }
      }
      owner[i] = fold;
    }
    return owner;
  }

  @override
  Widget build(BuildContext context) {
    return SliverList(
      delegate: SliverChildBuilderDelegate((context, i) {
        final owner = _owner[i];
        if (owner != null && !_opened.contains(owner)) return const SizedBox.shrink();
        final b = widget.blocks[i];
        final toggle = b is HeadingBlock && b.folded ? () => setState(() => _opened.contains(i) ? _opened.remove(i) : _opened.add(i)) : null;
        return BlockView(b, ctx: widget.context, first: i == 0, onToggle: toggle, open: _opened.contains(i));
      }, childCount: widget.blocks.length),
    );
  }
}

/// 블록 하나 — 위쪽 틈까지 스스로 둔다.
class BlockView extends StatelessWidget {
  const BlockView(this.block, {super.key, required this.ctx, this.first = false, this.onToggle, this.open = false});
  final Block block;
  final DocContext ctx;
  final bool first;
  final VoidCallback? onToggle;
  final bool open;

  double get _gap => first
      ? 0
      : switch (block) {
          HeadingBlock(:final level) => level <= 2 ? 40 : (level == 3 ? 30 : 24),
          TimelineStripBlock() => 26,
          TableBlock() => 8,
          TocBlock() => 30,
          FrontMacroBlock() => 20,
          _ => 12,
        };

  @override
  Widget build(BuildContext context) {
    final body = _body();
    // 블록마다 선택 단위(줄바꿈). 목차 · 연표 띠(그림 · 단추)는 선택에서 뺀다(P3-6 R5).
    final figure = block is TocBlock || block is TimelineStripBlock;
    return Padding(
      padding: EdgeInsets.only(top: _gap),
      child: figure ? SelectionContainer.disabled(child: body) : SelectionBlock(child: body),
    );
  }

  Widget _body() => switch (block) {
    HeadingBlock() => HeadingView(block as HeadingBlock, ctx: ctx, onToggle: onToggle, open: open),
    ParaBlock(:final inl) => Align(
      alignment: Alignment.topLeft,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: Metrics.bodyMax),
        child: ParaView(inl, ctx: ctx, style: TextStyles.body),
      ),
    ),
    ListBlock() => _ListView(block as ListBlock, ctx: ctx),
    QuoteBlock(:final blocks) => DecoratedBox(
      decoration: BoxDecoration(color: Ink.raised, borderRadius: BorderRadius.circular(Radii.chip)),
      child: Padding(padding: const EdgeInsets.fromLTRB(16, 12, 16, 12), child: _column(blocks)),
    ),
    FoldBlock() => _FoldView(block as FoldBlock, ctx: ctx),
    IndentBlock(:final depth, :final blocks) => Padding(
      padding: EdgeInsets.only(left: 24.0 * depth),
      child: _column(blocks),
    ),
    TableBlock() => TableView(block as TableBlock, ctx: ctx),
    CodeBlock(:final text) => DecoratedBox(
      decoration: BoxDecoration(color: Ink.raised, borderRadius: BorderRadius.circular(Radii.chip)),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.all(12),
        child: Text.rich(TextSpan(text: text, style: TextStyles.code(13.5, 22))),
      ),
    ),
    RuleBlock() => const Padding(
      padding: EdgeInsets.symmetric(vertical: 8),
      child: SizedBox(height: 1, child: ColoredBox(color: Ink.rule)),
    ),
    TocBlock() => Align(
      alignment: Alignment.topLeft,
      child: TocView(sections: ctx.sections, onSection: ctx.onSection),
    ),
    FootnotesBlock() => FootnotesView(ctx: ctx),
    TemplateBlock(:final status, :final target, :final blocks) => status == 'ok' ? _column(blocks) : _note('${_templateTrouble(status)} · ${target.title}'),
    EmbedBlock(:final target) => _note('끼우기 · ${target.title}'),
    FrontMacroBlock(:final name, :final arg) => DecoratedBox(
      decoration: BoxDecoration(color: Ink.raised, borderRadius: BorderRadius.circular(Radii.paper)),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        child: Text(
          arg == null ? name : '$name($arg)',
          style: TextStyles.of(14, 20, weight: FontWeight.w600, color: Ink.ink2),
        ),
      ),
    ),
    TimelineStripBlock() => TimelineStripView(block as TimelineStripBlock, ctx: ctx),
    UnsupportedBlock(:final kind) => _note(kind == 'html' ? 'HTML 블록은 그리지 않습니다' : '그리지 않는 블록 · $kind'),
    UnknownBlock() => _note((block as UnknownBlock).text.isEmpty ? '이 앱이 모르는 블록 · ${(block as UnknownBlock).t}' : (block as UnknownBlock).text),
  };

  Widget _column(List<Block> blocks) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    mainAxisSize: MainAxisSize.min,
    children: [for (final (i, b) in blocks.indexed) BlockView(b, ctx: ctx, first: i == 0)],
  );

  static Widget _note(String s) => Text.rich(TextSpan(text: s, style: TextStyles.meta));

  static String _templateTrouble(String status) => switch (status) {
    'missing' => '없는 틀',
    'cycle' => '틀이 자기를 다시 부름',
    'too_deep' => '틀을 너무 깊이 펼침',
    'too_many' => '틀을 너무 많이 펼침',
    'too_large' => '틀 글이 너무 큼',
    _ => '틀을 펼치지 못함',
  };
}

/// 문단 제목 — 번호(셋째 글자) · 제목 · 오른쪽 「편집」.
class HeadingView extends StatelessWidget {
  const HeadingView(this.h, {super.key, required this.ctx, this.onToggle, this.open = false});
  final HeadingBlock h;
  final DocContext ctx;
  final VoidCallback? onToggle;
  final bool open;

  TextStyle get _style => h.level <= 2 ? TextStyles.h2 : (h.level == 3 ? TextStyles.h3 : TextStyles.h4);

  @override
  Widget build(BuildContext context) {
    final style = _style;
    final section = h.section;
    Widget title = Semantics(
      header: true,
      headingLevel: (h.level).clamp(2, 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.baseline,
        textBaseline: TextBaseline.alphabetic,
        children: [
          if (h.number.isNotEmpty) ...[
            // 문단 번호는 꾸밈 — 끌어 선택에 들지 않는다(P3-6 R5).
            SelectionContainer.disabled(
              child: Text(
                h.number,
                style: style.copyWith(color: Ink.muted, fontWeight: FontWeight.w600, fontFeatures: const [FontFeature.tabularFigures()]),
              ),
            ),
            SizedBox(width: h.level <= 2 ? 12 : 10),
          ],
          Flexible(
            child: ParaView(h.inl, ctx: ctx, style: style),
          ),
          if (onToggle != null) ...[const SizedBox(width: 8), AppIcon(open ? Glyphs.chevronDown : Glyphs.chevronRight, size: 14)],
        ],
      ),
    );
    if (onToggle != null) title = Press(onTap: onToggle, child: title);
    final row = Row(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        Expanded(child: title),
        if (section != null && ctx.onEditSection != null)
          SelectionContainer.disabled(
            child: Press(
              link: true,
              onTap: () => ctx.onEditSection!(section),
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
              child: Text('편집', style: TextStyles.small),
            ),
          ),
      ],
    );
    if (section == null) return row;
    return KeyedSubtree(
      key: ValueKey('heading-$section'),
      child: KeyedSubtree(key: ctx.anchors.section(section), child: row),
    );
  }
}

class _ListView extends StatelessWidget {
  const _ListView(this.list, {required this.ctx});
  final ListBlock list;
  final DocContext ctx;

  String _marker(int i) {
    final n = list.start + i;
    String alpha(int n) => String.fromCharCode('a'.codeUnitAt(0) + (n - 1) % 26);
    String roman(int n) {
      const v = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1], s = ['m', 'cm', 'd', 'cd', 'c', 'xc', 'l', 'xl', 'x', 'ix', 'v', 'iv', 'i'];
      final b = StringBuffer();
      for (var k = 0; k < v.length; k++) {
        while (n >= v[k]) {
          b.write(s[k]);
          n -= v[k];
        }
      }
      return b.toString();
    }

    return switch (list.style) {
      'decimal' => '$n.',
      'lower_alpha' => '${alpha(n)}.',
      'upper_alpha' => '${alpha(n).toUpperCase()}.',
      'lower_roman' => '${roman(n)}.',
      'upper_roman' => '${roman(n).toUpperCase()}.',
      _ => '•',
    };
  }

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    mainAxisSize: MainAxisSize.min,
    children: [
      for (final (i, item) in list.items.indexed)
        Padding(
          padding: EdgeInsets.only(top: i == 0 ? 0 : 4),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 목록 표시는 꾸밈 — 끌어 선택에 들지 않는다(검토 M5).
              SelectionContainer.disabled(
                child: SizedBox(
                  width: 28,
                  child: Text(
                    _marker(i),
                    style: TextStyles.body.copyWith(color: Ink.muted),
                    textAlign: TextAlign.right,
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  mainAxisSize: MainAxisSize.min,
                  children: [for (final (k, b) in item.indexed) BlockView(b, ctx: ctx, first: k == 0)],
                ),
              ),
            ],
          ),
        ),
    ],
  );
}

class _FoldView extends StatefulWidget {
  const _FoldView(this.fold, {required this.ctx});
  final FoldBlock fold;
  final DocContext ctx;

  @override
  State<_FoldView> createState() => _FoldViewState();
}

class _FoldViewState extends State<_FoldView> {
  var _open = false;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    mainAxisSize: MainAxisSize.min,
    children: [
      Align(
        alignment: Alignment.topLeft,
        child: Press(
          onTap: () => setState(() => _open = !_open),
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              AppIcon(_open ? Glyphs.chevronDown : Glyphs.chevronRight, size: 12),
              const SizedBox(width: 6),
              Text(
                widget.fold.summary.isEmpty ? '펼치기' : widget.fold.summary,
                style: TextStyles.of(14, 20, weight: FontWeight.w600, color: Ink.ink2),
              ),
            ],
          ),
        ),
      ),
      if (_open)
        Padding(
          padding: const EdgeInsets.only(left: 18, top: 4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [for (final (i, b) in widget.fold.blocks.indexed) BlockView(b, ctx: widget.ctx, first: i == 0)],
          ),
        ),
    ],
  );
}

/// 표 — 첫 줄의 칸이 모두 굵으면 머리 줄(작은 셋째 글자). 칸을 가로로 합친 표는 칸마다 그 수만큼 폭을 준다.
/// 폭이 넓은 표는 가로로 밀린다(390에서 넘치지 않음, Review Focus 5).
class TableView extends StatelessWidget {
  const TableView(this.table, {super.key, required this.ctx});
  final TableBlock table;
  final DocContext ctx;

  static bool _allBold(List<Inline> inl) {
    final real = inl.where((i) => !(i is TextInline && i.s.trim().isEmpty)).toList();
    return real.isNotEmpty && real.every((i) => i is StyledInline && i.style == InlineStyle.bold);
  }

  static List<Inline> _unbold(List<Inline> inl) => [
    for (final i in inl)
      if (i is StyledInline && i.style == InlineStyle.bold) ...i.inl else i,
  ];

  /// 칸이 이보다 좁아지면 표를 가로로 민다.
  static const _minColumn = 96.0;

  @override
  Widget build(BuildContext context) {
    // 칸 폭(DocWidth — 좁은 칸에 그릴 때) 또는 화면 폭으로 고른다.
    final narrowCells = breakpointOf(DocWidth.of(context)) == Breakpoint.phone;
    final rows = table.rows;
    final head = rows.isNotEmpty && rows.first.isNotEmpty && rows.first.every((c) => _allBold(c.inl));
    final columns = rows.fold<int>(0, (m, r) => r.fold<int>(0, (s, c) => s + c.col) > m ? r.fold<int>(0, (s, c) => s + c.col) : m);
    final merged = rows.any((r) => r.any((c) => c.col > 1 || c.row > 1));
    Widget cell(int r, int c, DocCell cell) {
      final isHead = head && r == 0;
      final style = isHead ? TextStyles.of(12, 16, color: Ink.muted) : TextStyles.of(14, 20, tabular: true);
      final align = switch (cell.align) {
        CellAlign.center => TextAlign.center,
        CellAlign.end => TextAlign.right,
        CellAlign.start => TextAlign.left,
      };
      return DomTag(
        isHead ? 'th' : 'td',
        child: KeyedSubtree(
          key: ValueKey('cell-$r-$c'),
          // 선택하면 표가 줄마다 칸을 탭으로 잇는다(P3-6 R5 · 검토 I3).
          child: SelectionCell(
            row: r,
            col: c,
            child: DecoratedBox(
              decoration: BoxDecoration(
                border: isHead || r == 0 ? null : const Border(top: BorderSide(color: Ink.rule)),
              ),
              child: Padding(
                padding: EdgeInsets.fromLTRB(0, isHead ? 8 : 11, narrowCells ? 16 : 32, isHead ? 8 : 11),
                child: ParaView(isHead ? _unbold(cell.inl) : cell.inl, ctx: ctx, style: style, textAlign: align),
              ),
            ),
          ),
        ),
      );
    }

    final body = LayoutBuilder(
      builder: (context, c) {
        final narrow = c.maxWidth < 640;
        if (merged || narrow) {
          // 칸을 합친 표 · 좁은 폭: 칸마다 합친 수만큼의 몫(세로 합치기는 한 줄로 본다). 칸이 [_minColumn]보다 좁아지면
          // 짓누르지 않고 표 전체를 가로로 민다(최종 검토 #9).
          final width = columns * _minColumn > c.maxWidth ? columns * _minColumn : c.maxWidth;
          final grid = SizedBox(
            width: width,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                for (final (r, row) in rows.indexed)
                  IntrinsicHeight(
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        for (final (k, x) in row.indexed) Expanded(flex: x.col, child: cell(r, k, x)),
                        if (row.fold<int>(0, (s, x) => s + x.col) < columns) Spacer(flex: columns - row.fold<int>(0, (s, x) => s + x.col)),
                      ],
                    ),
                  ),
              ],
            ),
          );
          // 선택 단위(표)는 가로 굴림 안쪽에 — 굴림 상자가 제 선택 그릇을 끼워 칸들을 가로채므로(P3-6 R5).
          final table = SelectionTable(child: grid);
          return width > c.maxWidth ? SingleChildScrollView(scrollDirection: Axis.horizontal, child: table) : table;
        }
        return SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: ConstrainedBox(
            constraints: BoxConstraints(minWidth: c.maxWidth < 520 ? c.maxWidth : 520),
            child: SelectionTable(
              child: Table(
                defaultColumnWidth: const IntrinsicColumnWidth(),
                defaultVerticalAlignment: TableCellVerticalAlignment.top,
                children: [
                  for (final (r, row) in rows.indexed) TableRow(children: [for (final (k, x) in row.indexed) cell(r, k, x), for (var k = row.length; k < columns; k++) const SizedBox.shrink()]),
                ],
              ),
            ),
          ),
        );
      },
    );
    return DomTag(
      'table',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          // 캡션은 제 줄(검토 I3).
          if (table.caption != null)
            SelectionBlock(
              child: Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: ParaView(table.caption!, ctx: ctx, style: TextStyles.meta),
              ),
            ),
          body,
        ],
      ),
    );
  }
}

/// 각주 — 번호(먹 600) · 글(둘째 글자 13.5/24). 위첨자에서 뛰어 오는 자리.
class FootnotesView extends StatelessWidget {
  const FootnotesView({super.key, required this.ctx});
  final DocContext ctx;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    mainAxisSize: MainAxisSize.min,
    children: [
      // 각주마다 제 줄, 번호는 선택에 들지 않는다(검토 M5).
      for (final f in ctx.footnotes)
        KeyedSubtree(
          key: ctx.anchors.footnote(f.n),
          child: SelectionBlock(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SelectionContainer.disabled(
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(minWidth: 16),
                    child: Text(
                      '${f.n}',
                      style: TextStyles.footnote.copyWith(color: Ink.text, fontWeight: FontWeight.w600),
                    ),
                  ),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: ParaView(f.inl, ctx: ctx, style: TextStyles.footnote),
                ),
              ],
            ),
          ),
        ),
    ],
  );
}
