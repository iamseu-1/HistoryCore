//! 옛 Dart 해석기 검사(packages/core_engine/test/parser_test.dart)를 그대로 옮긴 정답지(설계 5-8 · 9-3).
use history_namumark::{parse, plain_inline, Align, Block, Document, Inline, ListStyle, ObjectRef, Options, Style};
use std::collections::BTreeMap;

fn doc(s: &str) -> Document {
    parse(s, &Options { current: Some("임진왜란".into()), ..Options::default() })
}

fn template(s: &str) -> Document {
    parse(s, &Options { template: true, current: Some("임진왜란".into()), ..Options::default() })
}

fn single(d: &Document) -> &Block {
    assert_eq!(d.blocks.len(), 1, "{:?}", d.blocks);
    &d.blocks[0]
}

fn inl(d: &Document) -> &[Inline] {
    match single(d) {
        Block::Paragraph { children, .. } => children,
        other => panic!("문단이 아님: {other:?}"),
    }
}

fn links(nodes: &[Inline]) -> Vec<&ObjectRef> {
    nodes.iter().filter_map(|n| if let Inline::Link { target, .. } = n { Some(target) } else { None }).collect()
}

// ---------------------------------------------------------------- 블록

#[test]
fn headings_get_numbers_and_source_ranges() {
    let src = "머리말\n== 개요 ==\n가\n=== 세부 ===\n나\n== 경과 ==\n다";
    let d = doc(src);
    let numbers: Vec<&str> = d.blocks.iter().filter_map(|b| if let Block::Heading { number, .. } = b { Some(number.as_str()) } else { None }).collect();
    assert_eq!(numbers, ["1", "1.1", "2"]);
    assert_eq!(d.sections.len(), 3);
    assert_eq!(&src[d.sections[0].start..d.sections[0].end], "== 개요 ==\n가\n=== 세부 ===\n나\n");
    assert_eq!(&src[d.sections[2].start..d.sections[2].end], "== 경과 ==\n다");
}

