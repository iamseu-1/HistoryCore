//! 깊거나 이상한 입력에서도 멈추지 않고, 스택이 넘치지 않고, 정해진 시간 안에 끝난다(Review Focus 1 · 2 · 5, 설계 9-3 퍼징).
use history_corpus::{Corpus, CorpusSpec, Ns, Rng, TextPool};
use history_namumark::{parse, plain_inline, render, Block, Options, RenderOptions};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// 시간 한계 — 디버그 짓기는 느리고 CI는 붐벼 "멈추지 않는다"만 넉넉히(30초) 보고,
/// 빠르기는 릴리스로 3초 안을 본다(CI가 `--release`로 이 파일을 한 번 더 돌린다 — 최종 검토 3).
const LIMIT: Duration = if cfg!(debug_assertions) { Duration::from_secs(30) } else { Duration::from_secs(3) };

fn timed(label: &str, src: &str) -> history_namumark::Document {
    let t = Instant::now();
    let d = parse(src, &Options::default());
    let took = t.elapsed();
    assert!(took < LIMIT, "{label}: {took:?} 걸림(입력 {}바이트)", src.len());
    d
}

/// 블록 안의 모든 글자(검사용).
fn all_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            Block::Paragraph { children, .. } | Block::Heading { children, .. } => out.push_str(&plain_inline(children)),
            Block::List { items, .. } => items.iter().for_each(|i| out.push_str(&all_text(i))),
            Block::Quote(c) | Block::Indent { children: c, .. } | Block::Fold { children: c, .. } => out.push_str(&all_text(c)),
            Block::Code { text, .. } | Block::Unsupported { text, .. } => out.push_str(text),
            _ => {}
        }
    }
    out
}

#[test]
fn five_thousand_nested_list_levels_do_not_overflow_the_stack() {
    // 옛 Dart의 목록 쌓기는 깊이를 재지 않았다.
    // 들여쓰기가 단마다 늘어 입력이 12.5MB다.
    let src: String = (1..=5_000).map(|k| format!("{}* {k}", " ".repeat(k))).collect::<Vec<_>>().join("\n");
    let d = timed("5,000단 목록", &src);
    assert!(matches!(d.blocks.first(), Some(Block::List { .. })));
    assert!(all_text(&d.blocks).contains("5000"));
}

#[test]
fn tens_of_thousands_of_unclosed_braces_on_one_line() {
    let src = "{{{".repeat(20_000);
    let d = timed("닫히지 않은 {{{ × 20,000", &src);
    assert_eq!(all_text(&d.blocks).matches("{{{").count(), 20_000);
}

#[test]
fn tens_of_thousands_of_unclosed_links_on_one_line() {
    let src = "[[가".repeat(20_000);
    let d = timed("닫히지 않은 [[ × 20,000", &src);
    assert_eq!(all_text(&d.blocks).matches("[[").count(), 20_000);
}

#[test]
fn tens_of_thousands_of_unclosed_footnotes_on_one_line() {
    let src = "[* 가".repeat(20_000);
    let d = timed("닫히지 않은 [* × 20,000", &src);
    assert!(d.footnotes.is_empty());
    assert_eq!(all_text(&d.blocks).matches("[*").count(), 20_000);
}

#[test]
fn tens_of_thousands_of_lines_opening_unclosed_brace_blocks() {
    let src = vec!["{{{가"; 20_000].join("\n");
    let d = timed("{{{ 줄 × 20,000", &src);
    assert!(all_text(&d.blocks).contains("가"));
}

#[test]
fn macro_looking_text_without_a_closer() {
    let src = "[a(".repeat(10_000);
    let _ = timed("[a( × 10,000", &src);
    let at = format!("@a={}", "가".repeat(30_000));
    let t = Instant::now();
    let _ = parse(&at.repeat(20), &Options { template: true, ..Options::default() });
    assert!(t.elapsed() < LIMIT);
}

