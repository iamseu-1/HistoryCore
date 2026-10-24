import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import '../ui/icons.dart';
import '../ui/tokens.dart';
import '../ui/type.dart';
import 'context.dart';
import 'model.dart';

/// 인라인 → TextSpan. 글 조각마다 다 풀린 모양을 단다(쌍둥이 · 시험이 조각만 보고도 모양을 안다).
/// 한자 구간은 글자마다 조각 — 올린 한 글자만 칠하고 자전 카드를 그 위에 띄운다.
class InlineSpans {
  InlineSpans(this.ctx, {this.hovered, this.onHanHover});
  final DocContext ctx;

  /// 올려 놓은 한자의 글 위치(UTF-16).
  final int? hovered;
  final void Function(int offset, String char, bool enter)? onHanHover;

  final recognizers = <GestureRecognizer>[];

  /// 한자 글자마다의 (위치, 글자) — 위치로도 찾는다.
  final hanja = <(int, String)>[];

  /// 링크마다의 (시작, 끝, 대상) — 올렸을 때 어느 링크인지(P3-6 R4).
  final links = <(int, int, ObjectAddress)>[];
  final hanjaAt = <int, String>{};
  var _offset = 0;

  void dispose() {
    for (final r in recognizers) {
      r.dispose();
    }
  }

  TextSpan build(List<Inline> inl, TextStyle style) => TextSpan(style: style, children: _spans(_trimBreaks(inl), style));

  TapGestureRecognizer _tap(VoidCallback? f) {
    final r = TapGestureRecognizer()..onTap = f;
    recognizers.add(r);
    return r;
  }

  /// 링크의 누름 — 어느 링크인지 들고 있어 우클릭이 조각에서 링크를 알아본다(P3-6 R3).
  LinkTap _linkTap(ObjectAddress target, VoidCallback f) {
    final r = LinkTap(target)..onTap = f;
    recognizers.add(r);
    return r;
  }

  TextSpan _text(String s, TextStyle style, {GestureRecognizer? recognizer, MouseCursor? cursor}) {
    _offset += s.length;
    return TextSpan(text: s, style: style, recognizer: recognizer, mouseCursor: cursor);
  }

  WidgetSpan _widget(Widget child, TextStyle style, {PlaceholderAlignment alignment = PlaceholderAlignment.baseline}) {
    _offset += 1;
    return WidgetSpan(child: child, style: style, alignment: alignment, baseline: TextBaseline.alphabetic);
  }

  List<InlineSpan> _spans(List<Inline> inl, TextStyle style) => [for (final i in inl) ..._span(i, style)];

