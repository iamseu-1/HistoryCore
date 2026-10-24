//! 바깥 앱의 읽기 전용 열기(spec/windows-api.md 3-1) — 만들지 않고 · 쓰지 않고 · 이 열기가 만든 곁 파일만 치운다.
use history_library::{Error, Library, Namespace, Reader, FILE, FOLDER, FORMAT};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// 한글 · 공백이 든 시험용 앱 자료 폴더(아직 만들지 않음).
fn place(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("서재 읽기 전용 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn temp(name: &str) -> PathBuf {
    let d = place(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn our_file(app: &Path) -> PathBuf {
    app.join(FOLDER).join(FILE)
}

/// 폴더 안 파일마다 (이름, 크기, 고친 때). 하위 폴더는 이름만. 파일마다 따로 묻는다 — Windows의 `DirEntry::metadata`는 디렉터리 항목에
/// 적어 둔 값이라 다른 손잡이가 열어 둔 파일(켜진 서재의 `-wal`)은 크기가 늦게 고쳐진다.
fn snapshot(dir: &Path) -> Vec<(String, u64, Option<SystemTime>)> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            let m = std::fs::metadata(e.path()).unwrap();
            let name = e.file_name().to_string_lossy().into_owned();
            if m.is_dir() {
                (name, 0, None)
            } else {
                (name, m.len(), Some(m.modified().unwrap()))
            }
        })
        .collect();
    out.sort();
    out
}

fn code(r: Result<Reader, Error>) -> &'static str {
    match r {
        Ok(_) => "열림",
        Err(e) => e.code(),
    }
}

#[test]
fn a_missing_folder_is_no_library_and_nothing_is_made() {
    let app = place("없는 폴더");
    assert_eq!(code(Reader::open_library(&app)), "no_library");
    assert!(!app.exists(), "폴더를 만들었다");
}

