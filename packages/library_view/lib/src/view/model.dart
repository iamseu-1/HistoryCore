// 그리기 모델 판 1(HistoryCore spec/namumark.md) → Dart 값. 엔진이 나중에 더한 블록 · 인라인이나 빠진 칸에도 던지지 않는다 —
// 모르는 것은 UnknownBlock · UnknownInline으로 읽어 그 자리에 글로 보인다(Review Focus 2).
// 이 패키지(library_view)는 엔진(history_core)에 기대지 않는다 — 모델 JSON과 DocContext의 콜백만 받는다(P5 R8).
// 아카이브 lib/src/view에서 옮겨 왔다(P5 걸음 4) — 이름은 그대로라 아카이브는 import만 바꾼다.

/// 링크 · 틀 · 끼우기의 대상. `ns`는 모델의 접두어(`문서` `분류` `틀` `파일` `대문` `연표` `字` `책`).
class ObjectAddress {
  const ObjectAddress(this.ns, this.key, {this.anchor});
  final String ns;
  final String key;
  final String? anchor;

  /// 같은 문서 안의 문단.
  bool get samePage => key.isEmpty;

  /// 화면에 보이는 제목 — 일반 문서는 접두어를 붙이지 않는다.
  String get title => ns == '문서' ? key : '$ns:$key';

  static ObjectAddress? parse(Object? j) {
    if (j is! Map) return null;
    return ObjectAddress(_s(j['ns']), _s(j['key']), anchor: j['anchor'] as String?);
  }

  @override
  bool operator ==(Object other) => other is ObjectAddress && other.ns == ns && other.key == key && other.anchor == anchor;

  @override
  int get hashCode => Object.hash(ns, key, anchor);
}

class Footnote {
  const Footnote(this.n, this.name, this.inl);
  final int n;
  final String? name;
  final List<Inline> inl;
}

class Section {
  const Section({required this.index, required this.level, required this.number, required this.title, required this.start, required this.end});
  final int index;
  final int level;
  final String number;
  final String title;

  /// 원문의 UTF-16 칸 위치 — 문단 편집이 이 범위를 자른다.
  final int start;
  final int end;
}

class DocModel {
  DocModel._(this.raw, this.redirect, this.blocks, this.footnotes, this.sections, this.categories, this.templateCategories, this.templates);

  /// 엔진이 준 JSON 그대로.
  final Map<String, dynamic> raw;
  final ObjectAddress? redirect;
  final List<Block> blocks;
  final List<Footnote> footnotes;
  final List<Section> sections;

  /// 본문에 적은 분류 — 첫 것이 집(설계 5-3).
  final List<String> categories;
  final List<String> templateCategories;
  final List<String> templates;

  static DocModel parse(Map<String, dynamic> j) => DocModel._(
        j,
        ObjectAddress.parse(j['redirect']),
        _blocks(j['blocks']),
        [
          for (final f in _l(j['footnotes']))
            if (f is Map) Footnote(_i(f['n']), f['name'] as String?, _inlines(f['inl'])),
        ],
        [
          for (final s in _l(j['sections']))
            if (s is Map)
              Section(index: _i(s['index']), level: _i(s['level'], 2), number: _s(s['number']), title: _s(s['title']), start: _i(s['start']), end: _i(s['end'])),
        ],
        _strings(j['categories']),
        _strings(j['template_categories']),
        _strings(j['templates']),
      );
}

sealed class Block {
  const Block();
}

class HeadingBlock extends Block {
  const HeadingBlock(this.level, this.number, this.section, this.folded, this.inl);
  final int level;
  final String number;

  /// 맨 위 단계 문단 차례(틀 · 인용 안은 null) — 「편집」이 이 문단을 연다.
  final int? section;
  final bool folded;
  final List<Inline> inl;
}

class ParaBlock extends Block {
  const ParaBlock(this.inl);
  final List<Inline> inl;
}

class ListBlock extends Block {
  const ListBlock(this.style, this.start, this.items);

  /// `bullet` `decimal` `lower_alpha` `upper_alpha` `lower_roman` `upper_roman`.
  final String style;
  final int start;
  final List<List<Block>> items;
}

class QuoteBlock extends Block {
  const QuoteBlock(this.blocks);
  final List<Block> blocks;
}

class FoldBlock extends Block {
  const FoldBlock(this.summary, this.blocks);
  final String summary;
  final List<Block> blocks;
}

class IndentBlock extends Block {
  const IndentBlock(this.depth, this.blocks);
  final int depth;
  final List<Block> blocks;
}

enum CellAlign { start, center, end }

class DocCell {
  const DocCell(this.col, this.row, this.align, this.inl);

  /// 가로 · 세로로 합친 칸 수.
  final int col;
  final int row;
  final CellAlign align;
  final List<Inline> inl;
}

class TableBlock extends Block {
  const TableBlock(this.caption, this.rows);
  final List<Inline>? caption;
  final List<List<DocCell>> rows;
}

