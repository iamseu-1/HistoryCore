import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';

/// 엔진 라이브러리를 찾는 곳. 환경 변수 `HISTORY_CORE_LIBRARY`가 먼저다(시험 · 측정용).
/// Android는 APK의 jniLibs에서, Windows는 실행 파일 옆에서 이름으로 찾는다.
String defaultLibraryPath() {
  final env = Platform.environment['HISTORY_CORE_LIBRARY'];
  if (env != null && env.isNotEmpty) return env;
  if (Platform.isAndroid) return 'libhistory_core.so';
  if (Platform.isWindows) return 'history_core.dll';
  throw UnsupportedError('HistoryCore는 Android · Windows만 지원한다: ${Platform.operatingSystem}');
}

typedef _CallNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _FreeNative = Void Function(Pointer<Utf8>);
typedef _FreeDart = void Function(Pointer<Utf8>);

/// 엔진 판 번호.
class EngineVersion {
  const EngineVersion(this.engine, this.abi);
  final String engine;
  final int abi;
}

/// 엔진이 돌려준 오류.
class HistoryCoreError implements Exception {
  HistoryCoreError(this.message);
  final String message;
  @override
  String toString() => 'HistoryCoreError: $message';
}

/// 엔진 라이브러리를 열지 못함(파일 없음 · 깨짐 · 다른 아키텍처).
class HistoryCoreLoadError extends HistoryCoreError {
  HistoryCoreLoadError(super.message);
}

/// 엔진을 따로 도는 isolate 하나에서 부른다. 메인 isolate는 답을 기다리기만 한다.
class HistoryCore {
  HistoryCore._(this._isolate, this._requests, this._replies);

  final Isolate _isolate;
  final SendPort _requests;
  final ReceivePort _replies;
  final _pending = <int, Completer<Map<String, dynamic>>>{};
  var _next = 0;

  /// [libraryPath]가 없으면 [defaultLibraryPath]. 열지 못하면 [HistoryCoreLoadError].
  static Future<HistoryCore> start({String? libraryPath}) async {
    final replies = ReceivePort();
    final first = Completer<Object?>();
    HistoryCore? core;
    replies.listen((msg) {
      if (!first.isCompleted) {
        first.complete(msg);
      } else {
        core?._onReply(msg);
      }
    });
    final isolate = await Isolate.spawn(_main, <Object?>[replies.sendPort, libraryPath ?? defaultLibraryPath()]);
    final hello = await first.future;
    if (hello is! SendPort) {
      isolate.kill(priority: Isolate.immediate);
      replies.close();
      throw HistoryCoreLoadError(hello.toString());
    }
    return core = HistoryCore._(isolate, hello, replies);
  }

  /// 요청 하나. 답은 늘 `ok`를 가진다.
  Future<Map<String, dynamic>> call(Map<String, Object?> request) {
    final id = _next++;
    final done = Completer<Map<String, dynamic>>();
    _pending[id] = done;
    _requests.send(<Object?>[id, jsonEncode(request)]);
    return done.future;
  }

  Future<EngineVersion> version() async {
    final r = await call({'op': 'version'});
    if (r['ok'] != true) throw HistoryCoreError(r['error'] as String? ?? '알 수 없는 오류');
    return EngineVersion(r['engine'] as String, r['abi'] as int);
  }

  void close() {
    _isolate.kill(priority: Isolate.immediate);
    _replies.close();
    for (final c in _pending.values) {
      c.completeError(StateError('엔진이 닫힘'));
    }
    _pending.clear();
  }

  void _onReply(Object? msg) {
    final list = msg as List<Object?>;
    final done = _pending.remove(list[0] as int);
    done?.complete(Map<String, dynamic>.from(jsonDecode(list[1] as String) as Map));
  }

  static void _main(List<Object?> boot) {
    final parent = boot[0] as SendPort;
    // 열기 · 함수 찾기 어느 쪽이 실패해도 부모에게 까닭을 보낸다(그러지 않으면 start가 영영 기다린다).
    final _CallNative call;
    final _FreeDart free;
    try {
      final lib = DynamicLibrary.open(boot[1] as String);
      call = lib.lookupFunction<_CallNative, _CallNative>('hc_call');
      free = lib.lookupFunction<_FreeNative, _FreeDart>('hc_free');
    } catch (e) {
      parent.send('엔진 라이브러리를 열지 못함(${boot[1]}): $e');
      return;
    }
    final requests = ReceivePort();
    parent.send(requests.sendPort);
    requests.listen((msg) {
      final list = msg as List<Object?>;
      final input = (list[1] as String).toNativeUtf8();
      try {
        final out = call(input);
        try {
          parent.send(<Object?>[list[0], out.toDartString()]);
        } finally {
          free(out);
        }
      } finally {
        malloc.free(input);
      }
    });
  }
}
