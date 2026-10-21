//! 100만 건 측정용 더미 서재(설계 9-1 P1). 정해진 씨앗에서 늘 같은 문서가 나오고,
//! 문서 i는 앞 문서 없이 바로 만들 수 있다(파일로 두지 않고 흘려 낸다).
mod doc;
mod pool;
mod rng;
mod tree;

pub use doc::{GenDoc, Ns, SOURCES};
pub use pool::TextPool;
pub use rng::Rng;
pub use tree::{Category, CategoryTree};

use std::collections::HashSet;

/// 더미 서재의 크기와 씨앗.
#[derive(Clone, Copy, Debug)]
pub struct CorpusSpec {
    /// 일반 문서 수(분류 문서는 따로 더해진다).
    pub docs: u32,
    pub seed: u64,
}

pub struct Corpus<'a> {
    spec: CorpusSpec,
    pool: &'a TextPool,
    tree: CategoryTree,
    titles: Vec<String>,
}

impl<'a> Corpus<'a> {
    pub fn new(spec: CorpusSpec, pool: &'a TextPool) -> Self {
        let tree = CategoryTree::build(spec.docs, spec.seed, pool);
        let mut seen: HashSet<String> = tree.categories().iter().map(|c| c.name.clone()).collect();
        let mut titles = Vec::with_capacity(spec.docs as usize);
        for i in 0..spec.docs {
            let base = doc::title_for(spec.seed, i, pool);
            let mut t = base.clone();
            let mut n = 2;
            while !seen.insert(t.clone()) {
                t = format!("{base} ({n})");
                n += 1;
            }
            titles.push(t);
        }
        Corpus { spec, pool, tree, titles }
    }

    pub fn spec(&self) -> CorpusSpec {
        self.spec
    }
    pub fn tree(&self) -> &CategoryTree {
        &self.tree
    }
    pub fn pool(&self) -> &TextPool {
        self.pool
    }
    /// 일반 문서 i의 제목.
    pub fn title(&self, i: u32) -> &str {
        &self.titles[i as usize]
    }
    /// 분류 문서 + 일반 문서의 수.
    pub fn len(&self) -> u32 {
        self.tree.categories().len() as u32 + self.spec.docs
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// 문서 하나(0..len). 분류 문서가 앞, 그다음 일반 문서.
    pub fn doc(&self, idx: u32) -> GenDoc {
        let cats = self.tree.categories().len() as u32;
        if idx < cats {
            doc::category_doc(self, idx)
        } else {
            doc::article(self, idx - cats, idx)
        }
    }
    /// 모든 문서를 차례로 흘려 낸다(한꺼번에 메모리에 두지 않는다).
    pub fn docs(&self) -> impl Iterator<Item = GenDoc> + '_ {
        (0..self.len()).map(move |i| self.doc(i))
    }
}
