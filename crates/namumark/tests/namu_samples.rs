//! 나무위키식 견본(설계 9-3) — 상자 · 접기 · 표가 많은 긴 문서도 끝까지 해석된다.
use history_namumark::{out_links, parse, Block, Document, Options};

fn doc(src: &str) -> Document {
    parse(src, &Options::default())
}

/// 블록 나무에서 `f`에 맞는 블록 수.
fn count(blocks: &[Block], f: &dyn Fn(&Block) -> bool) -> usize {
    blocks
        .iter()
        .map(|b| {
            let inner = match b {
                Block::List { items, .. } => items.iter().map(|i| count(i, f)).sum(),
                Block::Quote(c) | Block::Indent { children: c, .. } | Block::Fold { children: c, .. } => count(c, f),
                _ => 0,
            };
            inner + usize::from(f(b))
        })
        .sum()
}

fn links(d: &Document) -> Vec<String> {
    out_links(d).into_iter().map(|l| l.target.key).collect()
}

#[test]
fn hundreds_of_folds_leave_the_markup_after_them_intact() {
    // 최종 검토 1 — 여러 줄 `{{{` 블록마다 문서 끝까지를 이어 예산에서 빼던 탓에, 접기가 많은 긴 문서는 뒤의 링크 · 각주 · 분류가 글자가 됐다.
    let body = "임진왜란 때의 일. ".repeat(60);
    let mut src = String::new();
    for j in 0..300 {
        src.push_str(&format!("== 장 {j} ==\n"));
        src.push_str(&format!("{}#!folding 접기 {j}\n{body}\n{}\n", "{{{", "}}}"));
    }
    src.push_str("[[임진왜란]] 끝[* 각주]\n[[분류:조선]]");
    let d = doc(&src);
    assert_eq!(count(&d.blocks, &|b| matches!(b, Block::Fold { .. })), 300);
    assert_eq!(d.categories, ["조선"]);
    assert_eq!(d.footnotes.len(), 1);
    assert!(links(&d).contains(&"임진왜란".to_string()));
}

#[test]
fn hundreds_of_style_boxes_each_keep_their_link() {
    let mut src = String::new();
    for j in 0..400 {
        src.push_str(&format!("{}#!wiki style=\"border: 1px solid\"\n상자 [[문서 {j}]]\n{}\n", "{{{", "}}}"));
    }
    src.push_str("[[분류:조선]]");
    let d = doc(&src);
    assert_eq!(links(&d).iter().filter(|k| k.starts_with("문서 ")).count(), 400);
    assert_eq!(d.categories, ["조선"]);
}

#[test]
fn a_namu_wiki_shaped_article() {
    let src = r#"{{{#!wiki style="margin: -5px -10px; padding: 5px 10px"
[[임진왜란]]의 [[이순신|충무공]]
}}}
[목차]
== 개요 ==
'''이순신'''(李舜臣)은 조선의 장군이다.[* 『선조실록』 1권.]
{{{#!folding [ 펼치기 · 접기 ]
{{{#!folding 안쪽
 * [[한산도 대첩]]
 * [[명량 해전]]
}}}
}}}
|| 해 || 싸움 ||
|| 1592 || [[옥포 해전]] ||
== 생애 ==
{{{+1 '''크게'''}}} 쓴 글.[* 『난중일기』]
[[분류:조선의 장군]][[분류:임진왜란]]"#;
    let d = doc(src);
    assert_eq!(d.categories, ["조선의 장군", "임진왜란"]);
    assert_eq!(count(&d.blocks, &|b| matches!(b, Block::Fold { .. })), 2);
    assert_eq!(count(&d.blocks, &|b| matches!(b, Block::Table { .. })), 1);
    assert_eq!(d.footnotes.len(), 2);
    assert_eq!(d.sections.len(), 2);
    let l = links(&d);
    for want in ["임진왜란", "이순신", "한산도 대첩", "명량 해전", "옥포 해전"] {
        assert!(l.contains(&want.to_string()), "{want}: {l:?}");
    }
}
