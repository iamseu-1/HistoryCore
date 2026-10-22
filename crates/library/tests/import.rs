//! 한꺼번에 들이기 — 측정기 · 새 기기가 쓴다. `create`와 같은 규칙으로 넣는다.
use history_library::{Error, Library, Namespace, NewDoc};
use std::path::PathBuf;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("서재 시험 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn sample(n: usize) -> Vec<NewDoc> {
    (0..n)
        .map(|i| {
            if i < 10 {
                NewDoc { ns: Namespace::Category, title: format!("갈래 {i}"), body: format!("[[분류:뿌리]] 갈래 {i}") }
            } else {
                let body = if i == 500 { "첫 줄\r\n둘째 줄".to_string() } else { format!("{i}번 문서 [[분류:갈래 {}]]", i % 10) };
                NewDoc { ns: Namespace::Doc, title: format!("문서 {i}"), body }
            }
        })
        .collect()
}

#[test]
fn import_puts_many_docs_as_create_would() {
    let app = temp("들이기");
    let mut lib = Library::open(&app).unwrap();
    let saved = lib.import(&mut sample(1_000).into_iter(), 100).unwrap();
    assert_eq!(saved.len(), 1_000);
    let c = lib.counts().unwrap();
    assert_eq!((c.of(Namespace::Category), c.of(Namespace::Doc), c.deleted), (10, 990, 0));
    let found = lib.by_title(Namespace::Doc, "문서 500").unwrap();
    assert_eq!(found.len(), 1);
    let d = lib.get(found[0].id).unwrap().unwrap();
    assert_eq!(d.body, "첫 줄\n둘째 줄");
    assert_eq!(lib.history(d.id, 10, None).unwrap().len(), 1);
    let mut hlcs: Vec<u64> = saved.iter().map(|s| s.hlc.0).collect();
    hlcs.dedup();
    assert_eq!(hlcs.len(), 1_000, "판마다 다른 시각");
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_bad_title_stops_import_and_keeps_earlier_batches() {
    let app = temp("들이기 나쁜 제목");
    let mut lib = Library::open(&app).unwrap();
    let mut docs = sample(300);
    docs[249].title = "분류:조선".into();
    match lib.import(&mut docs.into_iter(), 100) {
        Err(Error::BadTitle(why)) => assert!(why.starts_with("250번째"), "{why}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(lib.counts().unwrap().total(), 200);
    assert!(lib.by_title(Namespace::Doc, "문서 230").unwrap().is_empty());
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}
