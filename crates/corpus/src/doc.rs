use crate::{pool::TextPool, rng::Rng, Corpus};
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ns {
    Doc,
    Category,
}

/// 만들어진 문서 하나.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenDoc {
    pub id: u32,
    pub ns: Ns,
    pub title: String,
    pub body: String,
    /// 집 분류(첫 분류)의 이름. 분류 안 된 문서와 뿌리 분류는 None.
    pub home: Option<String>,
    /// 큰 갈래(한국사 · 중국사 · …). 분류 안 된 문서는 None.
    pub group: Option<String>,
}

const SALT_TITLE: u64 = 11;
const SALT_BODY: u64 = 12;
const SALT_CAT: u64 = 13;

/// 각주에 인용하는 사료(앞쪽일수록 자주).
pub const SOURCES: [&str; 24] = [
    "조선왕조실록", "고려사", "삼국사기", "명사", "사기", "징비록", "난중일기", "원사", "송사", "삼국유사", "자치통감", "고려사절요",
    "승정원일기", "동국통감", "연려실기술", "일본서기", "한서", "후한서", "신증동국여지승람", "경국대전", "열하일기", "동사강목", "해동역사", "택리지",
];
/// 문서 제목에 붙는 꼬리.
const TITLE_SUFFIX: [&str; 5] = ["왕", "전쟁", "실록", "산성", "대첩"];
/// 없는 문서 링크의 꼬리. 없는 문서는 늘 세 토막(낱말 낱말 꼬리)이라 제목(한두 토막)과 겹치지 않는다.
const MISSING_SUFFIX: [&str; 5] = ["해전", "조약", "사건", "전투", "회의"];

pub(crate) fn title_for(seed: u64, i: u32, pool: &TextPool) -> String {
    let mut r = Rng::for_item(seed, SALT_TITLE, i as u64);
    match r.below(3) {
        0 => r.pick(&pool.words).to_string(),
        1 => format!("{} {}", r.pick(&pool.words), r.pick(&pool.words)),
        _ => format!("{} {}", r.pick(&pool.words), r.pick(&TITLE_SUFFIX)),
    }
}

pub(crate) fn category_doc(c: &Corpus, idx: u32) -> GenDoc {
    let tree = c.tree();
    let cat = tree.get(idx);
    let mut r = Rng::for_item(c.spec().seed, SALT_CAT, idx as u64);
    let mut body = String::new();
    if let Some(p) = cat.parent {
        let _ = writeln!(body, "[[분류:{}]]", tree.get(p).name);
    }
    let _ = writeln!(body, "{}", r.pick(&c.pool().sentences));
    GenDoc {
        id: idx,
        ns: Ns::Category,
        title: cat.name.clone(),
        body,
        home: cat.parent.map(|p| tree.get(p).name.clone()),
        group: cat.parent.map(|_| tree.get(cat.group).name.clone()),
    }
}

/// 일반 문서 i(0..docs). id는 분류 문서 다음부터 이어지는 번호.
pub(crate) fn article(c: &Corpus, i: u32, id: u32) -> GenDoc {
    let pool = c.pool();
    let tree = c.tree();
    let mut r = Rng::for_item(c.spec().seed, SALT_BODY, i as u64);
    let title = c.title(i).to_string();
    let mut b = String::with_capacity(4096);
    let (home, group) = if r.chance(0.01) {
        (None, None)
    } else {
        let leaf = tree.leaves()[r.skewed(tree.leaves().len() as u64) as usize];
        let cat = tree.get(leaf);
        let _ = write!(b, "[[분류:{}]]", cat.name);
        for _ in 0..r.below(3) {
            let th = tree.get(*r.pick(tree.themes()));
            let _ = write!(b, "[[분류:{}]]", th.name);
        }
        b.push('\n');
        (Some(cat.name.clone()), Some(tree.get(cat.group).name.clone()))
    };
    if r.chance(0.6) {
        let y0 = r.below(2_400) as i64 - 500;
        let y1 = y0 + r.below(60) as i64;
        let _ = writeln!(b, "[include(틀:정보 상자, 이름={title}, 기간={y0}..{y1})]");
    }
    let sections = if r.chance(0.05) { 8 + r.below(8) } else { 2 + r.below(5) };
    if sections >= 3 {
        b.push_str("[목차]\n");
    }
    for s in 0..sections {
        let h = if s > 0 && r.chance(0.3) { "===" } else { "==" };
        let _ = writeln!(b, "{h} {} {h}", r.pick(&pool.words));
        for _ in 0..(1 + r.below(3)) {
            paragraph(&mut b, c, &mut r, i);
        }
        if r.chance(0.1) {
            b.push_str("|| '''이름''' || '''해''' ||\n");
            for _ in 0..(2 + r.below(4)) {
                let _ = writeln!(b, "|| {} || {} ||", r.pick(&pool.words), 1_000 + r.below(900));
            }
        }
    }
    GenDoc { id, ns: Ns::Doc, title, body: b, home, group }
}

fn paragraph(b: &mut String, c: &Corpus, r: &mut Rng, me: u32) {
    let pool = c.pool();
    let docs = c.spec().docs;
    for _ in 0..(2 + r.below(5)) {
        b.push_str(*r.pick(&pool.sentences));
        if r.chance(0.35) {
            // 있는 문서로 링크 — 앞 번호(인기 문서)로 쏠린다. 자기 자신은 피한다.
            let mut j = r.skewed(docs as u64) as u32;
            if j == me {
                j = (j + 1) % docs;
            }
            let _ = write!(b, " [[{}]]", c.title(j));
        } else if r.chance(0.03) {
            let _ = write!(b, " [[{} {} {}]]", r.pick(&pool.words), r.pick(&pool.words), r.pick(&MISSING_SUFFIX));
        }
        if r.chance(0.15) {
            let _ = write!(b, "[* 『{}』 {}권.]", SOURCES[r.skewed(SOURCES.len() as u64) as usize], 1 + r.below(200));
        }
        b.push(' ');
    }
    b.push('\n');
}
