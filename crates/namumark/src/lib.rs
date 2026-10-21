//! 나무마크 해석기(설계 5-8) — 옛 Dart 해석기를 옮겼다. 서재를 모르는 순수 함수만 둔다.
pub mod anchors;
pub mod ast;
pub mod parser;
pub mod plain;
pub mod refs;

pub use ast::{Align, Block, Cell, Document, FootnoteDef, Inline, ListStyle, Section, Style};
pub use parser::{parse, Options};
pub use anchors::anchor_blocks;
pub use plain::{out_links, plain_document, plain_inline, LinkKind, OutLink};
pub use refs::{ObjectRef, CATEGORY, DEFAULT_NS, FILE, NAMESPACES, TEMPLATE, TIMELINE};
