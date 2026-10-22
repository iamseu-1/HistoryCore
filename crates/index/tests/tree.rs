//! 분류 나무와 폴더 수(설계 5-3, Review Focus 2).
use history_corpus::Rng;
use history_index::{Children, Engine, Folder};
use history_library::{DocId, Namespace, RevId};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("색인 나무 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn folder<'a>(c: &'a Children, name: &str) -> &'a Folder {
    c.categories.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("{name} 없음: {:?}", c.categories))
}

#[test]
fn home_is_the_first_category_and_parents_come_from_category_docs() {
    let app = temp("집과 부모");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Category, "조선 선조", "[[분류:조선]]", "").unwrap();
    e.create(Namespace::Category, "조선", "", "").unwrap();
    e.create(Namespace::Doc, "임진왜란", "[[분류:조선 선조]][[분류:전쟁]]", "").unwrap();
    e.create(Namespace::Doc, "경복궁", "[[분류:조선]]", "").unwrap();
    e.create(Namespace::Doc, "분류 없는 글", "글", "").unwrap();
    let root = e.children(None, 200, None).unwrap();
    assert_eq!(root.categories.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["조선"]); // 전쟁은 집이 아니다
    let joseon = folder(&root, "조선");
    assert_eq!((joseon.total, joseon.has_doc, joseon.cycle), (2, true, false));
    assert_eq!(root.uncategorized, Some(1));
    let inner = e.children(Some("조선"), 200, None).unwrap();
    assert_eq!(folder(&inner, "조선 선조").total, 1);
    assert_eq!(inner.docs.iter().map(|d| d.title.as_str()).collect::<Vec<_>>(), ["경복궁"]);
    assert_eq!(inner.uncategorized, None);
    let leaf = e.children(Some("조선 선조"), 200, None).unwrap();
    assert_eq!(leaf.docs.iter().map(|d| d.title.as_str()).collect::<Vec<_>>(), ["임진왜란"]);
    let none = e.children(Some(""), 200, None).unwrap(); // 「분류 안 됨」 폴더
    assert_eq!(none.docs.iter().map(|d| d.title.as_str()).collect::<Vec<_>>(), ["분류 없는 글"]);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_category_without_a_document_stands_at_the_top() {
    let app = temp("문서 없는 분류");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Doc, "가", "[[분류:고아]]", "").unwrap();
    let root = e.children(None, 200, None).unwrap();
    let f = folder(&root, "고아");
    assert_eq!((f.total, f.has_doc), (1, false));
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn cycles_are_cut_and_reported() {
    let app = temp("고리");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Category, "가", "[[분류:나]]", "").unwrap();
    e.create(Namespace::Category, "나", "[[분류:가]]", "").unwrap();
    e.create(Namespace::Category, "스스로", "[[분류:스스로]]", "").unwrap();
    e.create(Namespace::Doc, "글", "[[분류:나]]", "").unwrap();
    let root = e.children(None, 200, None).unwrap();
    assert_eq!(root.cycles, ["가", "스스로"]);
    let ga = folder(&root, "가");
    assert_eq!((ga.total, ga.cycle), (1, true));
    assert!(root.categories.iter().all(|f| f.name != "나"));
    assert_eq!(folder(&e.children(Some("가"), 200, None).unwrap(), "나").total, 1);
    assert!(folder(&root, "스스로").cycle);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn children_pages_with_after() {
    let app = temp("이어 받기");
    let mut e = Engine::open(&app).unwrap();
    for i in 0..450 {
        e.create(Namespace::Doc, &format!("문서 {i:03}"), "[[분류:큰 폴더]]", "").unwrap();
    }
    let mut got = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let page = e.children(Some("큰 폴더"), 200, after.as_deref()).unwrap();
        if page.docs.is_empty() {
            break;
        }
        assert!(page.docs.len() <= 200);
        after = page.docs.last().map(|d| d.title.clone());
        got.extend(page.docs.into_iter().map(|d| d.title));
    }
    let want: Vec<String> = (0..450).map(|i| format!("문서 {i:03}")).collect();
    assert_eq!(got, want);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

/// 시험 쪽의 서재 흉내 — 무작위로 고치고 기대하는 나무를 따로 센다.
struct Model {
    /// 분류 문서: 이름 → (ID, 판, 부모)
    cats: BTreeMap<String, (DocId, RevId, Option<String>)>,
    /// 일반 문서: ID → (판, 집)
    docs: BTreeMap<String, (DocId, RevId, Option<String>)>,
}