  Iterable<InlineSpan> _span(Inline i, TextStyle style) sync* {
    switch (i) {
      case TextInline(:final s):
        yield _text(s, style);
      case HanInline(:final s):
        final serif = TextStyles.hanja(
          style.fontSize ?? 16,
          line: (style.height ?? 1.75) * (style.fontSize ?? 16),
          color: style.color ?? Ink.text,
        ).copyWith(decoration: style.decoration, decorationColor: style.decorationColor, decorationStyle: style.decorationStyle, fontWeight: style.fontWeight);
        for (final rune in s.runes) {
          final ch = String.fromCharCode(rune);
          final at = _offset;
          hanja.add((at, ch));
          hanjaAt[at] = ch;
          final on = hovered == at;
          yield _text(
            ch,
            on ? serif.copyWith(backgroundColor: Ink.active) : serif,
            recognizer: ctx.onHanjaTap != null ? _tap(() => ctx.onHanjaTap!(ch)) : (ctx.onHanja == null ? null : _tap(() => onHanHover?.call(at, ch, true))),
            cursor: ctx.onHanja == null && ctx.onHanjaTap == null ? null : SystemMouseCursors.click,
          );
        }
      case CodeInline(:final s):
        yield _text(s, TextStyles.code((style.fontSize ?? 16) * 0.9, (style.height ?? 1.75) * (style.fontSize ?? 16)).copyWith(backgroundColor: Ink.raised));
      case StyledInline(style: InlineStyle.sup || InlineStyle.sub, :final inl):
        final small = style.copyWith(fontSize: (style.fontSize ?? 16) * 0.72, height: 1.2);
        yield _widget(
          Transform.translate(offset: Offset(0, i.style == InlineStyle.sup ? -(style.fontSize ?? 16) * 0.35 : (style.fontSize ?? 16) * 0.2), child: Text.rich(InlineSpans(ctx).build(inl, small))),
          style,
        );
      case StyledInline(style: final kind, :final inl):
        yield* _spans(inl, _styled(style, kind));
      case SizedInline(:final step, :final inl):
        final size = (style.fontSize ?? 16) * _sizeFactor(step);
        yield* _spans(inl, style.copyWith(fontSize: size));
      case LinkInline(:final target, inl: final label):
        yield* _link(target, label, style);
      case UrlInline(:final href, inl: final label):
        final s = _linkStyle(style, exists: true);
        final rec = _tap(() => ctx.onUrl?.call(href));
        if (label == null) {
          yield _text(href, s, recognizer: rec, cursor: SystemMouseCursors.click);
        } else {
          yield* _spans(label, s).map((span) => span is TextSpan && span.text != null ? TextSpan(text: span.text, style: span.style, recognizer: rec, mouseCursor: SystemMouseCursors.click) : span);
        }
      case FileInline(:final key):
        yield _text('[파일:$key]', style.copyWith(color: Ink.muted));
      case FootnoteRefInline(:final n):
        yield _widget(
          _FootnoteRef(n, ctx: ctx, color: style.color),
          style,
          alignment: PlaceholderAlignment.middle,
        );
      case RubyInline(:final base, :final ruby):
        final hasHan = RegExp(r'[㐀-鿿豈-﫿]').hasMatch(base);
        yield _widget(
          Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(ruby, style: TextStyles.of(10, 12, color: Ink.muted)),
              Text(base, style: hasHan ? TextStyles.hanja(style.fontSize ?? 16, line: style.fontSize ?? 16) : style.copyWith(height: 1)),
            ],
          ),
          style,
          alignment: PlaceholderAlignment.bottom,
        );
      case BreakInline():
        yield _text('\n', style);
      case MacroInline(:final name, :final args):
        final s = _macro(name, args);
        if (s.isNotEmpty) yield _text(s, style);
      case ParamInline(:final name, :final fallback):
        yield _text(fallback == null ? '@$name@' : '@$name=$fallback@', style.copyWith(color: Ink.muted));
      case UnknownInline():
        yield _text(i.text, style.copyWith(color: Ink.muted));
    }
  }

  static TextStyle _styled(TextStyle s, InlineStyle kind) => switch (kind) {
    InlineStyle.bold => s.copyWith(fontWeight: FontWeight.w700),
    InlineStyle.italic => s.copyWith(fontStyle: FontStyle.italic),
    InlineStyle.underline => s.copyWith(decoration: TextDecoration.combine([if (s.decoration != null) s.decoration!, TextDecoration.underline])),
    InlineStyle.strike => s.copyWith(decoration: TextDecoration.combine([if (s.decoration != null) s.decoration!, TextDecoration.lineThrough])),
    InlineStyle.sup || InlineStyle.sub => s,
  };

  Iterable<InlineSpan> _link(ObjectAddress target, List<Inline>? label, TextStyle style) sync* {
    final timeline = target.ns == '연표';
    final exists = timeline || target.samePage || ctx.exists(target);
    final s = _linkStyle(style, exists: exists);
    final rec = _linkTap(target, () => timeline ? ctx.onTimeline?.call(target.key) : ctx.onLink?.call(target));
    final text = label == null ? (target.samePage ? (target.anchor ?? '') : target.title) : null;
    final start = _offset;
    if (text != null) {
      yield _text(text, s, recognizer: rec, cursor: SystemMouseCursors.click);
    } else {
      for (final span in _spans(label!, s)) {
        yield span is TextSpan && span.text != null ? TextSpan(text: span.text, style: span.style, recognizer: rec, mouseCursor: SystemMouseCursors.click) : span;
      }
    }
    links.add((start, _offset, target));
    if (timeline) {
      yield _widget(
        Padding(
          padding: const EdgeInsets.only(left: 4),
          child: AppIcon(Glyphs.external, size: (style.fontSize ?? 16) * 0.85, color: Ink.ink2),
        ),
        style,
        alignment: PlaceholderAlignment.middle,
      );
    }
  }

  static TextStyle _linkStyle(TextStyle style, {required bool exists}) => exists
      ? style.copyWith(color: Ink.text, decoration: TextDecoration.underline, decorationColor: Ink.linkLine, decorationThickness: 1)
      : style.copyWith(color: Ink.muted, decoration: TextDecoration.underline, decorationColor: Ink.missingLine, decorationStyle: TextDecorationStyle.dotted);

  String _macro(String name, List<String> args) {
    final now = ctx.now();
    String ymd(DateTime d) => '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
    final date = args.isEmpty ? null : DateTime.tryParse(args.first.trim());
    return switch (name) {
      'date' => ymd(now),
      'age' when date != null => '${now.year - date.year - ((now.month < date.month || now.month == date.month && now.day < date.day) ? 1 : 0)}',
      'dday' when date != null => () {
        final days = DateTime(now.year, now.month, now.day).difference(DateTime(date.year, date.month, date.day)).inDays;
        return days == 0
            ? 'D-Day'
            : days < 0
            ? 'D$days'
            : 'D+$days';
      }(),
      _ => '',
    };
  }
}

