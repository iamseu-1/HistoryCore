import 'package:flutter/widgets.dart';

import 'svg.dart';
import 'tokens.dart';

/// 목업(design/remake/ha-data.js · ha-*.html)의 아이콘 — 경로 글 그대로. 16px 바깥선, 약 1.5 굵기.
abstract final class Glyphs {
  static const _r = StrokeCap.round;
  static const _j = StrokeJoin.round;

  static const chevronRight = SvgScene([SvgPath('M9 5l7 7-7 7', stroke: currentColor, strokeWidth: 2.4, cap: _r, join: _j)]);
  static const chevronDown = SvgScene([SvgPath('M5 9l7 7 7-7', stroke: currentColor, strokeWidth: 2.4, cap: _r, join: _j)]);
  static const caretDown = SvgScene([SvgPath('M6 9l6 6 6-6', stroke: currentColor, strokeWidth: 2.2, cap: _r, join: _j)]);
  static const folder = SvgScene([
    SvgPath('M3.5 7.2c0-1.1.9-2 2-2h3.7c.5 0 1 .2 1.4.6l1.3 1.4h6.6c1.1 0 2 .9 2 2v8.6c0 1.1-.9 2-2 2h-13c-1.1 0-2-.9-2-2z',
        stroke: currentColor, join: _j),
  ]);
  static const doc = SvgScene([
    SvgPath('M7.5 3.5h6l4.5 4.5v11.5c0 .6-.4 1-1 1H7.5c-.6 0-1-.4-1-1v-15c0-.6.4-1 1-1z', stroke: currentColor, join: _j),
    SvgPath('M13.5 3.7V8h4.3', stroke: currentColor, join: _j),
  ]);
  static const docDashed = SvgScene([
    SvgPath('M7.5 3.5h6l4.5 4.5v11.5c0 .6-.4 1-1 1H7.5c-.6 0-1-.4-1-1v-15c0-.6.4-1 1-1z', stroke: currentColor, join: _j, dash: [2.5, 2]),
  ]);
  static const tray = SvgScene([
    SvgPath('M4 13.5l2.1-6.6c.2-.7.9-1.2 1.6-1.2h8.6c.7 0 1.4.5 1.6 1.2l2.1 6.6v4c0 1.1-.9 2-2 2H6c-1.1 0-2-.9-2-2z', stroke: currentColor, join: _j),
    SvgPath('M4 13.5h4.5l1 2h5l1-2H20', stroke: currentColor, join: _j),
  ]);
  static const book = SvgScene([
    SvgPath('M5 4.5h11.5a2 2 0 0 1 2 2V19.5H7a2 2 0 0 1-2-2z', stroke: currentColor, join: _j),
    SvgPath('M5 17.5a2 2 0 0 1 2-2h11.5', stroke: currentColor),
  ]);
  static const home = SvgScene([
    SvgPath('M4 10.5L12 4l8 6.5V19a1 1 0 0 1-1 1h-4.5v-5.5h-5V20H5a1 1 0 0 1-1-1z', stroke: currentColor, join: _j),
  ]);
  static const clock = SvgScene([
    SvgCircle(12, 12, 8, stroke: currentColor),
    SvgPath('M12 7.5V12l3 2', stroke: currentColor, cap: _r),
  ]);
  static const stats = SvgScene([
    SvgCircle(6, 17, 1.6, fill: currentColor),
    SvgCircle(6, 12.5, 1.6, fill: currentColor),
    SvgCircle(12, 17, 1.6, fill: currentColor),
    SvgCircle(12, 12.5, 1.6, fill: currentColor),
    SvgCircle(12, 8, 1.6, fill: currentColor),
    SvgCircle(18, 17, 1.6, fill: currentColor),
    SvgCircle(18, 12.5, 1.6, fill: currentColor),
    SvgCircle(18, 8, 1.6, fill: currentColor),
    SvgCircle(18, 3.5, 1.6, fill: currentColor),
  ]);
  static const search = SvgScene([
    SvgCircle(10.5, 10.5, 6.5, stroke: currentColor, strokeWidth: 2),
    SvgPath('M15.5 15.5L20 20', stroke: currentColor, strokeWidth: 2, cap: _r),
  ]);
  static const cloudCheck = SvgScene([
    SvgPath('M7 18.5h10.2a4 4 0 0 0 .5-8 5.6 5.6 0 0 0-10.8-1.4A4.7 4.7 0 0 0 7 18.5z', stroke: currentColor, strokeWidth: 1.6, join: _j),
    SvgPath('M9.5 13.6l1.9 1.9 3.6-3.8', stroke: currentColor, strokeWidth: 1.6, cap: _r, join: _j),
  ]);
  static const cloudUp = SvgScene([
    SvgPath('M7 18.5h10.2a4 4 0 0 0 .5-8 5.6 5.6 0 0 0-10.8-1.4A4.7 4.7 0 0 0 7 18.5z', stroke: currentColor, strokeWidth: 1.6, join: _j),
    SvgPath('M12 15.5v-5M9.8 12.6L12 10.4l2.2 2.2', stroke: currentColor, strokeWidth: 1.6, cap: _r, join: _j),
  ]);
  static const edit = SvgScene([SvgPath('M4 20h4L19 9l-4-4L4 16z', stroke: currentColor, strokeWidth: 1.8, join: _j)]);
  static const timeline = SvgScene([SvgPath('M4 7h8M4 12h15M4 17h10', stroke: currentColor, strokeWidth: 1.8, cap: _r)]);
  static const external = SvgScene([SvgPath('M8 16L16 8M9.5 8H16v6.5', stroke: currentColor, strokeWidth: 2, cap: _r, join: _j)]);
  static const menu = SvgScene([SvgPath('M4 7h16M4 12h16M4 17h16', stroke: currentColor, strokeWidth: 1.6, cap: _r)]);
  static const plus = SvgScene([SvgPath('M12 5v14M5 12h14', stroke: currentColor, strokeWidth: 2, cap: _r)]);
  static const close = SvgScene([SvgPath('M6 6l12 12M18 6L6 18', stroke: currentColor, strokeWidth: 1.8, cap: _r)]);
}

class AppIcon extends StatelessWidget {
  const AppIcon(this.glyph, {super.key, this.size = 16, this.color = Ink.icon});
  final SvgScene glyph;
  final double size;
  final Color color;

  @override
  Widget build(BuildContext context) => ExcludeSemantics(
        child: CustomPaint(size: Size.square(size), painter: SvgPainter(glyph, color: color)),
      );
}
