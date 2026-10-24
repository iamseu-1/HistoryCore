import 'package:flutter/rendering.dart';
import 'package:path_drawing/path_drawing.dart';

import 'tokens.dart';
import 'type.dart';

/// 아이콘 · 그래프가 같이 쓰는 작은 SVG 모형. 같은 모양을 Flutter로 그리고([SvgPainter]),
/// 같은 글을 쌍둥이 HTML에 낸다([SvgExportable]) — 검사기가 보는 것과 화면이 같은 원본에서 나온다.
class SvgInk {
  const SvgInk(Color this.color) : current = false;
  const SvgInk._current() : color = null, current = true;
  final Color? color;

  /// SVG의 `currentColor` — 그리는 쪽이 준 색.
  final bool current;
}

const currentColor = SvgInk._current();

sealed class SvgShape {
  const SvgShape({this.fill, this.stroke, this.strokeWidth = 1.5});
  final SvgInk? fill;
  final SvgInk? stroke;
  final double strokeWidth;
}

class SvgPath extends SvgShape {
  const SvgPath(this.d, {super.fill, super.stroke, super.strokeWidth, this.cap = StrokeCap.butt, this.join = StrokeJoin.miter, this.dash});
  final String d;
  final StrokeCap cap;
  final StrokeJoin join;
  final List<double>? dash;
}

class SvgCircle extends SvgShape {
  const SvgCircle(this.cx, this.cy, this.r, {super.fill, super.stroke, super.strokeWidth});
  final double cx, cy, r;
}

class SvgRect extends SvgShape {
  const SvgRect(this.x, this.y, this.width, this.height, {this.rx = 0, super.fill, super.stroke, super.strokeWidth});
  final double x, y, width, height, rx;
}

enum SvgAnchor { start, middle, end }

class SvgText extends SvgShape {
  const SvgText(this.x, this.y, this.text, {this.size = 12, super.fill, this.anchor = SvgAnchor.start, this.weight = FontWeight.w400});
  final double x, y, size;
  final String text;
  final SvgAnchor anchor;
  final FontWeight weight;
}

/// 한 장의 그림: viewBox와 모양들.
class SvgScene {
  const SvgScene(this.shapes, {this.viewBox = const Rect.fromLTWH(0, 0, 24, 24)});
  final List<SvgShape> shapes;
  final Rect viewBox;
}

abstract interface class SvgExportable {
  /// 쌍둥이 HTML에 넣을 `<svg>` 글.
  String toSvg(Size size);
}

class SvgPainter extends CustomPainter implements SvgExportable {
  SvgPainter(this.scene, {this.color = Ink.icon});
  final SvgScene scene;
  final Color color;

  static final _paths = <String, Path>{};
  static Path _parse(String d) => _paths.putIfAbsent(d, () => parseSvgPathData(d));

  Color _resolve(SvgInk ink) => ink.current ? color : ink.color!;

  @override
  void paint(Canvas canvas, Size size) {
    final vb = scene.viewBox;
    final scale = (size.width / vb.width) < (size.height / vb.height) ? size.width / vb.width : size.height / vb.height;
    canvas.save();
    canvas.translate((size.width - vb.width * scale) / 2, (size.height - vb.height * scale) / 2);
    canvas.scale(scale);
    canvas.translate(-vb.left, -vb.top);
    for (final s in scene.shapes) {
      if (s is SvgText) {
        _text(canvas, s);
        continue;
      }
      final path = switch (s) {
        SvgPath(:final d) => _parse(d),
        SvgCircle(:final cx, :final cy, :final r) => Path()..addOval(Rect.fromCircle(center: Offset(cx, cy), radius: r)),
        SvgRect(:final x, :final y, :final width, :final height, :final rx) => Path()..addRRect(RRect.fromRectAndRadius(Rect.fromLTWH(x, y, width, height), Radius.circular(rx))),
        SvgText() => throw StateError('글은 위에서 그린다'),
      };
      if (s.fill case final f?) {
        canvas.drawPath(path, Paint()..color = _resolve(f));
      }
      if (s.stroke case final k?) {
        final paint = Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = s.strokeWidth
          ..color = _resolve(k);
        if (s is SvgPath) {
          paint
            ..strokeCap = s.cap
            ..strokeJoin = s.join;
        }
        final dash = s is SvgPath ? s.dash : null;
        canvas.drawPath(dash == null ? path : dashPath(path, dashArray: CircularIntervalList(dash)), paint);
      }
    }
    canvas.restore();
  }

