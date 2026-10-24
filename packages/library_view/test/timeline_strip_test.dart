import 'package:flutter_test/flutter_test.dart';
import 'package:library_view/library_view.dart';
import 'package:library_view/ui.dart';

/// 띠 그림의 눈금 글(아카이브 P5-2 걸음 6) — 서기 0년이 없는 셈의 자료를 천문 해로 그리는 쪽이 글을 되돌린다.
void main() {
  List<String> ticks(SvgScene s) => [
    for (final t in s.shapes.whereType<SvgText>())
      if (t.y == 74) t.text,
  ];

  testWidgets('눈금 글 — yearLabel이 없으면 해 그대로, 있으면 그 글', (t) async {
    const plain = TimelineStripData(title: '띠', from: -3, to: 3);
    expect(ticks(stripScene(plain, 600)), ['-3', '-1', '1', '3']);

    // 천문 해 −2 ~ 2 = 기원전 3년 ~ 서기 2년. 0은 기원전 1년.
    final bc = TimelineStripData(title: '띠', from: -2, to: 3, yearLabel: (y) => y <= 0 ? '기원전 ${1 - y}' : '$y');
    expect(ticks(stripScene(bc, 600)), ['기원전 3', '기원전 1', '2']);
  });
}
