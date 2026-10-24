import 'package:flutter_test/flutter_test.dart';
import 'package:library_view/library_view.dart';
import 'package:library_view/ui.dart';

import 'support.dart';

/// 띠 그림의 눈금 글(아카이브 P5-2 걸음 6) — 서기 0년이 없는 셈의 자료를 천문 해로 그리는 쪽이 글을 되돌린다.
void main() {
  List<String> ticks(SvgScene s) => [
    for (final t in s.shapes.whereType<SvgText>())
      if (t.y == 74) t.text,
  ];

  /// 해마다 가는 칸(세로 줄)의 수.
  int lines(SvgScene s) => s.shapes.whereType<SvgPath>().where((p) => p.d.endsWith('0v56')).length;

  testWidgets('눈금 글 — yearLabel이 없으면 해 그대로, 있으면 그 글', (t) async {
    const plain = TimelineStripData(title: '띠', from: -3, to: 3);
    expect(ticks(stripScene(plain, 600)), ['-3', '-1', '1', '3']);

    // 천문 해 −2 ~ 2 = 기원전 3년 ~ 서기 2년. 0은 기원전 1년.
    final bc = TimelineStripData(title: '띠', from: -2, to: 3, yearLabel: (y) => y <= 0 ? '기원전 ${1 - y}' : '$y');
    expect(ticks(stripScene(bc, 600)), ['기원전 3', '기원전 1', '2']);
  });

  testWidgets('가는 칸 — 짧은 띠는 해마다, 수백 해 띠는 눈금 글 자리에만(아카이브 P5-2 걸음 7)', (t) async {
    // 표본 띠(1592 ~ 1599 — 여덟 해)는 해마다 그대로.
    expect(lines(stripScene(sampleImjinStrip, 1048)), 8);
    expect(lines(stripScene(const TimelineStripData(title: '띠', from: 1900, to: 1939), 800)), stripYearLinesMost, reason: '$stripYearLinesMost해까지는 해마다');
    expect(lines(stripScene(const TimelineStripData(title: '띠', from: 1900, to: 1940), 800)), lessThan(10), reason: '넘으면 눈금 글 자리에만');
    // 기원전 620 ~ 서기 1(천문 해 −619 ~ 2) — 622해. 눈금 글이 넷이면 가는 칸도 그 넷.
    final long = TimelineStripData(title: '안개 부족 연맹', from: -619, to: 2, yearLabel: (y) => y <= 0 ? '기원전 ${1 - y}' : '$y');
    final scene = stripScene(long, 800);
    expect(ticks(scene), hasLength(4));
    expect(lines(scene), 4);
  });

  testWidgets('아주 긴 범위 — 수억 해 · 64비트 정수를 넘는 폭도 해마다 돌지 않고 눈금 넷(가지 검토)', (t) async {
    // 고치기 전에는 해마다 정수 하나를 담은 목록을 지어 수억 해면 그릴 때마다 멈추거나 메모리가 바닥났다.
    for (final (from, to) in [(1.0, 300000001.0), (-4.6e9, 2.0), (-9e18, 9e18), (-1e300, 1e300)]) {
      final watch = Stopwatch()..start();
      final scene = stripScene(TimelineStripData(title: '띠', from: from, to: to), 800);
      expect(watch.elapsed, lessThan(const Duration(seconds: 1)), reason: '$from ~ $to');
      expect(ticks(scene), hasLength(4), reason: '$from ~ $to');
      expect(lines(scene), 4, reason: '가는 칸도 눈금 글 자리에만 — $from ~ $to');
    }
    expect(ticks(stripScene(const TimelineStripData(title: '띠', from: 1, to: 300000001), 800)), ['1', '75000002', '150000003', '225000004']);
    // 유한하지 않은 축 · 뒤집힌 축은 눈금 없이(던지지 않음).
    for (final (from, to) in [(double.negativeInfinity, 3.0), (double.nan, 3.0), (1.0, double.infinity), (5.0, 1.0)]) {
      final scene = stripScene(TimelineStripData(title: '띠', from: from, to: to), 800);
      expect(ticks(scene), isEmpty, reason: '$from ~ $to');
      expect(lines(scene), 0, reason: '$from ~ $to');
    }
  });
}