  void _text(Canvas canvas, SvgText s) {
    final tp = TextPainter(
      // 그래프 안의 수는 고정폭(설계 3-4 · 검토 M4).
      text: TextSpan(
        text: s.text,
        style: TextStyles.of(s.size, s.size * 1.2, weight: s.weight, color: _resolve(s.fill ?? currentColor), tabular: true),
      ),
      textDirection: TextDirection.ltr,
    )..layout();
    final dx = switch (s.anchor) {
      SvgAnchor.start => 0.0,
      SvgAnchor.middle => tp.width / 2,
      SvgAnchor.end => tp.width,
    };
    // SVG의 y는 글자 바탕선이다.
    tp.paint(canvas, Offset(s.x - dx, s.y - tp.computeDistanceToActualBaseline(TextBaseline.alphabetic)));
    tp.dispose();
  }

  @override
  bool shouldRepaint(SvgPainter old) => old.scene != scene || old.color != color;

  @override
  String toSvg(Size size) {
    final vb = scene.viewBox;
    final b = StringBuffer(
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="${_n(vb.left)} ${_n(vb.top)} ${_n(vb.width)} ${_n(vb.height)}" '
      'width="${_n(size.width)}" height="${_n(size.height)}" fill="none" style="color:${hexOf(color)}">',
    );
    for (final s in scene.shapes) {
      final common = '${_ink('fill', s.fill)}${_ink('stroke', s.stroke)}${s.stroke == null ? '' : ' stroke-width="${_n(s.strokeWidth)}"'}';
      switch (s) {
        case SvgPath():
          final cap = s.cap == StrokeCap.butt ? '' : ' stroke-linecap="${s.cap.name}"';
          final join = s.join == StrokeJoin.miter ? '' : ' stroke-linejoin="${s.join.name}"';
          final dash = s.dash == null ? '' : ' stroke-dasharray="${s.dash!.map(_n).join(' ')}"';
          b.write('<path d="${s.d}"$common$cap$join$dash/>');
        case SvgCircle():
          b.write('<circle cx="${_n(s.cx)}" cy="${_n(s.cy)}" r="${_n(s.r)}"$common/>');
        case SvgRect():
          b.write('<rect x="${_n(s.x)}" y="${_n(s.y)}" width="${_n(s.width)}" height="${_n(s.height)}"${s.rx == 0 ? '' : ' rx="${_n(s.rx)}"'}$common/>');
        case SvgText():
          b.write(
            '<text x="${_n(s.x)}" y="${_n(s.y)}" font-family="Pretendard" font-size="${_n(s.size)}" font-weight="${s.weight.value}" style="font-variant-numeric:tabular-nums"'
            '${s.anchor == SvgAnchor.start ? '' : ' text-anchor="${s.anchor.name}"'}${_ink('fill', s.fill ?? currentColor)}>${escapeHtml(s.text)}</text>',
          );
      }
    }
    b.write('</svg>');
    return b.toString();
  }

  String _ink(String attr, SvgInk? ink) {
    if (ink == null) return attr == 'fill' ? ' fill="none"' : '';
    if (ink.current) return ' $attr="currentColor"';
    final c = ink.color!;
    final a = (c.a * 255).round();
    return ' $attr="${hexOf(c)}"${a == 255 ? '' : ' $attr-opacity="${_n(c.a)}"'}';
  }
}

String _n(double v) {
  final s = v.toStringAsFixed(3);
  return s.contains('.') ? s.replaceFirst(RegExp(r'\.?0+$'), '') : s;
}

/// `#rrggbb`(투명도는 따로).
String hexOf(Color c) {
  String h(double x) => (x * 255).round().toRadixString(16).padLeft(2, '0');
  return '#${h(c.r)}${h(c.g)}${h(c.b)}';
}

String escapeHtml(String s) => s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
