import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:history_core/history_core.dart';

import 'support.dart';

void main() {
  late Directory dir;
  late HistoryCore core;

  setUp(() async {
    dir = Directory.systemTemp.createTempSync('서재 다리 시험 ');
    if (Platform.isWindows) core = await HistoryCore.start(libraryPath: builtLibrary());
  });

  tearDown(() {
    if (Platform.isWindows) core.close();
    try {
      dir.deleteSync(recursive: true);
    } on FileSystemException {
      // 엔진이 연 파일이 잠깐 잠겨 있을 수 있다 — 임시 폴더라 남겨도 된다.
    }
  });

  test('만들고 읽고 고치고 판 기록을 본다', () async {
    final lib = await Library.open(core, dir.path);
    expect(lib.info.folder, endsWith('HistoryCore'));
    expect(lib.info.format, 1);
    final a = await lib.create(Namespace.doc, '鶴', '𪄃 😀 학은 물새다.', message: '처음');
    final d = await lib.get(a.doc);
    expect(d!.body, '𪄃 😀 학은 물새다.');
    expect(d.head, a.rev);
    expect(d.ns, Namespace.doc);
    final b = await lib.save(a.doc, a.rev, '둘째 판', message: '고침');
    expect(b.changed, isTrue);
    expect((await lib.save(a.doc, b.rev, '둘째 판')).changed, isFalse);
    final h = await lib.history(a.doc);
    expect(h.map((r) => r.id), [b.rev, a.rev]);
    expect(h.first.parent, a.rev);
    expect(await lib.revisionBody(a.rev), '𪄃 😀 학은 물새다.');
    final counts = await lib.counts();
    expect((counts.total, counts.live[Namespace.doc], counts.deleted), (1, 1, 0));
    expect(await lib.get('00000000-0000-7000-8000-000000000000'), isNull);
    await lib.close();
  }, skip: windowsOnly);

  test('낡은 판 위 저장은 StaleBaseError — 덮어쓰지 않는다', () async {
    // Review Focus 3
    final lib = await Library.open(core, dir.path);
    final a = await lib.create(Namespace.doc, '정유재란', '첫 판');
    final b = await lib.save(a.doc, a.rev, '다른 창의 저장');
    await expectLater(
      lib.save(a.doc, a.rev, '늦은 저장'),
      throwsA(isA<StaleBaseError>().having((e) => e.head, 'head', b.rev).having((e) => e.code, 'code', 'stale_base')),
    );
    expect((await lib.get(a.doc))!.body, '다른 창의 저장');
    await lib.close();
  }, skip: windowsOnly);

  test('이름을 바꾸면 옛 제목에 넘겨주기가 남고, 지우면 표시만 남는다', () async {
    final lib = await Library.open(core, dir.path);
    final a = await lib.create(Namespace.category, '조선 중기', '');
    final r = await lib.rename(a.doc, a.rev, '조선 중엽');
    final old = await lib.byTitle(Namespace.category, '조선 중기');
    expect(old.single.id, r.redirect);
    expect((await lib.get(r.redirect!))!.body, '#넘겨주기 분류:조선 중엽\n');
    final d = await lib.delete(a.doc, r.rev);
    expect(d.changed, isTrue);
    expect((await lib.get(a.doc))!.deleted, isTrue);
    expect((await lib.history(a.doc)).first.deleted, isTrue);
    await lib.close();
  }, skip: windowsOnly);

  test('서재 자리에 남의 파일이 있으면 not_a_library — 파일은 그대로', () async {
    // Review Focus 2
    final ours = File([dir.path, 'HistoryCore', 'library.db'].join(Platform.pathSeparator))
      ..createSync(recursive: true)
      ..writeAsStringSync('SQLite 아님');
    await expectLater(
      Library.open(core, dir.path),
      throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'not_a_library')),
    );
    expect(ours.readAsStringSync(), 'SQLite 아님');
  }, skip: windowsOnly);

  test('쓸 수 없는 제목은 bad_title', () async {
    final lib = await Library.open(core, dir.path);
    await expectLater(
      lib.create(Namespace.doc, '분류:조선', ''),
      throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'bad_title')),
    );
    await lib.close();
  }, skip: windowsOnly);

  test('미리보기: 서재의 틀을 펼친 그리기 모델', () async {
    final lib = await Library.open(core, dir.path);
    await lib.create(Namespace.template, '해전 정보', "'''@이름@'''");
    final model = await lib.render('[include(틀:해전 정보, 이름=한산도)]\n𪄃 鶴');
    expect(model['version'], 1);
    final block = (model['blocks'] as List).first as Map<String, dynamic>;
    expect(block['t'], 'template');
    expect(block['status'], 'ok');
    expect(((((block['blocks'] as List).first as Map)['inl'] as List).first as Map)['inl'], [
      {'t': 'text', 's': '한산도'},
    ]);
    final page = await lib.render('@이름=미상@', title: '해전 정보', ns: Namespace.template);
    expect(((((page['blocks'] as List).first as Map)['inl'] as List).first as Map)['t'], 'param');
    await lib.close();
  }, skip: windowsOnly);

  test('열기 · 찾기 · 나무 · 대문을 한 서재에서', () async {
    final lib = await Library.open(core, dir.path);
    await lib.create(Namespace.category, '조선', '');
    final a = await lib.create(Namespace.doc, '임진왜란', '[[분류:조선]] 1592년 [[이순신]] [[연표:01A]]');
    final opened = (await lib.openDoc(a.doc))!;
    expect(opened.doc.title, '임진왜란');
    expect(opened.stale, isFalse);
    expect((opened.model['blocks'] as List), isNotEmpty);
    final hits = await lib.searchBody('1592');
    expect(hits.single.doc.id, a.doc);
    expect(hits.single.snippet, contains('1592'));
    final root = await lib.treeChildren();
    expect(root.categories.single.name, '조선');
    expect(root.categories.single.total, 1);
    expect((await lib.treeChildren(category: '조선')).docs.single.title, '임진왜란');
    expect(await lib.titlesExist([(Namespace.doc, '임진왜란'), (Namespace.doc, '이순신')]), [a.doc, null]);
    expect((await lib.linksWanted()).single.title, '이순신');
    expect((await lib.timelineDocs('01A')).single.title, '임진왜란');
    final home = await lib.home();
    expect((home['groups'] as List).single['name'], '조선');
    expect((await lib.indexStatus()).stale, 0);
    expect(await lib.indexStep(), 0);
    await lib.close();
  }, skip: windowsOnly);

  test('동기화 — 합류 · 묶음 주고받기 · 충돌을 견주어 합침 · 내보내기', () async {
    final other = Directory.systemTemp.createTempSync('서재 다리 시험 나 ');
    final a = await Library.open(core, dir.path);
    final b = await Library.open(core, other.path);
    expect(b.info.library, isNot(a.info.library));
    await b.join(a.info.library);
    expect(b.info.library, a.info.library);
    final s = await a.create(Namespace.doc, '임진왜란', '첫 줄\n둘째 줄\n');
    final status = await a.syncStatus(afterSeq: 0);
    expect((status.pending, status.held, status.conflicts, status.device), (1, 0, 0, a.info.device));
    final first = (await a.syncExport(afterSeq: 0, no: 1))!;
    expect((first.count, first.more), (1, false));
    expect((await b.syncApply(first.pack)).added, 1);
    expect(await b.syncExport(afterSeq: 0, no: 1), isNull);
    await a.save(s.doc, s.rev, '첫 줄\n가\n');
    await b.save(s.doc, s.rev, '첫 줄\n나\n');
    final fromB = (await b.syncExport(afterSeq: 0, no: 1))!;
    final r = await a.syncApply(fromB.pack);
    expect(r.conflicts, [s.doc]);
    final d = (await a.get(s.doc))!;
    expect(d.conflict, hasLength(1));
    expect((await a.openDoc(s.doc))!.conflict, d.conflict);
    await a.save(s.doc, d.head, '첫 줄\n가나\n', merge: d.conflict.single);
    expect((await a.get(s.doc))!.conflict, isEmpty);
    expect((await a.syncStatus(afterSeq: 0)).conflicts, 0);
    await expectLater(b.syncApply(first.pack..[0] ^= 0xff), throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'bad_pack')));
    final stranger = Directory.systemTemp.createTempSync('서재 다리 시험 남 ');
    final c = await Library.open(core, stranger.path);
    await expectLater(c.syncApply(fromB.pack), throwsA(isA<WrongLibraryError>().having((e) => e.found, 'found', a.info.library)));
    final zip = File('${other.path}${Platform.pathSeparator}내보내기.zip');
    await b.backup(zip.path);
    expect(zip.lengthSync(), greaterThan(100));
    for (final lib in [a, b, c]) {
      await lib.close();
    }
  }, skip: windowsOnly);
}
