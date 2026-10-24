import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:yaml/yaml.dart';

/// HistoryCore CI 정의의 약속 — GitHub에서 돌기 전에 여기서 잡는다.
void main() {
  final workflow = loadYaml(File('../../.github/workflows/ci.yml').readAsStringSync()) as YamlMap;
  final jobs = workflow['jobs'] as YamlMap;

  List<YamlMap> steps(String job) => (jobs[job]['steps'] as YamlList).cast<YamlMap>().toList();
  String shellOf(String job, YamlMap step) => (step['shell'] ?? jobs[job]['defaults']?['run']?['shell'] ?? 'bash') as String;

  test('pwsh로 여러 명령을 도는 단계는 중간 명령의 실패도 잡는다', () {
    // GitHub의 pwsh 감싸개는 마지막 명령의 종료 코드만 본다 — 바깥 명령이 실패하면 바로 멈추게 한다.
    for (final job in jobs.keys.cast<String>()) {
      for (final s in steps(job)) {
        final run = s['run'] as String?;
        if (run == null || shellOf(job, s) != 'pwsh') continue;
        final lines = run.split('\n').map((l) => l.trim()).where((l) => l.isNotEmpty).toList();
        if (lines.length < 2) continue;
        expect(lines.first, r'$PSNativeCommandUseErrorActionPreference = $true', reason: '$job › ${s['name']}');
      }
    }
  });

  test('폰 측정기(hc-bench)도 arm64로 지어 본다', () {
    final runs = steps('android').map((s) => '${s['run']}');
    expect(runs.any((r) => r.contains('--bin hc-bench') && r.contains('--target aarch64-linux-android')), isTrue);
  });

  test('그리기 패키지(library_view)는 엔진을 짓지 않는 제 일에서 분석 · 검사한다', () {
    // 엔진에 기대지 않는다는 약속(P5 R8)을 CI도 지킨다 — 엔진 짓기 없이 pub get · analyze · test가 돌아야 한다.
    String dirOf(String job, YamlMap s) => (s['working-directory'] ?? jobs[job]['defaults']?['run']?['working-directory'] ?? '') as String;
    final found = <String>[];
    for (final job in jobs.keys.cast<String>()) {
      for (final s in steps(job)) {
        final run = s['run'] as String?;
        if (run == null || dirOf(job, s) != 'packages/library_view') continue;
        if (run.contains('flutter analyze') && run.contains('flutter test')) found.add(job);
      }
    }
    expect(found, hasLength(1));
    final runs = steps(found.single).map((s) => '${s['run'] ?? ''}${s['uses'] ?? ''}').join('\n');
    expect(runs, contains('flutter pub get'));
    expect(runs, isNot(contains('build-native')));
    expect(runs, isNot(contains('cargo')));
  });
}
