import 'dart:convert';
import 'dart:typed_data';

import 'engine.dart';

/// 서재 이름공간(설계 5-1). 이름이 다리의 `ns` 값이다.
enum Namespace { doc, category, template, file, front }

/// 고치기 시작한 판이 지금 머리가 아니다 — 그 사이 다른 저장이 있었다. 덮어쓰지 않았다. [head]가 지금 머리 판.
class StaleBaseError extends HistoryCoreError {
  StaleBaseError(super.message, {required this.head}) : super(code: 'stale_base');
  final String head;
}

/// 다른 서재의 묶음 — 들지 않았다. [found]가 묶음의 서재(spec/drive-format.md 4절).
class WrongLibraryError extends HistoryCoreError {
  WrongLibraryError(super.message, {required this.found}) : super(code: 'wrong_library');
  final String found;
}

class LibraryInfo {
  /// `library.open`의 답에서(읽기 전용 열기의 답도 같은 칸).
  factory LibraryInfo.fromJson(Map<String, dynamic> j) = LibraryInfo._;
  LibraryInfo._(Map<String, dynamic> j)
      : library = j['library'] as String,
        device = j['device'] as String,
        format = j['format'] as int,
        folder = j['folder'] as String;
  LibraryInfo._joined(LibraryInfo i, this.library)
      : device = i.device,
        format = i.format,
        folder = i.folder;
  LibraryInfo._withDevice(LibraryInfo i, this.device)
      : library = i.library,
        format = i.format,
        folder = i.folder;
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
        changed = j['changed'] as bool,
        head = (j['head'] as String?) ?? j['rev'] as String;
  final String doc;
  final String rev;
  final int hlc;

  /// 저장 뒤 지금 머리 — 보통 [rev]. 충돌 중이던 문서를 고쳐 다른 잎과 저절로 합쳤으면 합친 판이니, 편집기는 그 판의 본문으로 다시 맞춘다.
  final String head;

  /// 거짓이면 본문이 같아 판을 더하지 않았다.
  final bool changed;
}

class Renamed {
  Renamed._(Map<String, dynamic> j)
      : rev = j['rev'] as String,
        hlc = j['hlc'] as int,
        redirect = j['redirect'] as String?,
        head = (j['head'] as String?) ?? j['rev'] as String;
  final String rev;
  final int hlc;

  /// [Saved.head]와 같다.
  final String head;

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
        body = j['body'] as String,
        conflict = [for (final r in (j['conflict'] as List?) ?? const []) r as String];
  final String id;
  final Namespace ns;
  final String title;
  final String head;
  final int created;
  final int updated;
  final bool deleted;
  final String body;

  /// 충돌이면 머리가 아닌 다른 잎 판들(새 것부터) — 견주어 합쳐 [Library.save]의 `merge`로 푼다.
  final List<String> conflict;
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

/// `doc.open`의 답 — 머리 판의 그리기 모델(spec/namumark.md).
class Opened {
  Opened._(Map<String, dynamic> j)
      : doc = DocSummary._(j['doc'] as Map<String, dynamic>),
        deleted = (j['doc'] as Map<String, dynamic>)['deleted'] as bool,
        model = j['model'] as Map<String, dynamic>,
        redirectedFrom = j['redirected_from'] == null ? null : DocSummary._(j['redirected_from'] as Map<String, dynamic>),
        stale = j['stale'] as bool,
        conflict = [for (final r in (j['conflict'] as List?) ?? const []) r as String];
  final DocSummary doc;
  final bool deleted;
  final Map<String, dynamic> model;

  /// 넘겨주기를 따라왔으면 처음 연 문서.
  final DocSummary? redirectedFrom;

  /// 색인이 낡아 이번에 새로 그렸다.
  final bool stale;

  /// 충돌이면 머리가 아닌 다른 잎 판들 — [DocRecord.conflict]와 같다.
  final List<String> conflict;
}

/// 본문 찾기 한 건.
class SearchHit {
  SearchHit._(Map<String, dynamic> j)
      : doc = DocSummary._(j['doc'] as Map<String, dynamic>),
        snippet = j['snippet'] as String;
  final DocSummary doc;

