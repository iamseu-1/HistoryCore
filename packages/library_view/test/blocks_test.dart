import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:library_view/library_view.dart';
import 'package:library_view/ui.dart';

import 'support.dart';

// 아카이브 test/view/blocks_test.dart에서 옮겨 왔다(P5 걸음 4) — 표본 모델 · 표본 띠는 support.dart.
DocModel _fixture(String path) => fixture(path);

DocModel _model(List<Map<String, Object?>> blocks, {List<Map<String, Object?>> footnotes = const []}) => DocModel.parse({'version': 1, 'blocks': blocks, 'footnotes': footnotes});

Map<String, Object?> _para(List<Map<String, Object?>> inl) => {'t': 'para', 'inl': inl};
Map<String, Object?> _text(String s) => {'t': 'text', 's': s};
Map<String, Object?> _link(String key, {String ns = '문서'}) => {
  't': 'link',
  'target': {'ns': ns, 'key': key, 'anchor': null},
  'inl': null,
};

Future<void> _pump(WidgetTester t, List<Block> blocks, DocContext ctx, {double width = 1000}) async {
  t.view.physicalSize = Size(width, 900);
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.reset);
  await t.pumpWidget(
    WidgetsApp(
      color: Ink.text,
      builder: (_, _) => ColoredBox(
        color: Ink.surface,
        child: CustomScrollView(
          slivers: [SliverDocBlocks(blocks: blocks, context: ctx)],
        ),
      ),
    ),
  );
  await t.pump();
}

/// 선택 영역 안에 블록들을 펼친다(끌어 선택 · 복사 검사).
Future<void> _pumpSelectable(WidgetTester t, List<Block> blocks, DocContext ctx) async {
  t.view.physicalSize = const Size(1000, 900);
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.reset);
  await t.pumpWidget(
    WidgetsApp(
      color: Ink.text,
      builder: (_, _) => ColoredBox(
        color: Ink.surface,
        child: Overlay.wrap(
          child: SelectableRegion(
            selectionControls: emptyTextSelectionControls,
            child: CustomScrollView(
              slivers: [SliverDocBlocks(blocks: blocks, context: ctx)],
            ),
          ),
        ),
      ),
    ),
  );
  await t.pump();
}

/// 마우스로 [from]에서 [to]까지 끌어 선택하고 Ctrl C — 클립보드에 간 글.
Future<String> _dragCopy(WidgetTester t, Offset from, Offset to) async {
  final copied = <String>[];
  t.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
    if (call.method == 'Clipboard.setData') copied.add((call.arguments as Map)['text'] as String);
    return null;
  });
  addTearDown(() => t.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, null));
  final g = await t.startGesture(from, kind: PointerDeviceKind.mouse);
  await t.pump();
  await g.moveTo(to);
  await t.pump();
  await g.up();
  await t.pumpAndSettle();
  await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  await t.sendKeyEvent(LogicalKeyboardKey.keyC);
  await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  await t.pumpAndSettle();
  return copied.isEmpty ? '' : copied.last;
}

Map<String, Object?> _cell(String s) => {
  'col': 1,
  'row': 1,
  'align': 'start',
  'inl': s.isEmpty ? [] : [_text(s)],
};

/// 그 글이 든 글 조각의 모양.
TextStyle? _styleOf(WidgetTester t, String text) {
  TextStyle? found;
  for (final p in t.renderObjectList<RenderParagraph>(find.byType(RichText))) {
    p.text.visitChildren((span) {
      if (span is TextSpan && span.text == text) found = span.style;
      return found == null;
    });
    if (found != null) break;
  }
  return found;
}

String _allText(WidgetTester t) => t.renderObjectList<RenderParagraph>(find.byType(RichText)).map((p) => p.text.toPlainText()).join('\n');

