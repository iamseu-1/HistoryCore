import 'engine.dart';

/// 서재 이름공간(설계 5-1). 이름이 다리의 `ns` 값이다.
enum Namespace { doc, category, template, file, front }

/// 고치기 시작한 판이 지금 머리가 아니다 — 그 사이 다른 저장이 있었다. 덮어쓰지 않았다. [head]가 지금 머리 판.
class StaleBaseError extends HistoryCoreError {
  StaleBaseError(super.message, {required this.head}) : super(code: 'stale_base');
  final String head;
}

class LibraryInfo {
  LibraryInfo._(Map<String, dynamic> j)
      : library = j['library'] as String,
        device = j['device'] as String,
        format = j['format'] as int,
        folder = j['folder'] as String;
  final String library;
  final String device;
  final int format;

  /// 엔진의 자리(`<앱 자료 폴더>/HistoryCore`).
  final String folder;
}

class Saved {
  Saved._(Map<String, dynamic> j)
      : doc = j['doc'] as String,
        rev = j['rev'] as String,
        hlc = j['hlc'] as int,
        changed = j['changed'] as bool;
  final String doc;
  final String rev;
  final int hlc;

  /// 거짓이면 본문이 같아 판을 더하지 않았다.
  final bool changed;
}

class Renamed {
  Renamed._(Map<String, dynamic> j)
      : rev = j['rev'] as String,
        hlc = j['hlc'] as int,
        redirect = j['redirect'] as String?;
  final String rev;
  final int hlc;

  /// 옛 제목에 남긴 넘겨주기 문서.
  final String? redirect;
}

class DocRecord {
  DocRecord._(Map<String, dynamic> j)
      : id = j['id'] as String,
        ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        head = j['head'] as String,
        created = j['created'] as int,
        updated = j['updated'] as int,
        deleted = j['deleted'] as bool,
        body = j['body'] as String;
  final String id;
  final Namespace ns;
  final String title;
  final String head;
  final int created;
  final int updated;
  final bool deleted;
  final String body;
}

class DocSummary {
  DocSummary._(Map<String, dynamic> j)
      : id = j['id'] as String,
        ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        head = j['head'] as String,
        created = j['created'] as int,
        updated = j['updated'] as int;
  final String id;
  final Namespace ns;
  final String title;
  final String head;
  final int created;
  final int updated;
}

class RevisionInfo {
  RevisionInfo._(Map<String, dynamic> j)
      : id = j['id'] as String,
        doc = j['doc'] as String,
        parent = j['parent'] as String?,
        parent2 = j['parent2'] as String?,
        device = j['device'] as String,
        hlc = j['hlc'] as int,
        deleted = j['kind'] == 'deleted',
        ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        message = j['message'] as String,
        length = j['len'] as int;
  final String id;
  final String doc;
  final String? parent;
  final String? parent2;
  final String device;
  final int hlc;

  /// 지우기 표시 판.
  final bool deleted;
  final Namespace ns;
  final String title;
  final String message;

  /// 본문의 UTF-8 바이트 수.
  final int length;
}

class Counts {
  Counts._(Map<String, dynamic> j)
      : live = {for (final ns in Namespace.values) ns: j[ns.name] as int},
        deleted = j['deleted'] as int,
        total = j['total'] as int;

  /// 이름공간별 살아 있는 문서 수.
  final Map<Namespace, int> live;
  final int deleted;
  final int total;
}

/// 열린 서재 하나(spec/engine-api.md의 `library.*` · `doc.*` · `rev.*`).
class Library {
  Library._(this._core, this._handle, this.info);

  final HistoryCore _core;
  final int _handle;
  final LibraryInfo info;

  /// 앱 자료 폴더 [appDataDir] 아래 `HistoryCore/`에 서재를 연다(없으면 만든다). 그 폴더 바로 아래 옛 앱 파일은 건드리지 않는다.
  static Future<Library> open(HistoryCore core, String appDataDir) async {
    final r = _check(await core.call({'op': 'library.open', 'dir': appDataDir}));
    return Library._(core, r['handle'] as int, LibraryInfo._(r));
  }

  Future<Map<String, dynamic>> _call(String op, [Map<String, Object?> args = const {}]) async =>
      _check(await _core.call({'op': op, 'handle': _handle, ...args}));

  Future<Saved> create(Namespace ns, String title, String body, {String message = ''}) async =>
      Saved._(await _call('doc.create', {'ns': ns.name, 'title': title, 'body': body, 'message': message}));

  Future<Saved> save(String doc, String base, String body, {String message = ''}) async =>
      Saved._(await _call('doc.save', {'doc': doc, 'base': base, 'body': body, 'message': message}));

  Future<Renamed> rename(String doc, String base, String title, {bool keepRedirect = true, String message = ''}) async =>
      Renamed._(await _call('doc.rename', {'doc': doc, 'base': base, 'title': title, 'keep_redirect': keepRedirect, 'message': message}));

  Future<Saved> delete(String doc, String base, {String message = ''}) async =>
      Saved._(await _call('doc.delete', {'doc': doc, 'base': base, 'message': message}));

  /// 없는 문서면 null.
  Future<DocRecord?> get(String doc) async {
    try {
      return DocRecord._((await _call('doc.get', {'doc': doc}))['doc'] as Map<String, dynamic>);
    } on HistoryCoreError catch (e) {
      if (e.code == 'not_found') return null;
      rethrow;
    }
  }

  Future<List<DocSummary>> byTitle(Namespace ns, String title) async => [
        for (final d in (await _call('doc.by_title', {'ns': ns.name, 'title': title}))['docs'] as List)
          DocSummary._(d as Map<String, dynamic>),
      ];

  /// 판 기록, 새 것부터. [before](hlc)를 주면 그보다 앞선 판만.
  Future<List<RevisionInfo>> history(String doc, {int limit = 50, int? before}) async => [
        for (final r in (await _call('doc.history', {'doc': doc, 'limit': limit, 'before': before}))['revisions'] as List)
          RevisionInfo._(r as Map<String, dynamic>),
      ];

  /// 판 하나의 본문. 없는 판이면 null.
  Future<String?> revisionBody(String rev) async {
    try {
      return ((await _call('rev.get', {'rev': rev}))['revision'] as Map<String, dynamic>)['body'] as String;
    } on HistoryCoreError catch (e) {
      if (e.code == 'not_found') return null;
      rethrow;
    }
  }

  Future<Counts> counts() async => Counts._(await _call('library.counts'));

  Future<void> close() async {
    await _call('library.close');
  }
}

Map<String, dynamic> _check(Map<String, dynamic> r) {
  if (r['ok'] == true) return r;
  final message = r['error'] as String? ?? '알 수 없는 오류';
  final code = r['code'] as String?;
  if (code == 'stale_base') throw StaleBaseError(message, head: r['head'] as String);
  throw HistoryCoreError(message, code: code);
}
