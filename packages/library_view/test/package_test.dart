import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:library_view/ui.dart';
import 'package:yaml/yaml.dart';

/// 패키지의 약속(P5 R7 · R8) — 엔진에 기대지 않고, 글꼴을 싣지 않으며, 판은 0.1.0.
void main() {
  final pubspec = loadYaml(File('pubspec.yaml').readAsStringSync()) as YamlMap;

  test('기대는 것은 flutter · path_drawing뿐 — 엔진(history_core) · FFI에 기대지 않는다', () {
    expect((pubspec['dependencies'] as YamlMap).keys.toSet(), {'flutter', 'path_drawing'});
    for (final f in Directory('lib').listSync(recursive: true).whereType<File>().where((f) => f.path.endsWith('.dart'))) {
      final s = f.readAsStringSync();
      expect(s.contains('package:history_core') || s.contains('dart:ffi') || s.contains('dart:io'), isFalse, reason: f.path);
    }
  });

  test('글꼴 · 자산을 싣지 않는다 — 받는 앱이 선언할 가족 이름은 README에', () {
    final flutter = pubspec['flutter'] as YamlMap?;
    expect(flutter?['fonts'], isNull);
    expect(flutter?['assets'], isNull);
    final readme = File('README.md').readAsStringSync();
    for (final family in {TextStyles.sans, TextStyles.serif, TextStyles.mono, ...TextStyles.hanjaFallback}) {
      expect(readme, contains('`$family`'), reason: family);
    }
  });

  test('판은 0.1.0 — CHANGELOG 첫 절과 같다', () {
    expect(pubspec['version'], '0.1.0');
    final changelog = File('CHANGELOG.md').readAsLinesSync();
    expect(changelog.firstWhere((l) => l.startsWith('## ')), startsWith('## 0.1.0'));
  });
}
