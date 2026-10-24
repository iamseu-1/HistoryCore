import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:history_core/history_core.dart';

import 'support.dart';

const lantern = 'lantern-chronicle/p-lantern';

/// 폴더 안 파일마다 이름 · 크기 · 고친 때 · 바이트 지문(서재 파일이 그대로인지 보려고).
List<(String, int, DateTime, int)> snapshot(Directory folder) {
  final files = folder.listSync().whereType<File>().toList()..sort((a, b) => a.path.compareTo(b.path));
  return [
    for (final f in files) (f.uri.pathSegments.last, f.lengthSync(), f.lastModifiedSync(), fnv1a(f.readAsBytesSync())),
  ];
}

/// FNV-1a 32비트 — 바이트가 같은지만 본다(기록은 바이트 목록을 같은 객체인지로 견주므로 지문으로 바꾼다).
int fnv1a(List<int> bytes) {
  var h = 0x811c9dc5;
  for (final b in bytes) {
    h = ((h ^ b) * 0x01000193) & 0xffffffff;
  }
  return h;
}

void main() {
  late Directory dir;
  late HistoryCore core;

  setUp(() async {
    dir = Directory.systemTemp.createTempSync('서재 창구 시험 ');
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

  /// 보통 열기로 씨를 심고 닫는다(아카이브가 꺼진 서재) — 연표 링크 · 띠 · 넘겨주기 · 지운 문서 · 틀.
  Future<Map<String, String>> seed() async {
    final lib = await Library.open(core, dir.path);
    await lib.create(Namespace.template, '정보', "'''@이름@''' — 틀 글");
    final war = await lib.create(Namespace.doc, '임진왜란', '[include(틀:정보, 이름=임진왜란)]\n왜란 이야기 [[연표:$lantern|등불]]');
    final renamed = await lib.rename(war.doc, war.rev, '임진 전쟁');
    final ga = await lib.create(Namespace.doc, '가', '[연표($lantern, 범위=1592..1598)]\n[[연표:$lantern]]');
    await lib.create(Namespace.doc, '정유재란', '임진왜란 뒤의 전쟁');
    final gone = await lib.create(Namespace.doc, '라', '[[연표:$lantern]] 지울 문서');
    await lib.delete(gone.doc, gone.rev);
    await lib.close();
    return {'war': war.doc, 'redirect': renamed.redirect!, 'ga': ga.doc, 'gone': gone.doc};
  }

  test('씨를 심은 서재를 읽기 전용으로 읽는다 — 서재 파일은 바이트까지 그대로', () async {
    final ids = await seed();
    final folder = Directory('${dir.path}${Platform.pathSeparator}HistoryCore');
    final before = snapshot(folder);
    expect(before.map((f) => f.$1), containsAll(['library.db', 'index.db']));

    final w = await LibraryWindow.open(core, dir.path);
    expect(w.info.folder, endsWith('HistoryCore'));
    expect(w.info.format, 1);
    final v = await w.version();
    expect((v.apiVersion, v.library, v.engine), (windowApiVersion, w.info.library, (await core.version()).engine));

    final d = (await w.doc(ids['war']!))!;
    expect((d.id, d.ns, d.title), (ids['war'], Namespace.doc, '임진 전쟁'));
    expect(d.updated.isUtc, isTrue);
    expect(d.model['version'], 1);
    expect(d.model.toString(), contains('틀 글'));
    final r = (await w.doc(ids['redirect']!))!;
    expect(r.title, '임진왜란');
    expect(r.model['redirect'], containsPair('key', '임진 전쟁'));
    expect(await w.doc(ids['gone']!), isNull);
    expect(await w.doc('0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b'), isNull);

    expect((await w.docByTitle('임진왜란'))!.id, ids['redirect']);
    expect((await w.docByTitle('정보', ns: Namespace.template))!.ns, Namespace.template);
    expect(await w.docByTitle('없는 문서'), isNull);

    final docs = await w.docsForEntity('  $lantern ');
    expect(docs.map((x) => x.title), ['가', '임진 전쟁']);
    expect(docs.first.id, ids['ga']);
    expect(docs.first.updated.isUtc, isTrue);
    expect(await w.docsForEntity('없는/항목'), isEmpty);

    expect(await w.titlesExist([(Namespace.doc, '가'), (Namespace.doc, '없음'), (Namespace.doc, '라')]), [ids['ga'], null, null]);

    final hits = await w.search('임진');
    expect(hits.map((h) => h.title), ['임진 전쟁', '임진왜란', '정유재란']);
    expect(hits.first.snippet, '');
    expect(hits.last.snippet, contains('임진왜란'));
    expect(await w.search('임진', limit: 1), hasLength(1));

    await w.close();
    expect(snapshot(folder), before);
  }, skip: windowsOnly);

  test('쓰는 서재 곁의 따로 된 손잡이 — 쓰기는 read_only로 거절하고, 쓰는 쪽 저장은 다음 질의에 보인다', () async {
    final lib = await Library.open(core, dir.path);
    final a = await lib.create(Namespace.doc, '가', '첫 판');
    final w = await LibraryWindow.open(core, dir.path);
    expect((w.info.library, w.info.device), (lib.info.library, lib.info.device));

    final raw = await core.call({'op': 'library.open', 'dir': dir.path, 'read_only': true});
    expect(raw['read_only'], isTrue);
    for (final req in <Map<String, Object?>>[
      {'op': 'doc.save', 'doc': a.doc, 'base': a.rev, 'body': '덮어씀'},
      {'op': 'doc.open', 'doc': a.doc},
      {'op': 'index.step'},
      {'op': 'sync.export', 'after_seq': 0, 'no': 1},
    ]) {
      final r = await core.call({...req, 'handle': raw['handle']});
      expect((r['ok'], r['code']), (false, 'read_only'), reason: '$req');
    }
    expect((await lib.get(a.doc))!.body, '첫 판');

    expect(await w.search('임진'), isEmpty);
    await lib.save(a.doc, a.rev, '둘째 판 — 임진왜란');
    final hits = await w.search('임진');
    expect(hits.single.title, '가');
    expect(hits.single.snippet, contains('임진왜란'));
    expect((await w.doc(a.doc))!.model.toString(), contains('둘째 판'));

    await w.close();
    await expectLater(w.version(), throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'bad_handle')));
    expect((await lib.counts()).total, 1, reason: '읽기 전용 손잡이를 닫아도 보통 손잡이는 그대로');
    await lib.close();
  }, skip: windowsOnly);

  test('없는 서재는 no_library — 폴더를 만들지 않는다', () async {
    final missing = Directory('${dir.path}${Platform.pathSeparator}없는 곳');
    await expectLater(
      LibraryWindow.open(core, missing.path),
      throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'no_library')),
    );
    expect(missing.existsSync(), isFalse);
    await expectLater(
      LibraryWindow.open(core, dir.path),
      throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'no_library')),
    );
    expect(dir.listSync(), isEmpty);
  }, skip: windowsOnly);

  test('색인이 없으면 색인을 쓰는 질의만 no_index — 문서는 서재에서 그린다', () async {
    final ids = await seed();
    File('${dir.path}${Platform.pathSeparator}HistoryCore${Platform.pathSeparator}index.db').deleteSync();
    final w = await LibraryWindow.open(core, dir.path);
    await expectLater(w.docsForEntity(lantern), throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'no_index')));
    await expectLater(w.search('임진'), throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'no_index')));
    expect((await w.doc(ids['ga']!))!.title, '가');
    await w.close();
  }, skip: windowsOnly);

  test('옛 엔진 가르기의 기준 — 창구를 아는 엔진은 손잡이 0의 window.version에 bad_handle', () async {
    final r = await core.call({'op': 'window.version', 'handle': 0});
    expect((r['ok'], r['code']), (false, 'bad_handle'));
  }, skip: windowsOnly);

  // 창구를 모르는 옛 엔진 dll(main 8c7a7ba처럼 window.*가 없는 판)은 `read_only`를 모르는 칸으로 지나쳐 쓰는 열기를 한다 — 엔진 판 · ABI가
  // 새 엔진과 같아 판으로는 못 가른다(가지 검토). 옛 dll은 git 밖이라 손잡이 `HISTORY_CORE_OLD_LIBRARY`를 줄 때만 돈다.
  final oldLibrary = Platform.environment['HISTORY_CORE_OLD_LIBRARY'];
  final Object oldSkip = !Platform.isWindows
      ? 'Windows에서만'
      : (oldLibrary == null || !File(oldLibrary).existsSync() ? '옛 엔진 dll 손잡이(HISTORY_CORE_OLD_LIBRARY)가 없음' : false);
  test('옛 엔진이면 열지 않고 engine_too_old — 서재 · 폴더를 만들지도 고치지도 않는다', () async {
    await seed();
    final folder = Directory('${dir.path}${Platform.pathSeparator}HistoryCore');
    final before = snapshot(folder);
    final old = await HistoryCore.start(libraryPath: oldLibrary);
    try {
      final probe = await old.call({'op': 'window.version', 'handle': 0});
      expect((probe['ok'], probe['code']), (false, null), reason: '옛 엔진은 「모르는 op」 — $probe');
      final tooOld = throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'engine_too_old'));
      await expectLater(LibraryWindow.open(old, dir.path), tooOld);
      expect(snapshot(folder), before, reason: '서재 파일이 바이트까지 그대로');
      final missing = Directory('${dir.path}${Platform.pathSeparator}없는 곳');
      await expectLater(LibraryWindow.open(old, missing.path), tooOld);
      expect(missing.existsSync(), isFalse, reason: '옛 엔진의 쓰는 열기였다면 서재를 새로 만들었다');
    } finally {
      old.close();
    }
    final w = await LibraryWindow.open(core, dir.path);
    expect(w.info.library, isNotEmpty, reason: '새 엔진은 그대로 연다');
    await w.close();
  }, skip: oldSkip);

  test('값 형은 공개 fromJson으로 짓는다', () {
    final s = WindowDocSummary.fromJson({'id': 'a', 'ns': 'category', 'title': '조선', 'updated': '2026-10-24T03:15:00.123Z'});
    expect((s.id, s.ns, s.title, s.updated), ('a', Namespace.category, '조선', DateTime.utc(2026, 10, 24, 3, 15, 0, 123)));
    final d = WindowDoc.fromJson({
      'found': true,
      'id': 'b',
      'ns': 'doc',
      'title': '가',
      'updated': '1970-01-01T00:00:00.000Z',
      'model': {'version': 1, 'blocks': <Object?>[]},
    });
    expect((d.title, d.model['version'], d.updated.millisecondsSinceEpoch), ('가', 1, 0));
    final h = WindowHit.fromJson({'id': 'c', 'ns': 'doc', 'title': '나', 'snippet': '…임진…'});
    expect((h.id, h.ns, h.title, h.snippet), ('c', Namespace.doc, '나', '…임진…'));
    final v = WindowVersion.fromJson({'ok': true, 'api_version': 1, 'library': 'l', 'engine': '0.0.1'});
    expect((v.apiVersion, v.library, v.engine), (1, 'l', '0.0.1'));
  });
}
