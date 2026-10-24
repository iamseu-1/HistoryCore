//! 서재 창구(spec/windows-api.md 3-1 · 4절) — 바깥 앱(HistoryTime)이 같은 기기의 아카이브 서재를 **읽기 전용으로** 묻는다.
//!
//! 쓰는 길이 없는 따로 된 형이다(계획 R2 — 쓰는 `Engine`에 깃발을 두면 쓰는 메서드를 하나라도 빠뜨렸을 때 쓴다).
//! - 만들지 않는다: 서재가 없으면 `no_library`. 색인이 없거나 판 · 서재가 다르면 색인을 쓰는 질의만 `no_index`(다시 짓지 않는다 — R3).
//! - 쓰지 않는다: 두 파일 모두 `SQLITE_OPEN_READ_ONLY` · `query_only`로 연다. 낡은 문서의 모델은 그 자리에서 그리되 색인에 굳혀 넣지 않는다.
//! - 질의마다 열고, 읽기 거래 하나로 읽고, 곧 닫는다. 손잡이를 쥔 채로도 파일을 쥐지 않으므로 아카이브는 색인을 바꿔 끼우거나
//!   (통째 짓기 — Windows는 열린 파일을 지우지 못한다) 곁 파일을 거둘 수 있다. 이 열기가 만든 곁 파일(`-wal` · `-shm`)은 질의 끝에 치운다(R4).
use crate::engine::{draw, search_body_in, search_titles_in, timeline_docs_in, titles_exist_in, unzip_with, Hit};
use crate::store::{meta_i64, ReadIndex};
use history_library::{in_read_tx, DocId, DocSummary, Error, LibraryId, Namespace, Reader, Result, FOLDER};
use history_namumark::{ObjectRef, Templates};
use rusqlite::{params, Connection, OptionalExtension};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

/// 열린 서재 창구 하나 — 자리만 들고 있다(파일은 질의 동안만 연다).
pub struct Window {
    app_dir: PathBuf,
    folder: PathBuf,
    unz: zstd::bulk::Decompressor<'static>,
}

/// `window.doc` · `window.doc_by_title`의 답.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowDoc {
    pub summary: DocSummary,
    /// 그리기 모델 JSON 글 그대로(spec/namumark.md 판 1) — 다리가 풀지 않고 끼운다(깊은 중첩도 그대로).
    pub model: String,
}

impl Window {
    /// 앱 자료 폴더 `app_dir`의 서재(`HistoryCore/library.db`)를 창구로 연다 — 그 자리에 우리 서재가 있는지만 보고 곧 놓는다.
    /// 없거나 비었으면 `NoLibrary`(아무것도 만들지 않는다), 남의 파일이면 `NotALibrary`, 더 새 판이면 `NewerFormat`.
    pub fn open(app_dir: &Path) -> Result<Window> {
        drop(Reader::open_library(app_dir)?);
        Ok(Window { app_dir: app_dir.to_path_buf(), folder: app_dir.join(FOLDER), unz: zstd::bulk::Decompressor::new()? })
    }

    /// 서재 ID(`window.version`의 `library` — 아카이브가 다른 서재로 합류하면 바뀐다).
    pub fn version(&mut self) -> Result<LibraryId> {
        self.read(|_, r| r.library())
    }

    /// 문서 하나의 요약과 그리기 모델. 없거나 지운 문서면 None. 넘겨주기를 따라가지 않는다(모델의 `redirect`를 보고 묻는 쪽이 다시 묻는다).
    pub fn doc(&mut self, id: DocId) -> Result<Option<WindowDoc>> {
        self.read(|w, r| w.doc_in(r, id))
    }

    /// 제목(서재의 제목 규칙으로 맞춤)의 살아 있는 문서 — 같은 제목이 둘이면 먼저 만든 것. 제목으로 쓸 수 없는 글이면 None.
    pub fn doc_by_title(&mut self, ns: Namespace, title: &str) -> Result<Option<WindowDoc>> {
        self.read(|w, r| match r.first_by_title(ns, title)? {
            Some(id) => w.doc_in(r, id),
            None => Ok(None),
        })
    }

    /// 이 연표 항목을 잇는 문서(연표 링크 · 연표 띠) — (맞춘 항목ID, 문서). 항목ID는 앞뒤 공백을 떼고 NFC로 맞춘다(spec/windows-api.md 8절).
    /// 자리 꼬리(`#1` · `.mv<해>` · `~seg<수>`)는 떼지 않는다 — 원 ID로 묻는 것은 묻는 쪽 몫. `timeline.docs`와 같은 찾기다.
    pub fn docs_for_entity(&mut self, entity: &str) -> Result<(String, Vec<DocSummary>)> {
        let entity: String = entity.trim().nfc().collect();
        let docs = self.with_index(|_, _, idx| timeline_docs_in(idx, &entity))?;
        Ok((entity, docs))
    }

    /// 있는 문서 ID — 같은 차례, 없으면 None(`titles.exist`와 같다).
    pub fn titles_exist(&mut self, titles: &[(Namespace, String)]) -> Result<Vec<Option<DocId>>> {
        self.with_index(|_, _, idx| titles_exist_in(idx, titles))
    }

