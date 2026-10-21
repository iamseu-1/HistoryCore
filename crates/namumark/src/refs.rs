use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// 서재가 아는 이름공간 접두어(`분류:조선`의 `분류`). 서재 밖 자료(연표 · 字 · 책)도 든다.
/// `history_library::RESERVED_PREFIXES`와 같아야 한다.
pub const NAMESPACES: [&str; 8] = ["문서", "분류", "틀", "파일", "대문", "연표", "字", "책"];
pub const DEFAULT_NS: &str = "문서";
pub const CATEGORY: &str = "분류";
pub const TEMPLATE: &str = "틀";
pub const FILE: &str = "파일";
pub const TIMELINE: &str = "연표";

/// 이름공간과 제목(+앵커)으로 가리키는 대상. 같은 문서 안의 문단을 가리키면 `ns` · `key`가 비어 있다.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectRef {
    pub ns: String,
    pub key: String,
    pub anchor: Option<String>,
}

impl ObjectRef {
    pub fn new(ns: &str, key: &str) -> ObjectRef {
        ObjectRef { ns: ns.to_string(), key: key.to_string(), anchor: None }
    }

    /// 화면에 보이는 제목 — 일반 문서는 접두어를 붙이지 않는다.
    pub fn title(&self) -> String {
        if self.ns == DEFAULT_NS {
            self.key.clone()
        } else {
            format!("{}:{}", self.ns, self.key)
        }
    }

    /// 앵커를 뺀 비교용 키 `이름공간:제목`.
    pub fn path(&self) -> String {
        format!("{}:{}", self.ns, self.key)
    }

    pub fn without_anchor(&self) -> ObjectRef {
        ObjectRef { ns: self.ns.clone(), key: self.key.clone(), anchor: None }
    }

    pub fn is_same_page(&self) -> bool {
        self.key.is_empty()
    }

    /// `틀:조선의 전쟁#인자` 같은 글을 주소로 — 옛 Dart `ObjectRef.tryParse`와 같다. NFC로 맞춘다.
    /// 콜론 앞이 [NAMESPACES]에 있을 때만 이름공간으로 가른다. 쓸 글자가 없으면 None.
    pub fn parse(text: &str) -> Option<ObjectRef> {
        let nfc: String = text.trim().nfc().collect();
        let mut raw = nfc.as_str();
        if let Some(rest) = raw.strip_prefix(':') {
            raw = rest.trim();
        }
        if raw.is_empty() {
            return None;
        }
        let mut anchor = None;
        if let Some(hash) = raw.find('#') {
            let a = raw[hash + 1..].trim();
            if !a.is_empty() {
                anchor = Some(a.to_string());
            }
            raw = raw[..hash].trim();
        }
        if raw.is_empty() {
            return None;
        }
        let mut ns = DEFAULT_NS;
        if let Some(colon) = raw.find(':') {
            let head = raw[..colon].trim();
            if colon > 0 && NAMESPACES.contains(&head) {
                ns = NAMESPACES.iter().find(|n| **n == head).copied().unwrap_or(DEFAULT_NS);
                raw = raw[colon + 1..].trim();
            }
        }
        if raw.is_empty() {
            return None;
        }
        Some(ObjectRef { ns: ns.to_string(), key: raw.to_string(), anchor })
    }
}
