//! 나무마크 해석기 — 옛 Dart `packages/core_engine/lib/src/wiki/parser.dart`를 옮겼다.
//! 줄 단위로 블록을 가른 뒤 인라인을 해석한다. 어떤 입력에도 실패하지 않고, 닫히지 않은 표식은 글자로 남는다.
use crate::ast::*;
use crate::plain::plain_inline;
use crate::refs::{ObjectRef, CATEGORY, FILE};
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

/// 해석 선택.
#[derive(Clone, Debug)]
pub struct Options {
    /// 틀을 해석할 때만 `@인자@`를 받는다.
    pub template: bool,
    /// `[[/하위]]` · `[[../]]` 상대 링크의 기준 문서 제목.
    pub current: Option<String>,
    /// 중첩(인용 · 접기 · 인라인)이 이보다 깊으면 글자로 남긴다.
    pub max_depth: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { template: false, current: None, max_depth: 32 }
    }
}

/// 원문을 구문 트리로. 줄끝은 `\n`으로 맞춘다.
pub fn parse(source: &str, options: &Options) -> Document {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    Run::new(options.clone(), normalized).run()
}

static RULE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-{4,9}\s*$").unwrap());
static REDIRECT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^#(?:redirect|넘겨주기)\s+(.+?)\s*$").unwrap());
static LIST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^( +)(\*|1\.|a\.|A\.|i\.|I\.)(?:#(\d+))? ?(.*)$").unwrap());
static BLOCK_MACRO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\[(목차|tableofcontents|각주|footnote|clearfix)\]$").unwrap());
static INCLUDE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\[include\((.*)\)\]$").unwrap());
static EMBED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\[embed\((.*)\)\]$").unwrap());
/// 대문 매크로(설계 8-2) — 한 줄 전체일 때만.
static FRONT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[(주제 바로가기|오늘의 문서|오늘의 역사|알고 계셨나요|최근 바뀐 문서|만들 문서|쓰는 리듬|최근 찾은 한자)(?:\(([^)]*)\))?\]$").unwrap()
});
/// 문서 속 연표 띠 `[연표(<항목ID>, 범위=…)]`(설계 7-1).
static STRIP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[연표\((.*)\)\]$").unwrap());
pub(crate) static MACRO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[([A-Za-z가-힣]+)(?:\(([^\]]*)\))?\]").unwrap());
pub(crate) static PARAM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^@([^@\s=]+)(?:=([^@\n]*))?@").unwrap());
static SIZE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([+-])([1-5])\s").unwrap());
static CAPTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\|([^|]+)\|").unwrap());
static COLSPAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-(\d+)$").unwrap());
static ROWSPAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\^v]?\|(\d+)$").unwrap());
static CELL_ATTR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[#A-Za-z][0-9A-Za-z_#=,.%\s-]*$").unwrap());

const PAIRED: [(&str, Style); 7] = [
    ("'''", Style::Bold),
    ("''", Style::Italic),
    ("__", Style::Underline),
    ("~~", Style::Strike),
    ("--", Style::Strike),
    ("^^", Style::Superscript),
    (",,", Style::Subscript),
];

#[derive(Clone, Debug)]
pub(crate) struct Line {
    pub(crate) text: String,
    pub(crate) offset: usize,
}

struct HeadingInfo {
    level: u8,
    offset: usize,
    title: String,
}

pub(crate) struct Entry {
    indent: usize,
    marker: String,
    start_at: Option<u32>,
    text: String,
}

pub(crate) struct Run {
    pub(crate) opt: Options,
    source: String,
    footnotes: Vec<FootnoteDef>,
    named: HashMap<String, u32>,
    categories: Vec<String>,
    headings: Vec<HeadingInfo>,
    section_counter: usize,
    /// 짝 찾기 훑기 예산(바이트). 떨어지면 남은 표식은 글자로 남는다 — 닫히지 않은 표식 수만 개에도 O(n).
    budget: usize,
}

/// 문단 제목 `== 제목 ==` · 접힌 `==# 제목 #==`. Dart 정규식 `^(={1,6})(#?)\s*(.+?)\s*\2\1\s*$`를
/// 역참조 없이 — 등호 수를 많은 쪽부터, `#`은 있는 쪽부터 맞춰 본다(정규식의 되돌아가기와 같은 차례).
pub(crate) fn heading(line: &str) -> Option<(u8, bool, &str)> {
    let lead = line.bytes().take_while(|&b| b == b'=').count();
    for n in (1..=lead.min(6)).rev() {
        let after = &line[n..];
        for folded in [true, false] {
            let body = if folded {
                match after.strip_prefix('#') {
                    Some(b) => b,
                    None => continue,
                }
            } else {
                after
            };
            let close = format!("{}{}", if folded { "#" } else { "" }, "=".repeat(n));
            let end = body.trim_end();
            if let Some(inner) = end.strip_suffix(close.as_str()) {
                let title = inner.trim();
                if !title.is_empty() {
                    return Some((n as u8, folded, title));
                }
            }
        }
    }
    None
}

pub(crate) fn is_table_start(t: &str) -> bool {
    t.starts_with("||") || (CAPTION.is_match(t) && t.trim_end().ends_with("||"))
}

fn split_lines(source: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut offset = 0;
    for text in source.split('\n') {
        lines.push(Line { text: text.to_string(), offset });
        offset += text.len() + 1;
    }
    lines
}

/// 앞쪽 공백을 많아야 `n`바이트 걷어 낸다 — 문자 경계를 넘지 않는다(전각 공백 등).
fn dedent(t: &str, n: usize) -> (&str, usize) {
    let mut cut = 0;
    for (i, c) in t.char_indices() {
        if i >= n || !c.is_whitespace() || i + c.len_utf8() > n {
            break;
        }
        cut = i + c.len_utf8();
    }
    (&t[cut..], cut)
}

fn has_content(inlines: &[Inline]) -> bool {
    inlines.iter().any(|n| match n {
        Inline::Text(t) => !t.trim().is_empty(),
        Inline::Break => false,
        _ => true,
    })
}

/// `s[i..]`에서 많아야 `n`바이트 — 문자 경계에서 자른다(줄 안 매크로 · `@인자@` 정규식이 글 끝까지 훑지 않게).
fn window(s: &str, i: usize, n: usize) -> &str {
    let mut end = i.saturating_add(n).min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[i..end]
}

fn flush(buf: &mut String, out: &mut Vec<Inline>) {
    if !buf.is_empty() {
        out.push(Inline::Text(std::mem::take(buf)));
    }
}

/// 쉼표로 나누되 `\` 이스케이프와 `[ ]` · `( )` 안의 쉼표는 건드리지 않는다.
pub(crate) fn split_args(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut bracket = 0usize;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                buf.push(next);
                continue;
            }
        }
        if c == '[' || c == '(' {
            bracket += 1;
        }
        if (c == ']' || c == ')') && bracket > 0 {
            bracket -= 1;
        }
        if c == ',' && bracket == 0 {
            parts.push(buf.trim().to_string());
            buf.clear();
            continue;
        }
        buf.push(c);
    }
    if !buf.is_empty() || !parts.is_empty() {
        parts.push(buf.trim().to_string());
    }
    parts
}

