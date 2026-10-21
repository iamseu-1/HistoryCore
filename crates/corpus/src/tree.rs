use crate::{pool::TextPool, rng::Rng};
use std::collections::HashSet;

/// 분류 하나. `parent`가 없으면 뿌리. `group`은 큰 갈래(한국사 · 중국사 · …, 깊이 1)의 번호.
#[derive(Clone, Debug)]
pub struct Category {
    pub name: String,
    pub parent: Option<u32>,
    pub group: u32,
    pub depth: u8,
    pub leaf: bool,
}

/// 분류 나무 — 첫 분류가 집(설계 5-3). 뿌리 둘과 큰 갈래 여섯은 이름이 정해져 있고, 그 아래는 문서 수에 맞춰 는다.
/// 주제 분류(전쟁 · 인물 …)는 문서의 둘째 · 셋째 분류로만 쓰인다.
pub struct CategoryTree {
    cats: Vec<Category>,
    leaves: Vec<u32>,
    themes: Vec<u32>,
    groups: Vec<u32>,
}

pub const ROOTS: [(&str, [&str; 3]); 2] = [("동양사", ["한국사", "중국사", "일본사"]), ("서양사", ["고대", "중세", "근대"])];
pub const THEME_ROOT: &str = "주제";
pub const THEMES: [&str; 8] = ["전쟁", "인물", "제도", "외교", "문화", "경제", "종교", "과학"];
const MID_SUFFIX: [&str; 4] = ["시대", "왕조", "지방", "세력"];

impl CategoryTree {
    pub fn build(docs: u32, seed: u64, pool: &TextPool) -> Self {
        let mut cats = Vec::new();
        let mut groups = Vec::new();
        for (root, kids) in ROOTS {
            let r = cats.len() as u32;
            cats.push(Category { name: root.into(), parent: None, group: r, depth: 0, leaf: false });
            for k in kids {
                let g = cats.len() as u32;
                cats.push(Category { name: k.into(), parent: Some(r), group: g, depth: 1, leaf: false });
                groups.push(g);
            }
        }
        let t = cats.len() as u32;
        cats.push(Category { name: THEME_ROOT.into(), parent: None, group: t, depth: 0, leaf: false });
        let mut themes = Vec::new();
        for th in THEMES {
            themes.push(cats.len() as u32);
            cats.push(Category { name: th.into(), parent: Some(t), group: t, depth: 1, leaf: true });
        }
        let mut names: HashSet<String> = cats.iter().map(|c| c.name.clone()).collect();
        let mut leaves = Vec::new();
        // 문서 25편에 분류 하나꼴. 큰 갈래마다 둘째 층(√) · 셋째 층(잎).
        let per_group = ((docs / 25).max(groups.len() as u32 * 4)) / groups.len() as u32;
        let mids = ((per_group as f64).sqrt().ceil() as u32).max(2);
        let lows = (per_group / mids).max(1);
        let mut r = Rng::for_item(seed, 1, 0);
        for &g in &groups {
            for _ in 0..mids {
                let m = cats.len() as u32;
                let name = unique(&mut names, format!("{} {}", r.pick(&pool.words), r.pick(&MID_SUFFIX)));
                cats.push(Category { name, parent: Some(g), group: g, depth: 2, leaf: false });
                for _ in 0..lows {
                    let l = cats.len() as u32;
                    let name = unique(&mut names, format!("{} {}", r.pick(&pool.words), r.pick(&pool.words)));
                    cats.push(Category { name, parent: Some(m), group: g, depth: 3, leaf: true });
                    leaves.push(l);
                }
            }
        }
        CategoryTree { cats, leaves, themes, groups }
    }

    pub fn categories(&self) -> &[Category] {
        &self.cats
    }
    pub fn leaves(&self) -> &[u32] {
        &self.leaves
    }
    pub fn themes(&self) -> &[u32] {
        &self.themes
    }
    pub fn groups(&self) -> &[u32] {
        &self.groups
    }
    pub fn get(&self, i: u32) -> &Category {
        &self.cats[i as usize]
    }
    /// 뿌리까지 올라가는 길(자기 포함, 아래 → 위).
    pub fn path(&self, mut i: u32) -> Vec<u32> {
        let mut p = vec![i];
        while let Some(up) = self.cats[i as usize].parent {
            p.push(up);
            i = up;
        }
        p
    }
}

fn unique(seen: &mut HashSet<String>, base: String) -> String {
    if seen.insert(base.clone()) {
        return base;
    }
    let mut n = 2;
    loop {
        let t = format!("{base} {n}");
        if seen.insert(t.clone()) {
            return t;
        }
        n += 1;
    }
}
