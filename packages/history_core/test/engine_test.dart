import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:history_core/history_core.dart';

/// `tools/build-native.ps1`이 놓은 DLL(또는 HISTORY_CORE_LIBRARY).
String builtLibrary() =>
    Platform.environment['HISTORY_CORE_LIBRARY'] ??
    '${Directory.current.path}${Platform.pathSeparator}native${Platform.pathSeparator}windows-x64${Platform.pathSeparator}history_core.dll';

void main() {
  final windowsOnly = Platform.isWindows ? false : 'Windows에서만 — 미리 지은 DLL로 돈다';

  test('엔진 판 번호를 isolate 너머로 받는다', () async {
    final core = await HistoryCore.start(libraryPath: builtLibrary());
    addTearDown(core.close);
    final v = await core.version();
    expect(v.engine, '0.0.1');
    expect(v.abi, 1);
  }, skip: windowsOnly);

  test('모르는 op는 오류 답으로 온다', () async {
    final core = await HistoryCore.start(libraryPath: builtLibrary());
    addTearDown(core.close);
    final r = await core.call({'op': 'nope'});
    expect(r['ok'], false);
    expect(r['error'], contains('nope'));
  }, skip: windowsOnly);

  test('4바이트 글자가 다리를 그대로 건넌다', () async {
    // Review Focus 5
    final core = await HistoryCore.start(libraryPath: builtLibrary());
    addTearDown(core.close);
    final r = await core.call({'op': 'echo', 'text': '鶴 𪄃 😀 끝'});
    expect(r['text'], '鶴 𪄃 😀 끝');
  }, skip: windowsOnly);

  test('여러 요청을 한꺼번에 보내도 각자 제 답을 받는다', () async {
    // Review Focus 4
    final core = await HistoryCore.start(libraryPath: builtLibrary());
    addTearDown(core.close);
    final answers = await Future.wait([
      for (var i = 0; i < 50; i++) core.call({'op': 'echo', 'text': '요청$i'}),
    ]);
    for (var i = 0; i < 50; i++) {
      expect(answers[i]['text'], '요청$i');
    }
  }, skip: windowsOnly);

  test('한글 · 공백이 든 폴더의 DLL도 연다', () async {
    // Review Focus 1
    final dir = Directory.systemTemp.createTempSync('엔진 경로 시험 ');
    addTearDown(() {
      try {
        dir.deleteSync(recursive: true);
      } on FileSystemException {
        // 한 번 불러온 DLL은 시험 프로세스가 끝날 때까지 잠겨 있다 — 임시 폴더라 남겨도 된다.
      }
    });
    final copy = File(builtLibrary()).copySync('${dir.path}${Platform.pathSeparator}history_core.dll');
    final core = await HistoryCore.start(libraryPath: copy.path);
    final v = await core.version();
    core.close();
    expect(v.engine, '0.0.1');
  }, skip: windowsOnly);

  test('라이브러리가 없으면 HistoryCoreLoadError — 멈추지 않는다', () async {
    // Review Focus 2
    await expectLater(
      HistoryCore.start(libraryPath: 'no_such_history_core_library.dll'),
      throwsA(isA<HistoryCoreLoadError>()),
    );
  });
}
