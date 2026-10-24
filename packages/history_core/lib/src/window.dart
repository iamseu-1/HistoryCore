import 'engine.dart';
import 'library.dart';

/// 이 패키지가 아는 서재 창구 약속의 판(spec/windows-api.md 1절 — 창구 답마다의 `api_version`).
const windowApiVersion = 1;

/// `window.version`의 답.
class WindowVersion {
  WindowVersion.fromJson(Map<String, dynamic> j)
      : apiVersion = j['api_version'] as int,
        library = j['library'] as String,
        engine = j['engine'] as String;

  /// 창구 약속의 판 — [windowApiVersion]보다 크면 읽는 쪽은 그 기능을 끈다.
  final int apiVersion;

  /// 서재 UUID — 아카이브가 다른 서재로 합류하면 바뀐다.
  final String library;

  /// 엔진 판.
  final String engine;
}

/// 창구의 문서 요약(`window.docs_for_entity`의 한 줄).
class WindowDocSummary {
  WindowDocSummary.fromJson(Map<String, dynamic> j)
      : id = j['id'] as String,
        ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        updated = DateTime.parse(j['updated'] as String);
  final String id;
  final Namespace ns;
  final String title;

  /// 머리 판을 쓴 때(UTC, 밀리초까지).
  final DateTime updated;
}

/// `window.doc` · `window.doc_by_title`의 답 — 요약과 머리 판의 그리기 모델(spec/namumark.md 판 1).
/// 넘겨주기 문서면 따라가지 않은 그 문서 자신이고, [model]의 `redirect`를 보고 묻는 쪽이 다시 묻는다.
class WindowDoc extends WindowDocSummary {
  WindowDoc.fromJson(super.j)
      : model = j['model'] as Map<String, dynamic>,
        super.fromJson();
  final Map<String, dynamic> model;
}

/// `window.search`의 한 줄 — 제목으로 찾은 줄은 [snippet]이 빈 글이다.
class WindowHit {
  WindowHit.fromJson(Map<String, dynamic> j)
      : id = j['id'] as String,
        ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        snippet = j['snippet'] as String;
  final String id;
  final Namespace ns;
  final String title;

  /// 본문으로 찾았으면 첫 맞은 곳 앞뒤 글자(표식 없음), 제목으로 찾았으면 빈 글.
  final String snippet;
}

/// 다른 앱(아카이브)의 서재를 **읽기 전용**으로 묻는 창구(spec/windows-api.md 3-1 · 4절).
///
/// 만들지 않고(서재가 없으면 `no_library`) · 쓰지 않고 · 색인을 고치지 않는다 — 쓰는 메서드가 없는 따로 된 형이다.
/// 엔진은 질의마다 서재 파일을 열고 읽기 거래 하나로 읽고 곧 닫으므로, 열어 둔 채 여러 번 물어도 아카이브를 막지 않는다.
/// 색인이 없거나 판이 다르면 색인을 쓰는 질의([docsForEntity] · [titlesExist] · [search])는 `no_index`로 실패한다.
/// 실패는 모두 [HistoryCoreError]이고 [HistoryCoreError.code]로 가른다.
class LibraryWindow {
  LibraryWindow._(this._core, this._handle, this.info);

  final HistoryCore _core;
  final int _handle;

  /// 연 때의 서재 정보(서재 · 기기 ID · 형식 판 · 자리 `<앱 자료 폴더>/HistoryCore`).
  final LibraryInfo info;

