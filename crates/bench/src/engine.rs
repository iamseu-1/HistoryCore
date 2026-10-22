//! P2 서재 엔진(`history_index::Engine`)을 측정 판의 약속에 꽂는다 — 같은 더미 서재 · 같은 일감으로 P1 기준 구현과 견준다.
use crate::{Aggregates, Library, Result};
use history_corpus::{GenDoc, Ns};
use history_index::{Engine, HomeOptions};
use history_library::{DocId, Namespace, NewDoc};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// 한꺼번에 들이기의 트랜잭션 크기.
const IMPORT_BATCH: usize = 10_000;
/// 분류 폴더 한 쪽(화면이 한 번에 받는 수).
const FOLDER_PAGE: usize = 200;

pub struct EngineLibrary {
    dir: PathBuf,
    workers: usize,
    // 측정 판의 읽기 일은 `&self`라 엔진(풀기 문맥 · 나무 캐시를 고친다)을 RefCell에 둔다.
    engine: RefCell<Option<Engine>>,
    ids: Vec<DocId>,
    back: HashMap<DocId, u32>,
    import_ms: u64,
    index_ms: u64,
    rebuild: history_index::RebuildReport,
}

impl EngineLibrary {
    /// `dir/HistoryCore`에 서재와 색인을 둔다. `workers`는 통째 짓기의 해석 일꾼 수.
    pub fn open(dir: &Path, workers: usize) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(EngineLibrary { dir: dir.to_path_buf(), workers, engine: RefCell::new(None), ids: Vec::new(), back: HashMap::new(), import_ms: 0, index_ms: 0, rebuild: Default::default() })
    }

    fn with<T>(&self, f: impl FnOnce(&mut Engine) -> history_library::Result<T>) -> Result<T> {
        let mut e = self.engine.borrow_mut();
        let e = e.as_mut().ok_or("엔진이 열리지 않았다")?;
        Ok(f(e)?)
    }

    fn ours(&self, docs: impl IntoIterator<Item = DocId>) -> Vec<u32> {
        docs.into_iter().filter_map(|d| self.back.get(&d).copied()).collect()
    }
}

fn namespace(ns: Ns) -> Namespace {
    match ns {
        Ns::Doc => Namespace::Doc,
        Ns::Category => Namespace::Category,
    }
}

impl Library for EngineLibrary {
    fn name(&self) -> &'static str {
        "engine"
    }

    fn load(&mut self, docs: &mut dyn Iterator<Item = GenDoc>) -> Result<()> {
        *self.engine.borrow_mut() = None;
        let folder = self.dir.join(history_library::FOLDER);
        if folder.exists() {
            std::fs::remove_dir_all(&folder)?;
        }
        let mut e = Engine::open(&self.dir)?;
        let t = Instant::now();
        let mut gen_ids = Vec::new();
        let mut new_docs = docs.map(|d| {
            gen_ids.push(d.id);
            NewDoc { ns: namespace(d.ns), title: d.title, body: d.body }
        });
        let saved = e.lib.import(&mut new_docs, IMPORT_BATCH)?;
        drop(new_docs);
        self.import_ms = t.elapsed().as_millis() as u64;
        let t = Instant::now();
        self.rebuild = e.rebuild_index(self.workers)?;
        self.index_ms = t.elapsed().as_millis() as u64;
        let max = gen_ids.iter().copied().max().unwrap_or(0) as usize;
        self.ids = vec![DocId::new(); max + 1];
        self.back = HashMap::with_capacity(saved.len());
        for (g, s) in gen_ids.into_iter().zip(&saved) {
            self.ids[g as usize] = s.doc;
            self.back.insert(s.doc, g);
        }
        *self.engine.borrow_mut() = Some(e);
        Ok(())
    }

    fn find_title(&self, prefix: &str, limit: usize) -> Result<Vec<u32>> {
        let docs = self.with(|e| e.search_titles(Namespace::Doc, prefix, limit))?;
        Ok(self.ours(docs.into_iter().map(|d| d.id)))
    }

    fn search_body(&self, query: &str, limit: usize) -> Result<Vec<u32>> {
        let hits = self.with(|e| e.search_body(query, limit))?;
        Ok(self.ours(hits.into_iter().map(|h| h.doc.id)))
    }

    fn open(&self, id: u32) -> Result<Option<String>> {
        let doc = self.ids[id as usize];
        Ok(self.with(|e| e.open_doc(doc, true))?.map(|o| o.model))
    }

    fn save(&mut self, id: u32, body: &str) -> Result<()> {
        let doc = self.ids[id as usize];
        self.with(|e| {
            let head = e.lib.summary(doc)?.map(|(s, _)| s.head).ok_or(history_library::Error::NotFound(format!("문서 {doc}")))?;
            e.save(doc, head, body, "")
        })?;
        Ok(())
    }

    fn category_children(&self, category: &str) -> Result<Vec<u32>> {
        let c = self.with(|e| e.children(Some(category), FOLDER_PAGE, None))?;
        Ok(self.ours(c.docs.into_iter().map(|d| d.id)))
    }

    fn home_aggregates(&self) -> Result<Aggregates> {
        let h = self.with(|e| e.home(&HomeOptions::default()))?;
        Ok(Aggregates { groups: h.groups.into_iter().map(|g| (g.name, g.total)).collect(), recent: self.ours(h.recent.into_iter().map(|d| d.id)) })
    }

    fn reopen(&mut self) -> Result<()> {
        *self.engine.borrow_mut() = None;
        *self.engine.borrow_mut() = Some(Engine::open(&self.dir)?);
        Ok(())
    }

    fn bytes_on_disk(&self) -> Result<u64> {
        Ok(self.parts_on_disk()?.iter().map(|(_, b)| b).sum())
    }

    fn load_parts(&self) -> Vec<(&'static str, u64)> {
        let r = self.rebuild;
        vec![
            ("서재 들이기", self.import_ms),
            ("색인 짓기", self.index_ms),
            ("(짓기: 읽기", r.prepare_ms),
            ("해석 · 쓰기 · 조각 넷", r.build_ms),
            ("다 넣은 뒤 색인 · 셈", r.finish_ms),
            ("조각 옮기기", r.copy_ms),
            ("ANALYZE", r.analyze_ms),
            ("바꿔 끼우기", r.swap_ms),
            ("일꾼 합", r.worker_ms),
            ("쓰는 쪽", r.writer_ms),
            ("조각 하나 최대)", r.shard_ms),
        ]
    }

    fn disk_parts(&self) -> Vec<(&'static str, u64)> {
        self.parts_on_disk().unwrap_or_default()
    }
}

impl EngineLibrary {
    /// 원본(library.db) · 색인(index.db) 크기. WAL · 공유 메모리 파일도 그 몫에 넣는다.
    fn parts_on_disk(&self) -> Result<Vec<(&'static str, u64)>> {
        let folder = self.dir.join(history_library::FOLDER);
        let size = |stem: &str| -> u64 {
            ["", "-wal", "-shm"].iter().filter_map(|s| std::fs::metadata(folder.join(format!("{stem}{s}"))).ok()).map(|m| m.len()).sum()
        };
        Ok(vec![("원본 library.db", size("library.db")), ("색인 index.db", size("index.db"))])
    }
}
