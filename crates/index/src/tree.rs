//! 분류 나무(설계 5-3) — 일반 문서의 집은 첫 분류, 분류의 부모는 그 분류 문서의 첫 분류.
//! 분류 문서가 없는 분류는 맨 위에 선다. 고리는 고리마다 이름이 가장 작은 분류의 부모 줄을 끊고 알린다(기기마다 같은 나무).
//! 폴더 수 = 그 분류와 아래 분류가 집인 일반 문서 수. 분류마다 바로 든 수(`folder` 표)와 분류 문서의 부모만 읽어
//! 메모리에서 센다 — 분류 수만큼만 읽고 문서 수만큼 읽지 않는다.
use history_library::{DocSummary, Result};
use rusqlite::{params, Connection};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub total: u64,
    /// 분류 문서가 있다(없으면 「상위 분류 없음」으로 맨 위에 선다).
    pub has_doc: bool,
    /// 고리를 끊으려고 이 분류의 부모 줄을 끊었다.
    pub cycle: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Children {
    /// 하위 분류(이름 차례).
    pub categories: Vec<Folder>,
    /// 그 분류가 집인 일반 문서(제목 차례, `limit`까지).
    pub docs: Vec<DocSummary>,
    /// 맨 위에서만 — 「분류 안 됨」 문서 수.
    pub uncategorized: Option<u64>,
    /// 끊은 고리(이름 차례).
    pub cycles: Vec<String>,
}

#[derive(Default)]
struct Node {
    parent: Option<String>,
    children: Vec<String>,
    direct: u64,
    total: u64,
    has_doc: bool,
    cycle: bool,
}

pub(crate) struct Tree {
    nodes: BTreeMap<String, Node>,
    roots: Vec<String>,
    cycles: Vec<String>,
    uncategorized: u64,
}

impl Tree {
    pub(crate) fn load(db: &Connection) -> Result<Tree> {
        let mut nodes: BTreeMap<String, Node> = BTreeMap::new();
        let mut uncategorized = 0;
        {
            let mut st = db.prepare_cached("SELECT name, direct FROM folder")?;
            let mut rows = st.query([])?;
            while let Some(r) = rows.next()? {
                let (name, direct): (String, i64) = (r.get(0)?, r.get(1)?);
                if name.is_empty() {
                    uncategorized = direct.max(0) as u64;
                } else {
                    nodes.entry(name).or_default().direct = direct.max(0) as u64;
                }
            }
        }
        {
            // 같은 제목의 분류 문서가 둘이면 먼저 만든 것.
            let mut st = db.prepare_cached("SELECT title, home FROM item WHERE ns = 1 AND deleted = 0 ORDER BY title, created")?;
            let mut rows = st.query([])?;
            while let Some(r) = rows.next()? {
                let (title, home): (String, Option<String>) = (r.get(0)?, r.get(1)?);
                let n = nodes.entry(title).or_default();
                if n.has_doc {
                    continue;
                }
                n.has_doc = true;
                n.parent = home.filter(|h| !h.is_empty());
                if let Some(h) = n.parent.clone() {
                    nodes.entry(h).or_default();
                }
            }
        }
        let cycles = cut_cycles(&mut nodes);
        let names: Vec<String> = nodes.keys().cloned().collect();
        let mut roots = Vec::new();
        for name in &names {
            match nodes[name].parent.clone() {
                Some(p) => nodes.get_mut(&p).expect("부모 마디").children.push(name.clone()),
                None => roots.push(name.clone()),
            }
        }
        // 뿌리에서 내려가며 차례를 만들고 거꾸로 더한다(재귀 없이 — 나무가 깊어도 스택을 쓰지 않는다).
        let mut order = Vec::with_capacity(names.len());
        let mut stack = roots.clone();
        while let Some(n) = stack.pop() {
            stack.extend(nodes[&n].children.iter().cloned());
            order.push(n);
        }
        for n in order.iter().rev() {
            let below: u64 = nodes[n].children.iter().map(|c| nodes[c].total).sum();
            let node = nodes.get_mut(n).expect("마디");
            node.total = node.direct + below;
        }
        Ok(Tree { nodes, roots, cycles, uncategorized })
    }

