//! 그리기 모델(설계 5-6) — 저장할 때 한 번 굳혀 두고 library_view가 이것만 그린다.
//! 틀을 펼치고(넘겨주기 한 번 따라감), 각주를 그린 차례로 다시 매기고, 한자 구간을 가르고, 문단 범위를 UTF-16 칸으로 낸다.
use crate::ast::{Align, Block, Document, FootnoteDef, Inline, ListStyle, Style};
use crate::parser::{parse, parse_budgeted, redirect_of, Options};
use crate::refs::{ObjectRef, TEMPLATE};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

pub const MODEL_VERSION: u32 = 1;
/// 틀 안의 틀 … 이 깊이에서 멈춘다.
pub const MAX_TEMPLATE_DEPTH: usize = 8;
/// 한 문서에서 틀을 펼치는 횟수의 한계(틀이 틀을 여러 번 부르는 폭발을 막는다).
pub const MAX_TEMPLATE_EXPANSIONS: usize = 2_000;
/// 한 문서를 그리는 동안 읽은 틀 원문과 채운 글을 합친 한계(바이트) — 인자를 여러 번 넘기는 틀이
/// 단마다 글을 몇 배로 불려 멈추거나 메모리가 바닥나는 것을 막는다(최종 검토 2).
pub const MAX_TEMPLATE_BYTES: usize = 8 << 20;
/// 펼치는 틀 모두가 나눠 쓰는 짝 찾기 예산의 바닥(그 위에 채운 글 1바이트마다 64).
const TEMPLATE_WORK_FLOOR: usize = 1_000_000;

/// 틀 원문을 주는 곳 — 다리는 열린 서재에서, 시험은 메모리에서.
pub trait Templates {
    fn source(&self, target: &ObjectRef) -> Option<String>;
}

pub struct NoTemplates;

impl Templates for NoTemplates {
    fn source(&self, _: &ObjectRef) -> Option<String> {
        None
    }
}