class CodeBlock extends Block {
  const CodeBlock(this.lang, this.text);
  final String? lang;
  final String text;
}

class RuleBlock extends Block {
  const RuleBlock();
}

class TocBlock extends Block {
  const TocBlock();
}

class FootnotesBlock extends Block {
  const FootnotesBlock();
}

class TemplateBlock extends Block {
  const TemplateBlock(this.target, this.status, this.blocks);
  final ObjectAddress target;

  /// `ok` `missing` `cycle` `too_deep` `too_many` `too_large`.
  final String status;
  final List<Block> blocks;
}

class EmbedBlock extends Block {
  const EmbedBlock(this.target, this.options);
  final ObjectAddress target;
  final Map<String, String> options;
}

/// 대문 매크로 자리(설계 8-2) — 앱 위젯이 채운다.
class FrontMacroBlock extends Block {
  const FrontMacroBlock(this.name, this.arg);
  final String name;
  final String? arg;
}

class TimelineStripBlock extends Block {
  const TimelineStripBlock(this.entity, this.range);
  final String entity;

  /// `1592..1598`.
  final String? range;
}

class UnsupportedBlock extends Block {
  const UnsupportedBlock(this.kind, this.text);
  final String kind;
  final String text;
}

/// 이 앱이 모르는 블록 — 엔진이 나중에 더한 것.
class UnknownBlock extends Block {
  const UnknownBlock(this.t, this.raw);
  final String t;
  final Object? raw;

  /// 그 안의 글(있으면) — 셋째 글자로 보인다.
  String get text => _textOf(raw);
}

sealed class Inline {
  const Inline();
}

class TextInline extends Inline {
  const TextInline(this.s);
  final String s;
}

/// 한자 구간 — 누르면 자전 카드.
class HanInline extends Inline {
  const HanInline(this.s);
  final String s;
}

class CodeInline extends Inline {
  const CodeInline(this.s);
  final String s;
}

enum InlineStyle { bold, italic, underline, strike, sup, sub }

class StyledInline extends Inline {
  const StyledInline(this.style, this.inl);
  final InlineStyle style;
  final List<Inline> inl;
}

class SizedInline extends Inline {
  const SizedInline(this.step, this.inl);

  /// -5..+5.
  final int step;
  final List<Inline> inl;
}

class LinkInline extends Inline {
  const LinkInline(this.target, this.inl);
  final ObjectAddress target;

  /// null이면 제목을 보인다.
  final List<Inline>? inl;
}

class UrlInline extends Inline {
  const UrlInline(this.href, this.inl);
  final String href;
  final List<Inline>? inl;
}

class FileInline extends Inline {
  const FileInline(this.key, this.options);
  final String key;
  final Map<String, String> options;
}

class FootnoteRefInline extends Inline {
  const FootnoteRefInline(this.n);
  final int n;
}

class RubyInline extends Inline {
  const RubyInline(this.base, this.ruby);
  final String base;
  final String ruby;
}

class BreakInline extends Inline {
  const BreakInline();
}

class MacroInline extends Inline {
  const MacroInline(this.name, this.args);

  /// `date` `age` `dday` `anchor`.
  final String name;
  final List<String> args;
}

/// 틀 인자 자리 — 틀 문서 자체를 볼 때만.
class ParamInline extends Inline {
  const ParamInline(this.name, this.fallback);
  final String name;
  final String? fallback;
}

class UnknownInline extends Inline {
  const UnknownInline(this.t, this.raw);
  final String t;
  final Object? raw;
  String get text => _textOf(raw);
}

List<Block> _blocks(Object? j) => [for (final b in _l(j)) _block(b)];

Block _block(Object? j) {
  if (j is! Map) return UnknownBlock('', j);
  final t = _s(j['t']);
  return switch (t) {
    'heading' => HeadingBlock(_i(j['level'], 2), _s(j['number']), j['section'] is int ? j['section'] as int : null, j['folded'] == true, _inlines(j['inl'])),
    'para' => ParaBlock(_inlines(j['inl'])),
    'list' => ListBlock(_s(j['style'], 'bullet'), _i(j['start'], 1), [for (final item in _l(j['items'])) _blocks(item)]),
    'quote' => QuoteBlock(_blocks(j['blocks'])),
    'fold' => FoldBlock(_s(j['summary']), _blocks(j['blocks'])),
    'indent' => IndentBlock(_i(j['depth'], 1), _blocks(j['blocks'])),
    'table' => TableBlock(
        j['caption'] == null ? null : _inlines(j['caption']),
        [
          for (final row in _l(j['rows']))
            [
              for (final c in _l(row))
                if (c is Map)
                  DocCell(_i(c['col'], 1), _i(c['row'], 1), switch (c['align']) { 'center' => CellAlign.center, 'end' => CellAlign.end, _ => CellAlign.start },
                      _inlines(c['inl'])),
            ],
        ],
      ),
    'code' => CodeBlock(j['lang'] as String?, _s(j['text'])),
    'rule' => const RuleBlock(),
    'toc' => const TocBlock(),
    'footnotes' => const FootnotesBlock(),
    'template' => TemplateBlock(ObjectAddress.parse(j['target']) ?? const ObjectAddress('틀', ''), _s(j['status'], 'ok'), _blocks(j['blocks'])),
    'embed' => EmbedBlock(ObjectAddress.parse(j['target']) ?? const ObjectAddress('문서', ''), _stringMap(j['options'])),
    'front' => FrontMacroBlock(_s(j['name']), j['arg'] as String?),
    'timeline_strip' => TimelineStripBlock(_s(j['entity']), j['range'] as String?),
    'unsupported' => UnsupportedBlock(_s(j['kind']), _s(j['text'])),
    _ => UnknownBlock(t, j),
  };
}

