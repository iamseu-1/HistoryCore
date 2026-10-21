use history_library::{Error, Hlc, Library, Namespace, RevKind, RevId};
use std::path::PathBuf;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("서재 시험 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn create_then_read_back_any_text() {
    let app = temp("만들기");
    let mut lib = Library::open(&app).unwrap();
    let big = format!("{}𪄃 😀 끝", "임진왜란은 1592년에 일어났다. ".repeat(30_000)); // 약 1MB
    let a = lib.create(Namespace::Doc, " 鶴 ", "𪄃 😀 학은 물새다.", "처음").unwrap();
    let b = lib.create(Namespace::Category, "조선", &big, "").unwrap();
    assert!(a.changed && b.changed);
    let d = lib.get(a.doc).unwrap().unwrap();
    assert_eq!((d.title.as_str(), d.body.as_str(), d.head, d.deleted, d.ns), ("鶴", "𪄃 😀 학은 물새다.", a.rev, false, Namespace::Doc));
    assert_eq!(d.created, a.hlc);
    assert_eq!(lib.get(b.doc).unwrap().unwrap().body, big);
    let h = lib.history(a.doc, 10, None).unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!((h[0].parent, h[0].kind, h[0].message.as_str(), h[0].len), (None, RevKind::Content, "처음", "𪄃 😀 학은 물새다.".len() as u64));
    assert_eq!(h[0].device, lib.info().device);
    let c = lib.counts().unwrap();
    assert_eq!((c.of(Namespace::Doc), c.of(Namespace::Category), c.total(), c.deleted), (1, 1, 2, 0));
    assert!(lib.get(history_library::DocId::new()).unwrap().is_none());
    assert_eq!(lib.create(Namespace::Doc, "분류:조선", "x", "").err().unwrap().code(), "bad_title");
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn saving_adds_a_revision_and_keeps_the_old_one() {
    let app = temp("저장");
    let mut lib = Library::open(&app).unwrap();
    let a = lib.create(Namespace::Doc, "임진왜란", "첫 판", "").unwrap();
    let b = lib.save(a.doc, a.rev, "둘째 판", "고침").unwrap();
    assert!(b.changed && b.hlc > a.hlc && b.rev != a.rev);
    let same = lib.save(a.doc, b.rev, "둘째 판", "").unwrap();
    assert!(!same.changed);
    assert_eq!(same.rev, b.rev);
    let h = lib.history(a.doc, 10, None).unwrap();
    assert_eq!(h.iter().map(|r| r.id).collect::<Vec<_>>(), vec![b.rev, a.rev]);
    assert_eq!(h[0].parent, Some(a.rev));
    assert_eq!(lib.revision(a.rev).unwrap().unwrap().body, "첫 판");
    assert_eq!(lib.history(a.doc, 10, Some(b.hlc)).unwrap().len(), 1);
    assert!(lib.revision(RevId::new()).unwrap().is_none());
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn saving_on_a_stale_base_never_overwrites() {
    // Review Focus 3 — 편집을 연 뒤 다른 곳에서 먼저 저장했다.
    let app = temp("낡은 판");
    let mut lib = Library::open(&app).unwrap();
    let a = lib.create(Namespace::Doc, "정유재란", "첫 판", "").unwrap();
    let b = lib.save(a.doc, a.rev, "다른 창의 저장", "").unwrap();
    for e in [
        lib.save(a.doc, a.rev, "늦은 저장", "").err().unwrap(),
        lib.rename(a.doc, a.rev, "정유 전쟁", true, "").err().unwrap(),
        lib.delete(a.doc, a.rev, "").err().unwrap(),
    ] {
        assert!(matches!(e, Error::StaleBase { head } if head == b.rev), "{e}");
    }
    assert_eq!(lib.get(a.doc).unwrap().unwrap().body, "다른 창의 저장");
    assert_eq!(lib.history(a.doc, 10, None).unwrap().len(), 2);
    assert_eq!(lib.save(history_library::DocId::new(), a.rev, "x", "").err().unwrap().code(), "not_found");
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn renaming_leaves_a_redirect_by_default() {
    let app = temp("이름 바꾸기");
    let mut lib = Library::open(&app).unwrap();
    let a = lib.create(Namespace::Doc, "임진왜란", "본문", "").unwrap();
    let r = lib.rename(a.doc, a.rev, "임진 전쟁", true, "이름 바꿈").unwrap();
    let d = lib.get(a.doc).unwrap().unwrap();
    assert_eq!((d.title.as_str(), d.body.as_str(), d.head), ("임진 전쟁", "본문", r.rev));
    let old = lib.by_title(Namespace::Doc, "임진왜란").unwrap();
    assert_eq!(old.len(), 1);
    assert_eq!(Some(old[0].id), r.redirect);
    assert_eq!(lib.get(old[0].id).unwrap().unwrap().body, "#넘겨주기 임진 전쟁\n");
    let c = lib.create(Namespace::Category, "조선 중기", "", "").unwrap();
    let rc = lib.rename(c.doc, c.rev, "조선 중엽", true, "").unwrap();
    assert_eq!(lib.get(rc.redirect.unwrap()).unwrap().unwrap().body, "#넘겨주기 분류:조선 중엽\n");
    let n = lib.rename(a.doc, r.rev, "임진년 전쟁", false, "").unwrap();
    assert!(n.redirect.is_none());
    assert!(lib.by_title(Namespace::Doc, "임진 전쟁").unwrap().is_empty());
    assert_eq!(lib.rename(a.doc, n.rev, "임진년 전쟁", true, "").err().unwrap().code(), "bad_title");
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn deleting_is_a_marker_and_saving_on_it_brings_the_doc_back() {
    let app = temp("지우기");
    let mut lib = Library::open(&app).unwrap();
    let a = lib.create(Namespace::Doc, "가짜 문서", "본문", "").unwrap();
    let d = lib.delete(a.doc, a.rev, "지움").unwrap();
    assert!(d.changed);
    let got = lib.get(a.doc).unwrap().unwrap();
    assert!(got.deleted && got.body.is_empty());
    assert!(lib.by_title(Namespace::Doc, "가짜 문서").unwrap().is_empty());
    assert_eq!(lib.counts().unwrap().deleted, 1);
    assert_eq!(lib.counts().unwrap().total(), 0);
    assert!(!lib.delete(a.doc, d.rev, "").unwrap().changed);
    assert_eq!(lib.rename(a.doc, d.rev, "다른 이름", true, "").err().unwrap().code(), "deleted");
    assert_eq!(lib.history(a.doc, 10, None).unwrap().iter().map(|r| r.kind).collect::<Vec<_>>(), vec![RevKind::Deleted, RevKind::Content]);
    let back = lib.save(a.doc, d.rev, "되살린 본문", "되살림").unwrap();
    let got = lib.get(a.doc).unwrap().unwrap();
    assert!(!got.deleted && got.head == back.rev && got.body == "되살린 본문");
    assert_eq!(lib.counts().unwrap().total(), 1);
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn two_docs_may_share_a_title_and_come_back_in_creation_order() {
    // 설계 6-4: 같은 제목 문서 둘은 둘 다 남는다. 링크는 먼저 만든 쪽으로(색인 몫).
    let app = temp("같은 제목");
    let mut lib = Library::open(&app).unwrap();
    let a = lib.create(Namespace::Doc, "임진왜란", "PC에서", "").unwrap();
    let b = lib.create(Namespace::Doc, "임진왜란", "폰에서", "").unwrap();
    let found = lib.by_title(Namespace::Doc, "  임진왜란").unwrap();
    assert_eq!(found.iter().map(|d| d.id).collect::<Vec<_>>(), vec![a.doc, b.doc]);
    assert!(lib.by_title(Namespace::Category, "임진왜란").unwrap().is_empty());
    assert!(lib.by_title(Namespace::Doc, "[[").unwrap().is_empty());
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn revisions_keep_moving_forward_after_a_restart_with_the_clock_set_back() {
    // Review Focus 5
    let app = temp("시계");
    let mut lib = Library::open_with_clock(&app, Box::new(|| 2_000_000u64)).unwrap();
    let a = lib.create(Namespace::Doc, "가", "1", "").unwrap();
    drop(lib);
    let mut lib = Library::open_with_clock(&app, Box::new(|| 1_000_000u64)).unwrap();
    let b = lib.save(a.doc, a.rev, "2", "").unwrap();
    assert!(b.hlc > a.hlc);
    assert_eq!(b.hlc, Hlc::new(2_000_000, 1));
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}