double _sizeFactor(int step) => const {-5: 0.55, -4: 0.62, -3: 0.7, -2: 0.8, -1: 0.875, 1: 1.15, 2: 1.3, 3: 1.5, 4: 1.75, 5: 2.0}[step.clamp(-5, 5)] ?? 1.0;

/// 문단 앞뒤의 줄바꿈 — 분류만 있는 줄이 남긴 것은 그리지 않는다(Ruling: 엔진 보고).
List<Inline> _trimBreaks(List<Inline> inl) {
  var a = 0, b = inl.length;
  while (a < b && inl[a] is BreakInline) {
    a++;
  }
  while (b > a && inl[b - 1] is BreakInline) {
    b--;
  }
  return a == 0 && b == inl.length ? inl : inl.sublist(a, b);
}

/// 인라인의 글만(정보 상자 열쇠 · 원어 · 목차).
String plainTextOf(List<Inline> inl) => inl
    .map(
      (i) => switch (i) {
        TextInline(:final s) || HanInline(:final s) || CodeInline(:final s) => s,
        StyledInline(:final inl) || SizedInline(:final inl) => plainTextOf(inl),
        LinkInline(:final target, :final inl) => inl == null ? target.title : plainTextOf(inl),
        UrlInline(:final href, :final inl) => inl == null ? href : plainTextOf(inl),
        RubyInline(:final base) => base,
        BreakInline() => '\n',
        UnknownInline() => i.text,
        _ => '',
      },
    )
    .join();

/// 글 한 덩이(문단 · 칸 · 각주). 한자에 올리면 그 글자를 칠하고 [DocContext.onHanja]에 화면 자리를 알린다.
class ParaView extends StatefulWidget {
  const ParaView(this.inl, {super.key, required this.ctx, required this.style, this.textAlign = TextAlign.start});
  final List<Inline> inl;
  final DocContext ctx;
  final TextStyle style;
  final TextAlign textAlign;

  @override
  State<ParaView> createState() => _ParaViewState();
}

class _ParaViewState extends State<ParaView> {
  /// 이 문단의 글 상자 — 선택 영역 안에서는 Text가 MouseRegion(글자 커서)으로 감싸지므로 아래로 내려가 찾는다.
  RenderParagraph? _paragraph() {
    RenderObject? ro = context.findRenderObject();
    while (ro != null && ro is! RenderParagraph) {
      RenderObject? child;
      ro.visitChildren((c) => child ??= c);
      ro = child;
    }
    return ro as RenderParagraph?;
  }

  InlineSpans? _spans;
  int? _hovered;