  /// 앱 자료 폴더 [appDataDir](아카이브의 `LibraryDir`) 아래의 서재를 읽기 전용으로 연다.
  /// 같은 [core]에서 같은 폴더를 다시 열면 같은 손잡이다 — 하나를 닫으면 함께 닫힌다.
  ///
  /// 창구를 모르는 옛 엔진(`window.*`가 없는 판 — 엔진 판 · ABI가 새 엔진과 같아 판으로는 못 가른다)은 `read_only`를 모르는 칸으로
  /// 지나쳐 **쓰는 열기**를 한다(서재 · 색인을 만들고 고침). 그래서 열기 전에 값싼 물음 하나로 가르고(spec/windows-api.md 3-1의 2),
  /// 옛 엔진이면 열지 않고 `engine_too_old`로 실패한다. 연 답에 `read_only: true`가 없어도 곧 닫고 같은 코드로 실패한다.
  static Future<LibraryWindow> open(HistoryCore core, String appDataDir) async {
    // 손잡이 0은 엔진이 주지 않는 번호다 — 창구를 아는 엔진은 `bad_handle`, 옛 엔진은 「모르는 op」(code 없음).
    final probe = await core.call({'op': 'window.version', 'handle': 0});
    if (probe['code'] != 'bad_handle') throw _tooOld('창구 물음의 답이 「${probe['error'] ?? probe}」');
    final r = _answer(await core.call({'op': 'library.open', 'dir': appDataDir, 'read_only': true}));
    if (r['read_only'] != true) {
      try {
        await core.call({'op': 'library.close', 'handle': r['handle']});
      } catch (_) {}
      throw _tooOld('읽기 전용 열기의 답에 read_only가 없음');
    }
    return LibraryWindow._(core, r['handle'] as int, LibraryInfo.fromJson(r));
  }

  static HistoryCoreError _tooOld(String why) =>
      HistoryCoreError('엔진이 서재 창구(읽기 전용 열기)를 모르는 옛 판이라 열지 않음 — $why', code: 'engine_too_old');

  Future<Map<String, dynamic>> _call(String op, [Map<String, Object?> args = const {}]) async =>
      _answer(await _core.call({'op': op, 'handle': _handle, ...args}));

  Future<WindowVersion> version() async => WindowVersion.fromJson(await _call('window.version'));

  /// 문서 하나. 없거나 지운 문서면 null.
  Future<WindowDoc?> doc(String id) async => _found(await _call('window.doc', {'id': id}));

  /// 제목(서재의 제목 규칙으로 맞춤)의 살아 있는 문서 — 같은 제목이 둘이면 먼저 만든 것. 없거나 쓸 수 없는 제목이면 null.
  Future<WindowDoc?> docByTitle(String title, {Namespace ns = Namespace.doc}) async =>
      _found(await _call('window.doc_by_title', {'ns': ns.name, 'title': title}));

  /// 이 연표 항목(원 ID)을 잇는 문서 — 연표 링크 · 연표 띠, 제목 차례. 항목ID는 엔진이 앞뒤 공백을 떼고 NFC로 맞춘다.
  Future<List<WindowDocSummary>> docsForEntity(String entity) async => [
        for (final d in (await _call('window.docs_for_entity', {'entity': entity}))['docs'] as List)
          WindowDocSummary.fromJson(d as Map<String, dynamic>),
      ];

  /// 있는 문서 ID — 같은 차례, 없으면 null(1,000개까지).
  Future<List<String?>> titlesExist(List<(Namespace, String)> titles) async => [
        for (final d in (await _call('window.titles_exist', {
          'titles': [for (final (ns, t) in titles) {'ns': ns.name, 'title': t}],
        }))['docs'] as List)
          d as String?,
      ];

  /// 제목 앞부분(일반 문서)으로 찾은 것을 먼저, 이어 본문으로 찾은 것 — 같은 문서는 한 번. [limit]는 1 ~ 100으로 잘린다.
  Future<List<WindowHit>> search(String q, {int limit = 20}) async => [
        for (final h in (await _call('window.search', {'q': q, 'limit': limit}))['docs'] as List)
          WindowHit.fromJson(h as Map<String, dynamic>),
      ];

  Future<void> close() async {
    await _call('library.close');
  }
}

WindowDoc? _found(Map<String, dynamic> r) => r['found'] == true ? WindowDoc.fromJson(r) : null;

Map<String, dynamic> _answer(Map<String, dynamic> r) {
  if (r['ok'] == true) return r;
  throw HistoryCoreError(r['error'] as String? ?? '알 수 없는 오류', code: r['code'] as String?);
}