impl Model {
    /// 고리를 끊은 부모(엔진과 같은 규칙: 고리마다 이름이 가장 작은 분류의 부모 줄을 끊는다).
    fn parents(&self) -> BTreeMap<String, Option<String>> {
        let mut names: BTreeSet<String> = self.cats.keys().cloned().collect();
        for (_, _, p) in self.cats.values() {
            names.extend(p.clone());
        }
        for (_, _, h) in self.docs.values() {
            names.extend(h.clone());
        }
        let mut parent: BTreeMap<String, Option<String>> = names.iter().map(|n| (n.clone(), self.cats.get(n).and_then(|c| c.2.clone()))).collect();
        loop {
            let mut cut = None;
            'find: for start in parent.keys() {
                let mut seen = vec![start.clone()];
                let mut at = start.clone();
                while let Some(Some(p)) = parent.get(&at) {
                    if let Some(i) = seen.iter().position(|s| s == p) {
                        cut = seen[i..].iter().min().cloned();
                        break 'find;
                    }
                    seen.push(p.clone());
                    at = p.clone();
                }
            }
            match cut {
                Some(c) => {
                    parent.insert(c, None);
                }
                None => return parent,
            }
        }
    }

    fn expected(&self) -> (BTreeMap<String, u64>, u64) {
        let parent = self.parents();
        let mut total: BTreeMap<String, u64> = parent.keys().map(|k| (k.clone(), 0)).collect();
        let mut none = 0;
        for (_, _, h) in self.docs.values() {
            match h {
                None => none += 1,
                Some(h) => {
                    let mut at = Some(h.clone());
                    while let Some(c) = at {
                        *total.get_mut(&c).unwrap() += 1;
                        at = parent[&c].clone();
                    }
                }
            }
        }
        (total, none)
    }
}

/// 엔진 나무 전체를 맨 위부터 훑어 (이름 → total), 맨 위 합.
fn walk(e: &mut Engine) -> (BTreeMap<String, u64>, u64, u64) {
    let root = e.children(None, 10_000, None).unwrap();
    let top: u64 = root.categories.iter().map(|f| f.total).sum();
    let mut out = BTreeMap::new();
    let mut todo: Vec<String> = Vec::new();
    for f in &root.categories {
        out.insert(f.name.clone(), f.total);
        todo.push(f.name.clone());
    }
    while let Some(c) = todo.pop() {
        for f in e.children(Some(&c), 10_000, None).unwrap().categories {
            assert!(out.insert(f.name.clone(), f.total).is_none(), "{} 이 두 번 보임", f.name);
            todo.push(f.name);
        }
    }
    (out, top, root.uncategorized.unwrap())
}

#[test]
fn counts_stay_exact_under_random_edits() {
    let app = temp("무작위");
    let mut e = Engine::open(&app).unwrap();
    let mut rng = Rng::new(20261022);
    let pool = ["가", "나", "다", "라", "마", "바", "사", "아"];
    let mut m = Model { cats: BTreeMap::new(), docs: BTreeMap::new() };
    let body = |home: &Option<String>, extra: &str| match home {
        Some(h) => format!("[[분류:{h}]][[분류:{extra}]] 글"),
        None => "분류 없는 글".to_string(),
    };
    for step in 0..500 {
        let pick = |rng: &mut Rng| -> Option<String> { if rng.chance(0.15) { None } else { Some(rng.pick(&pool).to_string()) } };
        match rng.below(6) {
            0 | 1 => {
                let title = format!("문서 {step}");
                let home = pick(&mut rng);
                let s = e.create(Namespace::Doc, &title, &body(&home, *rng.pick(&pool)), "").unwrap();
                m.docs.insert(title, (s.doc, s.rev, home));
            }
            2 if !m.docs.is_empty() => {
                let k = m.docs.keys().nth(rng.below(m.docs.len() as u64) as usize).unwrap().clone();
                let (doc, rev, _) = m.docs[&k].clone();
                let home = pick(&mut rng);
                let s = e.save(doc, rev, &body(&home, *rng.pick(&pool)), "").unwrap();
                m.docs.insert(k, (doc, s.rev, home));
            }
            3 if !m.docs.is_empty() => {
                let k = m.docs.keys().nth(rng.below(m.docs.len() as u64) as usize).unwrap().clone();
                let (doc, rev, _) = m.docs.remove(&k).unwrap();
                e.delete(doc, rev, "").unwrap();
            }
            4 => {
                let name = rng.pick(&pool).to_string();
                let parent = pick(&mut rng);
                let text = parent.as_ref().map(|p| format!("[[분류:{p}]]")).unwrap_or_default();
                let entry = match m.cats.get(&name) {
                    Some((doc, rev, _)) => {
                        let s = e.save(*doc, *rev, &text, "").unwrap();
                        (*doc, s.rev, parent)
                    }
                    None => {
                        let s = e.create(Namespace::Category, &name, &text, "").unwrap();
                        (s.doc, s.rev, parent)
                    }
                };
                m.cats.insert(name, entry);
            }
            5 if !m.cats.is_empty() => {
                let k = m.cats.keys().nth(rng.below(m.cats.len() as u64) as usize).unwrap().clone();
                let (doc, rev, _) = m.cats.remove(&k).unwrap();
                e.delete(doc, rev, "").unwrap();
            }
            _ => continue,
        }
        let (want, none) = m.expected();
        let (got, top, uncategorized) = walk(&mut e);
        let want: BTreeMap<String, u64> = want.into_iter().filter(|(k, v)| *v > 0 || got.contains_key(k)).collect();
        let got: BTreeMap<String, u64> = got.into_iter().filter(|(k, v)| *v > 0 || want.contains_key(k)).collect();
        assert_eq!(got, want, "{step}번째");
        assert_eq!(top + uncategorized, m.docs.len() as u64, "{step}번째: 맨 위 합 + 분류 안 됨");
        assert_eq!(uncategorized, none, "{step}번째");
    }
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}
