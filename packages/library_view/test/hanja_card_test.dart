import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:library_view/library_view.dart';
import 'package:library_view/ui.dart';

const _hak = HanjaCard(
  char: '鶴',
  readings: '학 학',
  radical: '鳥',
  radicalStrokes: 10,
  strokes: 21,
  source: '《唐韻》下各切《集韻》《韻會》曷各切，𠀀音涸。鳥名。',
  gloss: '두루미. 몸이 크고 목과 다리가 길며, 흰 몸에 머리 꼭대기가 붉은 새. 오래 산다 하여 장수의 상징으로 쓰고, 그림과 시에 자주 나온다.',
  status: '모델 번역 · 검토 전',
);

Future<void> _pump(WidgetTester t, Widget card, {double width = 380}) async {
  t.view.physicalSize = Size(width, 700);
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.reset);
  await t.pumpWidget(
    WidgetsApp(
      color: Ink.text,
      builder: (_, _) => ColoredBox(
        color: Ink.ground,
        child: Align(alignment: Alignment.bottomCenter, child: card),
      ),
    ),
  );
  await t.pump();
}

List<String> _texts(WidgetTester t) => [for (final e in find.byType(Text).evaluate()) (e.widget as Text).data ?? (e.widget as Text).textSpan!.toPlainText()];

void main() {
  testWidgets('자료가 없으면 글자와 「자전에서 열기」만 — 누르면 onOpen', (t) async {
    var opened = 0;
    await _pump(t, HanjaCardView(char: '鶴', onOpen: () => opened++));
    expect(_texts(t), ['鶴', '자전에서 열기']);
    expect(find.textContaining('획'), findsNothing);
    expect(find.textContaining('표본'), findsNothing);
    await t.tap(find.text('자전에서 열기'));
    expect(opened, 1);
    expect(t.takeException(), isNull);
  });

  testWidgets('[missing]을 주면 글자 곁에 그 한 줄(아카이브 꼴)', (t) async {
    await _pump(t, HanjaCardView(char: '鶴', onOpen: () {}, missing: '표본 자전에 없는 글자'));
    expect(_texts(t), ['鶴', '鶴', '표본 자전에 없는 글자', '자전에서 열기']);
  });

  testWidgets('자료가 있으면 음훈 · 부수와 획 · 원문 · 번역 · 상태', (t) async {
    await _pump(t, HanjaCardView(char: '鶴', card: _hak, onOpen: () {}));
    expect(find.text('학 학'), findsOneWidget);
    expect(find.text('鳥부 10획 · 총 21획'), findsOneWidget);
    expect(find.text(_hak.source), findsOneWidget);
    expect(find.text(_hak.gloss), findsOneWidget);
    expect(find.text('모델 번역 · 검토 전'), findsOneWidget);
    expect(t.getSize(find.byType(HanjaCardView)).width, 320);
    expect(t.takeException(), isNull);
  });

  testWidgets('폰 아래 시트(폭 없음 · docked) — 380 폭에서 넘치지 않고 긴 글은 줄여 보인다', (t) async {
    for (final card in [null, _hak]) {
      await _pump(t, HanjaCardView(char: '𠀀', card: card, onOpen: () {}, width: null, docked: true));
      expect(t.takeException(), isNull);
      expect(t.getSize(find.byType(HanjaCardView)).width, 380);
    }
    final box = t.widget<DecoratedBox>(find.descendant(of: find.byType(HanjaCardView), matching: find.byType(DecoratedBox)).first).decoration as BoxDecoration;
    expect(box.borderRadius, const BorderRadius.vertical(top: Radius.circular(Radii.paper)));
  });

  test('「자전에서 열기」의 딥링크 — historyarchive://kangxi/<글자>, 퍼센트로 싸고 아카이브가 푸는 그대로 돌아온다', () {
    expect(kangxiUri('鶴').toString(), 'historyarchive://kangxi/%E9%B6%B4');
    for (final c in ['鶴', '𠀀', '學']) {
      final back = Uri.parse(kangxiUri(c).toString());
      expect((back.scheme, back.host, back.pathSegments.single), ('historyarchive', 'kangxi', c), reason: c);
    }
  });
}