  /// 마우스가 올라 있는 링크.
  ObjectAddress? _link;

  @override
  void dispose() {
    // 게으른 목록에서 문단이 사라지면 올려 둔 링크도 떠난 것으로(검토 M2).
    _leaveLink();
    _spans?.dispose();
    super.dispose();
  }

  void _han(int offset, String char, bool enter) {
    final p = _paragraph();
    if (p is! RenderParagraph) return;
    final boxes = p.getBoxesForSelection(TextSelection(baseOffset: offset, extentOffset: offset + char.length));
    if (boxes.isEmpty) return;
    final r = boxes.first.toRect();
    final rect = Rect.fromPoints(p.localToGlobal(r.topLeft), p.localToGlobal(r.bottomRight));
    setState(() => _hovered = offset);
    widget.ctx.onHanja?.call(char, rect);
  }

  /// 글자 위 마우스 — 어느 한자 위인지 찾는다(TextSpan 조각마다 MouseRegion을 달 수 없어 문단 하나에서 본다).
  void _hover(PointerHoverEvent e) {
    final spans = _spans;
    final p = _paragraph();
    if (spans == null || p is! RenderParagraph) return;
    final local = p.globalToLocal(e.position);
    // 마우스 자리의 글 위치 하나로 찾는다 — 글자 오른쪽 반이면 위치가 하나 뒤, 4바이트 한자는 둘 뒤(최종 검토 #7).
    final pos = p.getPositionForOffset(local).offset;
    if (widget.ctx.onLinkHover != null && spans.links.isNotEmpty) _hoverLink(p, spans, local, pos);
    if (spans.hanja.isEmpty || widget.ctx.onHanja == null) return;
    for (final at in [pos, pos - 1, pos - 2]) {
      final ch = spans.hanjaAt[at];
      if (ch == null) continue;
      final boxes = p.getBoxesForSelection(TextSelection(baseOffset: at, extentOffset: at + ch.length));
      if (boxes.any((b) => b.toRect().contains(local))) {
        if (_hovered != at) _han(at, ch, true);
        return;
      }
    }
    _leave();
  }

  /// 링크 위인가 — 글 위치가 링크 구간에 들고 그 구간의 상자가 마우스를 품으면(P3-6 R4).
  void _hoverLink(RenderParagraph p, InlineSpans spans, Offset local, int pos) {
    for (final (a, b, target) in spans.links) {
      if (pos < a || pos > b) continue;
      final box = p.getBoxesForSelection(TextSelection(baseOffset: a, extentOffset: b)).map((x) => x.toRect()).where((r) => r.contains(local)).firstOrNull;
      if (box == null) continue;
      if (!identical(_link, target)) {
        _leaveLink();
        _link = target;
        widget.ctx.onLinkHover!(target, Rect.fromPoints(p.localToGlobal(box.topLeft), p.localToGlobal(box.bottomRight)), true);
      }
      return;
    }
    _leaveLink();
  }

  void _leaveLink() {
    final l = _link;
    if (l == null) return;
    _link = null;
    widget.ctx.onLinkHover?.call(l, Rect.zero, false);
  }

  void _leave() {
    if (_hovered == null) return;
    setState(() => _hovered = null);
    widget.ctx.onHanjaExit?.call();
  }

  /// 우클릭 · 길게 누름 — 그 자리의 조각이 링크면 그 링크와 함께 화면에 알린다.
  /// 그 자리의 조각이 링크면 그 링크.
  ObjectAddress? _linkAt(Offset global) {
    final p = _paragraph();
    if (p is! RenderParagraph) return null;
    final local = p.globalToLocal(global);
    final span = p.text.getSpanForPosition(p.getPositionForOffset(local));
    return span is TextSpan && span.recognizer is LinkTap ? (span.recognizer as LinkTap).target : null;
  }

  void _menu(Offset global) => widget.ctx.onMenu?.call(MenuRequest(global, link: _linkAt(global)));

