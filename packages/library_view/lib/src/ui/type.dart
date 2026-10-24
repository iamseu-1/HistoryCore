import 'package:flutter/painting.dart';

import 'tokens.dart';

/// 글자(설계 3-2). 프리텐다드 한 벌 — 한자는 명조로 떨어진다(프리텐다드에 한자가 0자).
/// 모든 모양에 CSS와 같은 반-행간을 준다(`height` = 줄 높이 / 크기, 위아래로 고르게).
abstract final class TextStyles {
  static const sans = 'Pretendard';
  static const serif = 'NotoSerifKR';
  static const mono = 'D2Coding';
  static const hanjaFallback = ['NotoSerifKR', 'HanaMinA', 'HanaMinB'];
  static const _tnum = [FontFeature.tabularFigures()];

  static TextStyle of(double size, double line, {FontWeight weight = FontWeight.w400, Color color = Ink.text, bool tabular = false}) =>
      TextStyle(
        fontFamily: sans,
        fontFamilyFallback: hanjaFallback,
        fontSize: size,
        height: line / size,
        leadingDistribution: TextLeadingDistribution.even,
        fontWeight: weight,
        color: color,
        fontFeatures: tabular ? _tnum : null,
      );

  /// 한자 구간 · 원어 · 자전 견본 — 명조, 없는 글자는 花園明朝.
  static TextStyle hanja(double size, {double? line, Color color = Ink.text}) => TextStyle(
        fontFamily: serif,
        fontFamilyFallback: const ['HanaMinA', 'HanaMinB'],
        fontSize: size,
        height: line == null ? null : line / size,
        leadingDistribution: TextLeadingDistribution.even,
        fontWeight: FontWeight.w400,
        color: color,
      );

  /// 원문 · 코드 — D2Coding.
  static TextStyle code(double size, double line, {Color color = Ink.text}) => TextStyle(
        fontFamily: mono,
        fontFamilyFallback: hanjaFallback,
        fontSize: size,
        height: line / size,
        leadingDistribution: TextLeadingDistribution.even,
        color: color,
      );

  static final body = of(16, 28);
  static final h1 = of(28, 36, weight: FontWeight.w700);
  static final h1Phone = of(24, 32, weight: FontWeight.w700);
  static final h2 = of(20, 28, weight: FontWeight.w700);
  static final h3 = of(17, 24, weight: FontWeight.w700);
  static final h4 = of(16, 24, weight: FontWeight.w700);
  static final sectionNo = of(20, 28, weight: FontWeight.w600, color: Ink.muted, tabular: true);
  static final meta = of(13, 20, color: Ink.muted);
  static final small = of(12, 16, color: Ink.muted);
  static final line = of(14, 20);
  static final toc = of(14, 28);
  static final tocNo = of(14, 28, color: Ink.muted, tabular: true);
  static final footnote = of(13.5, 24, color: Ink.ink2);
  static final infoKey = of(13.5, 20, color: Ink.muted);
  static final infoValue = of(13.5, 20);
  static final infoTitle = of(16, 22, weight: FontWeight.w700);
  static final sup = of(11, 11, weight: FontWeight.w600);
  static final sidebar = of(14, 20, color: Ink.ink2);
  static final sidebarOn = of(14, 20, weight: FontWeight.w600);
  static final brand = of(15, 20, weight: FontWeight.w700);
  static final group = of(12, 16, color: Ink.muted);
  static final count = of(12, 16, color: Ink.muted, tabular: true);
  static final crumb = of(13, 20, color: Ink.muted);
  static final action = of(13, 20, color: Ink.ink2);
  static final primary = of(13, 20, weight: FontWeight.w600);
  static final topBarTitle = of(16, 22, weight: FontWeight.w600);
  static final cardTitle = of(18, 24, weight: FontWeight.w700);
}
