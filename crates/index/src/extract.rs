//! 한 문서에서 색인할 것을 뽑는다(설계 5-5) — 링크 · 분류 · 각주 · 사료 · 연표 연결 · 쓰는 틀 · 다루는 해.
use history_library::Namespace;
use history_namumark::{out_links, plain_inline, timeline_entities, Block, Document, LinkKind, Model};
use regex::Regex;
use std::sync::LazyLock;

/// 뽑기 표 — `item`(문서 번호)을 `doc`/`src`로 가리킨다.
pub(crate) const SCHEMA: &str = "
CREATE TABLE link(src INTEGER NOT NULL, kind INTEGER NOT NULL, ns TEXT NOT NULL, key TEXT NOT NULL);
CREATE INDEX link_src ON link(src);
CREATE INDEX link_target ON link(ns, key);
-- 본문 링크가 가리키는 대상별 문서 수(만들 문서 · 많이 불리는 문서).
CREATE TABLE target(ns TEXT NOT NULL, key TEXT NOT NULL, refs INTEGER NOT NULL, PRIMARY KEY(ns, key)) WITHOUT ROWID;
CREATE INDEX target_refs ON target(refs);
CREATE TABLE member(doc INTEGER NOT NULL, category TEXT NOT NULL, first INTEGER NOT NULL);
CREATE INDEX member_doc ON member(doc);
CREATE INDEX member_category ON member(category);
CREATE TABLE note(doc INTEGER NOT NULL, n INTEGER NOT NULL, text TEXT NOT NULL);
CREATE INDEX note_doc ON note(doc);
CREATE TABLE source(doc INTEGER NOT NULL, name TEXT NOT NULL);
CREATE INDEX source_doc ON source(doc);
CREATE INDEX source_name ON source(name);
CREATE TABLE entity(doc INTEGER NOT NULL, entity TEXT NOT NULL);
CREATE INDEX entity_doc ON entity(doc);
CREATE INDEX entity_entity ON entity(entity);
CREATE TABLE uses(doc INTEGER NOT NULL, template TEXT NOT NULL);
CREATE INDEX uses_doc ON uses(doc);
CREATE INDEX uses_template ON uses(template);
CREATE TABLE years(doc INTEGER PRIMARY KEY, y0 INTEGER NOT NULL, y1 INTEGER NOT NULL);
-- 분류마다 바로 든 살아 있는 일반 문서 수(빈 이름 = 분류 안 됨). 나무는 이것과 분류 문서의 부모만 읽는다.
CREATE TABLE folder(name TEXT PRIMARY KEY, direct INTEGER NOT NULL) WITHOUT ROWID;
CREATE INDEX item_home ON item(home, title) WHERE ns = 0 AND deleted = 0;
CREATE INDEX item_categories ON item(title, created) WHERE ns = 1 AND deleted = 0;
";

/// 이 문서 번호의 뽑기 행을 담은 표(다시 색인할 때 모두 지운다).
pub(crate) const PER_DOC: [(&str, &str); 7] =
    [("link", "src"), ("member", "doc"), ("note", "doc"), ("source", "doc"), ("entity", "doc"), ("uses", "doc"), ("years", "doc")];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Extracted {
    /// 서재 이름공간을 가리키는 링크(종류, 이름공간 접두어, 제목).
    pub(crate) links: Vec<(LinkKind, String, String)>,
    pub(crate) categories: Vec<String>,
    pub(crate) footnotes: Vec<String>,
    pub(crate) sources: Vec<String>,
    pub(crate) entities: Vec<String>,
    pub(crate) templates: Vec<String>,
    pub(crate) years: Option<(i32, i32)>,
}

static SOURCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"『([^』]+)』").unwrap());
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{1,4}").unwrap());

pub(crate) fn extract(model: &Model, doc: &Document) -> Extracted {
    let links = out_links(doc)
        .into_iter()
        .filter(|l| Namespace::ALL.iter().any(|n| n.prefix() == l.target.ns))
        .map(|l| (l.kind, l.target.ns, l.target.key))
        .collect();
    let footnotes: Vec<String> = doc.footnotes.iter().map(|f| plain_inline(&f.children)).collect();
    let mut sources = Vec::new();
    for f in &footnotes {
        for c in SOURCE.captures_iter(f) {
            let name = c[1].trim().to_string();
            if !name.is_empty() && !sources.contains(&name) {
                sources.push(name);
            }
        }
    }
    Extracted {
        links,
        categories: doc.categories.clone(),
        footnotes,
        sources,
        entities: timeline_entities(doc),
        templates: model.templates.clone(),
        years: info_box_years(&doc.blocks),
    }
}

/// 다루는 해 ② — 문서가 펼친 틀 중 인자 `기간` 또는 `연도`를 가진 첫 것(설계 5-5). ①(이어진 연표 항목의 기간)은 P5 창구 뒤로.
fn info_box_years(blocks: &[Block]) -> Option<(i32, i32)> {
    for b in blocks {
        let found = match b {
            Block::Include { args, .. } => args.get("기간").or_else(|| args.get("연도")).and_then(|v| years(v)),
            Block::List { items, .. } => items.iter().find_map(|i| info_box_years(i)),
            Block::Quote(c) | Block::Indent { children: c, .. } | Block::Fold { children: c, .. } => info_box_years(c),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// `1592` · `1592년` · `1592–1598` · `1592~1598` · `기원전 57년`(→ -57). 앞의 두 수까지 본다.
pub(crate) fn years(text: &str) -> Option<(i32, i32)> {
    let mut found = Vec::new();
    let mut from = 0;
    for m in NUMBER.find_iter(text) {
        let bc = text[from..m.start()].contains("기원전");
        let n: i32 = m.as_str().parse().ok()?;
        found.push(if bc { -n } else { n });
        from = m.end();
        if found.len() == 2 {
            break;
        }
    }
    let (a, b) = match found[..] {
        [a] => (a, a),
        [a, b] => (a, b),
        _ => return None,
    };
    Some((a.min(b), a.max(b)))
}

#[cfg(test)]
mod tests {
    use super::years;

    #[test]
    fn year_forms() {
        assert_eq!(years("1592"), Some((1592, 1592)));
        assert_eq!(years("1592년"), Some((1592, 1592)));
        assert_eq!(years("1592–1598"), Some((1592, 1598)));
        assert_eq!(years("1598~1592"), Some((1592, 1598)));
        assert_eq!(years("기원전 57년"), Some((-57, -57)));
        assert_eq!(years("기원전 37년 ~ 668년"), Some((-37, 668)));
        assert_eq!(years("모름"), None);
    }
}
