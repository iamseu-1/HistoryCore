use history_namumark::{parse, plain_inline, timeline_entities, Block, Inline, Options};

fn blocks(s: &str) -> Vec<Block> {
    parse(s, &Options::default()).blocks
}

#[test]
fn every_front_page_macro_on_its_own_line_is_a_block() {
    let names = ["주제 바로가기", "오늘의 문서", "오늘의 역사", "알고 계셨나요", "최근 바뀐 문서", "만들 문서", "쓰는 리듬", "최근 찾은 한자"];
    for name in names {
        assert_eq!(blocks(&format!("[{name}]")), [Block::Front { name: name.to_string(), arg: None }], "{name}");
    }
    assert_eq!(blocks("[최근 바뀐 문서(10)]"), [Block::Front { name: "최근 바뀐 문서".into(), arg: Some("10".into()) }]);
    assert_eq!(blocks("  [쓰는 리듬(주)]  ")[0], Block::Indent { depth: 2, children: vec![Block::Front { name: "쓰는 리듬".into(), arg: Some("주".into()) }] });
}

#[test]
fn a_front_macro_line_ends_the_paragraph_before_it() {
    let b = blocks("소개 글\n[오늘의 문서]\n뒷글");
    assert_eq!(b.len(), 3);
    assert!(matches!(&b[1], Block::Front { name, .. } if name == "오늘의 문서"));
}

#[test]
fn the_same_words_inside_a_sentence_stay_text() {
    match &blocks("어제 [오늘의 문서]를 봤다")[..] {
        [Block::Paragraph { children, .. }] => assert_eq!(plain_inline(children), "어제 [오늘의 문서]를 봤다"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(&blocks("[아무거나]")[..], [Block::Paragraph { .. }]));
}

#[test]
fn timeline_strips() {
    assert_eq!(blocks("[연표(01JB2X, 범위=1592..1598)]"), [Block::TimelineStrip { entity: "01JB2X".into(), range: Some("1592..1598".into()) }]);
    assert_eq!(blocks("[연표(01JB2X)]"), [Block::TimelineStrip { entity: "01JB2X".into(), range: None }]);
    assert!(matches!(&blocks("[연표()]")[..], [Block::Paragraph { .. }]));
}

#[test]
fn a_strip_entity_is_nfc_like_a_timeline_link() {
    // 링크 `[[연표:…]]`는 주소 해석에서 NFC로 맞춘다 — 띠도 같게 해야 색인 · 창구가 같은 항목ID로 찾는다(spec/windows-api.md 8절).
    use unicode_normalization::UnicodeNormalization;
    let nfd: String = "한국/세종".nfd().collect();
    assert_ne!(nfd, "한국/세종");
    assert_eq!(blocks(&format!("[연표({nfd}, 범위=1418..1450)]")), [Block::TimelineStrip { entity: "한국/세종".into(), range: Some("1418..1450".into()) }]);
    let d = parse(&format!("[[연표:{nfd}]]\n[연표({nfd})]"), &Options::default());
    assert_eq!(timeline_entities(&d), ["한국/세종"]);
}

#[test]
fn timeline_entities_from_links_and_strips_once_each_in_order() {
    let d = parse("[[연표:01A|임진왜란]] [[연표:01B]]\n[연표(01C, 범위=1592..1598)]\n[[연표:01A]] [[임진왜란]]", &Options::default());
    assert_eq!(timeline_entities(&d), ["01A", "01B", "01C"]);
    let link = match &d.blocks[0] {
        Block::Paragraph { children, .. } => children.iter().find_map(|n| if let Inline::Link { target, .. } = n { Some(target.clone()) } else { None }).unwrap(),
        other => panic!("{other:?}"),
    };
    assert_eq!((link.ns.as_str(), link.key.as_str()), ("연표", "01A"));
}
