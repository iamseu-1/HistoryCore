use history_namumark::{plain_inline, Inline, ObjectRef, Style, NAMESPACES};

#[test]
fn a_bare_title_is_a_document() {
    assert_eq!(ObjectRef::parse("  임진왜란 "), Some(ObjectRef::new("문서", "임진왜란")));
}

#[test]
fn known_prefixes_become_namespaces_and_unknown_ones_stay_in_the_title() {
    assert_eq!(ObjectRef::parse("틀:해전 정보"), Some(ObjectRef::new("틀", "해전 정보")));
    assert_eq!(ObjectRef::parse("分類:x").unwrap().ns, "문서");
    assert_eq!(ObjectRef::parse("1:1 회담"), Some(ObjectRef::new("문서", "1:1 회담")));
    assert_eq!(ObjectRef::parse("字:鶴"), Some(ObjectRef::new("字", "鶴")));
    assert_eq!(ObjectRef::parse("연표 : 임진왜란"), Some(ObjectRef::new("연표", "임진왜란")));
}

#[test]
fn anchors_leading_colons_and_empty_text() {
    let r = ObjectRef::parse(":분류:고려#역사").unwrap();
    assert_eq!((r.ns.as_str(), r.key.as_str(), r.anchor.as_deref()), ("분류", "고려", Some("역사")));
    assert_eq!(ObjectRef::parse("임진왜란#").unwrap().anchor, None);
    for empty in ["", "   ", ":", "#개요", "분류:"] {
        assert_eq!(ObjectRef::parse(empty), None, "{empty:?}");
    }
}

#[test]
fn titles_are_nfc() {
    assert_eq!(ObjectRef::parse("\u{1100}\u{1161}").unwrap().key, "가");
    assert_eq!(ObjectRef::parse("\u{F900}").unwrap().key, "\u{8C48}");
}

#[test]
fn title_path_and_without_anchor() {
    let r = ObjectRef { ns: "틀".into(), key: "해전".into(), anchor: Some("인자".into()) };
    assert_eq!(r.title(), "틀:해전");
    assert_eq!(r.path(), "틀:해전");
    assert_eq!(ObjectRef::new("문서", "가").title(), "가");
    assert_eq!(r.without_anchor().anchor, None);
    assert!(ObjectRef { ns: String::new(), key: String::new(), anchor: Some("개요".into()) }.is_same_page());
}

#[test]
fn the_namespace_list_is_the_libraries() {
    assert_eq!(NAMESPACES, ["문서", "분류", "틀", "파일", "대문", "연표", "字", "책"]);
}

#[test]
fn plain_text_of_inline_nodes() {
    let nodes = vec![
        Inline::Text("가".into()),
        Inline::Styled { style: Style::Bold, children: vec![Inline::Text("나".into())] },
        Inline::Link { target: ObjectRef::new("틀", "해전"), label: None },
        Inline::Ruby { base: "漢字".into(), ruby: "한자".into() },
        Inline::Break,
        Inline::Param { name: "이름".into(), fallback: Some("미상".into()) },
        Inline::FootnoteRef { number: 1, name: None },
    ];
    assert_eq!(plain_inline(&nodes), "가나틀:해전漢字(한자) 미상");
}
