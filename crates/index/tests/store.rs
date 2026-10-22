//! index.db — 정체 · 눈금 · 낡음 따라잡기 · 모델 꺼내기(Review Focus 1 · 5, P2-2 미룬 것 「깊은 모델」).
use history_index::Engine;
use history_library::{Error, Library, Namespace, NewDoc};
use history_namumark::{render, RenderOptions};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("색인 시험 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn index_path(app: &Path) -> PathBuf {
    app.join("HistoryCore").join("index.db")
}

/// 낡은 것이 없어질 때까지 step — 많아야 `rounds`번.
fn settle(e: &mut Engine, rounds: usize) {
    for _ in 0..rounds {
        if e.status().unwrap().stale == 0 {
            return;
        }
        e.step(Duration::from_millis(200)).unwrap();
    }
    panic!("따라잡지 못함: {:?}", e.status().unwrap());
}

#[test]
fn a_fresh_engine_indexes_on_save() {
    let app = temp("저장하면 색인");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "임진왜란", "'''임진왜란'''은 1592년 [[조선]]에서", "").unwrap();
    let o = e.open_doc(a.doc, true).unwrap().unwrap();
    assert!(o.model.contains("1592") && !o.stale, "{}", o.model);
    assert_eq!(o.doc.title, "임진왜란");
    assert!(!o.deleted && o.redirected_from.is_none());
    assert_eq!(e.status().unwrap().stale, 0);
    let s = e.save(a.doc, a.rev, "고친 글 1598", "").unwrap();
    let o = e.open_doc(s.doc, true).unwrap().unwrap();
    assert!(o.model.contains("1598") && !o.model.contains("1592") && !o.stale);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn an_index_cut_before_commit_is_caught_up_on_the_next_open() {
    let app = temp("색인 끊김");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "끊김", "첫 글", "").unwrap();
    e.fail_before_index_commit = true;
    e.save(a.doc, a.rev, "둘째 글", "").unwrap(); // 서재 커밋은 되었다
    drop(e);
    let mut e = Engine::open(&app).unwrap();
    assert_eq!(e.status().unwrap().stale, 1);
    let o = e.open_doc(a.doc, true).unwrap().unwrap();
    assert!(o.stale && o.model.contains("둘째"), "{}", o.model);
    settle(&mut e, 5);
    assert!(!e.open_doc(a.doc, true).unwrap().unwrap().stale);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn opening_does_not_rebuild() {
    let app = temp("열 때 짓지 않음");
    {
        let mut lib = Library::open(&app).unwrap();
        let docs = (0..1_000).map(|i| NewDoc { ns: Namespace::Doc, title: format!("문서 {i}"), body: format!("{i}번 [[문서 {}]]", i + 1) });
        lib.import(&mut docs.into_iter(), 500).unwrap();
    }
    let t = Instant::now();
    let mut e = Engine::open(&app).unwrap();
    let limit = if cfg!(debug_assertions) { Duration::from_secs(5) } else { Duration::from_secs(1) };
    assert!(t.elapsed() < limit, "{:?}", t.elapsed());
    let s = e.status().unwrap();
    assert_eq!((s.stale, s.building), (1_000, false));
    settle(&mut e, 200);
    assert_eq!(e.status().unwrap().total, 1_000);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn revisions_arriving_without_the_engine_are_caught_up() {
    // 동기화가 판만 넣고 간 경우를 흉내 낸다.
    let app = temp("엔진 없이 온 판");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "멀리서", "옛 글", "").unwrap();
    drop(e);
    {
        let mut lib = Library::open(&app).unwrap();
        lib.save(a.doc, a.rev, "다른 기기의 글", "").unwrap();
    }
    let mut e = Engine::open(&app).unwrap();
    assert_eq!(e.status().unwrap().stale, 1);
    let o = e.open_doc(a.doc, true).unwrap().unwrap();
    assert!(o.stale && o.model.contains("다른 기기"));
    settle(&mut e, 5);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn the_old_apps_root_index_db_is_untouched() {
    // Review Focus 5 — 옛 앱은 자료 폴더 바로 아래에 index.db를 둔다.
    let app = temp("옛 앱 색인");
    let old = app.join("index.db");
    std::fs::write(&old, "옛 앱의 색인 파일").unwrap();
    let before = std::fs::metadata(&old).unwrap().modified().unwrap();
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "가", "나", "").unwrap();
    e.save(a.doc, a.rev, "다", "").unwrap();
    drop(e);
    assert_eq!(std::fs::read_to_string(&old).unwrap(), "옛 앱의 색인 파일");
    assert_eq!(std::fs::metadata(&old).unwrap().modified().unwrap(), before);
    assert!(index_path(&app).exists());
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn our_broken_index_is_rebuilt_and_a_foreign_one_is_refused() {
    let app = temp("깨진 색인");
    {
        let mut lib = Library::open(&app).unwrap();
        lib.create(Namespace::Doc, "가", "나", "").unwrap();
    }
    std::fs::write(index_path(&app), b"this is not sqlite at all, just bytes").unwrap();
    let mut e = Engine::open(&app).unwrap();
    assert_eq!(e.status().unwrap().stale, 1);
    settle(&mut e, 5);
    drop(e);

    let foreign = temp("남의 색인");
    std::fs::create_dir_all(foreign.join("HistoryCore")).unwrap();
    {
        let db = rusqlite::Connection::open(index_path(&foreign)).unwrap();
        db.execute_batch("PRAGMA application_id = 305419896; CREATE TABLE t(x);").unwrap();
    }
    let bytes = std::fs::read(index_path(&foreign)).unwrap();
    assert!(matches!(Engine::open(&foreign), Err(Error::NotALibrary(_))));
    assert_eq!(std::fs::read(index_path(&foreign)).unwrap(), bytes);
    std::fs::remove_dir_all(&app).unwrap();
    std::fs::remove_dir_all(&foreign).unwrap();
}

#[test]
fn deep_models_pass_through_untouched() {
    // P2-2 미룬 것 — 틀 8단 × 접기 20단이면 모델 JSON 중첩이 serde_json 읽기 한계(128)를 넘는다.
    // 엔진은 모델을 글 그대로 저장하고 꺼내므로 다시 읽지 않는다.
    let app = temp("깊은 모델");
    let mut e = Engine::open(&app).unwrap();
    let mut templates = BTreeMap::new();
    for d in 0..8 {
        let mut body = String::new();
        for k in 0..20 {
            body.push_str(&format!("{{{{{{#!folding {d}-{k}\n"));
        }
        body.push_str(&format!("[include(틀:T{})]\n", d + 1));
        for _ in 0..20 {
            body.push_str("}}}\n");
        }
        e.create(Namespace::Template, &format!("T{d}"), &body, "").unwrap();
        templates.insert(format!("틀:T{d}"), body);
    }
    let src = "[include(틀:T0)]";
    let a = e.create(Namespace::Doc, "깊은 문서", src, "").unwrap();
    let want = serde_json::to_string(&render(src, &RenderOptions { title: Some("깊은 문서".into()), template: false }, &templates)).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&want).is_err(), "중첩이 한계를 넘어야 하는 시험");
    let got = e.open_doc(a.doc, true).unwrap().unwrap().model;
    assert_eq!(got, want);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}