  /// 첫 맞은 곳 앞뒤 글자(표식 없음).
  final String snippet;
}

/// 분류 폴더 하나(`tree.children`의 하위 분류).
class Folder {
  Folder._(Map<String, dynamic> j)
      : name = j['name'] as String,
        total = j['total'] as int,
        hasDoc = j['has_doc'] as bool,
        cycle = j['cycle'] as bool;
  final String name;

  /// 그 분류와 아래 분류가 집인 일반 문서 수.
  final int total;

  /// 분류 문서가 있다(없으면 「상위 분류 없음」).
  final bool hasDoc;

  /// 고리를 끊으려고 부모 줄을 끊었다.
  final bool cycle;
}

class FolderPage {
  FolderPage._(Map<String, dynamic> j)
      : categories = [for (final c in j['categories'] as List) Folder._(c as Map<String, dynamic>)],
        docs = [for (final d in j['docs'] as List) DocSummary._(d as Map<String, dynamic>)],
        uncategorized = j['uncategorized'] as int?,
        cycles = [for (final c in j['cycles'] as List) c as String];
  final List<Folder> categories;
  final List<DocSummary> docs;

  /// 맨 위에서만 — 「분류 안 됨」 문서 수.
  final int? uncategorized;
  final List<String> cycles;
}

/// 없는 문서를 가리킨 링크.
class Wanted {
  Wanted._(Map<String, dynamic> j)
      : ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        refs = j['refs'] as int;
  final Namespace ns;
  final String title;
  final int refs;
}

class IndexStatus {
  IndexStatus._(Map<String, dynamic> j)
      : stale = j['stale'] as int,
        total = j['total'] as int,
        building = j['building'] as bool;

  /// 다시 색인해야 하는 문서 수 — 0이 될 때까지 [Library.indexStep]을 뒤에서 부른다.
  final int stale;
  final int total;
  final bool building;
}

/// 내보낸 묶음(`sync.export`).
class SyncExport {
  SyncExport._(Map<String, dynamic> j)
      : pack = base64.decode(j['pack'] as String),
        seq = j['seq'] as int,
        count = j['count'] as int,
        more = j['more'] as bool;
  final Uint8List pack;

  /// 묶음에 든 마지막 판의 `rev.seq`.
  final int seq;
  final int count;

  /// 상한에 닿아 남은 판이 있다 — 다음 번호로 이어서 낸다.
  final bool more;
}

class SyncStatus {
  SyncStatus._(Map<String, dynamic> j)
      : library = j['library'] as String,
        device = j['device'] as String,
        seq = j['seq'] as int,
        pending = j['pending'] as int,
        held = j['held'] as int,
        conflicts = j['conflicts'] as int;
  final String library;
  final String device;
  final int seq;

  /// `after_seq` 뒤에 이 기기가 만든 판 수.
  final int pending;
  final int held;

  /// 충돌로 남은 문서 수.
  final int conflicts;
}

class SyncApplied {
  SyncApplied._(Map<String, dynamic> j)
      : added = j['added'] as int,
        skipped = j['skipped'] as int,
        held = j['held'] as int,
        forked = j['forked'] as int,
        merged = j['merged'] as int,
        conflicts = [for (final d in j['conflicts'] as List) d as String],
        revived = [for (final d in j['revived'] as List) d as String];
  final int added;
  final int skipped;
  final int held;
  final int forked;
  final int merged;
  final List<String> conflicts;
  final List<String> revived;
}

/// 열린 서재 하나(spec/engine-api.md의 `library.*` · `doc.*` · `rev.*` · 색인 op · `sync.*`).
class Library {
  Library._(this._core, this._handle, this._info);

  final HistoryCore _core;
  final int _handle;
  LibraryInfo _info;
  LibraryInfo get info => _info;

