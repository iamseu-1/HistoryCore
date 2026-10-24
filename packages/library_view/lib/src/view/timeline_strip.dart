import 'package:flutter/widgets.dart';

import '../ui/dom.dart';
import '../ui/icons.dart';
import '../ui/press.dart';
import '../ui/svg.dart';
import '../ui/tokens.dart';
import '../ui/type.dart';
import 'context.dart';
import 'model.dart';

/// 연표 창구 `strip(항목ID, 범위)`의 답(설계 7-3) — 시대 띠와 사건. 해는 소수(1592.5 = 1592년 한가운데).
///
/// 해는 **그림 축의 해**다 — 축이 고르게 이어지도록 서기 0년이 있는 셈(천문 해: 0 = 기원전 1년)으로 준다. 서기 0년이 없는 셈(−1 다음이 1)의
/// 자료(HistoryTime 창구)는 주는 쪽이 천문 해로 바꾸고, 눈금 글은 [yearLabel]로 되돌린다.
class TimelineStripData {
  const TimelineStripData({required this.title, required this.from, required this.to, this.spans = const [], this.events = const [], this.yearLabel});
  final String title;
  final double from;
  final double to;
  final List<StripSpan> spans;
  final List<StripEvent> events;

  /// 눈금 글 — 축의 정수 해(천문 해)를 글로. 없으면 그 수 그대로(`1592`).
  final String Function(int year)? yearLabel;
}

class StripSpan {
  const StripSpan(this.label, this.from, this.to, {this.strong = false});
  final String label;
  final double from;
  final double to;

  /// 짙은 띠(전쟁 · 침략)와 옅은 띠(휴전 · 협상).
  final bool strong;
}

class StripEvent {
  const StripEvent(this.label, this.at);
  final String label;
  final double at;
}

/// `1592..1598` → (1592, 1598).
(int, int)? parseRange(String? range) {
  final m = RegExp(r'^\s*(-?\d+)\s*\.\.\s*(-?\d+)\s*$').firstMatch(range ?? '');
  return m == null ? null : (int.parse(m.group(1)!), int.parse(m.group(2)!));
}

const stripHeight = 78.0;

/// 띠 한 장의 그림 — 폭 [width]에 맞춰 짓는다(폰에서 글자가 줄지 않게). 해마다 가는 칸, 시대 띠, 사건 점(주홍, R7), 두 해마다 눈금.
SvgScene stripScene(TimelineStripData d, double width) {
  const left = 8.0, right = 18.0;
  final span = (d.to - d.from) <= 0 ? 1.0 : d.to - d.from;
  double x(double year) => left + (year - d.from) / span * (width - left - right);
  double textWidth(String s) {
    final tp = TextPainter(
      text: TextSpan(text: s, style: TextStyles.of(12, 14)),
      textDirection: TextDirection.ltr,
    )..layout();
    final w = tp.width;
    tp.dispose();
    return w;
  }

  final shapes = <SvgShape>[];
  final years = [for (var y = d.from.ceil(); y <= d.to.floor(); y++) y];
  for (final y in years) {
    shapes.add(SvgPath('M${x(y.toDouble()).toStringAsFixed(1)} 0v56', stroke: const SvgInk(Ink.rule), strokeWidth: 1));
  }
  for (final s in d.spans) {
    final a = x(s.from), b = x(s.to);
    shapes.add(SvgRect(a, 6, b - a, 22, rx: 3, fill: SvgInk(s.strong ? Ink.active : const Color(0xFFF2F1EF))));
    if (textWidth(s.label) + 20 < b - a) shapes.add(SvgText(a + 10, 21, s.label, fill: SvgInk(s.strong ? Ink.ink2 : Ink.muted)));
  }
  final events = [...d.events]..sort((a, b) => a.at.compareTo(b.at));
  for (final (i, e) in events.indexed) {
    final cx = x(e.at);
    shapes.add(SvgCircle(cx, 40, 3.5, fill: const SvgInk(Vermilion.c500)));
    final w = textWidth(e.label);
    final next = i + 1 < events.length ? x(events[i + 1].at) : width;
    if (cx + 8 + w + 6 < next && cx + 8 + w < width) {
      shapes.add(SvgText(cx + 8, 44, e.label, fill: const SvgInk(Ink.ink2)));
    } else if (cx + w / 2 > width - 4) {
      shapes.add(SvgText(cx + 4, 56, e.label, fill: const SvgInk(Ink.ink2), anchor: SvgAnchor.end));
    } else {
      shapes.add(SvgText(cx - 6, 56, e.label, fill: const SvgInk(Ink.ink2)));
    }
  }
  final step = years.length > 8 ? (years.length / 4).ceil() : 2;
  for (final (i, y) in years.indexed) {
    if (i % step == 0) shapes.add(SvgText(x(y.toDouble()), 74, d.yearLabel?.call(y) ?? '$y', fill: const SvgInk(Ink.muted)));
  }
  return SvgScene(shapes, viewBox: Rect.fromLTWH(0, 0, width, stripHeight));
}

