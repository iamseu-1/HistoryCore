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
}