  /// 앱 자료 폴더 [appDataDir] 아래 `HistoryCore/`에 서재를 연다(없으면 만든다). 그 폴더 바로 아래 옛 앱 파일은 건드리지 않는다.
  static Future<Library> open(HistoryCore core, String appDataDir) async {
    final r = _check(await core.call({'op': 'library.open', 'dir': appDataDir}));
    return Library._(core, r['handle'] as int, LibraryInfo._(r));
  }

  Future<Map<String, dynamic>> _call(String op, [Map<String, Object?> args = const {}]) async =>
      _check(await _core.call({'op': op, 'handle': _handle, ...args}));

  Future<Saved> create(Namespace ns, String title, String body, {String message = ''}) async =>
      Saved._(await _call('doc.create', {'ns': ns.name, 'title': title, 'body': body, 'message': message}));

  /// [merge]를 주면 견주어 합친 판 — 앞 판이 [base](머리)와 [merge](충돌의 다른 잎) 둘이다.
  Future<Saved> save(String doc, String base, String body, {String message = '', String? merge}) async =>
      Saved._(await _call('doc.save', {'doc': doc, 'base': base, 'body': body, 'message': message, 'merge': ?merge}));

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

  /// 저장하지 않은 글의 그리기 모델(편집기 미리보기). 틀은 이 서재에서 읽는다 — 모양은 spec/namumark.md.
  /// [ns]가 [Namespace.template]이면 틀 문서 자체로 보고 `@인자@`를 자리로 남긴다.
  Future<Map<String, dynamic>> render(String text, {String? title, Namespace ns = Namespace.doc}) async =>
      (await _call('namumark.parse', {'text': text, 'title': title, 'ns': ns.name}))['model'] as Map<String, dynamic>;

  Future<Counts> counts() async => Counts._(await _call('library.counts'));

  /// 머리 판의 그리기 모델. 넘겨주기면 [follow]일 때 한 번 따라간다. 없는 문서면 null.
  Future<Opened?> openDoc(String doc, {bool follow = true}) async {
    try {
      return Opened._(await _call('doc.open', {'doc': doc, 'follow': follow}));
    } on HistoryCoreError catch (e) {
      if (e.code == 'not_found') return null;
      rethrow;
    }
  }

  /// 있는 문서 ID — 같은 차례, 없으면 null(링크 색).
  Future<List<String?>> titlesExist(List<(Namespace, String)> titles) async => [
        for (final d in (await _call('titles.exist', {
          'titles': [for (final (ns, t) in titles) {'ns': ns.name, 'title': t}],
        }))['docs'] as List)
          d as String?,
      ];

  /// 제목이 [prefix]로 시작하는 문서(제목 차례).
  Future<List<DocSummary>> searchTitles(String prefix, {Namespace ns = Namespace.doc, int limit = 20}) async => [
        for (final d in (await _call('search.titles', {'prefix': prefix, 'ns': ns.name, 'limit': limit}))['docs'] as List)
          DocSummary._(d as Map<String, dynamic>),
      ];

  /// 본문 찾기 — 한글 · 한자 2글자 조각, 첫 [limit]건(문서 만든 차례).
  Future<List<SearchHit>> searchBody(String query, {int limit = 20}) async => [
        for (final h in (await _call('search.body', {'query': query, 'limit': limit}))['hits'] as List)
          SearchHit._(h as Map<String, dynamic>),
      ];

  /// 분류 폴더 하나. [category]가 null이면 맨 위, 빈 글이면 「분류 안 됨」. [after](제목)부터 이어 받는다.
  Future<FolderPage> treeChildren({String? category, int limit = 200, String? after}) async =>
      FolderPage._(await _call('tree.children', {'category': category, 'limit': limit, 'after': after}));

  /// 이 문서로 링크한 문서(제목 차례).
  Future<List<DocSummary>> linksBack(String doc, {int limit = 200}) async => [
        for (final d in (await _call('links.back', {'doc': doc, 'limit': limit}))['docs'] as List)
          DocSummary._(d as Map<String, dynamic>),
      ];

