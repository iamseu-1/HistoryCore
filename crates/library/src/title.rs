use crate::error::{Error, Result};
use crate::ns::RESERVED_PREFIXES;
use unicode_normalization::UnicodeNormalization;

pub const MAX_TITLE_CHARS: usize = 200;

/// 붙여 넣은 글에 흔히 섞이는 보이지 않는 글자 — 지운다: 폭 없는 공백 · 단어 잇기 · BOM · 무른 하이픈.
/// (ZWJ · ZWNJ는 이모지 · 문자 조합에 쓰이므로 남긴다.)
const INVISIBLE: [char; 4] = ['\u{200B}', '\u{2060}', '\u{FEFF}', '\u{00AD}'];

/// 글의 방향을 몰래 바꾸거나 줄을 나누는 글자 — 받지 않는다.
fn is_layout_control(c: char) -> bool {
    matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{2028}' | '\u{2029}')
}

/// 제목을 저장할 꼴로 맞춘다 — NFC(호환 한자는 통합 한자로) · 보이지 않는 글자 지움 · 앞뒤 공백 없앰.
/// 링크 문법을 깨는 글자(`[` `]` `|` `#` · 제어 문자 · 방향 · 줄 나눔 글자), `:`로 시작, 이름공간 접두어로 시작하는 제목은 받지 않는다.
pub fn normalize_title(raw: &str) -> Result<String> {
    let nfc: String = raw.nfc().filter(|c| !INVISIBLE.contains(c)).collect();
    let title = nfc.trim();
    if title.is_empty() {
        return Err(Error::BadTitle("비어 있음".into()));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(Error::BadTitle(format!("{MAX_TITLE_CHARS}자를 넘음")));
    }
    if let Some(c) = title.chars().find(|&c| c.is_control() || is_layout_control(c) || matches!(c, '[' | ']' | '|' | '#')) {
        return Err(Error::BadTitle(format!("쓸 수 없는 글자 {c:?}")));
    }
    if title.starts_with(':') {
        return Err(Error::BadTitle("`:`로 시작함".into()));
    }
    if let Some((head, _)) = title.split_once(':') {
        if RESERVED_PREFIXES.contains(&head.trim()) {
            return Err(Error::BadTitle(format!("이름공간 접두어 «{}:»로 시작함", head.trim())));
        }
    }
    Ok(title.to_string())
}
