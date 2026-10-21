//! 구문 트리 — 옛 Dart `ast.dart`를 옮겼다. 글 위치(`start` · `end` · `offset`)는 맞춘 원문(줄끝 `\n`)의 바이트 위치다.
use crate::refs::ObjectRef;
use std::collections::BTreeMap;

/// 문서 한 편의 구문 트리.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    /// 줄끝을 `\n`으로 맞춘 원문 — 위치는 이 글의 바이트 위치다.
    pub source: String,
    pub blocks: Vec<Block>,
    /// `#넘겨주기` 대상. 있으면 본문은 비어 있다.
    pub redirect: Option<ObjectRef>,
    /// 본문에 적은 `[[분류:…]]`의 제목, 적은 차례(첫 것이 집 — 설계 5-3).
    pub categories: Vec<String>,
    /// 맨 위 단계 문단 제목마다 하나(문단 편집 · 목차).
    pub sections: Vec<Section>,
    /// 나온 차례대로 번호가 붙은 각주. 이름 붙인 각주의 재사용은 한 번만 든다.
    pub footnotes: Vec<FootnoteDef>,
}

/// 원문에서 문단 하나가 차지하는 범위: `start`는 제목 줄의 첫 바이트, `end`는 다음 같은 단계 이상 제목의 앞.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub index: usize,
    pub level: u8,
    pub number: String,
    pub title: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FootnoteDef {
    pub number: u32,
    pub name: Option<String>,
    pub children: Vec<Inline>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListStyle {
    Bullet,
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub col_span: u32,
    pub row_span: u32,
    pub align: Align,
    pub children: Vec<Inline>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// `section`은 맨 위 단계 문단 제목의 차례(인용 · 접기 안의 제목은 None).
    Heading { level: u8, number: String, section: Option<usize>, folded: bool, children: Vec<Inline> },
    Paragraph { children: Vec<Inline>, start: Option<usize>, end: Option<usize> },
    List { style: ListStyle, start: u32, items: Vec<Vec<Block>> },
    Quote(Vec<Block>),
    Table { caption: Option<Vec<Inline>>, rows: Vec<Vec<Cell>> },
    Code { language: Option<String>, text: String },
    Fold { summary: String, children: Vec<Block> },
    /// `{{{#!wiki}}}` · 앞 공백 들여쓰기 — 모양 없이 블록을 묶는다.
    Indent { depth: usize, children: Vec<Block> },
    Rule,
    Include { target: ObjectRef, args: BTreeMap<String, String> },
    Embed { target: ObjectRef, options: BTreeMap<String, String>, raw: String },
    /// `[목차]` → "목차", `[각주]` → "각주".
    Macro(String),
    /// 대문 매크로 자리(설계 8-2) — 앱 위젯이 채운다.
    Front { name: String, arg: Option<String> },
    /// 문서 속 연표 띠 `[연표(<항목ID>, 범위=…)]`(설계 7-1).
    TimelineStrip { entity: String, range: Option<String> },
    /// 받지 않는 문법(`{{{#!html}}}`). 글자로 남긴다.
    Unsupported { kind: String, text: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Bold,
    Italic,
    Underline,
    Strike,
    Superscript,
    Subscript,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text(String),
    Styled { style: Style, children: Vec<Inline> },
    /// 글자 크기 -5..+5.
    Size { step: i8, children: Vec<Inline> },
    Link { target: ObjectRef, label: Option<Vec<Inline>> },
    Url { uri: String, label: Option<Vec<Inline>> },
    File { file: ObjectRef, options: BTreeMap<String, String> },
    FootnoteRef { number: u32, name: Option<String> },
    Ruby { base: String, ruby: String },
    Code(String),
    Break,
    /// `[date]` · `[age(…)]` · `[dday(…)]` · `[anchor(…)]`.
    Macro { name: String, args: Vec<String> },
    /// 틀 안에서만 받는 `@이름@` · `@이름=기본값@`.
    Param { name: String, fallback: Option<String> },
}