    fn folders(&self, names: &[String]) -> Vec<Folder> {
        names
            .iter()
            .map(|n| {
                let node = &self.nodes[n];
                Folder { name: n.clone(), total: node.total, has_doc: node.has_doc, cycle: node.cycle }
            })
            .collect()
    }

    /// 맨 위 분류와 그 수(대문 모음의 갈래).
    pub(crate) fn top(&self) -> Vec<Folder> {
        self.folders(&self.roots)
    }

    pub(crate) fn uncategorized(&self) -> u64 {
        self.uncategorized
    }

    /// 이 분류의 맨 위 조상(갈래). 나무에 없으면 None.
    pub(crate) fn group_of(&self, category: &str) -> Option<&str> {
        let mut at = self.nodes.get_key_value(category)?;
        while let Some(p) = &at.1.parent {
            at = self.nodes.get_key_value(p.as_str())?;
        }
        Some(at.0.as_str())
    }

    pub(crate) fn children(&self, db: &Connection, category: Option<&str>, limit: usize, after: Option<&str>) -> Result<Children> {
        let (categories, uncategorized) = match category {
            None => (self.top(), Some(self.uncategorized)),
            Some(c) => (self.nodes.get(c).map(|n| self.folders(&n.children)).unwrap_or_default(), None),
        };
        let docs = match category {
            None => Vec::new(),
            Some(c) => docs_in(db, c, limit, after)?,
        };
        Ok(Children { categories, docs, uncategorized, cycles: self.cycles.clone() })
    }
}

/// 부모 줄이 고리를 이루면 고리마다 이름이 가장 작은 분류의 부모를 끊는다. 끊은 이름들(이름 차례).
/// 마디마다 부모가 많아야 하나라 고리는 서로 겹치지 않고, 어느 차례로 끊어도 같은 나무가 된다.
fn cut_cycles(nodes: &mut BTreeMap<String, Node>) -> Vec<String> {
    let names: Vec<String> = nodes.keys().cloned().collect();
    // 없음 = 아직, 1 = 지금 길에 있음, 2 = 끝남
    let mut state: HashMap<String, u8> = HashMap::with_capacity(names.len());
    let mut cut = Vec::new();
    for start in names {
        if state.contains_key(&start) {
            continue;
        }
        let mut path: Vec<String> = Vec::new();
        let mut at = start;
        loop {
            state.insert(at.clone(), 1);
            path.push(at.clone());
            let Some(p) = nodes[&at].parent.clone() else { break };
            match state.get(&p).copied() {
                Some(1) => {
                    let i = path.iter().position(|s| *s == p).expect("길 위");
                    let smallest = path[i..].iter().min().expect("고리").clone();
                    let node = nodes.get_mut(&smallest).expect("마디");
                    node.parent = None;
                    node.cycle = true;
                    cut.push(smallest);
                    break;
                }
                Some(_) => break,
                None => at = p,
            }
        }
        for n in path {
            state.insert(n, 2);
        }
    }
    cut.sort();
    cut
}

/// 그 분류가 집인 살아 있는 일반 문서, 제목 차례로 `after` 뒤부터. 빈 이름은 「분류 안 됨」.
fn docs_in(db: &Connection, category: &str, limit: usize, after: Option<&str>) -> Result<Vec<DocSummary>> {
    let sql = if category.is_empty() {
        "SELECT doc, ns, title, rev, created, updated FROM item WHERE home IS NULL AND ns = 0 AND deleted = 0 AND title > ?2 AND ?1 = '' ORDER BY title LIMIT ?3"
    } else {
        "SELECT doc, ns, title, rev, created, updated FROM item WHERE home = ?1 AND ns = 0 AND deleted = 0 AND title > ?2 ORDER BY title LIMIT ?3"
    };
    let mut st = db.prepare_cached(sql)?;
    let rows = st.query_map(params![category, after.unwrap_or(""), limit as i64], summary_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// `doc, ns, title, rev, created, updated` 열 차례의 행 → 요약.
pub(crate) fn summary_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<DocSummary> {
    Ok(DocSummary { id: r.get(0)?, ns: r.get(1)?, title: r.get(2)?, head: r.get(3)?, created: r.get(4)?, updated: r.get(5)? })
}
