import 'dart:convert';
import 'dart:io';

import 'package:library_view/library_view.dart';

/// 표본 모델 — 아카이브 표본 서재를 엔진으로 그린 정답지(아카이브 test/fixtures/sample/models)의 사본.
Map<String, dynamic> fixtureJson(String path) => jsonDecode(File('test/fixtures/models/$path.json').readAsStringSync()) as Map<String, dynamic>;

DocModel fixture(String path) => DocModel.parse(fixtureJson(path));

/// 표본 모델 모두 — (이름공간/제목, JSON).
List<(String, Map<String, dynamic>)> allFixtures() {
  final root = Directory('test/fixtures/models');
  final files = root.listSync(recursive: true).whereType<File>().where((f) => f.path.endsWith('.json')).toList()
    ..sort((a, b) => a.path.compareTo(b.path));
  return [
    for (final f in files)
      (f.path.substring(root.path.length + 1).replaceAll('\\', '/').replaceAll('.json', ''), jsonDecode(f.readAsStringSync()) as Map<String, dynamic>),
  ];
}

/// 목업(ha-view.html)의 「임진왜란」 띠 — 아카이브 `sampleImjinStrip`과 같은 값.
const sampleImjinStrip = TimelineStripData(
  title: '임진왜란',
  from: 1592,
  to: 1599,
  spans: [StripSpan('1차 침략', 1592.25, 1593.5, strong: true), StripSpan('강화 협상', 1593.5, 1597.0), StripSpan('정유재란', 1597.0, 1598.92, strong: true)],
  events: [StripEvent('부산 상륙', 1592.25), StripEvent('한산도', 1592.55), StripEvent('명량', 1597.7), StripEvent('노량', 1598.9)],
);
