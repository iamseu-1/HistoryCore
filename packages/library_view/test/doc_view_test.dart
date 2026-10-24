import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:library_view/library_view.dart';
import 'package:library_view/ui.dart';

import 'support.dart';

/// HistoryTime 상세 칸 — 넓은 화면(1200) 속 380 폭.
const _column = 380.0;

Future<void> _pump(WidgetTester t, Widget child, {double screen = 1200, double height = 900}) async {
  t.view.physicalSize = Size(screen, height);
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.reset);
  await t.pumpWidget(
    WidgetsApp(
      color: Ink.text,
      builder: (_, _) => ColoredBox(color: Ink.surface, child: child),
    ),
  );
  await t.pump();
}

/// 게으른 목록이 문서 전부를 짓게 — 넘침 검사가 화면 밖 블록까지 본다.
void _buildEverything() {
  final before = ArchiveScroll.cacheExtent;
  ArchiveScroll.cacheExtent = 100000;
  addTearDown(() => ArchiveScroll.cacheExtent = before);
}

/// 그 글이 든 문단을 화면에 들인 뒤 그 글 조각을 누른다.
Future<void> _tapText(WidgetTester t, String s) async {
  await t.ensureVisible(find.textContaining(s, findRichText: true, skipOffstage: false).first);
  await t.pumpAndSettle();
  await t.tapOnText(find.textRange.ofSubstring(s));
  await t.pump();
}

/// 가로로 미는 굴림(넓은 표 · 코드) 속인가 — 그 글은 칸보다 넓어도 된다.
bool _inSideways(Element e) {
  var found = false;
  e.visitAncestorElements((a) {
    final w = a.widget;
    if (w is Scrollable && (w.axisDirection == AxisDirection.right || w.axisDirection == AxisDirection.left)) found = true;
    return !found;
  });
  return found;
}

/// 위젯이 화면(높이 [height]) 안에 보이는가.
bool _onScreen(WidgetTester t, Finder f, {double height = 900}) {
  final r = t.getRect(f);
  return r.bottom > 0 && r.top < height;
}

