//! 이름 붙인 앵커 `[anchor(…)]` → 그것을 품은 맨 위 블록의 차례. 같은 이름은 처음 것이 이긴다.
//! 접힌 상자 안의 앵커는 상자를 가리킨다(상자는 늘 보인다). 옛 Dart `anchors.dart`.
use crate::ast::{Block, Document, Inline};
use std::collections::BTreeMap;

pub fn anchor_blocks(doc: &Document) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for (i, b) in doc.blocks.iter().enumerate() {
        visit(b, i, &mut out);
    }
    out
}

fn visit(b: &Block, top: usize, out: &mut BTreeMap<String, usize>) {
    match b {
        Block::Heading { children, .. } | Block::Paragraph { children, .. } => inlines(children, top, out),
        Block::Quote(children) | Block::Indent { children, .. } | Block::Fold { children, .. } => children.iter().for_each(|c| visit(c, top, out)),
        Block::List { items, .. } => items.iter().flatten().for_each(|c| visit(c, top, out)),
        Block::Table { caption, rows } => {
            if let Some(c) = caption {
                inlines(c, top, out);
            }
            rows.iter().flatten().for_each(|cell| inlines(&cell.children, top, out));
        }
        _ => {}
    }
}

fn inlines(nodes: &[Inline], top: usize, out: &mut BTreeMap<String, usize>) {
    for n in nodes {
        match n {
            Inline::Macro { name, args } if name == "anchor" => {
                if let Some(a) = args.first().map(|a| a.trim()).filter(|a| !a.is_empty()) {
                    out.entry(a.to_string()).or_insert(top);
                }
            }
            Inline::Styled { children, .. } | Inline::Size { children, .. } => inlines(children, top, out),
            Inline::Link { label: Some(l), .. } | Inline::Url { label: Some(l), .. } => inlines(l, top, out),
            _ => {}
        }
    }
}
