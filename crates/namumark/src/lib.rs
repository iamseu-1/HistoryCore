//! 나무마크 해석기(설계 5-8) — 옛 Dart 해석기를 옮겼다. 서재를 모르는 순수 함수만 둔다.
pub mod anchors;
pub mod ast;
pub mod model;
pub mod parser;
pub mod plain;
pub mod refs;

pub use ast::{Align, Block, Cell, Document, FootnoteDef, Inline, ListStyle, Section, Style};
pub use model::{render, MAlign, MBlock, MCell, MFootnote, MInline, MListStyle, MSection, MStyle, Model, NoTemplates, RenderOptions, TemplateStatus, Templates, MAX_TEMPLATE_BYTES, MAX_TEMPLATE_DEPTH, MAX_TEMPLATE_EXPANSIONS, MODEL_VERSION};
pub use parser::{parse, redirect_of, Options};
pub use anchors::anchor_blocks;
pub use plain::{out_links, plain_document, plain_inline, timeline_entities, LinkKind, OutLink};
pub use refs::{ObjectRef, CATEGORY, DEFAULT_NS, FILE, NAMESPACES, TEMPLATE, TIMELINE};
