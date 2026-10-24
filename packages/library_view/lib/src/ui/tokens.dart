import 'package:flutter/painting.dart';

/// 무채색 장부(설계 3-1)의 색. 틀(chrome)에는 색이 없다 — 색은 [Vermilion]으로 그래프 안에만.
abstract final class Ink {
  /// 돌빛 바탕(얼굴).
  static const ground = Color(0xFFF4F3F1);

  /// 흰 문서 종이.
  static const surface = Color(0xFFFFFFFF);

  /// 한 단계 바탕(정보 상자 · 자전 견본).
  static const raised = Color(0xFFF6F5F3);

  /// 지금 위치 · 주 동작의 옅은 회색 칠 · 누르는 동안.
  static const active = Color(0xFFE7E6E3);

  /// 올림(hover) — 지금 위치보다 한 단계 옅다.
  static const hover = Color(0xFFECEBE8);

  /// 사이드바 찾기 칸.
  static const findField = Color(0xFFEAE9E6);
  static const findText = Color(0xFF5E5E5E);

  static const border = Color(0xFFE4E3E0);
  static const rule = Color(0xFFEFEEEC);
  static const control = Color(0xFFD4D3D0);

  /// 먹 · 둘째 · 셋째 글자(흰 바탕 · 돌빛 바탕 모두 4.5:1 이상).
  static const text = Color(0xFF0A0A0A);
  static const ink2 = Color(0xFF3D3D3D);
  static const muted = Color(0xFF6B6B6B);
  static const icon = Color(0xFF8C8C8C);

  /// 링크 밑줄 · 없는 문서의 점선.
  static const linkLine = Color(0x470A0A0A);
  static const missingLine = Color(0xFF9A9A9A);

  /// 떠 있는 것(자전 카드 · 잇기 목록)만의 그림자.
  static const floatShadow = [
    BoxShadow(color: Color(0x14000000), spreadRadius: 1),
    BoxShadow(color: Color(0x1A000000), offset: Offset(0, 6), blurRadius: 20),
  ];
}

/// 그래프 한 색 — 그래프 안에만(설계 3-1).
abstract final class Vermilion {
  static const c50 = Color(0xFFFBEEE9);
  static const c100 = Color(0xFFF3C9BD);
  static const c300 = Color(0xFFE58C74);
  static const c500 = Color(0xFFCF4F35);
  static const c700 = Color(0xFF8F2A1B);
}

abstract final class Radii {
  static const paper = 6.0;
  static const control = 5.0;
  static const chip = 4.0;
}

/// 틀 치수(목업 ha-base.css · ha-view.html).
abstract final class Metrics {
  static const sidebar = 256.0;
  static const treeRow = 30.0;
  static const control = 32.0;
  static const topBar = 48.0;
  static const headRow = 56.0;
  static const canvasGutter = 24.0;
  static const infoBox = 300.0;
  static const infoGap = 32.0;
  static const bodyMax = 760.0;
}

enum Breakpoint { phone, tablet, desktop }

/// 640 아래 폰, 1100 아래 태블릿(사이드바가 위 막대로 접힌다), 그 위는 열린 판.
Breakpoint breakpointOf(double width) => width < 640
    ? Breakpoint.phone
    : width < 1100
        ? Breakpoint.tablet
        : Breakpoint.desktop;

/// 게으른 목록이 화면 밖을 얼마나 미리 짓는지. 검사 틀은 크게 바꿔 내용 전부가 렌더 트리에 있게 한다.
abstract final class ArchiveScroll {
  static double cacheExtent = 250;
}