List<Inline> _inlines(Object? j) => [for (final i in _l(j)) _inline(i)];

Inline _inline(Object? j) {
  if (j is! Map) return UnknownInline('', j);
  final t = _s(j['t']);
  return switch (t) {
    'text' => TextInline(_s(j['s'])),
    'han' => HanInline(_s(j['s'])),
    'code' => CodeInline(_s(j['s'])),
    'style' => switch (InlineStyle.values.where((s) => s.name == j['style']).firstOrNull) {
        final s? => StyledInline(s, _inlines(j['inl'])),
        null => UnknownInline(t, j),
      },
    'size' => SizedInline(_i(j['step']), _inlines(j['inl'])),
    'link' => LinkInline(ObjectAddress.parse(j['target']) ?? const ObjectAddress('문서', ''), j['inl'] == null ? null : _inlines(j['inl'])),
    'url' => UrlInline(_s(j['href']), j['inl'] == null ? null : _inlines(j['inl'])),
    'file' => FileInline(_s(j['key']), _stringMap(j['options'])),
    'fn' => FootnoteRefInline(_i(j['n'])),
    'ruby' => RubyInline(_s(j['base']), _s(j['ruby'])),
    'br' => const BreakInline(),
    'macro' => MacroInline(_s(j['name']), _strings(j['args'])),
    'param' => ParamInline(_s(j['name']), j['fallback'] as String?),
    _ => UnknownInline(t, j),
  };
}

List<Object?> _l(Object? j) => j is List ? j : const [];
String _s(Object? j, [String fallback = '']) => j is String ? j : fallback;
int _i(Object? j, [int fallback = 0]) => j is int ? j : fallback;
List<String> _strings(Object? j) => [for (final s in _l(j)) if (s is String) s];
Map<String, String> _stringMap(Object? j) => j is Map ? {for (final e in j.entries) '${e.key}': '${e.value}'} : const {};

/// 모르는 것 속의 글 — `s` · `text` · `inl` · `blocks`를 따라 모은다.
String _textOf(Object? j) {
  if (j is String) return j;
  if (j is List) return j.map(_textOf).join();
  if (j is Map) {
    if (j['s'] is String) return j['s'] as String;
    if (j['text'] is String) return j['text'] as String;
    return [j['inl'], j['blocks']].map(_textOf).join();
  }
  return '';
}

/// 문서가 가리키는 대상 — 본문 링크(펼친 틀 · 표 · 각주 안까지)와 넘겨주기. 처음 나온 차례.
Iterable<ObjectAddress> linkTargets(DocModel m) sync* {
  if (m.redirect case final r?) yield r;
  Iterable<ObjectAddress> inl(List<Inline> list) sync* {
    for (final i in list) {
      switch (i) {
        case LinkInline(:final target, inl: final sub):
          yield target;
          if (sub != null) yield* inl(sub);
        case StyledInline(inl: final sub) || SizedInline(inl: final sub):
          yield* inl(sub);
        case UrlInline(inl: final sub?):
          yield* inl(sub);
        default:
      }
    }
  }

  Iterable<ObjectAddress> blocks(List<Block> list) sync* {
    for (final b in list) {
      switch (b) {
        case HeadingBlock(inl: final i) || ParaBlock(inl: final i):
          yield* inl(i);
        case ListBlock(:final items):
          for (final item in items) {
            yield* blocks(item);
          }
        case QuoteBlock(blocks: final sub) || FoldBlock(blocks: final sub) || IndentBlock(blocks: final sub) || TemplateBlock(blocks: final sub):
          yield* blocks(sub);
        case TableBlock(:final caption, :final rows):
          if (caption != null) yield* inl(caption);
          for (final row in rows) {
            for (final c in row) {
              yield* inl(c.inl);
            }
          }
        default:
      }
    }
  }

  yield* blocks(m.blocks);
  for (final f in m.footnotes) {
    yield* inl(f.inl);
  }
}