  /// 만들 문서 — 가리킨 문서 수 많은 차례.
  Future<List<Wanted>> linksWanted({int limit = 50}) async => [
        for (final w in (await _call('links.wanted', {'limit': limit}))['wanted'] as List)
          Wanted._(w as Map<String, dynamic>),
      ];

  /// 대문 모음(spec/engine-api.md 「home.aggregates」). 날은 기기 시간대로 자른다.
  Future<Map<String, dynamic>> home({int recent = 10, int wanted = 10, int days = 84}) async =>
      _call('home.aggregates', {'recent': recent, 'wanted': wanted, 'days': days, 'tz': DateTime.now().timeZoneOffset.inMinutes});

  /// 서재 통계(spec/engine-api.md 「stats.library」).
  Future<Map<String, dynamic>> stats() async => _call('stats.library', {'tz': DateTime.now().timeZoneOffset.inMinutes});

  /// 이 연표 항목을 잇는 문서.
  Future<List<DocSummary>> timelineDocs(String entity) async => [
        for (final d in (await _call('timeline.docs', {'entity': entity}))['docs'] as List)
          DocSummary._(d as Map<String, dynamic>),
      ];

  Future<IndexStatus> indexStatus() async => IndexStatus._(await _call('index.status'));

  /// 낡은 문서를 [maxMs] 동안 고친다. 남은 수를 돌려준다 — 0이 될 때까지 뒤에서 부른다.
  Future<int> indexStep({int maxMs = 200}) async => (await _call('index.step', {'max_ms': maxMs}))['stale'] as int;

  /// [afterSeq] 뒤에 이 기기가 만든 판을 묶음 [no]로 — 없으면 null(spec/drive-format.md 3절).
  Future<SyncExport?> syncExport({required int afterSeq, required int no}) async {
    final r = await _call('sync.export', {'after_seq': afterSeq, 'no': no});
    return r['pack'] == null ? null : SyncExport._(r);
  }

  /// 묶음을 든다 — 판 ID로 걸러 두 번 받아도 한 번만. 다른 서재의 묶음이면 [WrongLibraryError].
  Future<SyncApplied> syncApply(Uint8List pack) async => SyncApplied._(await _call('sync.apply', {'pack': base64.encode(pack)}));

  Future<SyncStatus> syncStatus({required int afterSeq}) async => SyncStatus._(await _call('sync.status', {'after_seq': afterSeq}));

  /// 드라이브의 서재 [library]에 합류한다(spec/drive-format.md 6절) — 이 기기의 문서는 그대로 남아 다음 내보내기에 모두 나간다.
  Future<void> join(String library) async {
    await _call('sync.join', {'library': library});
    _info = LibraryInfo._joined(_info, library);
  }

  /// 새 기기 ID — 자료 폴더 복사 · 되살린 ZIP으로 두 곳이 한 ID를 쓰게 됐을 때(spec/drive-format.md 6절). 옛 ID의 판은 물려받는다.
  Future<String> rotateDevice() async {
    final device = (await _call('sync.rotate'))['device'] as String;
    _info = LibraryInfo._withDevice(_info, device);
    return device;
  }

  /// 서재 원본의 한 때 사본과 읽어보기를 ZIP 하나로 [path]에.
  Future<void> backup(String path) async {
    await _call('library.backup', {'path': path});
  }

  Future<void> close() async {
    await _call('library.close');
  }
}

Map<String, dynamic> _check(Map<String, dynamic> r) {
  if (r['ok'] == true) return r;
  final message = r['error'] as String? ?? '알 수 없는 오류';
  final code = r['code'] as String?;
  if (code == 'stale_base') throw StaleBaseError(message, head: r['head'] as String);
  if (code == 'wrong_library') throw WrongLibraryError(message, found: r['found'] as String);
  throw HistoryCoreError(message, code: code);
}
