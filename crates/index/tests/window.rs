//! 서재 창구(spec/windows-api.md 3-1 · 4절) — 바깥 앱이 서재를 읽기 전용으로 묻는다. 만들지 않고 · 쓰지 않고 · 색인을 고치지 않는다.
use history_index::{Engine, Window};
use history_library::{DocId, DocSummary, Error, Namespace, FOLDER};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use unicode_normalization::UnicodeNormalization;

/// 한글 · 공백이 든 시험용 앱 자료 폴더(아직 만들지 않음).
fn place(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("서재 창구 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn temp(name: &str) -> PathBuf {
    let d = place(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 폴더 안 파일마다 (이름, 크기, 고친 때). 파일마다 따로 묻는다 — Windows의 `DirEntry::metadata`는 디렉터리 항목에 적어 둔 값이라
/// 다른 손잡이가 열어 둔 파일(켜진 아카이브의 `-wal`)은 크기가 늦게 고쳐진다.
fn snapshot(dir: &Path) -> Vec<(String, u64, SystemTime)> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            let m = std::fs::metadata(e.path()).unwrap();
            (e.file_name().to_string_lossy().into_owned(), m.len(), m.modified().unwrap())
        })
        .collect();
    out.sort();
    out
}

fn names(dir: &Path) -> Vec<String> {
    snapshot(dir).into_iter().map(|(n, ..)| n).collect()
}

/// 파일 이름 → 바이트(곁 파일 빼고).
fn bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "db"))
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read(&p).unwrap()))
        .collect();
    out.sort();
    out
}

fn code<T>(r: Result<T, Error>) -> &'static str {
    match r {
        Ok(_) => "됨",
        Err(e) => e.code(),
    }
}

fn titles(docs: &[DocSummary]) -> Vec<&str> {
    docs.iter().map(|d| d.title.as_str()).collect()
}

fn settle(e: &mut Engine) {
    for _ in 0..200 {
        e.step(Duration::from_millis(200)).unwrap();
        if e.status().unwrap().stale == 0 {
            return;
        }
    }
    panic!("따라잡지 못함");
}

const LANTERN: &str = "lantern-chronicle/p-lantern";

/// 시험 서재 — 연표 링크 · 띠 · 넘겨주기 · 지운 문서 · 틀 · 같은 제목 둘.
struct Fixture {
    war: DocId,
    redirect: DocId,
    gone: DocId,
    first_twin: DocId,
}

fn fill(e: &mut Engine) -> Fixture {
    e.create(Namespace::Template, "정보", "'''@이름@''' — 틀 글", "").unwrap();
    let war = e.create(Namespace::Doc, "임진왜란", "[include(틀:정보, 이름=임진왜란)]\n왜란 이야기 [[연표:한국/세종]]", "").unwrap();
    let renamed = e.rename(war.doc, war.rev, "임진 전쟁", true, "").unwrap();
    e.create(Namespace::Doc, "나", &format!("[[연표:{LANTERN}|등불]]"), "").unwrap();
    e.create(Namespace::Doc, "가", &format!("[연표({LANTERN}, 범위=1592..1598)]\n[[연표:{LANTERN}]]"), "").unwrap();
    e.create(Namespace::Doc, "다", "[[연표:다른/항목]]", "").unwrap();
    e.create(Namespace::Doc, "Zeta", &format!("[[연표:{LANTERN}]]"), "").unwrap();
    let gone = e.create(Namespace::Doc, "라", &format!("[[연표:{LANTERN}]] 지울 문서"), "").unwrap();
    e.delete(gone.doc, gone.rev, "").unwrap();
    let nfd: String = "한국/세종".nfd().collect();
    // 띠는 한 줄을 다 차지해야 띠다(spec/namumark.md).
    e.create(Namespace::Doc, "마", &format!("[연표({nfd})]\n풀어 쓴 띠"), "").unwrap();
    let first_twin = e.create(Namespace::Doc, "쌍둥이", "먼저", "").unwrap();
    e.create(Namespace::Doc, "쌍둥이", "나중", "").unwrap();
    e.create(Namespace::Doc, "정유재란", "임진왜란 뒤의 전쟁", "").unwrap();
    e.create(Namespace::Doc, "임진강", "강 이름", "").unwrap();
    Fixture { war: war.doc, redirect: renamed.redirect.unwrap(), gone: gone.doc, first_twin: first_twin.doc }
}