#[test]
fn a_folder_without_the_file_is_no_library_and_stays_as_it_was() {
    let app = temp("파일 없음");
    assert_eq!(code(Reader::open_library(&app)), "no_library");
    assert!(snapshot(&app).is_empty(), "엔진 폴더를 만들었다");
    std::fs::create_dir_all(app.join(FOLDER)).unwrap();
    assert_eq!(code(Reader::open_library(&app)), "no_library");
    assert!(snapshot(&app.join(FOLDER)).is_empty(), "서재 파일을 만들었다");
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn an_empty_file_or_an_empty_database_is_no_library_and_stays_as_it_was() {
    let app = temp("빈 파일");
    std::fs::create_dir_all(app.join(FOLDER)).unwrap();
    std::fs::write(our_file(&app), b"").unwrap();
    let before = snapshot(&app.join(FOLDER));
    assert_eq!(code(Reader::open_library(&app)), "no_library");
    assert_eq!(snapshot(&app.join(FOLDER)), before);
    // 표가 없는 SQLite 파일(서재가 만들어지다 만 자리)도 서재가 없는 것이다.
    std::fs::remove_file(our_file(&app)).unwrap();
    Connection::open(our_file(&app)).unwrap().execute_batch("CREATE TABLE x(a); DROP TABLE x;").unwrap();
    assert!(std::fs::metadata(our_file(&app)).unwrap().len() > 0);
    let bytes = std::fs::read(our_file(&app)).unwrap();
    assert_eq!(code(Reader::open_library(&app)), "no_library");
    assert_eq!(std::fs::read(our_file(&app)).unwrap(), bytes);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn someone_elses_sqlite_is_refused_and_left_alone() {
    let app = temp("남의 파일");
    std::fs::create_dir_all(app.join(FOLDER)).unwrap();
    Connection::open(our_file(&app)).unwrap().execute_batch("CREATE TABLE objects(id INTEGER PRIMARY KEY);").unwrap();
    let before = snapshot(&app.join(FOLDER));
    let bytes = std::fs::read(our_file(&app)).unwrap();
    let e = Reader::open_library(&app).err().expect("열려서는 안 된다");
    assert!(matches!(&e, Error::NotALibrary(p) if p.as_path() == our_file(&app).as_path()), "{e}");
    assert_eq!(std::fs::read(our_file(&app)).unwrap(), bytes);
    if cfg!(windows) {
        assert_eq!(snapshot(&app.join(FOLDER)), before);
    }
    std::fs::write(our_file(&app), "SQLite가 아닌 글").unwrap();
    assert_eq!(code(Reader::open_library(&app)), "not_a_library");
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_newer_format_is_refused_and_left_alone() {
    let app = temp("새 형식");
    drop(Library::open(&app).unwrap());
    Connection::open(our_file(&app)).unwrap().execute_batch(&format!("PRAGMA user_version = {};", FORMAT + 1)).unwrap();
    let bytes = std::fs::read(our_file(&app)).unwrap();
    let e = Reader::open_library(&app).err().expect("열려서는 안 된다");
    assert!(matches!(e, Error::NewerFormat { found, supported } if found == FORMAT + 1 && supported == FORMAT), "{e}");
    assert_eq!(std::fs::read(our_file(&app)).unwrap(), bytes);
    std::fs::remove_dir_all(&app).unwrap();
}

/// 문서 넷 — 같은 제목 둘 · 지운 문서 · 틀.
fn fill(lib: &mut Library) -> [history_library::Saved; 4] {
    let a = lib.create(Namespace::Doc, "임진왜란", "첫째", "").unwrap();
    let b = lib.create(Namespace::Doc, "임진왜란", "둘째(같은 제목)", "").unwrap();
    let gone = lib.create(Namespace::Doc, "지운 문서", "곧 지움", "").unwrap();
    lib.delete(gone.doc, gone.rev, "").unwrap();
    let t = lib.create(Namespace::Template, "정보", "'''@이름@'''", "").unwrap();
    [a, b, gone, t]
}

#[test]
fn the_reader_answers_like_the_library() {
    let app = temp("읽기");
    let mut lib = Library::open(&app).unwrap();
    let [a, b, gone, t] = fill(&mut lib);
    let r = Reader::open_library(&app).unwrap();
    assert_eq!(r.library().unwrap(), lib.info().library);
    assert_eq!(r.last_seq().unwrap(), lib.last_seq().unwrap());
    let (sa, deleted) = r.summary(a.doc).unwrap().unwrap();
    assert_eq!((sa.title.as_str(), sa.head, deleted), ("임진왜란", a.rev, false));
    assert_eq!(r.summary(gone.doc).unwrap().map(|(_, d)| d), Some(true));
    assert_eq!(r.summary(history_library::DocId::new()).unwrap(), None);
    assert_eq!(r.first_by_title(Namespace::Doc, " 임진왜란 ").unwrap(), Some(a.doc));
    assert_eq!(r.first_by_title(Namespace::Doc, "지운 문서").unwrap(), None);
    assert_eq!(r.first_by_title(Namespace::Doc, "분류:쓸 수 없는 제목").unwrap(), None);
    assert_eq!(r.body_by_title(Namespace::Template, "정보").unwrap().as_deref(), Some("'''@이름@'''"));
    assert_eq!(r.body_by_title(Namespace::Doc, "임진왜란").unwrap().as_deref(), Some("첫째"));
    assert_eq!(r.body_by_title(Namespace::Doc, "지운 문서").unwrap(), None);
    assert_eq!(r.body_by_title(Namespace::Template, "없는 틀").unwrap(), None);
    let _ = (b, t);
    drop(r);
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn reading_a_closed_library_leaves_every_file_as_it_was() {
    let app = temp("꺼진 서재");
    let mut lib = Library::open(&app).unwrap();
    let [a, ..] = fill(&mut lib);
    drop(lib);
    let folder = app.join(FOLDER);
    let before = snapshot(&folder);
    let bytes = std::fs::read(our_file(&app)).unwrap();
    for _ in 0..2 {
        let r = Reader::open_library(&app).unwrap();
        r.read(|r| {
            assert!(r.get(a.doc)?.is_some());
            assert!(r.first_by_title(Namespace::Doc, "임진왜란")?.is_some());
            // 읽는 동안 SQLite가 곁 파일을 만든다 — 치우기가 하는 일이 있다.
            assert!(folder.join("library.db-wal").exists() && folder.join("library.db-shm").exists(), "{:?}", snapshot(&folder));
            Ok(())
        })
        .unwrap();
        drop(r);
        assert_eq!(std::fs::read(our_file(&app)).unwrap(), bytes, "서재 파일이 바뀜");
        if cfg!(windows) {
            // 곁 파일(-wal · -shm)도 남지 않는다 — 이 열기가 만든 것은 놓을 때 치운다.
            assert_eq!(snapshot(&folder), before);
        }
    }
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn reading_alongside_the_writing_library_sees_only_committed_snapshots_and_never_blocks_it() {
    let app = temp("켜진 서재");
    let mut lib = Library::open(&app).unwrap();
    let [a, ..] = fill(&mut lib);
    let folder = app.join(FOLDER);
    let before = snapshot(&folder);
    assert!(before.iter().any(|(n, ..)| n.ends_with("-wal")), "쓰는 쪽이 켜져 있으면 WAL이 있다: {before:?}");
    let r = Reader::open_library(&app).unwrap();
    let saved = r
        .read(|r| {
            assert_eq!(r.get(a.doc)?.unwrap().body, "첫째");
            // 읽기 거래가 열린 동안에도 쓰는 쪽은 막히지 않는다 — 읽는 쪽은 거래를 연 때의 판을 본다.
            let s = lib.save(a.doc, a.rev, "고친 글", "").unwrap();
            assert_eq!(r.get(a.doc)?.unwrap().body, "첫째");
            Ok(s)
        })
        .unwrap();
    // 다음 거래에서는 새 판이 보인다.
    let now = r.read(|r| r.get(a.doc)).unwrap().unwrap();
    assert_eq!((now.body.as_str(), now.head), ("고친 글", saved.rev));
    drop(r);
    // 쓰는 쪽이 켜진 채 읽어도 이 읽기는 아무 파일도 만들거나 지우지 않는다(WAL은 쓰는 쪽의 저장으로 자랐다).
    let after: Vec<String> = snapshot(&folder).into_iter().map(|(n, ..)| n).collect();
    assert_eq!(after, before.iter().map(|(n, ..)| n.clone()).collect::<Vec<_>>());
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn reading_alongside_an_idle_library_changes_nothing() {
    let app = temp("가만한 서재");
    let mut lib = Library::open(&app).unwrap();
    let [a, ..] = fill(&mut lib);
    let folder = app.join(FOLDER);
    let before = snapshot(&folder);
    let r = Reader::open_library(&app).unwrap();
    r.read(|r| r.get(a.doc)).unwrap().unwrap();
    drop(r);
    assert_eq!(snapshot(&folder), before);
    // 쓰는 쪽은 그대로 쓸 수 있다.
    lib.save(a.doc, a.rev, "다시", "").unwrap();
    drop(lib);
    std::fs::remove_dir_all(&app).unwrap();
}
