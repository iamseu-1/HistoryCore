//! 본문 찾기(설계 5-5 「검색」). 한글 · 한자를 2글자 조각으로 잘라 contentless FTS5(`detail=none`)에 넣는다.
//! 3글자 넘는 덩이는 조각 AND로 후보를 얻고, 저장해 둔 글자(plain)로 확인한다.
//! 조각내기는 Rust가 하고 FTS5에는 띄어 쓴 조각을 넣는다(`tokenize='ascii'` — 비ASCII 글자는 모두 낱말 글자, 공백에서만 끊는다).
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

/// 문서 번호(`rowid`)마다 조각만 든다. 지우기는 `DELETE FROM fts WHERE rowid = ?`.
pub const FTS_SCHEMA: &str = "CREATE VIRTUAL TABLE fts USING fts5(t, content='', contentless_delete=1, detail=none, tokenize='ascii');";

/// 조각 색인은 넷으로 나눈다(문서 번호 % 4) — 통째 짓기가 넷을 나란히 짓고, 찾을 때는 넷을 번호 차례로 합친다.
/// FTS5는 쓰는 쪽 하나로 초당 2–3천 편이라 100만 편을 한 표에 지으면 8분이 넘는다(bench/README.md).
pub const SHARDS: usize = 4;
pub const SHARD_TABLES: [&str; SHARDS] = ["fts0", "fts1", "fts2", "fts3"];

pub fn shard_of(rowid: i64) -> usize {
    rowid.rem_euclid(SHARDS as i64) as usize
}

/// `FTS_SCHEMA`와 같은 선택으로 이름만 바꾼 표.
pub fn shard_schema(table: &str) -> String {
    FTS_SCHEMA.replacen("TABLE fts ", &format!("TABLE {table} "), 1)
}

/// 넣을 때와 찾을 때 같은 꼴로 — NFC(결합 자모로 친 한글을 음절로) + 소문자.
pub fn normalize(text: &str) -> String {
    text.nfc().flat_map(char::to_lowercase).collect()
}

/// 저장해 두는 글자(plain) — NFC만 한다(대소문자는 둔다: 미리보기에 그대로 보인다).
/// 확인할 덩이는 한글 · 한자뿐이라 대소문자가 없으니, 찾을 때 이 글을 다시 정규화하지 않고 그대로 견준다.
pub fn stored_plain(text: &str) -> String {
    text.nfc().collect()
}

