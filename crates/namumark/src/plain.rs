//! 검색 색인과 미리보기에 쓰는 글자만 뽑는다 — 옛 Dart `plain.dart`.
use crate::ast::{Block, Document, Inline};
use crate::refs::ObjectRef;
use std::collections::HashSet;

pub fn plain_inline(nodes: &[Inline]) -> String {
    let mut out = String::new();
    walk_inline(nodes, &mut out);
    out
}

fn walk_inline(nodes: &[Inline], out: &mut String) {
    for n in nodes {
        match n {
            Inline::Text(t) | Inline::Code(t) => out.push_str(t),
            Inline::Styled { children, .. } | Inline::Size { children, .. } => walk_inline(children, out),
            Inline::Link { target, label } => match label {
                Some(l) => walk_inline(l, out),
                None if target.key.is_empty() => out.push_str(target.anchor.as_deref().unwrap_or("")),
                None => out.push_str(&target.title()),
            },
            Inline::Url { uri, label } => match label {
                Some(l) => walk_inline(l, out),
                None => out.push_str(uri),
            },
            Inline::Ruby { base, ruby } => {
                out.push_str(base);
                out.push('(');
                out.push_str(ruby);
                out.push(')');
            }
            Inline::Break => out.push(' '),
            Inline::Param { name, fallback } => out.push_str(fallback.as_deref().unwrap_or(name)),
            Inline::File { .. } | Inline::FootnoteRef { .. } | Inline::Macro { .. } => {}
        }
    }
}

/// 검색 색인과 미리보기에 쓰는 문서 글. 틀과 끼우기는 펼치지 않는다. 공백 · 빈 줄은 하나로.
pub fn plain_document(doc: &Document) -> String {
    let mut out = String::new();
    plain_blocks(&doc.blocks, &mut out);
    for f in &doc.footnotes {
        out.push_str(&plain_inline(&f.children));
        out.push('\n');
    }
    collapse(&out)
}

fn plain_blocks(blocks: &[Block], out: &mut String) {
    for b in blocks {
        match b {
            Block::Heading { children, .. } | Block::Paragraph { children, .. } => {
                out.push_str(&plain_inline(children));
                out.push('\n');
            }
            Block::List { items, .. } => items.iter().for_each(|item| plain_blocks(item, out)),
            Block::Quote(children) | Block::Indent { children, .. } => plain_blocks(children, out),
            Block::Table { caption, rows } => {
                if let Some(c) = caption {
                    out.push_str(&plain_inline(c));
                    out.push('\n');
                }
                for row in rows {
                    out.push_str(&row.iter().map(|c| plain_inline(&c.children)).collect::<Vec<_>>().join(" "));
                    out.push('\n');
                }
            }
            Block::Code { text, .. } | Block::Unsupported { text, .. } => {
                out.push_str(text);
                out.push('\n');
            }
            Block::Fold { summary, children } => {
                out.push_str(summary);
                out.push('\n');
                plain_blocks(children, out);
            }
            Block::Rule | Block::Include { .. } | Block::Embed { .. } | Block::Macro(_) | Block::Front { .. } | Block::TimelineStrip { .. } => {}
        }
    }
}

/// 옛 Dart와 같은 차례 — 먼저 `[ \t]+` → 공백 하나, 그다음 `\n{2,}` → `\n`, 앞뒤를 걷는다.
fn collapse(s: &str) -> String {
    let mut spaced = String::with_capacity(s.len());
    let mut in_space = false;
    for c in s.chars() {
        if c == ' ' || c == '\t' {
            if !in_space {
                spaced.push(' ');
            }
            in_space = true;
        } else {
            spaced.push(c);
            in_space = false;
        }
    }
    let mut out = String::with_capacity(spaced.len());
    let mut newlines = 0;
    for c in spaced.chars() {
        if c == '\n' {
            newlines += 1;
            if newlines == 1 {
                out.push('\n');
            }
        } else {
            newlines = 0;
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// 링크 종류 — 옛 설계서 `link.kind`와 같은 번호.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkKind {
    Body = 0,
    Include = 1,
    Redirect = 2,
    File = 3,
    Embed = 4,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutLink {
    pub target: ObjectRef,
    pub kind: LinkKind,
}

/// 이 문서가 가리키는 다른 대상. 같은 대상은 종류마다 한 번, 앵커는 뺀다. 같은 문서 안 링크는 넣지 않는다.
pub fn out_links(doc: &Document) -> Vec<OutLink> {
    let mut w = LinkWalk { seen: HashSet::new(), out: Vec::new() };
    if let Some(r) = &doc.redirect {
        w.add(r, LinkKind::Redirect);
    }
    w.blocks(&doc.blocks);
    for f in &doc.footnotes {
        w.inlines(&f.children);
    }
    w.out
}

struct LinkWalk {
    seen: HashSet<(LinkKind, String)>,
    out: Vec<OutLink>,
}

impl LinkWalk {
    fn add(&mut self, r: &ObjectRef, kind: LinkKind) {
        if r.key.is_empty() {
            return;
        }
        let r = r.without_anchor();
        if self.seen.insert((kind, r.path())) {
            self.out.push(OutLink { target: r, kind });
        }
    }

    fn inlines(&mut self, nodes: &[Inline]) {
        for n in nodes {
            match n {
                Inline::Link { target, label } => {
                    if !target.is_same_page() {
                        self.add(target, LinkKind::Body);
                    }
                    if let Some(l) = label {
                        self.inlines(l);
                    }
                }
                Inline::File { file, .. } => self.add(file, LinkKind::File),
                Inline::Styled { children, .. } | Inline::Size { children, .. } => self.inlines(children),
                Inline::Url { label: Some(l), .. } => self.inlines(l),
                _ => {}
            }
        }
    }

    fn blocks(&mut self, blocks: &[Block]) {
        for b in blocks {
            match b {
                Block::Heading { children, .. } | Block::Paragraph { children, .. } => self.inlines(children),
                Block::List { items, .. } => items.iter().for_each(|item| self.blocks(item)),
                Block::Quote(children) | Block::Indent { children, .. } | Block::Fold { children, .. } => self.blocks(children),
                Block::Table { caption, rows } => {
                    if let Some(c) = caption {
                        self.inlines(c);
                    }
                    for row in rows {
                        for cell in row {
                            self.inlines(&cell.children);
                        }
                    }
                }
                Block::Include { target, .. } => self.add(target, LinkKind::Include),
                Block::Embed { target, .. } => self.add(target, LinkKind::Embed),
                _ => {}
            }
        }
    }
}