  @override
  Widget build(BuildContext context) {
    _spans?.dispose();
    final spans = _spans = InlineSpans(widget.ctx, hovered: _hovered, onHanHover: _han);
    Widget text = Text.rich(spans.build(widget.inl, widget.style), textAlign: widget.textAlign);
    final wantHover = (widget.ctx.onHanja != null && _hasHan(widget.inl)) || (widget.ctx.onLinkHover != null && _hasLink(widget.inl));
    if (wantHover) {
      text = MouseRegion(
        onHover: _hover,
        onExit: (_) {
          _leave();
          _leaveLink();
        },
        opaque: false,
        child: text,
      );
    }
    if (widget.ctx.onMenu == null) return text;
    // 우클릭은 어디서나, 길게 누름은 링크 위에서만 듣는다 — 글 위의 길게 누름은 선택(SelectableRegion)에 넘긴다(P3-6 R5).
    return GestureDetector(
      behavior: HitTestBehavior.translucent,
      onSecondaryTapUp: (d) => _menu(d.globalPosition),
      child: RawGestureDetector(
        behavior: HitTestBehavior.translucent,
        gestures: {
          _LinkLongPress: GestureRecognizerFactoryWithHandlers<_LinkLongPress>(() => _LinkLongPress(onLink: (at) => _linkAt(at) != null), (r) => r.onLongPressStart = (d) => _menu(d.globalPosition)),
        },
        child: text,
      ),
    );
  }
}

/// 링크 위에서 시작한 길게 누름만 받는다.
class _LinkLongPress extends LongPressGestureRecognizer {
  /// 선택 영역(SelectableRegion)의 길게 누름보다 조금 먼저 받아 링크 위에서는 메뉴가 이긴다. 터치 · 펜만(마우스는 우클릭, 검토 M4).
  _LinkLongPress({required this.onLink}) : super(duration: const Duration(milliseconds: 450), supportedDevices: const {PointerDeviceKind.touch, PointerDeviceKind.stylus});
  final bool Function(Offset global) onLink;

  @override
  bool isPointerAllowed(PointerDownEvent event) => onLink(event.position) && super.isPointerAllowed(event);
}

/// 각주 표시(위 첨자) — 누르면 각주로, 올리면 각주 글 카드(P3-6 R4).
class _FootnoteRef extends StatelessWidget {
  const _FootnoteRef(this.n, {required this.ctx, this.color});
  final int n;
  final DocContext ctx;
  final Color? color;

  void _hover(BuildContext context, bool enter) {
    final f = ctx.onFootnoteHover;
    final box = context.findRenderObject() as RenderBox?;
    if (f == null || box == null || !box.hasSize) return;
    f(n, box.localToGlobal(Offset.zero) & box.size, enter);
  }

  @override
  Widget build(BuildContext context) => SelectionContainer.disabled(
    // 각주 표시는 선택에 들지 않는다(P3-6 R5).
    child: MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => _hover(context, true),
      onExit: (_) => _hover(context, false),
      child: GestureDetector(
        key: ValueKey('fnref-$n'),
        behavior: HitTestBehavior.opaque,
        onTap: () => ctx.onFootnote?.call(n),
        child: Padding(
          padding: const EdgeInsets.only(left: 1),
          child: Transform.translate(
            offset: const Offset(0, -6),
            child: Text('$n', style: TextStyles.sup.copyWith(color: color)),
          ),
        ),
      ),
    ),
  );
}

/// 링크 조각의 누름 — 대상을 들고 있다.
class LinkTap extends TapGestureRecognizer {
  LinkTap(this.target);
  final ObjectAddress target;
}

bool _hasLink(List<Inline> inl) => inl.any(
  (i) => switch (i) {
    LinkInline() => true,
    StyledInline(:final inl) || SizedInline(:final inl) => _hasLink(inl),
    _ => false,
  },
);

bool _hasHan(List<Inline> inl) => inl.any(
  (i) => switch (i) {
    HanInline() => true,
    StyledInline(:final inl) || SizedInline(:final inl) => _hasHan(inl),
    LinkInline(inl: final sub?) || UrlInline(inl: final sub?) => _hasHan(sub),
    _ => false,
  },
);
