import 'package:flutter_test/flutter_test.dart';
import 'package:library_view/library_view.dart';

import 'support.dart';

void main() {
  test('정답지 「임진왜란」을 블록 · 각주 · 문단 · 분류로 읽는다', () {
    final m = fixture('doc/임진왜란');
    expect(m.blocks.map((b) => b.runtimeType), [
      TemplateBlock, TocBlock, HeadingBlock, ParaBlock, HeadingBlock, ParaBlock, TimelineStripBlock, //
      HeadingBlock, TableBlock, HeadingBlock, ParaBlock, HeadingBlock, FootnotesBlock,
    ]);
    expect(m.footnotes.length, 3);
    expect(m.sections.map((s) => s.number), ['1', '2', '2.1', '3', '4']);
    expect(m.categories, ['조선 선조', '전쟁', '16세기']);
    expect(m.templates, ['틀:전쟁 정보']);
    final strip = m.blocks[6] as TimelineStripBlock;
    expect((strip.entity, strip.range), ('01JB2X7K9M', '1592..1598'));
    final para = m.blocks[3] as ParaBlock;
    expect(para.inl[1], isA<HanInline>().having((h) => h.s, 's', '壬辰倭亂'));
    expect(para.inl[3], isA<FootnoteRefInline>().having((f) => f.n, 'n', 1));
    final link = para.inl[5] as LinkInline;
    expect((link.target.ns, link.target.key, link.target.title), ('문서', '정유재란', '정유재란'));
    expect(m.raw, fixtureJson('doc/임진왜란'));
  });

  test('넘겨주기 모델은 대상 주소를 갖고 블록이 없다', () {
    final m = fixture('doc/임진전쟁');
    expect(m.redirect?.title, '임진왜란');
    expect(m.blocks, isEmpty);
  });

  test('모르는 블록과 빠진 칸 — 던지지 않고 읽는다', () {
    final m = DocModel.parse({
      'version': 1,
      'blocks': [
        {'t': 'hologram', 'beam': 3},
        {'t': 'para'},
        {'t': 'heading', 'inl': [{'t': 'sparkle', 's': '반짝'}]},
        {'t': 'table', 'rows': [[{'inl': [{'t': 'text', 's': '칸'}]}]]},
        'not a map',
      ],
    });
    expect(m.blocks[0], isA<UnknownBlock>().having((b) => b.t, 't', 'hologram'));
    expect(m.blocks[1], isA<ParaBlock>().having((b) => b.inl, 'inl', isEmpty));
    final h = m.blocks[2] as HeadingBlock;
    expect((h.level, h.number), (2, ''));
    expect(h.inl.single, isA<UnknownInline>().having((i) => i.text, 'text', '반짝'));
    final cell = (m.blocks[3] as TableBlock).rows.single.single;
    expect((cell.col, cell.row, cell.align), (1, 1, CellAlign.start));
    expect(m.blocks[4], isA<UnknownBlock>());
    expect(m.footnotes, isEmpty);
    expect(m.categories, isEmpty);
  });

  test('spec/namumark.md 표의 블록 · 인라인을 모두 제 형으로 읽는다(모르는 것 없음)', () {
    final m = fixture('all_kinds');
    final blocks = <Type>{};
    final inlines = <Type>{};
    void inl(List<Inline> list) {
      for (final i in list) {
        inlines.add(i.runtimeType);
        switch (i) {
          case StyledInline(inl: final sub) || SizedInline(inl: final sub):
            inl(sub);
          case LinkInline(inl: final sub?) || UrlInline(inl: final sub?):
            inl(sub);
          default:
        }
      }
    }

    void walk(List<Block> list) {
      for (final b in list) {
        blocks.add(b.runtimeType);
        switch (b) {
          case HeadingBlock(inl: final i) || ParaBlock(inl: final i):
            inl(i);
          case ListBlock(:final items):
            items.forEach(walk);
          case QuoteBlock(blocks: final sub) || FoldBlock(blocks: final sub) || IndentBlock(blocks: final sub) || TemplateBlock(blocks: final sub):
            walk(sub);
          case TableBlock(:final caption, :final rows):
            if (caption != null) inl(caption);
            for (final r in rows) {
              for (final c in r) {
                inl(c.inl);
              }
            }
          default:
        }
      }
    }

    walk(m.blocks);
    for (final f in m.footnotes) {
      inl(f.inl);
    }
    expect(blocks, {
      HeadingBlock, ParaBlock, ListBlock, QuoteBlock, FoldBlock, IndentBlock, TableBlock, CodeBlock, RuleBlock, TocBlock, //
      FootnotesBlock, TemplateBlock, EmbedBlock, FrontMacroBlock, TimelineStripBlock, UnsupportedBlock,
    });
    expect(inlines, {
      TextInline, HanInline, CodeInline, StyledInline, SizedInline, LinkInline, UrlInline, FileInline, FootnoteRefInline, //
      RubyInline, BreakInline, MacroInline, ParamInline,
    });
    // 인라인 모양 여섯 · 목록 꼴 여섯 · 틀 상태 여섯이 모두 든다.
    final styles = <InlineStyle>{};
    void styled(List<Inline> list) {
      for (final i in list) {
        if (i case StyledInline(:final style, inl: final sub)) {
          styles.add(style);
          styled(sub);
        }
      }
    }

    for (final b in m.blocks.whereType<ParaBlock>()) {
      styled(b.inl);
    }
    expect(styles, InlineStyle.values.toSet());
    expect(m.blocks.whereType<ListBlock>().map((l) => l.style).toSet(), {'bullet', 'decimal', 'lower_alpha', 'upper_alpha', 'lower_roman', 'upper_roman'});
    expect(m.blocks.whereType<TemplateBlock>().map((t) => t.status).toSet(), {'ok', 'missing', 'cycle', 'too_deep', 'too_many', 'too_large'});
  });

  test('주소 — 일반 문서는 접두어 없이, 같은 문서 안은 samePage · 링크 대상은 처음 나온 차례', () {
    expect(const ObjectAddress('분류', '조선').title, '분류:조선');
    expect(const ObjectAddress('문서', '이순신').title, '이순신');
    expect(const ObjectAddress('', '', anchor: '결과').samePage, isTrue);
    expect(ObjectAddress.parse({'ns': '연표', 'key': '01JB2X7K9M', 'anchor': null}), const ObjectAddress('연표', '01JB2X7K9M'));
    expect(ObjectAddress.parse('글'), isNull);
    final targets = linkTargets(fixture('doc/임진왜란')).map((a) => a.title).toList();
    // 정보 상자 틀 속 연표 링크가 맨 앞 — 본문 링크는 그 뒤.
    expect(targets.first, '연표:01JB2X7K9M');
    expect(targets, contains('정유재란'));
    expect(linkTargets(fixture('doc/임진전쟁')).single.title, '임진왜란');
  });
}