#[test]
fn folded_heading() {
    match single(&doc("==# 접힘 #==")) {
        Block::Heading { folded, children, .. } => {
            assert!(*folded);
            assert_eq!(plain_inline(children), "접힘");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn tables_with_colspan_rowspan_align_and_caption() {
    match single(&doc("|캡션|| a || b ||\n||<-2><:> 합침 ||\n||<|2> 세로 || c ||")) {
        Block::Table { caption, rows } => {
            assert_eq!(plain_inline(caption.as_ref().unwrap()), "캡션");
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[1].len(), 1);
            assert_eq!(rows[1][0].col_span, 2);
            assert_eq!(rows[1][0].align, Align::Center);
            assert_eq!(rows[2][0].row_span, 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_empty_cell_in_the_old_syntax_spans_into_the_next() {
    match single(&doc("|||| 두 칸 ||\n|| a || b ||")) {
        Block::Table { rows, .. } => {
            assert_eq!(rows[0].len(), 1);
            assert_eq!(rows[0][0].col_span, 2);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn list_depth_comes_from_leading_spaces() {
    match single(&doc(" * 하나\n  * 하나.하나\n * 둘\n")) {
        Block::List { style, items, .. } => {
            assert_eq!(*style, ListStyle::Bullet);
            assert_eq!(items.len(), 2);
            assert!(matches!(items[0].last(), Some(Block::List { .. })));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn numbered_list_start() {
    match single(&doc(" 1.#5 다섯")) {
        Block::List { style, start, .. } => assert_eq!((*style, *start), (ListStyle::Decimal, 5)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn fold_code_quote_and_rule() {
    let d = doc("{{{#!folding 더 보기\n안쪽 글\n}}}\n{{{#!syntax dart\nvoid main() {}\n}}}\n> 인용\n----");
    assert!(matches!(&d.blocks[0], Block::Fold { summary, .. } if summary == "더 보기"));
    assert!(matches!(&d.blocks[1], Block::Code { language: Some(l), text } if l == "dart" && text == "void main() {}"));
    assert!(matches!(&d.blocks[2], Block::Quote(_)));
    assert!(matches!(&d.blocks[3], Block::Rule));
}

#[test]
fn redirects_only_on_the_first_line() {
    assert_eq!(doc("#넘겨주기 임진왜란").redirect, Some(ObjectRef::new("문서", "임진왜란")));
    assert_eq!(doc("#redirect 틀:해전").redirect, Some(ObjectRef::new("틀", "해전")));
    assert_eq!(doc("글\n#넘겨주기 임진왜란").redirect, None);
}

#[test]
fn include_and_embed() {
    let d = doc("[include(틀:해전 정보, 이름=한산도, 날짜=1592\\, 7월)]\n[embed(연표:임진왜란, 범위=1592..1598)]");
    match &d.blocks[0] {
        Block::Include { target, args } => {
            assert_eq!(target, &ObjectRef::new("틀", "해전 정보"));
            let want: BTreeMap<String, String> = [("이름", "한산도"), ("날짜", "1592, 7월")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
            assert_eq!(args, &want);
        }
        other => panic!("{other:?}"),
    }
    match &d.blocks[1] {
        Block::Embed { target, options, .. } => {
            assert_eq!(target.ns, "연표");
            assert_eq!(options.get("범위").map(String::as_str), Some("1592..1598"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn comment_lines_are_dropped() {
    assert_eq!(doc("## 주석\n본문").blocks.len(), 1);
}

#[test]
fn unclosed_markers_stay_as_text() {
    assert_eq!(plain_inline(inl(&doc("'''굵게 [[링크 {{{코드"))), "'''굵게 [[링크 {{{코드");
}

#[test]
fn very_deep_nesting_still_stops() {
    let src = format!("{} 깊다", ">".repeat(80));
    let _ = doc(&src);
}

// ---------------------------------------------------------------- 인라인

#[test]
fn bold_italic_strike_superscript() {
    let d = doc("'''굵게''' ''기울임'' ~~취소~~ ^^위^^");
    let n = inl(&d);
    let styles: Vec<Style> = [0, 2, 4, 6].iter().map(|&i| if let Inline::Styled { style, .. } = &n[i] { *style } else { panic!("{:?}", n[i]) }).collect();
    assert_eq!(styles, [Style::Bold, Style::Italic, Style::Strike, Style::Superscript]);
}

#[test]
fn kinds_of_links() {
    let d = doc("[[이순신]] [[한산도 대첩|한산도]] [[#개요]] [[https://example.com|밖]] [[연표:임진왜란]] [[/하위]]");
    let n = inl(&d);
    let l = links(n);
    assert_eq!(l[0], &ObjectRef::new("문서", "이순신"));
    let label = n.iter().find_map(|x| if let Inline::Link { label: Some(lab), .. } = x { Some(lab) } else { None }).unwrap();
    assert_eq!(plain_inline(label), "한산도");
    assert!(l[2].is_same_page());
    assert_eq!(l[3].ns, "연표");
    assert_eq!(l[4].key, "임진왜란/하위");
    let urls: Vec<&String> = n.iter().filter_map(|x| if let Inline::Url { uri, .. } = x { Some(uri) } else { None }).collect();
    assert_eq!(urls, ["https://example.com"]);
}

#[test]
fn category_tags_leave_the_body_and_escaped_ones_are_links() {
    let d = doc("본문[[분류:조선]][[:분류:고려]]");
    assert_eq!(d.categories, ["조선"]);
    assert_eq!(links(inl(&d)), [&ObjectRef::new("분류", "고려")]);
}

#[test]
fn footnotes_and_reused_named_footnotes() {
    let d = doc("가[* 첫째] 나[*a 둘째] 다[*a]");
    assert_eq!(d.footnotes.iter().map(|f| f.number).collect::<Vec<_>>(), [1, 2]);
    let refs: Vec<u32> = inl(&d).iter().filter_map(|x| if let Inline::FootnoteRef { number, .. } = x { Some(*number) } else { None }).collect();
    assert_eq!(refs, [1, 2, 2]);
}

#[test]
fn ruby_size_and_raw() {
    let d = doc("[ruby(漢字, ruby=한자)] {{{+2 크게}}} {{{[[링크 아님]]}}}");
    let n = inl(&d);
    assert!(matches!(&n[0], Inline::Ruby { ruby, .. } if ruby == "한자"));
    assert!(matches!(&n[2], Inline::Size { step: 2, .. }));
    assert!(matches!(&n[4], Inline::Code(t) if t == "[[링크 아님]]"));
}

#[test]
fn color_syntax_keeps_only_the_text() {
    assert_eq!(plain_inline(inl(&doc("{{{#red 빨강}}}"))), "빨강");
}

#[test]
fn template_parameters_only_in_templates() {
    let d = template("@이름@ @지휘=미상@");
    let n = inl(&d);
    assert!(matches!(&n[0], Inline::Param { name, .. } if name == "이름"));
    assert!(matches!(&n[2], Inline::Param { fallback: Some(f), .. } if f == "미상"));
    assert!(!inl(&doc("메일 a@b.c@d")).iter().any(|x| matches!(x, Inline::Param { .. })));
}

#[test]
fn file_embeds() {
    let d = doc("[[파일:지도.png|width=320&align=center]]");
    match inl(&d) {
        [Inline::File { file, options }] => {
            assert_eq!(file.key, "지도.png");
            assert_eq!(options.get("width").map(String::as_str), Some("320"));
        }
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------- Rust로 옮기며 더한 것

#[test]
fn four_byte_characters_and_wide_spaces_beside_markers_do_not_break() {
    // Review Focus 2 — Rust는 바이트로 자른다. 표식 바로 옆의 𪄃 · 😀 · 전각 공백 · 결합 자모.
    let d = doc("𪄃'''鶴'''😀 [[𪄃|😀]] [*𪄃 각주] \\𪄃 {{{𪄃}}}\n\u{3000}* 전각\n  \u{3000}들여\n ㅎ\u{1161}");
    assert!(!d.blocks.is_empty());
    let t = doc("||𪄃||😀||\n||<-2>𪄃||");
    assert!(matches!(&t.blocks[0], Block::Table { .. }));
}

#[test]
fn crlf_is_normalized_and_offsets_point_into_the_normalized_source() {
    let d = doc("== 가 ==\r\n본문\r== 나 ==\r\n끝");
    assert_eq!(d.source, "== 가 ==\n본문\n== 나 ==\n끝");
    assert_eq!(&d.source[d.sections[1].start..d.sections[1].end], "== 나 ==\n끝");
}

#[test]
fn a_nested_footnote_gets_its_own_number() {
    let d = doc("가[* 겉[* 속]]");
    assert_eq!(d.footnotes.iter().map(|f| f.number).collect::<Vec<_>>(), [1, 2]);
}

#[test]
fn a_bare_table_border_does_not_crash() {
    let _ = doc("||");
    let _ = doc("|||");
}