#[test]
fn a_normal_long_document_keeps_all_its_markup() {
    // 예산은 정상 문서를 막지 않는다 — 0.6MB 문서의 표식이 끝까지 해석된다.
    let unit = "== 장 ==\n'''굵게''' [[임진왜란|전쟁]][* 『선조실록』 1권.] {{{+1 크게}}} ~~취소~~\n || 가 || 나 ||\n";
    let src = unit.repeat(6_000);
    let d = timed("정상 문서 0.6MB", &src);
    assert_eq!(d.footnotes.len(), 6_000);
    assert!(!all_text(&d.blocks).contains("'''"));
}

#[test]
fn random_markup_soup_never_panics() {
    // 설계 9-3 퍼징 — 정해진 씨앗의 뒤섞인 표식 5,000편(틀 모드까지 10,000번). 실패하면 그 번호와 글을 보인다.
    const TOKENS: [&str; 56] = [
        "[[", "]]", "{{{", "}}}", "||", "|", "'''", "''", "~~", "--", "^^", ",,", "__", "[*", "]", "[", "(", ")", "\n", "\n * ",
        "\n  1.#3 ", "\n== 가 ==\n", "\n==# 접힘 #==\n", "\n> ", "\\", "@x@", "@y=기본@", "#넘겨주기 ", "[include(틀:가, a=1)]",
        "[embed(연표:x)]", "[ruby(漢, ruby=한)]", "[br]", "[목차]", "{{{#!folding 접기\n", "{{{#!syntax rust\n", "{{{+2 ", "{{{#red ",
        "<-2>", "<|2>", "<:>", "가", "鶴", "𪄃", "😀", "\u{3000}", "\u{1100}", "\u{1161}", " ", "임진왜란", "분류:조선", "파일:a.png",
        "연표:x", "#개요", "https://a.b", "\r\n", "## 주석\n",
    ];
    // 그리기 모델까지 — 틀 채우기 · 넘겨주기 · 각주 다시 매김 · UTF-16 위치(최종 검토 4).
    let templates: BTreeMap<String, String> = [
        ("틀:가", "[[@a@]] @b=기본@[* 틀 각주 @a@]\n[include(틀:나, a=@a@@a@)]\n[[분류:틀 분류]]"),
        ("틀:나", "#넘겨주기 틀:다"),
        ("틀:다", "== @a@ ==\n || @a@ || [*x 다] ||\n[include(틀:가, a=@a@)]"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let mut rng = Rng::new(20261021);
    for case in 0..5_000 {
        let n = 1 + rng.below(300) as usize;
        let src: String = (0..n).map(|_| *rng.pick(&TOKENS)).collect();
        for template in [false, true] {
            let r = std::panic::catch_unwind(|| {
                let d = parse(&src, &Options { template, current: Some("가/나".into()), ..Options::default() });
                let _ = all_text(&d.blocks);
                let m = render(&src, &RenderOptions { title: Some("가/나".into()), template }, &templates);
                let units = d.source.encode_utf16().count();
                assert!(m.sections.iter().all(|s| s.start <= s.end && s.end <= units), "{:?}", m.sections);
            });
            assert!(r.is_ok(), "{case}번째(틀 {template})에서 멈춤: {src:?}");
        }
    }
}

#[test]
fn dummy_library_documents_read_their_home_category() {
    // Review Focus 5 — P1 더미 서재 문서의 첫 [[분류:…]]가 집으로 읽힌다(설계 5-3).
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 500, seed: 7 }, &pool);
    for d in c.docs().filter(|d| d.ns == Ns::Doc) {
        let parsed = parse(&d.body, &Options::default());
        assert_eq!(parsed.categories.first(), d.home.as_ref(), "{}", d.title);
    }
}

#[test]
fn the_same_text_always_gives_the_same_tree() {
    let src = "== 가 ==\n[[나]][* 다] {{{#!folding 라\n마\n}}}\n || 바 || 사 ||";
    assert_eq!(parse(src, &Options::default()), parse(src, &Options::default()));
}
