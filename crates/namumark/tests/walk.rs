use history_namumark::{anchor_blocks, out_links, parse, plain_document, LinkKind, Options};

fn doc(s: &str) -> history_namumark::Document {
    parse(s, &Options::default())
}

#[test]
fn links_are_taken_once_per_kind() {
    // 옛 parser_test.dart의 21번째 검사.
    let links = out_links(&doc("[[가]] [[가]] [[파일:다.png]]\n[include(틀:나)]\n#가 아님"));
    let got: Vec<(u8, String)> = links.iter().map(|l| (l.kind as u8, l.target.path())).collect();
    assert_eq!(got, [(0, "문서:가".to_string()), (3, "파일:다.png".to_string()), (1, "틀:나".to_string())]);
}

#[test]
fn links_inside_labels_footnotes_tables_and_folds_count_and_same_page_links_do_not() {
    let d = doc("[[가|[[나]]]] [[#개요]] [* [[다]]]\n|| [[라]] ||\n{{{#!folding 접기\n[[마#절]]\n}}}\n[embed(연표:임진왜란)]");
    let got: Vec<(LinkKind, String)> = out_links(&d).into_iter().map(|l| (l.kind, l.target.path())).collect();
    for want in ["문서:가", "문서:라", "문서:마", "문서:다"] {
        assert!(got.contains(&(LinkKind::Body, want.to_string())), "{want} 없음: {got:?}");
    }
    assert!(got.contains(&(LinkKind::Embed, "연표:임진왜란".to_string())));
    assert!(!got.iter().any(|(_, p)| p == ":"), "같은 문서 링크가 들어감");
    assert!(out_links(&doc("#넘겨주기 틀:새 이름")).iter().any(|l| l.kind == LinkKind::Redirect && l.target.path() == "틀:새 이름"));
}

#[test]
fn plain_text_of_a_document_for_search() {
    let d = doc("== 개요 ==\n'''임진왜란'''은   1592년[* 『선조실록』 26권.]\n\n\n|| 가 || 나 ||\n{{{#!folding 더 보기\n[ruby(漢字, ruby=한자)]\n}}}\n[include(틀:정보)]\n----");
    assert_eq!(plain_document(&d), "개요\n임진왜란은 1592년\n가 나\n더 보기\n漢字(한자)\n『선조실록』 26권.");
}

#[test]
fn anchors_point_to_their_top_level_block_and_the_first_one_wins() {
    let d = doc("첫 문단 [anchor(가)]\n\n{{{#!folding 접기\n안쪽 [anchor(나)]\n}}}\n\n다시 [anchor(가)]");
    let a = anchor_blocks(&d);
    assert_eq!(a.get("가"), Some(&0));
    assert_eq!(a.get("나"), Some(&1));
    assert_eq!(a.len(), 2);
}