void main() {
  testWidgets('표본 모델 모두 — 380 폭에서 넘침 없이 그린다(띠 자료가 있을 때와 없을 때)', (t) async {
    _buildEverything();
    final all = allFixtures();
    expect(all.length, greaterThanOrEqualTo(18));
    for (final (name, json) in all) {
      for (final strip in [null, (String _, (int, int)? _) async => sampleImjinStrip]) {
        await _pump(
          t,
          LibraryDocView(key: ValueKey('$name-${strip != null}'), model: json, title: name.split('/').last, updated: DateTime.utc(2026, 10, 24, 3), width: _column, ctx: DocContext(strip: strip)),
        );
        await t.pump();
        expect(t.takeException(), isNull, reason: name);
        expect(t.getSize(find.byType(CustomScrollView)).width, _column, reason: name);
        // 글 상자는 칸 밖으로 나가지 않는다 — 가로로 밀리는 표 · 코드 속 글만 빼고.
        for (final e in find.byType(RichText, skipOffstage: false).evaluate()) {
          if (_inSideways(e)) continue;
          final box = e.renderObject! as RenderBox;
          if (!box.hasSize || !box.attached) continue;
          final right = box.localToGlobal(Offset(box.size.width, 0)).dx;
          expect(right, lessThanOrEqualTo(_column + 0.5), reason: '$name · ${(e.widget as RichText).text.toPlainText()}');
        }
      }
    }
  });

  testWidgets('머리 · 정보 상자 · 본문 · 각주 — 380 폭에서는 정보 상자가 본문 위에, 넓으면 오른쪽에', (t) async {
    _buildEverything();
    final json = fixtureJson('doc/임진왜란');
    await _pump(t, LibraryDocView(model: json, title: '임진왜란', updated: DateTime(2026, 10, 24), width: _column));
    expect(find.text('임진왜란'), findsWidgets);
    expect(find.text('壬辰倭亂'), findsWidgets);
    expect(find.text('2026년 10월 24일 고침'), findsOneWidget);
    expect(find.byType(InfoBoxView), findsOneWidget);
    final info = t.getRect(find.byType(InfoBoxView));
    expect(info.left, 16);
    expect(info.width, _column - 32);
    final firstHeading = t.getRect(find.byKey(const ValueKey('heading-0')));
    expect(firstHeading.top, greaterThan(info.bottom));
    // 정보 상자의 틀 블록은 본문에서 뺀다 — 열쇠 「기간」은 한 번만.
    expect(find.text('기간', skipOffstage: false), findsOneWidget);
    // 각주는 문서 끝에.
    expect(find.byType(FootnotesView, skipOffstage: false), findsOneWidget);

    await _pump(t, LibraryDocView(model: json, title: '임진왜란', width: 1000));
    final wide = t.getRect(find.byType(InfoBoxView));
    expect(wide.width, Metrics.infoBox);
    expect(wide.right, closeTo(1000 - 16, 0.5));
    expect(t.getRect(find.byKey(const ValueKey('float-0'))).right, lessThanOrEqualTo(wide.left - Metrics.infoGap + 0.5));
    expect(t.takeException(), isNull);
  });

  testWidgets('고친 날을 모르면 그 줄을 빼고, head: false면 본문부터', (t) async {
    await _pump(t, LibraryDocView(model: fixtureJson('doc/이순신'), title: '이순신', width: _column));
    expect(find.textContaining('고침'), findsNothing);
    expect(find.byType(DocHeadView), findsOneWidget);
    await _pump(t, LibraryDocView(model: fixtureJson('doc/이순신'), title: '이순신', width: _column, head: false));
    expect(find.byType(DocHeadView), findsNothing);
    expect(t.getRect(find.byType(BlockView).first).top, 20);
  });

  testWidgets('[각주]가 없는 문서는 끝에 각주를 둔다', (t) async {
    final json = {
      'version': 1,
      'blocks': [
        {
          't': 'para',
          'inl': [
            {'t': 'text', 's': '전쟁이다.'},
            {'t': 'fn', 'n': 1},
          ],
        },
      ],
      'footnotes': [
        {
          'n': 1,
          'name': null,
          'inl': [
            {'t': 'text', 's': '끝에 붙은 각주'},
          ],
        },
      ],
    };
    await _pump(t, LibraryDocView(model: json, title: '각주', width: _column));
    expect(find.byType(FootnotesView, skipOffstage: false), findsOneWidget);
    expect(find.text('끝에 붙은 각주', findRichText: true, skipOffstage: false), findsOneWidget);
  });

  testWidgets('문서 링크 · 연표 링크 · 바깥 주소 · 한자 · 분류 · 띠 「HistoryTime에서 열기」 누름은 콜백으로 나간다', (t) async {
    _buildEverything();
    final links = <ObjectAddress>[];
    final timelines = <String>[];
    final urls = <String>[];
    final hanja = <String>[];
    final ctx = DocContext(
      exists: (a) => a.key != '없는 문서' && a.key != '없는 분류',
      onLink: links.add,
      onTimeline: timelines.add,
      onUrl: urls.add,
      onHanjaTap: hanja.add,
    );
    await _pump(t, LibraryDocView(model: fixtureJson('all_kinds'), title: '모든 꼴', width: _column, ctx: ctx));

    await _tapText(t, '정유재란');
    expect(links.last, const ObjectAddress('문서', '정유재란'));
    await _tapText(t, '없는 링크');
    expect(links.last, const ObjectAddress('문서', '없는 문서', anchor: '끝'));
    await _tapText(t, '연표 링크');
    expect(timelines.last, '01JB2X7K9M');
    await _tapText(t, '한국사데이터베이스');
    expect(urls.last, 'https://db.history.go.kr');
    await _tapText(t, '鶴');
    expect(hanja.last, '鶴');
    await _tapText(t, '𠀀');
    expect(hanja.last, '𠀀');
    await _tapText(t, '鳴');
    expect(hanja.last, '鳴');

    // 분류 줄 — 있는 분류는 실선, 없는 분류는 점선. 누르면 「분류:」 주소로.
    await t.ensureVisible(find.byKey(const ValueKey('category-표본'), skipOffstage: false));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const ValueKey('category-표본')));
    expect(links.last, const ObjectAddress('분류', '표본'));

    // 문서 속 연표 띠의 「HistoryTime에서 열기」.
    final open = find.text('HistoryTime에서 열기', skipOffstage: false).first;
    await t.ensureVisible(open);
    await t.pumpAndSettle();
    await t.tap(open);
    expect(timelines.last, '01JB2X7K9M');
    expect(t.takeException(), isNull);
  });

  testWidgets('한자에 올리기를 받는 앱에는 글자와 그 자리를 알린다', (t) async {
    final seen = <(String, Rect)>[];
    final json = {
      'version': 1,
      'blocks': [
        {
          't': 'para',
          'inl': [
            {'t': 'han', 's': '鶴翼'},
          ],
        },
      ],
    };
    await _pump(t, LibraryDocView(model: json, title: '한자', width: _column, head: false, ctx: DocContext(onHanja: (c, r) => seen.add((c, r)))));
    final p = t.renderObject<RenderParagraph>(find.byType(RichText).first);
    final box = p.getBoxesForSelection(const TextSelection(baseOffset: 1, extentOffset: 2)).first;
    final mouse = await t.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: Offset.zero);
    await mouse.moveTo(p.localToGlobal(box.toRect().center));
    await t.pump();
    expect(seen.single.$1, '翼');
    expect(seen.single.$2.left, lessThan(_column));
  });

  testWidgets('연표 띠 — 자료가 없으면 「제목 · 범위」 자리, 있으면 칸 폭의 그림', (t) async {
    _buildEverything();
    await _pump(t, LibraryDocView(model: fixtureJson('all_kinds'), title: '모든 꼴', width: _column));
    expect(find.text('연표 · 1592–1598', skipOffstage: false), findsOneWidget);
    expect(find.text('연표', skipOffstage: false), findsOneWidget);
    expect(find.text('HistoryTime이 이 기기에 있으면 이 자리에 사건이 보입니다.', skipOffstage: false), findsNWidgets(2));
    expect(find.byKey(const ValueKey('timeline-strip'), skipOffstage: false), findsNothing);

    final asked = <(String, (int, int)?)>[];
    await _pump(
      t,
      LibraryDocView(
        model: fixtureJson('all_kinds'),
        title: '모든 꼴',
        width: _column,
        ctx: DocContext(
          strip: (entity, range) async {
            asked.add((entity, range));
            return entity == '01JB2X7K9M' ? sampleImjinStrip : null;
          },
        ),
      ),
    );
    await t.pump();
    expect(asked, [('01JB2X7K9M', (1592, 1598)), ('01JB2X7K9N', null)]);
    expect(find.byKey(const ValueKey('timeline-strip'), skipOffstage: false), findsOneWidget);
    expect(t.getSize(find.byKey(const ValueKey('timeline-strip'), skipOffstage: false)).width, _column - 32);
    expect(find.text('HistoryTime이 이 기기에 있으면 이 자리에 사건이 보입니다.', skipOffstage: false), findsOneWidget);
    // 창구가 던져도 자리로 접는다.
    await _pump(t, LibraryDocView(model: fixtureJson('all_kinds'), title: '모든 꼴', width: _column, ctx: DocContext(strip: (_, _) => Future.error(StateError('창구 없음')))));
    await t.pump();
    expect(find.text('HistoryTime이 이 기기에 있으면 이 자리에 사건이 보입니다.', skipOffstage: false), findsNWidgets(2));
    expect(t.takeException(), isNull);

    // HistoryTime이 그릴 때는 띠의 두 글을 제 말로 — 단추는 그대로 onTimeline.
    final flown = <String>[];
    await _pump(
      t,
      LibraryDocView(
        model: fixtureJson('all_kinds'),
        title: '모든 꼴',
        width: _column,
        ctx: DocContext(stripOpenLabel: '연표에서 보기', stripMissing: '이 연표에 없는 항목입니다.', onTimeline: flown.add),
      ),
    );
    await t.pump();
    expect(find.text('HistoryTime에서 열기', skipOffstage: false), findsNothing);
    expect(find.text('이 연표에 없는 항목입니다.', skipOffstage: false), findsNWidgets(2));
    final open = find.text('연표에서 보기', skipOffstage: false).last;
    await t.ensureVisible(open);
    await t.pumpAndSettle();
    await t.tap(open);
    expect(flown, ['01JB2X7K9N']);
  });

  testWidgets('각주 위첨자 · 목차를 누르면(받는 앱이 뛰기를 주지 않으면) 이 굴림 안에서 그 자리로 뛴다', (t) async {
    final ctx = DocContext();
    await _pump(t, LibraryDocView(model: fixtureJson('doc/임진왜란'), title: '임진왜란', width: _column, ctx: ctx), height: 600);
    final fn = find.byKey(const ValueKey('fnref-1'));
    await t.scrollUntilVisible(fn, 200, scrollable: find.byType(Scrollable).first);
    await t.pumpAndSettle();
    await t.tap(fn);
    await t.pumpAndSettle();
    final note = ctx.anchors.footnote(1);
    expect(note.currentContext, isNotNull);
    expect(_onScreen(t, find.byKey(note), height: 600), isTrue);

    // 목차의 「결과」(문단 3) — 맨 위로 돌아가 누른다.
    final scroll = t.state<ScrollableState>(find.byType(Scrollable).first);
    scroll.position.jumpTo(0);
    await t.pumpAndSettle();
    final toc = find.descendant(of: find.byType(TocView), matching: find.text('결과'));
    await t.ensureVisible(toc);
    await t.pumpAndSettle();
    await t.tap(toc);
    await t.pumpAndSettle();
    expect(_onScreen(t, find.byKey(ctx.anchors.section(3)), height: 600), isTrue);
  });

  testWidgets('받는 앱이 뛰기를 주면 그것을 부른다', (t) async {
    int? footnote;
    await _pump(t, LibraryDocView(model: fixtureJson('doc/임진왜란'), title: '임진왜란', width: _column, ctx: DocContext(onFootnote: (n) => footnote = n)));
    final fn = find.byKey(const ValueKey('fnref-1'));
    await t.ensureVisible(fn);
    await t.pumpAndSettle();
    await t.tap(fn);
    expect(footnote, 1);
  });

  testWidgets('넘겨주기 문서는 따라가지 않고 대상을 보인다 — 누르면 onLink', (t) async {
    ObjectAddress? went;
    await _pump(t, LibraryDocView(model: fixtureJson('doc/임진전쟁'), title: '임진전쟁', width: _column, ctx: DocContext(onLink: (a) => went = a)));
    expect(find.text('넘겨주기 문서입니다.'), findsOneWidget);
    expect(find.text('「임진왜란」로'), findsOneWidget);
    expect(find.byType(BlockView), findsNothing);
    await t.tap(find.byKey(const ValueKey('redirect')));
    expect(went, const ObjectAddress('문서', '임진왜란'));
  });

  testWidgets('폭을 주지 않으면 놓인 폭으로 — 표는 화면이 아니라 칸 폭으로 꼴을 고른다', (t) async {
    _buildEverything();
    await _pump(
      t,
      Align(
        alignment: Alignment.topLeft,
        child: SizedBox(width: _column, child: LibraryDocView(model: fixtureJson('all_kinds'), title: '모든 꼴')),
      ),
    );
    expect(t.takeException(), isNull);
    double cellRight(WidgetTester t) => (t.widget<Padding>(find.descendant(of: find.byKey(const ValueKey('cell-1-0'), skipOffstage: false), matching: find.byType(Padding, skipOffstage: false), skipOffstage: false).first).padding as EdgeInsets).right;
    // 화면은 1200이지만 칸이 380이라 좁은 칸의 여백(16).
    expect(cellRight(t), 16);
    // 아카이브처럼 블록만 그리면 화면 폭(1200 — 넓음)을 본다.
    await _pump(
      t,
      CustomScrollView(
        scrollCacheExtent: const ScrollCacheExtent.pixels(100000),
        slivers: [SliverDocBlocks(blocks: fixture('all_kinds').blocks, context: DocContext())],
      ),
    );
    expect(cellRight(t), 32);
  });

  testWidgets('SliverLibraryDoc — 받는 앱의 굴림 속에 제 머리와 함께', (t) async {
    await _pump(
      t,
      Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: _column,
          child: CustomScrollView(
            slivers: [
              const SliverToBoxAdapter(child: SizedBox(height: 40, child: Text('항목 머리'))),
              SliverPadding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                sliver: SliverLibraryDoc(model: fixtureJson('doc/한산도 대첩'), title: '한산도 대첩'),
              ),
            ],
          ),
        ),
      ),
    );
    expect(t.takeException(), isNull);
    expect(find.text('항목 머리'), findsOneWidget);
    expect(t.getRect(find.byType(DocHeadView)).top, 40);
    expect(t.getRect(find.byType(DocHeadView)).width, _column - 24);
  });

  testWidgets('같은 모델을 다시 받으면 펼친 접힌 문단이 그대로다', (t) async {
    final json = fixtureJson('all_kinds');
    Widget view() => LibraryDocView(model: json, title: '모든 꼴', width: _column, head: false);
    _buildEverything();
    await _pump(t, view());
    expect(find.text('접힌 문단 속 글', findRichText: true, skipOffstage: false), findsNothing);
    final folded = find.text('접힌 문단', findRichText: true, skipOffstage: false);
    await t.ensureVisible(folded);
    await t.pumpAndSettle();
    await t.tap(folded);
    await t.pump();
    expect(find.text('접힌 문단 속 글', findRichText: true, skipOffstage: false), findsOneWidget);
    await t.pumpWidget(
      WidgetsApp(
        color: Ink.text,
        builder: (_, _) => ColoredBox(color: Ink.surface, child: view()),
      ),
    );
    await t.pump();
    expect(find.text('접힌 문단 속 글', findRichText: true, skipOffstage: false), findsOneWidget);
  });
}