pub(crate) fn key_values(args: &[String]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for a in args {
        if let Some(eq) = a.find('=') {
            if eq > 0 {
                out.insert(a[..eq].trim().to_string(), a[eq + 1..].trim().to_string());
            }
        }
    }
    out
}

fn options(label: Option<&str>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(label) = label {
        for part in label.split('&') {
            if let Some(eq) = part.find('=') {
                if eq > 0 {
                    out.insert(part[..eq].trim().to_string(), part[eq + 1..].trim().to_string());
                }
            }
        }
    }
    out
}

/// `sep`으로 나누되 `[[ ]]` · `{{{ }}}` · `[ ]` 안과 `\` 뒤는 건드리지 않는다.
pub(crate) fn split_top_level(s: &str, sep: &str) -> Vec<String> {
    let b = s.as_bytes();
    let sep_b = sep.as_bytes();
    let (mut link, mut brace, mut bracket) = (0usize, 0usize, 0usize);
    let mut out = Vec::new();
    let (mut last, mut i) = (0usize, 0usize);
    while i < b.len() {
        if b[i] == b'\\' {
            i += 2;
            continue;
        }
        if b[i..].starts_with(b"[[") {
            link += 1;
            i += 2;
            continue;
        }
        if b[i..].starts_with(b"]]") && link > 0 {
            link -= 1;
            i += 2;
            continue;
        }
        if b[i..].starts_with(b"{{{") {
            brace += 1;
            i += 3;
            continue;
        }
        if b[i..].starts_with(b"}}}") && brace > 0 {
            brace -= 1;
            i += 3;
            continue;
        }
        if b[i] == b'[' {
            bracket += 1;
        }
        if b[i] == b']' && bracket > 0 {
            bracket -= 1;
        }
        if link == 0 && brace == 0 && bracket == 0 && b[i..].starts_with(sep_b) {
            out.push(s[last..i].to_string());
            i += sep_b.len();
            last = i;
            continue;
        }
        i += 1;
    }
    out.push(s[last.min(s.len())..].to_string());
    out
}