#[test]
fn opening_where_there_is_no_library_makes_nothing() {
    let app = place("없는 폴더");
    assert_eq!(code(Window::open(&app)), "no_library");
    assert!(!app.exists(), "폴더를 만들었다");
    let app = temp("빈 폴더");
    std::fs::create_dir_all(app.join(FOLDER)).unwrap();
    assert_eq!(code(Window::open(&app)), "no_library");
    assert!(names(&app.join(FOLDER)).is_empty(), "파일을 만들었다");
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn someone_elses_file_and_a_newer_library_are_refused() {
    let app = temp("남의 파일");
    std::fs::create_dir_all(app.join(FOLDER)).unwrap();
    let path = app.join(FOLDER).join(history_library::FILE);
    Connection::open(&path).unwrap().execute_batch("CREATE TABLE objects(id INTEGER PRIMARY KEY);").unwrap();
    assert_eq!(code(Window::open(&app)), "not_a_library");
    std::fs::remove_file(&path).unwrap();
    drop(Engine::open(&app).unwrap());
    Connection::open(&path).unwrap().execute_batch(&format!("PRAGMA user_version = {};", history_library::FORMAT + 1)).unwrap();
    assert_eq!(code(Window::open(&app)), "newer_format");
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn docs_for_entity_is_the_index_answer_for_the_nfc_id() {
    let app = temp("항목 문서");
    let mut e = Engine::open(&app).unwrap();
    let f = fill(&mut e);
    let mut w = Window::open(&app).unwrap();
    let (entity, docs) = w.docs_for_entity(&format!("  {LANTERN} ")).unwrap();
    assert_eq!(entity, LANTERN);
    // 지운 문서 뺌 · 링크와 띠를 함께 둔 문서도 한 번 · 제목 차례(UTF-8 바이트 — 라틴 대문자가 한글보다 앞).
    assert_eq!(titles(&docs), ["Zeta", "가", "나"]);
    assert!(docs.iter().all(|d| d.id != f.gone));
    assert_eq!(docs, e.timeline_docs(LANTERN).unwrap());
    // 풀어 쓴(NFD) ID로 물어도, 풀어 쓴 띠도(R5) 같은 항목.
    let nfd: String = "한국/세종".nfd().collect();
    let (entity, docs) = w.docs_for_entity(&nfd).unwrap();
    assert_eq!(entity, "한국/세종");
    assert_eq!(titles(&docs), ["마", "임진 전쟁"]);
    // 자리 꼬리는 떼지 않는다(원 ID로 묻는 것은 묻는 쪽 몫).
    assert!(w.docs_for_entity(&format!("{LANTERN}#1")).unwrap().1.is_empty());
    assert!(w.docs_for_entity("없는/항목").unwrap().1.is_empty());
    drop(w);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn doc_answers_the_head_model_without_following_redirects() {
    let app = temp("문서");
    let mut e = Engine::open(&app).unwrap();
    let f = fill(&mut e);
    let mut w = Window::open(&app).unwrap();
    let d = w.doc(f.war).unwrap().expect("있는 문서");
    assert_eq!((d.summary.id, d.summary.title.as_str(), d.summary.ns), (f.war, "임진 전쟁", Namespace::Doc));
    assert_eq!(d.model, e.open_doc(f.war, false).unwrap().unwrap().model);
    assert!(d.model.contains("틀 글"), "틀을 펼친 모델: {}", d.model);
    // 넘겨주기 문서는 따라가지 않는다 — 그 문서 자신과 `redirect`.
    let r = w.doc(f.redirect).unwrap().expect("넘겨주기 문서");
    assert_eq!((r.summary.id, r.summary.title.as_str()), (f.redirect, "임진왜란"));
    assert!(r.model.contains("\"redirect\""), "{}", r.model);
    assert_eq!(r.model, e.open_doc(f.redirect, false).unwrap().unwrap().model);
    // 지운 문서 · 없는 문서는 없는 것.
    assert!(w.doc(f.gone).unwrap().is_none());
    assert!(w.doc(DocId::new()).unwrap().is_none());
    // 제목으로 — 맞춘 제목 · 같은 제목이면 먼저 만든 것 · 쓸 수 없는 제목은 없음.
    assert_eq!(w.doc_by_title(Namespace::Doc, "임진왜란").unwrap().map(|d| d.summary.id), Some(f.redirect));
    assert_eq!(w.doc_by_title(Namespace::Doc, " 임진 전쟁 ").unwrap().map(|d| d.summary.id), Some(f.war));
    assert_eq!(w.doc_by_title(Namespace::Doc, "쌍둥이").unwrap().map(|d| d.summary.id), Some(f.first_twin));
    assert_eq!(w.doc_by_title(Namespace::Template, "정보").unwrap().map(|d| d.summary.ns), Some(Namespace::Template));
    assert!(w.doc_by_title(Namespace::Doc, "라").unwrap().is_none());
    assert!(w.doc_by_title(Namespace::Doc, "없는 문서").unwrap().is_none());
    assert!(w.doc_by_title(Namespace::Doc, "분류:쓸 수 없음").unwrap().is_none());
    assert_eq!(w.version().unwrap(), e.lib.info().library);
    drop(w);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn titles_exist_and_search_titles_first_then_body() {
    let app = temp("제목 찾기");
    let mut e = Engine::open(&app).unwrap();
    fill(&mut e);
    e.create(Namespace::Category, "조선", "", "").unwrap();
    let mut w = Window::open(&app).unwrap();
    let asked: Vec<(Namespace, String)> = [(Namespace::Doc, "임진 전쟁"), (Namespace::Doc, "없음"), (Namespace::Doc, " 쌍둥이 "), (Namespace::Category, "조선"), (Namespace::Doc, "분류:조선"), (Namespace::Doc, "라")]
        .into_iter()
        .map(|(n, t)| (n, t.to_string()))
        .collect();
    let got = w.titles_exist(&asked).unwrap();
    assert_eq!(got, e.titles_exist(&asked).unwrap());
    assert_eq!(got.iter().map(Option::is_some).collect::<Vec<_>>(), [true, false, true, true, false, false]);
    // 제목 앞부분이 먼저(제목 차례), 이어 본문 — 같은 문서는 한 번.
    let hits = w.search("임진", 20).unwrap();
    let t: Vec<&str> = hits.iter().map(|h| h.doc.title.as_str()).collect();
    assert_eq!(t, ["임진 전쟁", "임진강", "임진왜란", "정유재란"]);
    assert!(hits[..3].iter().all(|h| h.snippet.is_empty()));
    assert!(hits[3].snippet.contains("임진왜란"), "{}", hits[3].snippet);
    assert_eq!(w.search("임진", 2).unwrap().len(), 2);
    // 제목 가운데의 낱말은 제목 앞부분이 아니다 — 본문에서만 찾는다.
    let body = w.search("전쟁", 20).unwrap();
    assert_eq!(body.iter().map(|h| h.doc.title.as_str()).collect::<Vec<_>>(), ["정유재란"]);
    assert!(body[0].snippet.contains("전쟁"), "{}", body[0].snippet);
    assert!(w.search("없는말", 20).unwrap().is_empty());
    drop(w);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_stale_index_is_drawn_from_the_head_without_writing_anything() {
    let app = temp("낡은 색인");
    let mut e = Engine::open(&app).unwrap();
    let f = fill(&mut e);
    let head = e.lib.get(f.war).unwrap().unwrap().head;
    e.fail_before_index_commit = true;
    let s = e.save(f.war, head, "[include(틀:정보, 이름=새 판)]\n새 본문 [[연표:새/항목]]", "").unwrap();
    drop(e);
    let folder = app.join(FOLDER);
    let (before, files) = (snapshot(&folder), bytes(&folder));
    let mut w = Window::open(&app).unwrap();
    let d = w.doc(f.war).unwrap().unwrap();
    assert_eq!(d.summary.head, s.rev);
    assert!(d.model.contains("새 본문") && d.model.contains("새 판"), "{}", d.model);
    // 색인을 쓰는 질의는 색인에 든 대로(아카이브가 다시 색인할 때까지 옛 값).
    assert!(w.docs_for_entity("새/항목").unwrap().1.is_empty());
    drop(w);
    assert_eq!(bytes(&folder), files, "읽기 전용 열기가 파일을 바꿈");
    if cfg!(windows) {
        assert_eq!(snapshot(&folder), before);
    }
    // 쓰는 쪽이 그때 그리는 모델과 같다.
    let mut e = Engine::open(&app).unwrap();
    let o = e.open_doc(f.war, false).unwrap().unwrap();
    assert!(o.stale);
    assert_eq!(o.model, d.model);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_doc_whose_template_changed_is_drawn_with_the_new_template() {
    let app = temp("바뀐 틀");
    let mut e = Engine::open(&app).unwrap();
    let t = e.create(Namespace::Template, "머리", "옛 틀 글", "").unwrap();
    let a = e.create(Namespace::Doc, "가", "[include(틀:머리)]", "").unwrap();
    e.save(t.doc, t.rev, "새 틀 글", "").unwrap();
    let stale: i64 = e.debug_index().query_row("SELECT count(*) FROM stale", [], |r| r.get(0)).unwrap();
    assert_eq!(stale, 1, "틀을 쓰는 문서가 낡음 표에 든다");
    let mut w = Window::open(&app).unwrap();
    let d = w.doc(a.doc).unwrap().unwrap();
    assert!(d.model.contains("새 틀 글") && !d.model.contains("옛 틀 글"), "{}", d.model);
    settle(&mut e);
    assert_eq!(e.open_doc(a.doc, false).unwrap().unwrap().model, d.model);
    drop(w);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_lagging_index_redraws_docs_that_use_templates_but_not_the_others() {
    // 틀 저장이 색인되기 전에 끊겼다 — 틀을 쓰는 문서는 아직 낡음 표에 없지만, 색인이 서재보다 뒤처졌으니 다시 그린다.
    let app = temp("뒤처진 색인");
    let mut e = Engine::open(&app).unwrap();
    let t = e.create(Namespace::Template, "머리", "옛 틀 글", "").unwrap();
    let a = e.create(Namespace::Doc, "가", "[include(틀:머리)]", "").unwrap();
    let plain = e.create(Namespace::Doc, "나", "틀 없는 글", "").unwrap();
    let plain_model = e.open_doc(plain.doc, false).unwrap().unwrap().model;
    e.fail_before_index_commit = true;
    e.save(t.doc, t.rev, "새 틀 글", "").unwrap();
    e.fail_before_index_commit = false;
    let stale: i64 = e.debug_index().query_row("SELECT count(*) FROM stale", [], |r| r.get(0)).unwrap();
    assert_eq!(stale, 0, "끊긴 저장은 낡음 표에 넣지 못했다");
    assert!(e.index_seq().unwrap() < e.lib.last_seq().unwrap());
    let mut w = Window::open(&app).unwrap();
    let d = w.doc(a.doc).unwrap().unwrap();
    assert!(d.model.contains("새 틀 글") && !d.model.contains("옛 틀 글"), "{}", d.model);
    assert_eq!(w.doc(plain.doc).unwrap().unwrap().model, plain_model);
    // 쓰는 쪽이 따라잡은 뒤 그리는 모델과 같다.
    settle(&mut e);
    assert_eq!(e.open_doc(a.doc, false).unwrap().unwrap().model, d.model);
    drop(w);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn reading_beside_an_idle_open_archive_leaves_every_file_as_it_was() {
    // 아카이브가 켜져 있되 쓰지 않는 동안 — 질의 전후로 파일 목록 · 크기 · 고친 때가 같고, 아카이브는 그대로 쓸 수 있다.
    let app = temp("가만한 아카이브");
    let mut e = Engine::open(&app).unwrap();
    let f = fill(&mut e);
    let folder = app.join(FOLDER);
    let before = snapshot(&folder);
    assert!(names(&folder).iter().any(|n| n.ends_with("-wal")), "켜진 아카이브는 WAL이 있다: {before:?}");
    let mut w = Window::open(&app).unwrap();
    w.version().unwrap();
    w.doc(f.war).unwrap().unwrap();
    w.doc_by_title(Namespace::Doc, "임진왜란").unwrap().unwrap();
    w.docs_for_entity(LANTERN).unwrap();
    w.titles_exist(&[(Namespace::Doc, "가".into())]).unwrap();
    w.search("임진", 20).unwrap();
    drop(w);
    assert_eq!(snapshot(&folder), before);
    let head = e.lib.get(f.war).unwrap().unwrap().head;
    e.save(f.war, head, "다시 고침", "").unwrap();
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn without_a_usable_index_the_index_queries_say_no_index_and_doc_still_answers() {
    let app = temp("색인 없음");
    let mut e = Engine::open(&app).unwrap();
    let f = fill(&mut e);
    let model = e.open_doc(f.war, false).unwrap().unwrap().model;
    let library = e.lib.info().library;
    drop(e);
    let folder = app.join(FOLDER);
    let index = folder.join(history_index::store::FILE);
    let saved = std::fs::read(&index).unwrap();
    let check = |why: &str| {
        let mut w = Window::open(&app).unwrap();
        assert_eq!(code(w.docs_for_entity(LANTERN)), "no_index", "{why}");
        assert_eq!(code(w.titles_exist(&[(Namespace::Doc, "가".into())])), "no_index", "{why}");
        assert_eq!(code(w.search("임진", 20)), "no_index", "{why}");
        assert_eq!(w.doc(f.war).unwrap().unwrap().model, model, "{why}");
        assert_eq!(w.version().unwrap(), library, "{why}");
    };
    // 1) 색인 파일이 없다 — 만들지 않는다.
    std::fs::remove_file(&index).unwrap();
    check("없음");
    assert!(!index.exists(), "색인을 만들었다");
    // 2) 판이 다르다 · 3) 다른 서재의 색인 · 4) 남의 SQLite — 다시 짓지 않고 바이트도 그대로.
    let edits = [
        format!("PRAGMA user_version = {};", history_index::store::FORMAT - 1),
        format!("UPDATE meta SET value = '{}' WHERE key = 'library';", history_library::LibraryId::new()),
        "PRAGMA application_id = 7;".to_string(),
    ];
    for sql in edits {
        std::fs::write(&index, &saved).unwrap();
        Connection::open(&index).unwrap().execute_batch(&sql).unwrap();
        let changed = std::fs::read(&index).unwrap();
        check(&sql);
        assert!(std::fs::read(&index).unwrap() == changed, "{sql}: 색인 파일을 바꿈");
    }
    std::fs::write(&index, "SQLite가 아닌 글").unwrap();
    check("깨짐");
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn reading_while_the_archive_is_closed_leaves_every_file_as_it_was() {
    let app = temp("꺼진 아카이브");
    let mut e = Engine::open(&app).unwrap();
    let f = fill(&mut e);
    drop(e);
    let folder = app.join(FOLDER);
    let (before, files) = (snapshot(&folder), bytes(&folder));
    assert_eq!(names(&folder), ["index.db", "library.db"]);
    let mut w = Window::open(&app).unwrap();
    w.version().unwrap();
    w.doc(f.war).unwrap().unwrap();
    w.doc_by_title(Namespace::Doc, "임진왜란").unwrap().unwrap();
    w.docs_for_entity(LANTERN).unwrap();
    w.titles_exist(&[(Namespace::Doc, "가".into())]).unwrap();
    w.search("임진", 20).unwrap();
    // 손잡이를 쥔 채여도 질의 사이에는 파일을 쥐지 않는다.
    if cfg!(windows) {
        assert_eq!(snapshot(&folder), before);
    }
    drop(w);
    assert_eq!(bytes(&folder), files);
    if cfg!(windows) {
        assert_eq!(snapshot(&folder), before);
    }
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn reading_while_the_archive_is_open_sees_its_saves_and_never_blocks_it() {
    let app = temp("켜진 아카이브");
    let mut e = Engine::open(&app).unwrap();
    let a = e.create(Namespace::Doc, "가", "첫 글 [[연표:E1]]", "").unwrap();
    let folder = app.join(FOLDER);
    let before = names(&folder);
    let mut w = Window::open(&app).unwrap();
    assert!(w.doc(a.doc).unwrap().unwrap().model.contains("첫 글"));
    assert_eq!(names(&folder), before, "켜진 아카이브 곁에서 읽어도 파일이 생기거나 없어지지 않는다");
    // 쓰는 쪽이 저장하면 다음 질의에 보인다.
    let s = e.save(a.doc, a.rev, "고친 글 [[연표:E2]]", "").unwrap();
    let d = w.doc(a.doc).unwrap().unwrap();
    assert_eq!(d.summary.head, s.rev);
    assert!(d.model.contains("고친 글"));
    assert_eq!(titles(&w.docs_for_entity("E2").unwrap().1), ["가"]);
    assert!(w.docs_for_entity("E1").unwrap().1.is_empty());
    // 쓰는 쪽이 거래를 연 채(아직 거두지 않음)여도 읽히고, 거둔 판만 보인다.
    let raw = Connection::open(folder.join(history_library::FILE)).unwrap();
    raw.execute_batch("BEGIN IMMEDIATE; UPDATE doc SET title = '반쯤 쓴 제목';").unwrap();
    assert_eq!(w.doc(a.doc).unwrap().unwrap().summary.title, "가");
    raw.execute_batch("ROLLBACK;").unwrap();
    drop(raw);
    // 손잡이를 쥔 채로도 아카이브는 색인을 통째로 다시 지어 바꿔 끼울 수 있다(창구가 파일을 쥐고 있지 않다).
    e.rebuild_index(2).unwrap();
    assert_eq!(titles(&w.docs_for_entity("E2").unwrap().1), ["가"]);
    // 아카이브가 다른 서재로 합류하면 창구도 새 서재 ID를 본다.
    let joined = history_library::LibraryId::new();
    e.join(joined).unwrap();
    assert_eq!(w.version().unwrap(), joined);
    assert_eq!(titles(&w.docs_for_entity("E2").unwrap().1), ["가"]);
    drop(w);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}
