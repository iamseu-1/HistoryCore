//! 나무마크 해석기(설계 5-8) — 옛 Dart 해석기를 옮겼다. 서재를 모르는 순수 함수만 둔다.
pub mod ast;
pub mod plain;
pub mod refs;

pub use ast::{Align, Block, Cell, Document, FootnoteDef, Inline, ListStyle, Section, Style};
pub use plain::plain_inline;
pub use refs::{ObjectRef, CATEGORY, DEFAULT_NS, FILE, NAMESPACES, TEMPLATE, TIMELINE};