void main() {
  testWidgets('문단 제목은 번호와 「편집」 — 누르면 그 문단을 편집', (t) async {
    int? edited;
    await _pump(t, _fixture('doc/임진왜란').blocks, DocContext(onEditSection: (s) => edited = s));
    expect(find.text('2.1'), findsOneWidget);
    expect(find.text('주요 해전'), findsOneWidget);
    await t.tap(find.descendant(of: find.byKey(const ValueKey('heading-2')), matching: find.text('편집')));
    expect(edited, 2);
  });

  testWidgets('있는 링크는 실선 밑줄, 없는 링크는 셋째 글자 점선', (t) async {
    await _pump(
      t,
      _model([
        _para([_link('정유재란'), _text(' · '), _link('노량 해전')]),
      ]).blocks,
      DocContext(exists: (a) => a.key != '노량 해전'),
    );
    final there = _styleOf(t, '정유재란')!;
    expect(there.decoration, TextDecoration.underline);
    expect(there.decorationStyle, isNot(TextDecorationStyle.dotted));
    expect(there.color, Ink.text);
    final missing = _styleOf(t, '노량 해전')!;
    expect(missing.decorationStyle, TextDecorationStyle.dotted);
    expect(missing.color, Ink.muted);
  });

  testWidgets('링크를 누르면 그 주소로, 바깥 링크는 그 주소로', (t) async {
    ObjectAddress? went;
    String? url;
    await _pump(
      t,
      _model([
        _para([_link('정유재란')]),
        _para([
          {
            't': 'url',
            'href': 'https://db.history.go.kr',
            'inl': [_text('한국사데이터베이스')],
          },
        ]),
      ]).blocks,
      DocContext(onLink: (a) => went = a, onUrl: (h) => url = h),
    );
    await t.tap(find.text('정유재란', findRichText: true));
    expect(went?.key, '정유재란');
    await t.tap(find.text('한국사데이터베이스', findRichText: true));
    expect(url, 'https://db.history.go.kr');
  });

  testWidgets('각주 위첨자를 누르면 그 각주로', (t) async {
    int? footnote;
    final m = _model(
      [
        _para([
          _text('전쟁이다.'),
          {'t': 'fn', 'n': 1},
        ]),
        {'t': 'footnotes'},
      ],
      footnotes: [
        {
          'n': 1,
          'name': null,
          'inl': [_text('임진전쟁이라고도 부른다.')],
        },
      ],
    );
    await _pump(t, m.blocks, DocContext(footnotes: m.footnotes, onFootnote: (n) => footnote = n));
    await t.tap(find.byKey(const ValueKey('fnref-1')));
    expect(footnote, 1);
    expect(find.text('임진전쟁이라고도 부른다.', findRichText: true), findsOneWidget);
  });

  testWidgets('한자 구간은 명조, 루비는 위에 읽기', (t) async {
    await _pump(
      t,
      _model([
        _para([
          _text('학익진('),
          {'t': 'han', 's': '鶴翼陣'},
          _text(')'),
        ]),
        _para([
          {'t': 'ruby', 'base': '懲毖錄', 'ruby': '징비록'},
        ]),
      ]).blocks,
      DocContext(),
    );
    expect(_styleOf(t, '鶴')?.fontFamily, 'NotoSerifKR');
    expect(find.text('징비록'), findsOneWidget);
    expect(find.text('懲毖錄'), findsOneWidget);
  });

  testWidgets('한자에 올리면 그 글자를 알린다', (t) async {
    final seen = <String>[];
    await _pump(
      t,
      _model([
        _para([
          {'t': 'han', 's': '鶴翼'},
        ]),
      ]).blocks,
      DocContext(onHanja: (c, _) => seen.add(c)),
    );
    final p = t.renderObject<RenderParagraph>(find.byType(RichText).first);
    final box = p.getBoxesForSelection(const TextSelection(baseOffset: 0, extentOffset: 1)).first;
    final mouse = await t.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: Offset.zero);
    await mouse.moveTo(p.localToGlobal(box.toRect().center));
    await t.pump();
    expect(seen, ['鶴']);
  });

  testWidgets('문단 앞뒤의 줄바꿈(분류 줄이 남긴 것)은 그리지 않는다', (t) async {
    await _pump(
      t,
      _model([
        _para([
          {'t': 'br'},
          _text('삼국의 표본 문서다.'),
          {'t': 'br'},
        ]),
      ]).blocks,
      DocContext(),
    );
    final p = t.renderObject<RenderParagraph>(find.byType(RichText).first);
    expect(p.text.toPlainText(), '삼국의 표본 문서다.');
  });

  testWidgets('표 — 칸 글 · 첫 줄이 모두 굵으면 머리 줄, 가로 합친 칸', (t) async {
    await _pump(t, _fixture('doc/임진왜란').blocks, DocContext());
    expect(find.text('날짜(음력)', findRichText: true), findsOneWidget);
    expect(_styleOf(t, '날짜(음력)')?.color, Ink.muted);
    expect(find.text('1598. 11. 19.', findRichText: true), findsOneWidget);
    await _pump(
      t,
      _model([
        {
          't': 'table',
          'caption': null,
          'rows': [
            [
              {
                'col': 2,
                'row': 1,
                'align': 'center',
                'inl': [_text('합친 칸')],
              },
            ],
            [
              {
                'col': 1,
                'row': 1,
                'align': 'start',
                'inl': [_text('가')],
              },
              {
                'col': 1,
                'row': 1,
                'align': 'end',
                'inl': [_text('나')],
              },
            ],
          ],
        },
      ]).blocks,
      DocContext(),
    );
    expect(t.getSize(find.text('합친 칸', findRichText: true)).width, lessThan(t.getSize(find.byKey(const ValueKey('cell-0-0'))).width));
    expect(t.getSize(find.byKey(const ValueKey('cell-0-0'))).width, greaterThan(t.getSize(find.byKey(const ValueKey('cell-1-0'))).width));
  });

  testWidgets('접기는 눌러 펼치고, 코드는 D2Coding', (t) async {
    await _pump(
      t,
      _model([
        {
          't': 'fold',
          'summary': '더 보기',
          'blocks': [
            _para([_text('숨은 글')]),
          ],
        },
        {'t': 'code', 'lang': 'rust', 'text': 'fn main() {}'},
      ]).blocks,
      DocContext(),
    );
    expect(find.text('숨은 글', findRichText: true), findsNothing);
    await t.tap(find.text('더 보기'));
    await t.pump();
    expect(find.text('숨은 글', findRichText: true), findsOneWidget);
    expect(_styleOf(t, 'fn main() {}')?.fontFamily, 'D2Coding');
  });

  testWidgets('없는 틀 · 대문 매크로 자리 · 모르는 블록은 글로 보인다', (t) async {
    await _pump(
      t,
      _model([
        {
          't': 'template',
          'target': {'ns': '틀', 'key': '사라진 틀', 'anchor': null},
          'status': 'missing',
          'blocks': [],
        },
        {'t': 'front', 'name': '오늘의 문서', 'arg': null},
        {'t': 'hologram', 'text': '빛나는 글'},
        {'t': 'unsupported', 'kind': 'html', 'text': '<b>x</b>'},
      ]).blocks,
      DocContext(),
    );
    final all = _allText(t);
    expect(all, contains('없는 틀 · 틀:사라진 틀'));
    expect(all, contains('오늘의 문서'));
    expect(all, contains('빛나는 글'));
    expect(all, contains('HTML'));
  });

  testWidgets('목차는 문단 번호 · 제목, 누르면 그 문단으로', (t) async {
    int? went;
    final m = _fixture('doc/임진왜란');
    await t.pumpWidget(
      WidgetsApp(
        color: Ink.text,
        builder: (_, _) => Align(
          alignment: Alignment.topLeft,
          child: TocView(sections: m.sections, onSection: (i) => went = i),
        ),
      ),
    );
    expect(find.text('목차'), findsOneWidget);
    expect(find.text('2.1'), findsOneWidget);
    await t.tap(find.text('결과'));
    expect(went, 3);
  });

  test('제목 옆 원어는 첫 문단의 「제목(한자)」', () {
    expect(originalTitle(_fixture('doc/임진왜란'), '임진왜란'), '壬辰倭亂');
    expect(originalTitle(_fixture('doc/이순신'), '이순신'), '李舜臣');
    expect(originalTitle(_fixture('doc/대도'), '대도'), '大都');
    expect(originalTitle(_fixture('doc/임진왜란'), '정유재란'), isNull);
    expect(originalTitle(_fixture('doc/삼국 표본 001'), '삼국 표본 001'), isNull);
  });

  test('정보 상자는 「틀:…정보」 속 첫 표 — 제목과 다섯 줄', () {
    final box = infoBoxOf(_fixture('doc/임진왜란'))!;
    expect(box.blockIndex, 0);
    expect(box.title, '임진왜란');
    expect(box.subtitle, '壬辰倭亂');
    expect(box.rows.map((r) => r.key), ['기간', '장소', '교전', '결과', '연표']);
    expect(infoBoxOf(_fixture('doc/이순신')), isNull);
  });

  testWidgets('연표 띠 — 창구가 없으면 「제목 · 범위」 자리, 있으면 사건 점(주홍)', (t) async {
    const block = TimelineStripBlock('01JB2X7K9M', '1592..1598');
    await _pump(t, [block], DocContext());
    expect(find.text('연표 · 1592–1598'), findsOneWidget);
    expect(_allText(t), contains('HistoryTime'));
    await _pump(t, [block], DocContext(strip: (_, _) async => sampleImjinStrip));
    await t.pump();
    final painter = t.widget<CustomPaint>(find.byKey(const ValueKey('timeline-strip'))).painter! as TimelineStripPainter;
    expect(painter.toSvg(const Size(1048, 78)), contains('#cf4f35'));
  });

  testWidgets('5,000 문단 문서도 보이는 블록만 짓는다', (t) async {
    final blocks = [
      for (var i = 0; i < 5000; i++) ParaBlock([TextInline('문단 $i')]),
    ];
    await _pump(t, blocks, DocContext());
    expect(find.byType(ParaView).evaluate().length, lessThan(120));
    expect(find.text('문단 0', findRichText: true), findsOneWidget);
  });

  testWidgets('한자가 수백 자인 문단에서도 올린 그 글자를 안다(최종 검토 #7)', (t) async {
    final seen = <String>[];
    final han = String.fromCharCodes([for (var i = 0; i < 400; i++) 0x4E00 + i]);
    await _pump(
      t,
      _model([
        _para([
          {'t': 'han', 's': han},
        ]),
      ]).blocks,
      DocContext(onHanja: (c, _) => seen.add(c)),
    );
    final p = t.renderObject<RenderParagraph>(find.byType(RichText).first);
    final box = p.getBoxesForSelection(const TextSelection(baseOffset: 300, extentOffset: 301)).first;
    final mouse = await t.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: Offset.zero);
    await mouse.moveTo(p.localToGlobal(box.toRect().center));
    await t.pump();
    expect(seen, [String.fromCharCode(0x4E00 + 300)]);
  });

  testWidgets('표 복사 — 빈 칸도 탭을 지키고 줄마다 줄바꿈, 캡션은 제 줄(검토 I3) · 목록 표시와 각주 번호는 들지 않음(M5)', (t) async {
    final table = {
      't': 'table',
      'caption': [_text('해전 표')],
      'rows': [
        [_cell('a'), _cell(''), _cell('c')],
        [_cell('d'), _cell('e'), _cell('')],
      ],
    };
    final list = {
      't': 'list',
      'style': 'decimal',
      'start': 1,
      'items': [
        [
          _para([_text('첫째')]),
        ],
        [
          _para([_text('둘째')]),
        ],
      ],
    };
    await _pumpSelectable(
      t,
      _model(
        [
          table,
          list,
          {'t': 'footnotes'},
        ],
        footnotes: [
          {
            'n': 1,
            'name': null,
            'inl': [_text('첫 각주')],
          },
          {
            'n': 2,
            'name': null,
            'inl': [_text('둘째 각주')],
          },
        ],
      ).blocks,
      DocContext(
        footnotes: [
          const Footnote(1, null, [TextInline('첫 각주')]),
          const Footnote(2, null, [TextInline('둘째 각주')]),
        ],
      ),
    );
    final text = await _dragCopy(t, t.getTopLeft(find.text('해전 표', findRichText: true)) + const Offset(1, 4), t.getBottomRight(find.text('둘째 각주', findRichText: true)) - const Offset(1, 1));
    expect(text, '해전 표\na\t\tc\nd\te\n첫째\n둘째\n첫 각주\n둘째 각주\n');
  });

  testWidgets('폰의 넓은 표는 칸을 짓누르지 않고 가로로 밀린다(최종 검토 #9)', (t) async {
    final wide = {
      't': 'table',
      'caption': null,
      'rows': [
        for (var r = 0; r < 2; r++)
          [
            for (var c = 0; c < 9; c++)
              {
                'col': 1,
                'row': 1,
                'align': 'start',
                'inl': [_text('칸 $r-$c')],
              },
          ],
      ],
    };
    await _pump(t, _model([wide]).blocks, DocContext(), width: 390);
    expect(t.getSize(find.byKey(const ValueKey('cell-1-0'))).width, greaterThanOrEqualTo(72));
    expect(t.takeException(), isNull);
    final merged = {
      't': 'table',
      'caption': null,
      'rows': [
        [
          {
            'col': 6,
            'row': 1,
            'align': 'center',
            'inl': [_text('합친 칸')],
          },
        ],
        [
          for (var c = 0; c < 6; c++)
            {
              'col': 1,
              'row': 1,
              'align': 'start',
              'inl': [_text('칸 $c')],
            },
        ],
      ],
    };
    await _pump(t, _model([merged]).blocks, DocContext(), width: 390);
    expect(t.getSize(find.byKey(const ValueKey('cell-1-0'))).width, greaterThanOrEqualTo(72));
  });
}