impl Run {
    pub(crate) fn new(opt: Options, source: String) -> Run {
        let budget = source.len().saturating_mul(64).saturating_add(1_000_000);
        Run { opt, source, footnotes: Vec::new(), named: HashMap::new(), categories: Vec::new(), headings: Vec::new(), section_counter: 0, budget }
    }

    fn run(mut self) -> Document {
        let lines = split_lines(&self.source);
        for line in &lines {
            let t = line.text.trim();
            if t.is_empty() {
                continue;
            }
            if let Some(c) = REDIRECT.captures(t) {
                if let Some(target) = ObjectRef::parse(&c[1]) {
                    return Document { source: self.source, blocks: Vec::new(), redirect: Some(target), categories: Vec::new(), sections: Vec::new(), footnotes: Vec::new() };
                }
            }
            break;
        }
        let mut blocks = self.blocks(&lines, 0, true);
        let sections = self.finish_headings(&mut blocks);
        Document { source: self.source, blocks, redirect: None, categories: self.categories, sections, footnotes: self.footnotes }
    }

    /// 문단 번호를 매기고 원문 범위를 정한다.
    fn finish_headings(&self, blocks: &mut [Block]) -> Vec<Section> {
        if self.headings.is_empty() {
            return Vec::new();
        }
        let min = self.headings.iter().map(|h| h.level).min().unwrap_or(1);
        let mut counters = [0usize; 7];
        let numbers: Vec<String> = self
            .headings
            .iter()
            .map(|h| {
                let d = (h.level - min) as usize;
                counters[d] += 1;
                for c in counters.iter_mut().skip(d + 1) {
                    *c = 0;
                }
                counters[..=d].iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".")
            })
            .collect();
        let mut sections = Vec::new();
        for (i, h) in self.headings.iter().enumerate() {
            let end = self.headings[i + 1..].iter().find(|o| o.level <= h.level).map(|o| o.offset).unwrap_or(self.source.len());
            sections.push(Section { index: i, level: h.level, number: numbers[i].clone(), title: h.title.clone(), start: h.offset, end });
        }
        let mut k = 0;
        for b in blocks.iter_mut() {
            if let Block::Heading { number, section: Some(idx), .. } = b {
                *number = numbers[k].clone();
                *idx = k;
                k += 1;
            }
        }
        sections
    }

    pub(crate) fn blocks(&mut self, lines: &[Line], depth: usize, top: bool) -> Vec<Block> {
        let mut out = Vec::new();
        if depth > self.opt.max_depth {
            let text = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
            out.push(Block::Paragraph { children: vec![Inline::Text(text)], start: None, end: None });
            return out;
        }
        let mut i = 0;
        while i < lines.len() {
            let text = lines[i].text.as_str();
            let trimmed = text.trim();
            if trimmed.is_empty() || text.starts_with("##") {
                i += 1;
                continue;
            }
            if let Some((level, folded, title)) = heading(text) {
                let children = self.inline(title, depth + 1);
                let section = if top {
                    self.headings.push(HeadingInfo { level, offset: lines[i].offset, title: plain_inline(&children) });
                    self.section_counter += 1;
                    Some(self.section_counter - 1)
                } else {
                    None
                };
                out.push(Block::Heading { level, number: String::new(), section, folded, children });
                i += 1;
                continue;
            }
            if RULE.is_match(text) {
                out.push(Block::Rule);
                i += 1;
                continue;
            }
            if text.trim_start().starts_with("{{{") {
                let consumed = self.brace_block(lines, i, depth, &mut out);
                if consumed > 0 {
                    i += consumed;
                    continue;
                }
            }
            if is_table_start(text) {
                i += self.table(lines, i, depth, &mut out);
                continue;
            }
            if text.starts_with('>') {
                let mut quoted = Vec::new();
                while i < lines.len() && lines[i].text.starts_with('>') {
                    let (mut t, mut off) = (&lines[i].text[1..], lines[i].offset + 1);
                    if let Some(rest) = t.strip_prefix(' ') {
                        t = rest;
                        off += 1;
                    }
                    quoted.push(Line { text: t.to_string(), offset: off });
                    i += 1;
                }
                let inner = self.blocks(&quoted, depth + 1, false);
                out.push(Block::Quote(inner));
                continue;
            }
            if LIST.is_match(text) {
                i += self.list(lines, i, depth, &mut out);
                continue;
            }
            if text.starts_with(' ') {
                let mut group = Vec::new();
                let mut min_indent = usize::MAX;
                while i < lines.len() && lines[i].text.starts_with(' ') && !lines[i].text.trim().is_empty() && !LIST.is_match(&lines[i].text) {
                    let t = &lines[i].text;
                    min_indent = min_indent.min(t.len() - t.trim_start().len());
                    group.push(lines[i].clone());
                    i += 1;
                }
                let dedented: Vec<Line> = group
                    .iter()
                    .map(|l| {
                        let (t, cut) = dedent(&l.text, min_indent);
                        Line { text: t.to_string(), offset: l.offset + cut }
                    })
                    .collect();
                let inner = self.blocks(&dedented, depth + 1, false);
                out.push(Block::Indent { depth: min_indent, children: inner });
                continue;
            }
            if let Some(c) = FRONT.captures(trimmed) {
                let arg = c.get(2).map(|g| g.as_str().trim()).filter(|a| !a.is_empty()).map(str::to_string);
                out.push(Block::Front { name: c[1].to_string(), arg });
                i += 1;
                continue;
            }
            if let Some(c) = STRIP.captures(trimmed) {
                let args = split_args(&c[1]);
                if let Some(entity) = args.first().filter(|e| !e.is_empty()) {
                    let range = key_values(&args[1..]).get("범위").cloned();
                    out.push(Block::TimelineStrip { entity: entity.clone(), range });
                    i += 1;
                    continue;
                }
            }
            if let Some(c) = BLOCK_MACRO.captures(trimmed) {
                let name = c[1].to_lowercase();
                if name == "목차" || name == "tableofcontents" {
                    out.push(Block::Macro("목차".into()));
                }
                if name == "각주" || name == "footnote" {
                    out.push(Block::Macro("각주".into()));
                }
                i += 1;
                continue;
            }
            if let Some(c) = INCLUDE.captures(trimmed) {
                let args = split_args(&c[1]);
                if let Some(target) = args.first().and_then(|a| ObjectRef::parse(a)) {
                    out.push(Block::Include { target, args: key_values(&args[1..]) });
                    i += 1;
                    continue;
                }
            }
            if let Some(c) = EMBED.captures(trimmed) {
                let args = split_args(&c[1]);
                if let Some(target) = args.first().and_then(|a| ObjectRef::parse(a)) {
                    out.push(Block::Embed { target, options: key_values(&args[1..]), raw: trimmed.to_string() });
                    i += 1;
                    continue;
                }
            }
            let first = i;
            let mut para: Vec<&str> = Vec::new();
            while i < lines.len() {
                let t = lines[i].text.as_str();
                if t.trim().is_empty() || t.starts_with("##") || self.starts_block(t) {
                    break;
                }
                para.push(t);
                i += 1;
            }
            if para.is_empty() {
                // 어떤 규칙에도 맞지 않는 시작 줄 — 글자로 남긴다.
                para.push(text);
                i += 1;
            }
            let joined = para.join("\n");
            let inlines = self.inline(&joined, depth + 1);
            if has_content(&inlines) {
                let end = lines[i - 1].offset + lines[i - 1].text.len();
                out.push(Block::Paragraph { children: inlines, start: Some(lines[first].offset), end: Some(end) });
            }
        }
        out
    }

    pub(crate) fn starts_block(&mut self, t: &str) -> bool {
        if heading(t).is_some() || RULE.is_match(t) {
            return true;
        }
        if is_table_start(t) || t.starts_with('>') || t.starts_with(' ') {
            return true;
        }
        let trimmed = t.trim();
        if BLOCK_MACRO.is_match(trimmed) || INCLUDE.is_match(trimmed) || EMBED.is_match(trimmed) || FRONT.is_match(trimmed) || STRIP.is_match(trimmed) {
            return true;
        }
        if t.starts_with("{{{") && self.match_braces(t.as_bytes(), t.find("{{{").unwrap_or(0)).is_none() {
            return true;
        }
        false
    }


    /// 훑기가 `limit`에 닿으면 예산이 떨어진 것 — 0으로 두고 없음.
    fn charge(&mut self, start: usize, at: usize, limit: usize, len: usize) {
        if at >= limit && limit < len {
            self.budget = 0;
        } else {
            self.budget = self.budget.saturating_sub(at.saturating_sub(start));
        }
    }

    /// `start`의 `{{{`와 짝을 이루는 `}}}`의 시작 위치.
    pub(crate) fn match_braces(&mut self, b: &[u8], start: usize) -> Option<usize> {
        let limit = start.saturating_add(self.budget).min(b.len());
        let (mut depth, mut i) = (0usize, start);
        while i < limit {
            if b[i..].starts_with(b"{{{") {
                depth += 1;
                i += 3;
                continue;
            }
            if b[i..].starts_with(b"}}}") {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    self.charge(start, i, limit, b.len());
                    return Some(i);
                }
                i += 3;
                continue;
            }
            if b[i] == b'\\' {
                i += 2;
                continue;
            }
            i += 1;
        }
        self.charge(start, i, limit, b.len());
        None
    }

    /// `[[`와 짝인 `]]`의 시작 위치. 바깥 단계에서 줄이 바뀌면 없음.
    pub(crate) fn match_pair(&mut self, b: &[u8], start: usize) -> Option<usize> {
        let limit = start.saturating_add(self.budget).min(b.len());
        let (mut depth, mut i) = (0usize, start);
        while i < limit {
            if b[i] == b'\\' {
                i += 2;
                continue;
            }
            if b[i..].starts_with(b"[[") {
                depth += 1;
                i += 2;
                continue;
            }
            if b[i..].starts_with(b"]]") {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    self.charge(start, i, limit, b.len());
                    return Some(i);
                }
                i += 2;
                continue;
            }
            if b[i] == b'\n' && depth <= 1 {
                self.charge(start, i, limit, b.len());
                return None;
            }
            i += 1;
        }
        self.charge(start, i, limit, b.len());
        None
    }

    /// `[*`의 `[`와 짝인 `]`의 위치.
    pub(crate) fn match_bracket(&mut self, b: &[u8], start: usize) -> Option<usize> {
        let limit = start.saturating_add(self.budget).min(b.len());
        let (mut depth, mut i) = (0usize, start);
        while i < limit {
            match b[i] {
                b'\\' => {
                    i += 2;
                    continue;
                }
                b'[' => depth += 1,
                b']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        self.charge(start, i, limit, b.len());
                        return Some(i);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        self.charge(start, i, limit, b.len());
        None
    }

    /// 여러 줄에 걸친 `{{{ … }}}` 블록. 처리한 줄 수, 해당하지 않으면 0.
    fn brace_block(&mut self, lines: &[Line], i: usize, depth: usize, out: &mut Vec<Block>) -> usize {
        let first = lines[i].text.as_str();
        let lead = first.len() - first.trim_start().len();
        let is_bang = first[lead..].starts_with("{{{#!");
        // 한 줄 안에서 닫힌 `{{{ }}}`는 인라인이다. `#!` 블록만 예외.
        if !is_bang && self.match_braces(first.as_bytes(), lead).is_some() {
            return 0;
        }
        if self.budget == 0 {
            return 0;
        }
        let mut all = String::new();
        let mut starts = Vec::new();
        for l in &lines[i..] {
            if all.len() > self.budget {
                // 닫히지 않은 `{{{` 줄이 수만 개 — 남은 줄을 매번 잇지 않는다.
                self.budget = 0;
                return 0;
            }
            starts.push(all.len());
            all.push_str(&l.text);
            all.push('\n');
        }
        self.budget = self.budget.saturating_sub(all.len());
        let Some(close) = self.match_braces(all.as_bytes(), lead) else {
            return 0;
        };
        let end_line = starts.iter().rposition(|&s| s <= close).unwrap_or(0);
        let header_end = all[lead..].find('\n').map(|p| p + lead);
        let (header, inner, inner_start) = match header_end {
            Some(he) if close >= he => {
                let inner_start = he + 1;
                let mut inner = if inner_start <= close { all[inner_start..close].to_string() } else { String::new() };
                if inner.ends_with('\n') {
                    inner.pop();
                }
                (all[lead + 3..he].to_string(), inner, inner_start)
            }
            _ => (all[lead + 3..close].to_string(), String::new(), close),
        };
        let mut inner_lines = Vec::new();
        let mut off = lines[i].offset + inner_start;
        for t in inner.split('\n') {
            inner_lines.push(Line { text: t.to_string(), offset: off });
            off += t.len() + 1;
        }
        let h = header.trim();
        if let Some(rest) = h.strip_prefix("#!folding") {
            let summary = rest.trim();
            let children = self.blocks(&inner_lines, depth + 1, false);
            out.push(Block::Fold { summary: if summary.is_empty() { "펼치기".into() } else { summary.to_string() }, children });
        } else if let Some(rest) = h.strip_prefix("#!syntax") {
            let lang = rest.trim();
            out.push(Block::Code { language: (!lang.is_empty()).then(|| lang.to_string()), text: inner.clone() });
        } else if h.starts_with("#!wiki") {
            let children = self.blocks(&inner_lines, depth + 1, false);
            out.push(Block::Indent { depth: 0, children });
        } else if h.starts_with("#!html") {
            out.push(Block::Unsupported { kind: "html".into(), text: inner.clone() });
        } else if h.starts_with("#!") {
            out.push(Block::Code { language: None, text: inner.clone() });
        } else if SIZE.is_match(&format!("{h} ")) || h.starts_with('#') {
            // 크기 · 색 블록. 색은 그리지 않는다.
            let children = self.blocks(&inner_lines, depth + 1, false);
            out.push(Block::Indent { depth: 0, children });
        } else {
            let text = if h.is_empty() { inner.clone() } else if inner.is_empty() { header.clone() } else { format!("{header}\n{inner}") };
            out.push(Block::Code { language: None, text });
        }
        let rest = all[close + 3..].split('\n').next().unwrap_or("");
        if !rest.trim().is_empty() {
            let para = self.inline(rest, depth + 1);
            if has_content(&para) {
                out.push(Block::Paragraph { children: para, start: None, end: None });
            }
        }
        end_line + 1
    }

    fn table(&mut self, lines: &[Line], start: usize, depth: usize, out: &mut Vec<Block>) -> usize {
        let mut i = start;
        let mut rows = Vec::new();
        let mut caption = None;
        while i < lines.len() {
            let t = lines[i].text.as_str();
            if !is_table_start(t) {
                break;
            }
            let mut buf = t.to_string();
            i += 1;
            while !row_closed(&buf) && i < lines.len() && !lines[i].text.starts_with("||") {
                buf.push('\n');
                buf.push_str(&lines[i].text);
                i += 1;
            }
            let mut row = buf.trim_end().to_string();
            if !row.starts_with("||") {
                if let Some(m) = CAPTION.captures(&row) {
                    let whole = m.get(0).map(|g| g.end()).unwrap_or(0);
                    if caption.is_none() {
                        let text = m[1].to_string();
                        caption = Some(self.inline(&text, depth + 1));
                    }
                    row = format!("|{}", &row[whole..]);
                    if !row.starts_with("||") {
                        row = format!("|{row}");
                    }
                }
            }
            // 옛 Dart는 끝 `||`를 먼저 떼고 앞 `||`를 뗐다 — `||` 하나뿐인 줄에서 범위를 넘었다. 앞부터 뗀다.
            let mut body = row.strip_prefix("||").unwrap_or(&row).to_string();
            if body.ends_with("||") {
                body.truncate(body.len() - 2);
            }
            rows.push(self.row(&body, depth));
        }
        out.push(Block::Table { caption, rows });
        i - start
    }

    fn row(&mut self, row: &str, depth: usize) -> Vec<Cell> {
        let parts = split_top_level(row, "||");
        let mut cells = Vec::new();
        let mut span = 0u32;
        for part in &parts {
            if part.is_empty() {
                span += 1;
                continue;
            }
            let mut content: &str = part;
            let (mut col_span, mut row_span, mut align) = (1 + span, 1u32, Align::Start);
            span = 0;
            loop {
                let trimmed = content.trim_start();
                if !trimmed.starts_with('<') {
                    break;
                }
                let Some(close) = trimmed.find('>') else { break };
                let attr = trimmed[1..close].trim();
                if let Some(c) = COLSPAN.captures(attr) {
                    col_span = c[1].parse::<u64>().map_or(64, |n| n.clamp(1, 64)) as u32;
                } else if let Some(c) = ROWSPAN.captures(attr) {
                    row_span = c[1].parse::<u64>().map_or(256, |n| n.clamp(1, 256)) as u32;
                } else if attr == ":" {
                    align = Align::Center;
                } else if attr == "(" {
                    align = Align::Start;
                } else if attr == ")" {
                    align = Align::End;
                } else if !CELL_ATTR.is_match(attr) {
                    break;
                }
                content = &trimmed[close + 1..];
            }
            let children = self.inline(content.trim(), depth + 1);
            cells.push(Cell { col_span, row_span, align, children });
        }
        cells
    }

    fn list(&mut self, lines: &[Line], start: usize, depth: usize, out: &mut Vec<Block>) -> usize {
        let mut entries: Vec<Entry> = Vec::new();
        let mut i = start;
        while i < lines.len() {
            let t = lines[i].text.as_str();
            if t.trim().is_empty() {
                break;
            }
            if let Some(c) = LIST.captures(t) {
                entries.push(Entry { indent: c[1].len(), marker: c[2].to_string(), start_at: c.get(3).and_then(|m| m.as_str().parse().ok()), text: c[4].to_string() });
            } else if t.starts_with(' ') && !entries.is_empty() {
                let e = entries.last_mut().unwrap();
                e.text.push('\n');
                e.text.push_str(t.trim());
            } else {
                break;
            }
            i += 1;
        }
        let mut pos = 0;
        while pos < entries.len() {
            let (block, next) = self.build_list(&entries, pos, entries[pos].indent, depth);
            out.push(block);
            pos = next;
        }
        i - start
    }

    pub(crate) fn build_list(&mut self, entries: &[Entry], start: usize, indent: usize, depth: usize) -> (Block, usize) {
        let marker = entries[start].marker.clone();
        let mut items: Vec<Vec<Block>> = Vec::new();
        let mut i = start;
        while i < entries.len() {
            let e = &entries[i];
            if e.indent < indent {
                break;
            }
            if e.indent > indent && depth < self.opt.max_depth {
                if items.is_empty() {
                    items.push(Vec::new());
                }
                let (nested, next) = self.build_list(entries, i, e.indent, depth + 1);
                items.last_mut().unwrap().push(nested);
                i = next;
                continue;
            }
            // 너무 깊은 목록은 이 단계의 항목으로 이어 붙인다(스택이 넘치지 않게).
            if e.indent == indent && e.marker != marker && !items.is_empty() {
                break;
            }
            let inl = self.inline(&e.text, depth + 1);
            items.push(if has_content(&inl) { vec![Block::Paragraph { children: inl, start: None, end: None }] } else { Vec::new() });
            i += 1;
        }
        let style = match marker.as_str() {
            "1." => ListStyle::Decimal,
            "a." => ListStyle::LowerAlpha,
            "A." => ListStyle::UpperAlpha,
            "i." => ListStyle::LowerRoman,
            "I." => ListStyle::UpperRoman,
            _ => ListStyle::Bullet,
        };
        (Block::List { style, start: entries[start].start_at.unwrap_or(1), items }, i)
    }

    // ---------------------------------------------------------------- 인라인

    pub(crate) fn inline(&mut self, s: &str, depth: usize) -> Vec<Inline> {
        if depth > self.opt.max_depth {
            return vec![Inline::Text(s.to_string())];
        }
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut buf = String::new();
        let mut i = 0;
        'outer: while i < b.len() {
            let c = b[i];
            if c == b'\\' && i + 1 < b.len() {
                let ch = s[i + 1..].chars().next().unwrap();
                buf.push(ch);
                i += 1 + ch.len_utf8();
                continue;
            }
            if c == b'\n' {
                flush(&mut buf, &mut out);
                out.push(Inline::Break);
                i += 1;
                continue;
            }
            if b[i..].starts_with(b"'''''") {
                if let Some(rel) = s[i + 5..].find("'''''") {
                    let end = i + 5 + rel;
                    if end > i + 5 && !s[i + 5..end].contains('\n') {
                        flush(&mut buf, &mut out);
                        let inner = self.inline(&s[i + 5..end], depth + 1);
                        out.push(Inline::Styled { style: Style::Bold, children: vec![Inline::Styled { style: Style::Italic, children: inner }] });
                        i = end + 5;
                        continue;
                    }
                }
            }
            for (marker, style) in PAIRED {
                if !b[i..].starts_with(marker.as_bytes()) {
                    continue;
                }
                let from = i + marker.len();
                if let Some(rel) = s[from..].find(marker) {
                    let end = from + rel;
                    if end > from && !s[from..end].contains('\n') {
                        flush(&mut buf, &mut out);
                        let inner = self.inline(&s[from..end], depth + 1);
                        out.push(Inline::Styled { style, children: inner });
                        i = end + marker.len();
                        continue 'outer;
                    }
                }
                break;
            }
            if b[i..].starts_with(b"{{{") {
                if let Some(end) = self.match_braces(b, i) {
                    flush(&mut buf, &mut out);
                    let inner = &s[i + 3..end];
                    if let Some(m) = SIZE.captures(inner) {
                        let step = m[2].parse::<i8>().unwrap_or(1) * if &m[1] == "+" { 1 } else { -1 };
                        let rest = &inner[m.get(0).map(|g| g.end()).unwrap_or(0)..];
                        let children = self.inline(rest, depth + 1);
                        out.push(Inline::Size { step, children });
                    } else if inner.starts_with('#') && !inner.starts_with("#!") {
                        let content = inner.find(' ').map(|sp| &inner[sp + 1..]).unwrap_or("");
                        let children = self.inline(content, depth + 1);
                        out.extend(children);
                    } else {
                        out.push(Inline::Code(inner.to_string()));
                    }
                    i = end + 3;
                    continue;
                }
            }
            if b[i..].starts_with(b"[[") {
                if let Some(end) = self.match_pair(b, i) {
                    flush(&mut buf, &mut out);
                    if let Some(node) = self.link(&s[i + 2..end], depth) {
                        out.push(node);
                    }
                    i = end + 2;
                    continue;
                }
            }
            if b[i..].starts_with(b"[*") {
                if let Some(end) = self.match_bracket(b, i) {
                    flush(&mut buf, &mut out);
                    let node = self.footnote(&s[i + 2..end], depth);
                    out.push(node);
                    i = end + 1;
                    continue;
                }
            }
            if c == b'[' {
                if let Some(m) = MACRO.captures(window(s, i, 256)) {
                    if let Some(node) = macro_node(&m[1], m.get(2).map(|g| g.as_str())) {
                        flush(&mut buf, &mut out);
                        out.push(node);
                        i += m.get(0).map(|g| g.end()).unwrap_or(1);
                        continue;
                    }
                }
            }
            if self.opt.template && c == b'@' {
                if let Some(m) = PARAM.captures(window(s, i, 256)) {
                    flush(&mut buf, &mut out);
                    out.push(Inline::Param { name: m[1].to_string(), fallback: m.get(2).map(|g| g.as_str().to_string()) });
                    i += m.get(0).map(|g| g.end()).unwrap_or(1);
                    continue;
                }
            }
            let ch = s[i..].chars().next().unwrap();
            buf.push(ch);
            i += ch.len_utf8();
        }
        flush(&mut buf, &mut out);
        out
    }



    fn link(&mut self, inner: &str, depth: usize) -> Option<Inline> {
        let parts = split_top_level(inner, "|");
        let target_text = parts[0].trim();
        let label = (parts.len() > 1).then(|| parts[1..].join("|"));
        if target_text.starts_with("http://") || target_text.starts_with("https://") {
            let label = label.map(|l| self.inline(&l, depth + 1));
            return Some(Inline::Url { uri: target_text.to_string(), label });
        }
        if let Some(anchor) = target_text.strip_prefix('#') {
            let anchor = anchor.trim();
            let label = label.map(|l| self.inline(&l, depth + 1));
            let target = ObjectRef { ns: String::new(), key: String::new(), anchor: (!anchor.is_empty()).then(|| anchor.to_string()) };
            return Some(Inline::Link { target, label });
        }
        let mut target = target_text.to_string();
        let escaped = target.starts_with(':');
        if escaped {
            target.remove(0);
        }
        if let Some(current) = self.opt.current.clone() {
            if target.starts_with('/') {
                target = format!("{current}{target}");
            } else if let Some(rest) = target.strip_prefix("..") {
                if rest.starts_with('/') {
                    let parent = current.rfind('/').map(|cut| &current[..cut]).unwrap_or(&current);
                    target = format!("{parent}{rest}");
                    if target.ends_with('/') {
                        target.pop();
                    }
                }
            }
        }
        let Some(r) = ObjectRef::parse(&target) else {
            return Some(Inline::Text(format!("[[{inner}]]")));
        };
        if !escaped && r.ns == CATEGORY {
            if !self.categories.contains(&r.key) {
                self.categories.push(r.key);
            }
            return None;
        }
        if !escaped && r.ns == FILE {
            return Some(Inline::File { file: r.without_anchor(), options: options(label.as_deref()) });
        }
        let label = label.map(|l| self.inline(&l, depth + 1));
        Some(Inline::Link { target: r, label })
    }

    fn footnote(&mut self, inner: &str, depth: usize) -> Inline {
        let (name, content): (Option<String>, &str) = if !inner.is_empty() && !inner.starts_with(' ') {
            match inner.find(' ') {
                Some(sp) => (Some(inner[..sp].to_string()), &inner[sp + 1..]),
                None => (Some(inner.to_string()), ""),
            }
        } else {
            (None, inner.strip_prefix(' ').unwrap_or(inner))
        };
        let reuse = name.as_ref().filter(|_| content.trim().is_empty()).and_then(|n| self.named.get(n).copied());
        if let Some(number) = reuse {
            return Inline::FootnoteRef { number, name };
        }
        // 번호를 먼저 잡아 둔다 — 옛 Dart는 안쪽 각주가 같은 번호를 받았다.
        let number = self.footnotes.len() as u32 + 1;
        self.footnotes.push(FootnoteDef { number, name: name.clone(), children: Vec::new() });
        if let Some(n) = &name {
            self.named.entry(n.clone()).or_insert(number);
        }
        let children = self.inline(content, depth + 1);
        self.footnotes[(number - 1) as usize].children = children;
        Inline::FootnoteRef { number, name }
    }
}

fn row_closed(row: &str) -> bool {
    let r = row.trim_end();
    r.len() >= 4 && r.ends_with("||")
}

fn macro_node(name: &str, args: Option<&str>) -> Option<Inline> {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "br" => Some(Inline::Break),
        "date" | "datetime" => Some(Inline::Macro { name: "date".into(), args: Vec::new() }),
        "age" | "dday" | "anchor" => args.map(|a| Inline::Macro { name: lower.clone(), args: vec![a.trim().to_string()] }),
        "ruby" => {
            let parts = split_args(args?);
            let base = parts.first()?.clone();
            let kv = key_values(&parts[1..]);
            Some(Inline::Ruby { base, ruby: kv.get("ruby").cloned().unwrap_or_default() })
        }
        _ => None,
    }
}