/// 2글자 조각으로 자르는 「덩이 글자」 — 한글 음절과 한자(그리기 모델의 한자 구간과 같은 범위).
fn is_block(c: char) -> bool {
    matches!(c as u32, 0xAC00..=0xD7A3 | 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

enum Piece {
    /// 덩이 글자가 이어진 것.
    Run(Vec<char>),
    /// 영문 · 숫자 같은 낱말 글자가 이어진 것.
    Word(String),
}

/// 정규화한 글을 덩이 · 낱말로 가른다. 나머지(공백 · 구두점 · 이모지)는 가르기만 한다.
fn pieces(normalized: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let (mut run, mut word) = (Vec::new(), String::new());
    let flush = |run: &mut Vec<char>, word: &mut String, out: &mut Vec<Piece>| {
        if !run.is_empty() {
            out.push(Piece::Run(std::mem::take(run)));
        }
        if !word.is_empty() {
            out.push(Piece::Word(std::mem::take(word)));
        }
    };
    for c in normalized.chars() {
        if is_block(c) {
            if !word.is_empty() {
                flush(&mut Vec::new(), &mut word, &mut out);
            }
            run.push(c);
        } else if c.is_alphanumeric() {
            if !run.is_empty() {
                flush(&mut run, &mut String::new(), &mut out);
            }
            word.push(c);
        } else {
            flush(&mut run, &mut word, &mut out);
        }
    }
    flush(&mut run, &mut word, &mut out);
    out
}

/// 넣을 조각 — 덩이 `c1…cn`은 `c1c2, c2c3, …`과 끝 글자 `cn`(1글자 찾기가 앞붙이 질의로 모든 자리를 만나도록), 낱말은 그대로.
/// 처음 나온 차례, 문서 안 같은 조각은 한 번.
pub fn index_tokens(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut put = |t: String| {
        if seen.insert(t.clone()) {
            out.push(t);
        }
    };
    for p in pieces(&normalize(text)) {
        match p {
            Piece::Run(cs) => {
                for w in cs.windows(2) {
                    put(w.iter().collect());
                }
                put(cs[cs.len() - 1].to_string());
            }
            Piece::Word(w) => put(w),
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    /// 모두 든 문서가 후보(AND).
    pub must: Vec<String>,
    /// 이 글자로 시작하는 조각이 든 문서(1글자 덩이).
    pub prefix: Vec<String>,
    /// 후보의 글자(정규화)에 그대로 들어 있어야 하는 덩이 — 3글자 이상. 2글자 덩이는 조각이 곧 그 글자라 확인하지 않는다.
    pub verify: Vec<String>,
}

/// 검색어를 질의로. 조각이 하나도 없으면 None.
pub fn parse_query(q: &str) -> Option<Query> {
    let mut query = Query { must: Vec::new(), prefix: Vec::new(), verify: Vec::new() };
    let push = |v: &mut Vec<String>, t: String| {
        if !v.contains(&t) {
            v.push(t);
        }
    };
    for p in pieces(&normalize(q)) {
        match p {
            Piece::Run(cs) if cs.len() == 1 => push(&mut query.prefix, cs[0].to_string()),
            Piece::Run(cs) => {
                for w in cs.windows(2) {
                    push(&mut query.must, w.iter().collect());
                }
                if cs.len() > 2 {
                    push(&mut query.verify, cs.iter().collect());
                }
            }
            Piece::Word(w) => push(&mut query.must, w),
        }
    }
    (!query.must.is_empty() || !query.prefix.is_empty()).then_some(query)
}

/// FTS5 MATCH 글. 조각은 낱말 글자뿐이라 따옴표 안에 `"`가 들 일이 없다.
pub fn fts_match(q: &Query) -> String {
    let must = q.must.iter().map(|t| format!("\"{t}\""));
    let prefix = q.prefix.iter().map(|t| format!("\"{t}\"*"));
    must.chain(prefix).collect::<Vec<_>>().join(" AND ")
}

/// 후보 문서의 글자(`stored_plain`으로 맞춘 것)가 질의를 정말 담는가. 후보마다 불리므로 글 전체를 다시 정규화하지 않는다.
pub fn verify(plain: &str, q: &Query) -> bool {
    q.verify.iter().all(|v| plain.contains(v.as_str()))
}

/// 첫 맞은 곳 앞뒤 `around`글자. 공백은 하나로 줄이고, 자른 끝에 `…`.
pub fn snippet(plain: &str, q: &Query, around: usize) -> String {
    let needles: Vec<&String> = q.verify.iter().chain(&q.must).chain(&q.prefix).collect();
    // 원문에서 바로 찾고, 대소문자 · 정규화 차이로 못 찾으면 정규화한 글에서 찾는다.
    let (text, at) = match needles.iter().find_map(|n| plain.find(n.as_str()).map(|i| (i, n.len()))) {
        Some(hit) => (plain.to_string(), Some(hit)),
        None => {
            let n = normalize(plain);
            let hit = needles.iter().find_map(|x| n.find(x.as_str()).map(|i| (i, x.len())));
            (n, hit)
        }
    };
    let (start, len) = at.unwrap_or((0, 0));
    let before: Vec<char> = text[..start].chars().collect();
    let after: Vec<char> = text[start + len..].chars().collect();
    let from = before.len().saturating_sub(around);
    let to = around.min(after.len());
    let mut s = String::new();
    if from > 0 {
        s.push('…');
    }
    s.extend(&before[from..]);
    s.push_str(&text[start..start + len]);
    s.extend(&after[..to]);
    if to < after.len() {
        s.push('…');
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