    /// 제목 앞부분(일반 문서)으로 찾은 것을 먼저(제목 차례 · `snippet`은 빈 글), 이어 본문으로 찾은 것 — 같은 문서는 한 번, `limit`까지.
    pub fn search(&mut self, q: &str, limit: usize) -> Result<Vec<Hit>> {
        self.with_index(|unz, r, idx| {
            let mut out: Vec<Hit> = search_titles_in(idx, Namespace::Doc, q, limit)?.into_iter().map(|doc| Hit { doc, snippet: String::new() }).collect();
            if out.len() < limit {
                // 본문 줄 중 제목 줄과 겹치는 것은 많아야 제목 줄 수만큼이라, `limit`개를 받으면 빈자리를 채우기에 넉넉하다.
                for hit in search_body_in(idx, unz, r, q, limit)? {
                    if out.len() == limit {
                        break;
                    }
                    if !out.iter().any(|h| h.doc.id == hit.doc.id) {
                        out.push(hit);
                    }
                }
            }
            Ok(out)
        })
    }

    /// 서재를 읽기 전용으로 열어 읽기 거래 하나 안에서 `f`를 부르고 닫는다(곁 파일 치우기는 `Reader`가 놓일 때).
    fn read<T>(&mut self, f: impl FnOnce(&mut Self, &Reader) -> Result<T>) -> Result<T> {
        let lib = Reader::open_library(&self.app_dir)?;
        lib.read(|r| f(self, r))
    }

    /// `read` 안에서 색인도 읽기 전용으로 열어 그 읽기 거래 안에서 `f`를 부른다. 쓸 수 있는 색인이 없으면 `NoIndex`.
    fn with_index<T>(&mut self, f: impl FnOnce(&mut zstd::bulk::Decompressor<'static>, &Reader, &Connection) -> Result<T>) -> Result<T> {
        self.read(|w, r| {
            let idx = ReadIndex::open(&w.folder, r.library()?)?.ok_or(Error::NoIndex)?;
            in_read_tx(&idx.db, || f(&mut w.unz, r, &idx.db))
        })
    }

    fn doc_in(&mut self, r: &Reader, id: DocId) -> Result<Option<WindowDoc>> {
        let Some((summary, false)) = r.summary(id)? else {
            return Ok(None);
        };
        if let Some(model) = self.indexed_model(r, &summary)? {
            return Ok(Some(WindowDoc { summary, model }));
        }
        // 색인이 없거나 낡았다 — 같은 읽기 거래에서 머리 판과 틀을 읽어 그린다(굳혀 넣지 않는다).
        let Some(d) = r.get(id)? else {
            return Ok(None);
        };
        let templates = ReaderTemplates { reader: r, error: RefCell::new(None) };
        let (model, _) = draw(&d, &templates);
        if let Some(e) = templates.error.into_inner() {
            return Err(e);
        }
        Ok(Some(WindowDoc { summary, model: serde_json::to_string(&model).expect("모델 JSON") }))
    }

    /// 색인에 굳혀 둔 모델 — 색인이 있고, 그 문서를 지금 머리 판으로 색인했고, 다시 그릴 문서(`stale` — 바뀐 틀을 쓰는 문서)가 아닐 때만.
    /// 색인이 서재보다 뒤처졌으면(눈금 < 마지막 판) 그 사이 틀이 바뀌었을 수 있어, 틀을 쓰는 문서도 None — 그 자리에서 그린다.
    fn indexed_model(&mut self, r: &Reader, s: &DocSummary) -> Result<Option<String>> {
        let Some(idx) = ReadIndex::open(&self.folder, r.library()?)? else {
            return Ok(None);
        };
        let unz = &mut self.unz;
        in_read_tx(&idx.db, || {
            let row: Option<(i64, history_library::RevId, bool, bool)> = idx
                .db
                .prepare_cached(
                    "SELECT i.rowid, i.rev, EXISTS(SELECT 1 FROM stale WHERE doc = i.doc), EXISTS(SELECT 1 FROM uses WHERE doc = i.rowid) FROM item i WHERE i.doc = ?1",
                )?
                .query_row(params![s.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
                .optional()?;
            let Some((rowid, rev, stale, uses_templates)) = row else {
                return Ok(None);
            };
            if rev != s.head || stale || (uses_templates && meta_i64(&idx.db, "seq")? < r.last_seq()?) {
                return Ok(None);
            }
            let z: Option<Vec<u8>> = idx.db.prepare_cached("SELECT z FROM model WHERE rowid = ?1")?.query_row([rowid], |row| row.get(0)).optional()?;
            z.map(|z| unzip_with(unz, &z)).transpose()
        })
    }
}

/// 창구가 그릴 때의 틀 원문 — 같은 읽기 연결 · 같은 거래에서 읽는다(쓰는 엔진의 `LibraryTemplates`와 같은 셈: 이름공간 · 맞춘 제목의
/// 살아 있는 문서 중 먼저 만든 것의 머리 본문). 읽기 오류는 붙잡아 두었다가 그리기가 끝난 뒤 오류로 낸다 — `missing`으로 굳지 않도록.
struct ReaderTemplates<'a> {
    reader: &'a Reader,
    error: RefCell<Option<Error>>,
}

impl Templates for ReaderTemplates<'_> {
    fn source(&self, target: &ObjectRef) -> Option<String> {
        let ns = Namespace::ALL.into_iter().find(|n| n.prefix() == target.ns)?;
        match self.reader.body_by_title(ns, &target.key) {
            Ok(body) => body,
            Err(e) => {
                self.error.borrow_mut().get_or_insert(e);
                None
            }
        }
    }
}