/// 열쇠는 `ObjectRef::path()`(`틀:해전 정보`).
impl Templates for BTreeMap<String, String> {
    fn source(&self, target: &ObjectRef) -> Option<String> {
        self.get(&target.path()).cloned()
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderOptions {
    /// 이 문서의 제목(상대 링크 · 틀 쪽 자기 부름 막기).
    pub title: Option<String>,
    /// 틀 문서 자체를 볼 때 — `@인자@`를 자리로 남긴다.
    pub template: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateStatus {
    Ok,
    Missing,
    Cycle,
    TooDeep,
    TooMany,
    TooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MStyle {
    Bold,
    Italic,
    Underline,
    Strike,
    Sup,
    Sub,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MAlign {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MListStyle {
    Bullet,
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub version: u32,
    pub redirect: Option<ObjectRef>,
    pub blocks: Vec<MBlock>,
    pub footnotes: Vec<MFootnote>,
    pub sections: Vec<MSection>,
    /// 본문에 적은 분류(첫 것이 집).
    pub categories: Vec<String>,
    /// 펼친 틀이 단 분류 — 집은 정하지 않는다.
    pub template_categories: Vec<String>,
    /// 펼치려 한 틀(있든 없든, 넘겨주기 대상 포함) — 「틀을 쓰는 곳」 색인.
    pub templates: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MSection {
    pub index: usize,
    pub level: u8,
    pub number: String,
    pub title: String,
    /// 원문(줄끝 `\n`)의 UTF-16 칸 위치 — Dart 글 위치와 같다.
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MFootnote {
    pub n: u32,
    pub name: Option<String>,
    pub inl: Vec<MInline>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MCell {
    pub col: u32,
    pub row: u32,
    pub align: MAlign,
    pub inl: Vec<MInline>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum MBlock {
    Heading { level: u8, number: String, section: Option<usize>, folded: bool, inl: Vec<MInline> },
    Para { inl: Vec<MInline> },
    List { style: MListStyle, start: u32, items: Vec<Vec<MBlock>> },
    Quote { blocks: Vec<MBlock> },
    Table { caption: Option<Vec<MInline>>, rows: Vec<Vec<MCell>> },
    Code { lang: Option<String>, text: String },
    Fold { summary: String, blocks: Vec<MBlock> },
    Indent { depth: usize, blocks: Vec<MBlock> },
    Rule,
    /// 펼친 틀. `status`가 ok가 아니면 `blocks`는 비어 있고 그 자리에 까닭을 그린다.
    Template { target: ObjectRef, status: TemplateStatus, blocks: Vec<MBlock> },
    Embed { target: ObjectRef, options: BTreeMap<String, String> },
    Toc,
    Footnotes,
    Front { name: String, arg: Option<String> },
    TimelineStrip { entity: String, range: Option<String> },
    Unsupported { kind: String, text: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum MInline {
    Text { s: String },
    /// 한자 구간 — 누르면 자전 카드.
    Han { s: String },
    Style { style: MStyle, inl: Vec<MInline> },
    Size { step: i8, inl: Vec<MInline> },
    /// 있는 문서인지는 굳히지 않는다 — 볼 때 `titles.exist`로 묻는다(설계 5-6).
    Link { target: ObjectRef, inl: Option<Vec<MInline>> },
    Url { href: String, inl: Option<Vec<MInline>> },
    File { key: String, options: BTreeMap<String, String> },
    Fn { n: u32 },
    Ruby { base: String, ruby: String },
    Code { s: String },
    Br,
    Macro { name: String, args: Vec<String> },
    Param { name: String, fallback: Option<String> },
}

static FILL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"@([^@\s=]+)(?:=([^@\n]*))?@").unwrap());

/// 틀 원문의 `@이름@` · `@이름=기본값@`을 해석하기 전에 글로 채운다(나무위키처럼 — 인자 안의 표식도 해석된다).
fn fill_params(source: &str, args: &BTreeMap<String, String>) -> String {
    FILL.replace_all(source, |c: &regex::Captures| match args.get(&c[1]) {
        Some(v) => v.clone(),
        None => c.get(2).map(|g| g.as_str().to_string()).unwrap_or_default(),
    })
    .into_owned()
}

/// `fill_params`가 낼 글의 길이 — 채우기 전에 크기 한계를 본다.
fn filled_len(source: &str, args: &BTreeMap<String, String>) -> usize {
    FILL.captures_iter(source).fold(source.len(), |len, c| {
        let with = match args.get(&c[1]) {
            Some(v) => v.len(),
            None => c.get(2).map_or(0, |g| g.len()),
        };
        len.saturating_add(with) - c[0].len()
    })
}

fn is_han(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

/// 글을 한자 구간과 나머지로 가른다.
fn split_han(t: &str, out: &mut Vec<MInline>) {
    let mut run = String::new();
    let mut han = false;
    for c in t.chars() {
        let h = is_han(c);
        if h != han && !run.is_empty() {
            let s = std::mem::take(&mut run);
            out.push(if han { MInline::Han { s } } else { MInline::Text { s } });
        }
        han = h;
        run.push(c);
    }
    if !run.is_empty() {
        out.push(if han { MInline::Han { s: run } } else { MInline::Text { s: run } });
    }
}

/// 바이트 위치들을 UTF-16 칸 위치로 — 한 번 훑는다.
fn utf16_positions(src: &str, bytes: &[usize]) -> Vec<usize> {
    let mut want: Vec<(usize, usize)> = bytes.iter().copied().enumerate().map(|(i, b)| (b, i)).collect();
    want.sort();
    let mut out = vec![0; bytes.len()];
    let (mut units, mut k) = (0usize, 0usize);
    for (at, c) in src.char_indices() {
        while k < want.len() && want[k].0 <= at {
            out[want[k].1] = units;
            k += 1;
        }
        units += c.len_utf16();
    }
    while k < want.len() {
        out[want[k].1] = units;
        k += 1;
    }
    out
}

/// 원문을 그리기 모델로.
pub fn render(source: &str, options: &RenderOptions, templates: &dyn Templates) -> Model {
    render_document(source, options, templates).0
}

/// `render`와 같되 해석한 구문 트리도 돌려준다 — 색인이 같은 원문을 두 번 해석하지 않도록.
pub fn render_document(source: &str, options: &RenderOptions, templates: &dyn Templates) -> (Model, Document) {
    let doc = parse(source, &Options { template: options.template, current: options.title.clone(), ..Options::default() });
    if let Some(r) = &doc.redirect {
        let model = Model { version: MODEL_VERSION, redirect: Some(r.clone()), blocks: Vec::new(), footnotes: Vec::new(), sections: Vec::new(), categories: Vec::new(), template_categories: Vec::new(), templates: Vec::new() };
        return (model, doc);
    }
    let mut chain = Vec::new();
    if let (true, Some(t)) = (options.template, &options.title) {
        chain.push(format!("{TEMPLATE}:{t}"));
    }
    let mut b = Builder {
        templates,
        cache: HashMap::new(),
        chain,
        expansions: 0,
        bytes: 0,
        work: TEMPLATE_WORK_FLOOR,
        defs: vec![doc.footnotes.clone()],
        numbers: HashMap::new(),
        footnotes: Vec::new(),
        template_categories: Vec::new(),
        used: Vec::new(),
    };
    let blocks = b.blocks(&doc.blocks, 0);
    let mut bytes = Vec::with_capacity(doc.sections.len() * 2);
    for s in &doc.sections {
        bytes.push(s.start);
        bytes.push(s.end);
    }
    let units = utf16_positions(&doc.source, &bytes);
    let sections = doc
        .sections
        .iter()
        .enumerate()
        .map(|(i, s)| MSection { index: s.index, level: s.level, number: s.number.clone(), title: s.title.clone(), start: units[2 * i], end: units[2 * i + 1] })
        .collect();
    let model = Model { version: MODEL_VERSION, redirect: None, blocks, footnotes: b.footnotes, sections, categories: doc.categories.clone(), template_categories: b.template_categories, templates: b.used };
    (model, doc)
}

struct Builder<'t> {
    templates: &'t dyn Templates,
    /// 읽은 틀 원문(없던 것도) — 한 번 그리는 동안 같은 틀은 한 번만 읽는다.
    cache: HashMap<String, Option<String>>,
    /// 지금 펼치는 중인 틀의 사슬(고리 막기).
    chain: Vec<String>,
    expansions: usize,
    /// 읽은 틀 원문과 채운 글의 합(`MAX_TEMPLATE_BYTES`까지).
    bytes: usize,
    /// 남은 짝 찾기 예산 — 틀마다 새로 받지 않고 나눠 쓴다.
    work: usize,
    /// 원문마다(0 = 문서, 1.. = 펼친 틀 하나하나)의 각주 정의.
    defs: Vec<Vec<FootnoteDef>>,
    /// (원문, 원문 속 번호) → 모델 번호.
    numbers: HashMap<(usize, u32), u32>,
    footnotes: Vec<MFootnote>,
    template_categories: Vec<String>,
    used: Vec<String>,
}

impl Builder<'_> {
    fn blocks(&mut self, blocks: &[Block], inst: usize) -> Vec<MBlock> {
        let mut out = Vec::with_capacity(blocks.len());
        for b in blocks {
            let m = match b {
                Block::Heading { level, number, section, folded, children } => MBlock::Heading {
                    level: *level,
                    number: if inst == 0 { number.clone() } else { String::new() },
                    section: if inst == 0 { *section } else { None },
                    folded: *folded,
                    inl: self.inlines(children, inst),
                },
                Block::Paragraph { children, .. } => MBlock::Para { inl: self.inlines(children, inst) },
                Block::List { style, start, items } => MBlock::List {
                    style: match style {
                        ListStyle::Bullet => MListStyle::Bullet,
                        ListStyle::Decimal => MListStyle::Decimal,
                        ListStyle::LowerAlpha => MListStyle::LowerAlpha,
                        ListStyle::UpperAlpha => MListStyle::UpperAlpha,
                        ListStyle::LowerRoman => MListStyle::LowerRoman,
                        ListStyle::UpperRoman => MListStyle::UpperRoman,
                    },
                    start: *start,
                    items: items.iter().map(|item| self.blocks(item, inst)).collect(),
                },
                Block::Quote(children) => MBlock::Quote { blocks: self.blocks(children, inst) },
                Block::Table { caption, rows } => MBlock::Table {
                    caption: caption.as_ref().map(|c| self.inlines(c, inst)),
                    rows: rows
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|cell| MCell {
                                    col: cell.col_span,
                                    row: cell.row_span,
                                    align: match cell.align {
                                        Align::Start => MAlign::Start,
                                        Align::Center => MAlign::Center,
                                        Align::End => MAlign::End,
                                    },
                                    inl: self.inlines(&cell.children, inst),
                                })
                                .collect()
                        })
                        .collect(),
                },
                Block::Code { language, text } => MBlock::Code { lang: language.clone(), text: text.clone() },
                Block::Fold { summary, children } => MBlock::Fold { summary: summary.clone(), blocks: self.blocks(children, inst) },
                Block::Indent { depth, children } => MBlock::Indent { depth: *depth, blocks: self.blocks(children, inst) },
                Block::Rule => MBlock::Rule,
                Block::Include { target, args } => self.include(target, args),
                Block::Embed { target, options, .. } => MBlock::Embed { target: target.clone(), options: options.clone() },
                Block::Macro(name) if name == "목차" => MBlock::Toc,
                Block::Macro(_) => MBlock::Footnotes,
                Block::Front { name, arg } => MBlock::Front { name: name.clone(), arg: arg.clone() },
                Block::TimelineStrip { entity, range } => MBlock::TimelineStrip { entity: entity.clone(), range: range.clone() },
                Block::Unsupported { kind, text } => MBlock::Unsupported { kind: kind.clone(), text: text.clone() },
            };
            out.push(m);
        }
        out
    }

    fn include(&mut self, target: &ObjectRef, args: &BTreeMap<String, String>) -> MBlock {
        let path = target.path();
        self.note(&path);
        let stop = |status| MBlock::Template { target: target.clone(), status, blocks: Vec::new() };
        if self.chain.contains(&path) {
            return stop(TemplateStatus::Cycle);
        }
        if self.chain.len() >= MAX_TEMPLATE_DEPTH {
            return stop(TemplateStatus::TooDeep);
        }
        if self.expansions >= MAX_TEMPLATE_EXPANSIONS {
            return stop(TemplateStatus::TooMany);
        }
        let mut source = match self.read(target) {
            Ok(s) => s,
            Err(status) => return stop(status),
        };
        let mut real = target.clone();
        if let Some(next) = redirect_of(&source) {
            // 이름 바뀐 틀 — 넘겨주기는 한 번만 따라간다.
            let next_path = next.path();
            self.note(&next_path);
            if next_path == path || self.chain.contains(&next_path) {
                return stop(TemplateStatus::Cycle);
            }
            match self.read(&next) {
                Ok(s) if redirect_of(&s).is_none() => {
                    source = s;
                    real = next;
                }
                Ok(_) => return stop(TemplateStatus::Missing),
                Err(status) => return stop(status),
            }
        }
        let size = filled_len(&source, args);
        if self.bytes.saturating_add(size) > MAX_TEMPLATE_BYTES {
            return stop(TemplateStatus::TooLarge);
        }
        self.bytes += size;
        self.expansions += 1;
        let filled = fill_params(&source, args);
        let budget = self.work.saturating_add(filled.len().saturating_mul(64));
        let (tdoc, left) = parse_budgeted(&filled, &Options { template: false, current: Some(real.key.clone()), ..Options::default() }, budget);
        self.work = left;
        for c in &tdoc.categories {
            if !self.template_categories.contains(c) {
                self.template_categories.push(c.clone());
            }
        }
        let inst = self.defs.len();
        self.defs.push(tdoc.footnotes.clone());
        self.chain.push(real.path());
        let blocks = self.blocks(&tdoc.blocks, inst);
        self.chain.pop();
        MBlock::Template { target: target.clone(), status: TemplateStatus::Ok, blocks }
    }

    /// 틀 원문. 읽은 글도 크기 한계에 센다 — 한계를 넘는 틀은 훑지도 베끼지도 않는다.
    fn read(&mut self, target: &ObjectRef) -> Result<String, TemplateStatus> {
        let templates = self.templates;
        let Some(s) = self.cache.entry(target.path()).or_insert_with(|| templates.source(target)) else {
            return Err(TemplateStatus::Missing);
        };
        if self.bytes.saturating_add(s.len()) > MAX_TEMPLATE_BYTES {
            return Err(TemplateStatus::TooLarge);
        }
        self.bytes += s.len();
        Ok(s.clone())
    }

    fn note(&mut self, path: &str) {
        if !self.used.iter().any(|u| u == path) {
            self.used.push(path.to_string());
        }
    }

    fn inlines(&mut self, nodes: &[Inline], inst: usize) -> Vec<MInline> {
        let mut out = Vec::with_capacity(nodes.len());
        for n in nodes {
            match n {
                Inline::Text(t) => split_han(t, &mut out),
                Inline::Styled { style, children } => {
                    let style = match style {
                        Style::Bold => MStyle::Bold,
                        Style::Italic => MStyle::Italic,
                        Style::Underline => MStyle::Underline,
                        Style::Strike => MStyle::Strike,
                        Style::Superscript => MStyle::Sup,
                        Style::Subscript => MStyle::Sub,
                    };
                    let inl = self.inlines(children, inst);
                    out.push(MInline::Style { style, inl });
                }
                Inline::Size { step, children } => {
                    let inl = self.inlines(children, inst);
                    out.push(MInline::Size { step: *step, inl });
                }
                Inline::Link { target, label } => {
                    let inl = label.as_ref().map(|l| self.inlines(l, inst));
                    out.push(MInline::Link { target: target.clone(), inl });
                }
                Inline::Url { uri, label } => {
                    let inl = label.as_ref().map(|l| self.inlines(l, inst));
                    out.push(MInline::Url { href: uri.clone(), inl });
                }
                Inline::File { file, options } => out.push(MInline::File { key: file.key.clone(), options: options.clone() }),
                Inline::FootnoteRef { number, .. } => {
                    let n = self.footnote(inst, *number);
                    out.push(MInline::Fn { n });
                }
                Inline::Ruby { base, ruby } => out.push(MInline::Ruby { base: base.clone(), ruby: ruby.clone() }),
                Inline::Code(t) => out.push(MInline::Code { s: t.clone() }),
                Inline::Break => out.push(MInline::Br),
                Inline::Macro { name, args } => out.push(MInline::Macro { name: name.clone(), args: args.clone() }),
                Inline::Param { name, fallback } => out.push(MInline::Param { name: name.clone(), fallback: fallback.clone() }),
            }
        }
        out
    }

    /// 그린 차례로 각주 번호를 매긴다. 같은 원문의 같은 번호(이름 붙인 각주의 재사용)는 같은 번호.
    fn footnote(&mut self, inst: usize, number: u32) -> u32 {
        if let Some(&n) = self.numbers.get(&(inst, number)) {
            return n;
        }
        let n = self.footnotes.len() as u32 + 1;
        self.numbers.insert((inst, number), n);
        let def = self.defs[inst].iter().find(|d| d.number == number).cloned();
        self.footnotes.push(MFootnote { n, name: def.as_ref().and_then(|d| d.name.clone()), inl: Vec::new() });
        let inl = match &def {
            Some(d) => self.inlines(&d.children, inst),
            None => Vec::new(),
        };
        self.footnotes[(n - 1) as usize].inl = inl;
        n
    }
}
