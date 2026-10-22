//! 통째 짓기 — 하나씩 저장해 지은 색인과 같고, 끊겨도 옛 색인은 그대로.
use history_index::Engine;
use history_library::Namespace;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("색인 통째 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn settle(e: &mut Engine) {
    for _ in 0..200 {
        e.step(Duration::from_millis(200)).unwrap();
        if e.status().unwrap().stale == 0 && !e.stats_pending().unwrap() {
            return;
        }
    }
    panic!("따라잡지 못함");
}

/// 300편 서재 — 틀(넘겨주기 하나 포함) · 분류 문서 · 일반 문서 · 지운 문서 · 이름 바꾼 문서.
fn fill(e: &mut Engine) {
    e.create(Namespace::Template, "정보", "'''@이름@''' @기간@[* 틀 각주 『틀 사료』]\n[[분류:틀 분류]]", "").unwrap();
    let old = e.create(Namespace::Template, "옛 틀", "옛 틀 글", "").unwrap();
    e.rename(old.doc, old.rev, "새 틀", true, "").unwrap();
    for g in 0..5 {
        e.create(Namespace::Category, &format!("갈래 {g}"), "", "").unwrap();
        e.create(Namespace::Category, &format!("갈래 {g} 아래"), &format!("[[분류:갈래 {g}]]"), "").unwrap();
    }
    for i in 0..290 {
        let body = format!(
            "== 개요 ==\n[include(틀:정보, 이름=문서 {i}, 기간={})]\n임진왜란 {i}번 이야기 [[문서 {}]] [[없는 {}]][* 『사료 {}』]\n[include(틀:옛 틀)]\n[[연표:E{}]]\n[[분류:갈래 {} 아래]]",
            1500 + i,
            (i + 7) % 290,
            i % 13,
            i % 5,
            i % 11,
            i % 5
        );
        let s = e.create(Namespace::Doc, &format!("문서 {i}"), &body, "").unwrap();
        if i % 37 == 0 {
            e.delete(s.doc, s.rev, "").unwrap();
        } else if i % 41 == 0 {
            e.rename(s.doc, s.rev, &format!("바꾼 문서 {i}"), i % 2 == 0, "").unwrap();
        }
    }
}

/// 비교할 표 — 내용을 정렬해 글로.
fn dump(db: &Connection) -> Vec<String> {
    let tables = [
        "SELECT doc, rev, ns, title, home, rns, rkey, created, updated, deleted FROM item",
        "SELECT i.doc, m.z FROM model m JOIN item i ON i.rowid = m.rowid",
        "SELECT i.doc, p.z FROM plain p JOIN item i ON i.rowid = p.rowid",
        "SELECT i.doc, l.kind, l.ns, l.key FROM link l JOIN item i ON i.rowid = l.src",
        "SELECT ns, key, refs, present FROM target",
        "SELECT i.doc, m.category, m.first FROM member m JOIN item i ON i.rowid = m.doc",
        "SELECT i.doc, s.name FROM source s JOIN item i ON i.rowid = s.doc",
        "SELECT i.doc, x.entity FROM entity x JOIN item i ON i.rowid = x.doc",
        "SELECT i.doc, u.template FROM uses u JOIN item i ON i.rowid = u.doc",
        "SELECT i.doc, y.y0, y.y1 FROM years y JOIN item i ON i.rowid = y.doc",
        "SELECT name, direct FROM folder",
        "SELECT hour, n FROM stat_edit",
        "SELECT hour, n FROM stat_new",
        "SELECT name, refs FROM source_count",
        "SELECT decade, docs FROM decade",
        "SELECT a, b, links FROM group_link",
        "SELECT doc FROM stale",
    ];
    let mut out = Vec::new();
    for sql in tables {
        let mut st = db.prepare(sql).unwrap();
        let n = st.column_count();
        let mut rows: Vec<String> = st
            .query_map([], |r| Ok((0..n).map(|i| format!("{:?}", r.get_ref(i).unwrap())).collect::<Vec<_>>().join("|")))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        rows.sort();
        out.push(format!("{sql}\n{}", rows.join("\n")));
    }
    out
}

fn searches(e: &mut Engine) -> Vec<Vec<String>> {
    ["임진왜란", "이야기", "문서", "틀", "옛 틀 글", "鶴", "없는"]
        .iter()
        .map(|q| e.search_body(q, 500).unwrap().into_iter().map(|h| h.doc.title).collect())
        .collect()
}

fn index_db(app: &Path) -> PathBuf {
    app.join("HistoryCore").join("index.db")
}

#[test]
fn rebuild_equals_incremental() {
    let app = temp("같은 색인");
    let mut e = Engine::open(&app).unwrap();
    fill(&mut e);
    settle(&mut e);
    let before = dump(e.debug_index());
    let found = searches(&mut e);
    assert!(found[0].len() > 200, "{}", found[0].len());
    e.rebuild_index(4).unwrap();
    settle(&mut e);
    let after = dump(e.debug_index());
    for (b, a) in before.iter().zip(&after) {
        assert_eq!(a, b);
    }
    assert_eq!(searches(&mut e), found);
    for t in ["fts0", "fts1", "fts2", "fts3"] {
        e.debug_index().execute(&format!("INSERT INTO {t}({t}) VALUES ('integrity-check')"), []).unwrap();
    }
    // 지은 색인에 이어서 저장해도 맞는다.
    let s = e.create(Namespace::Doc, "새로 쓴 글", "임진왜란 다음 이야기", "").unwrap();
    assert_eq!(e.search_body("다음 이야기", 20).unwrap().into_iter().map(|h| h.doc.id).collect::<Vec<_>>(), [s.doc]);
    drop(e);
    let mut e = Engine::open(&app).unwrap();
    assert_eq!(e.status().unwrap().stale, 0);
    assert_eq!(e.search_body("다음 이야기", 20).unwrap().len(), 1);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_cut_rebuild_leaves_the_old_index() {
    let app = temp("끊긴 통째");
    let mut e = Engine::open(&app).unwrap();
    fill(&mut e);
    settle(&mut e);
    let before = dump(e.debug_index());
    e.fail_rebuild_after = Some(100);
    assert!(e.rebuild_index(4).is_err());
    e.fail_rebuild_after = None;
    assert_eq!(dump(e.debug_index()), before);
    assert!(e.search_body("임진왜란", 5).unwrap().len() == 5);
    let leftovers = || std::fs::read_dir(app.join("HistoryCore")).unwrap().filter(|f| f.as_ref().unwrap().file_name().to_string_lossy().contains("building")).count();
    assert!(leftovers() > 0);
    drop(e);
    let e = Engine::open(&app).unwrap();
    assert_eq!(leftovers(), 0);
    assert!(index_db(&app).exists());
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}