class TimelineStripPainter extends SvgPainter {
  TimelineStripPainter(TimelineStripData data, double width) : super(stripScene(data, width), color: Ink.ink2);
}

/// 문서 속 연표 띠(설계 7-1). 창구가 답하지 않으면 「제목 · 범위」 자리만(R8).
class TimelineStripView extends StatefulWidget {
  const TimelineStripView(this.block, {super.key, required this.ctx});
  final TimelineStripBlock block;
  final DocContext ctx;

  @override
  State<TimelineStripView> createState() => _TimelineStripViewState();
}

class _TimelineStripViewState extends State<TimelineStripView> {
  late Future<TimelineStripData?> _data;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(TimelineStripView old) {
    super.didUpdateWidget(old);
    if (old.block.entity != widget.block.entity || old.block.range != widget.block.range || old.ctx.strip != widget.ctx.strip) _load();
  }

  void _load() {
    final strip = widget.ctx.strip;
    _data = strip == null ? Future.value(null) : strip(widget.block.entity, parseRange(widget.block.range)).catchError((_) => null);
  }

  @override
  Widget build(BuildContext context) {
    final range = parseRange(widget.block.range);
    final label = range == null ? '연표' : '연표 · ${range.$1}–${range.$2}';
    return DomTag(
      'figure',
      attrs: const {'class': 'timeline'},
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              // 폰에서 긴 기간 이름표가 「HistoryTime에서 열기」를 밀어내지 않게 — 줄여 보인다(P3-6에서 잡힌 넘침).
              Flexible(
                child: Text(
                  label,
                  style: TextStyles.of(13, 20, weight: FontWeight.w600),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  softWrap: false,
                ),
              ),
              const SizedBox(width: 8),
              const Spacer(),
              Press(
                link: true,
                label: widget.ctx.stripOpenLabel,
                onTap: widget.ctx.onTimeline == null ? null : () => widget.ctx.onTimeline!(widget.block.entity),
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(widget.ctx.stripOpenLabel, style: TextStyles.action),
                    const SizedBox(width: 4),
                    const AppIcon(Glyphs.external, size: 12, color: Ink.ink2),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          FutureBuilder<TimelineStripData?>(
            future: _data,
            builder: (context, snap) {
              final data = snap.data;
              if (data == null) {
                return Text(snap.connectionState == ConnectionState.done ? widget.ctx.stripMissing : ' ', style: TextStyles.meta);
              }
              // 아래 해 눈금 글자가 다음 문단 제목에 붙지 않게 조금 띄운다.
              return Padding(
                padding: const EdgeInsets.only(bottom: 10),
                child: LayoutBuilder(
                  builder: (context, c) => CustomPaint(key: const ValueKey('timeline-strip'), size: Size(c.maxWidth, stripHeight), painter: TimelineStripPainter(data, c.maxWidth)),
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}
