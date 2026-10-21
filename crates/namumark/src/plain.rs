//! 검색 색인과 미리보기에 쓰는 글자만 뽑는다 — 옛 Dart `plain.dart`.
use crate::ast::Inline;

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
