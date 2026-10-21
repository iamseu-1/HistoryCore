use history_library::{Error, Library, FILE, FOLDER, FORMAT};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// 한글 · 공백이 든 시험용 앱 자료 폴더.
fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("서재 시험 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 폴더 바로 아래 파일들의 이름과 바이트(하위 폴더는 뺀다).
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read(&p).unwrap()))
        .collect();
    out.sort();
    out
}

fn our_file(app: &Path) -> PathBuf {
    app.join(FOLDER).join(FILE)
}

#[test]
fn a_fresh_library_lives_in_its_own_subfolder_and_keeps_its_ids() {
    let app = temp("새 서재");
    let lib = Library::open(&app).unwrap();
    let info = lib.info().clone();
    assert_eq!(info.format, FORMAT);
    assert_eq!(info.folder, app.join(FOLDER));
    assert!(our_file(&app).is_file());
    drop(lib);
    // 자료 폴더 바로 아래에는 엔진 폴더 말고 아무것도 생기지 않는다.
    let top: Vec<_> = std::fs::read_dir(&app).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(top, vec![std::ffi::OsString::from(FOLDER)]);
    let again = Library::open(&app).unwrap();
    assert_eq!(again.info(), &info);
    drop(again);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn the_old_apps_files_in_the_same_folder_stay_byte_for_byte() {
    // Review Focus 1 — 옛 앱은 자료 폴더 바로 아래에 library.db · index.db · local.db를 두었다.
    let app = temp("옛 앱 파일");
    let old = Connection::open(app.join("library.db")).unwrap();
    old.execute_batch("CREATE TABLE objects(id INTEGER PRIMARY KEY, title TEXT); INSERT INTO objects(title) VALUES ('임진왜란');")
        .unwrap();
    drop(old);
    std::fs::write(app.join("index.db"), "옛 색인 바이트").unwrap();
    std::fs::write(app.join("local.db"), "옛 기기 상태 바이트").unwrap();
    let before = files(&app);
    let lib = Library::open(&app).unwrap();
    drop(lib);
    assert_eq!(files(&app), before);
    assert!(our_file(&app).is_file());
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn another_sqlite_file_in_our_place_is_refused_and_left_alone() {
    // Review Focus 2 — 옛 앱 DB를 손으로 옮겨 둔 경우.
    let app = temp("남의 DB");
    std::fs::create_dir_all(app.join(FOLDER)).unwrap();
    let other = Connection::open(our_file(&app)).unwrap();
    other.execute_batch("CREATE TABLE objects(id INTEGER PRIMARY KEY);").unwrap();
    drop(other);
    let before = std::fs::read(our_file(&app)).unwrap();
    let e = Library::open(&app).err().expect("열려서는 안 된다");
    assert!(matches!(&e, Error::NotALibrary(p) if p.as_path() == our_file(&app).as_path()), "{e}");
    assert_eq!(e.code(), "not_a_library");
    assert_eq!(std::fs::read(our_file(&app)).unwrap(), before);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_broken_file_in_our_place_is_refused_and_left_alone() {
    let app = temp("깨진 파일");
    std::fs::create_dir_all(app.join(FOLDER)).unwrap();
    std::fs::write(our_file(&app), "이건 SQLite 파일이 아니다 — 아무 글").unwrap();
    let before = std::fs::read(our_file(&app)).unwrap();
    let e = Library::open(&app).err().expect("열려서는 안 된다");
    assert_eq!(e.code(), "not_a_library");
    assert_eq!(std::fs::read(our_file(&app)).unwrap(), before);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn a_newer_format_is_refused_and_left_alone() {
    let app = temp("새 형식");
    drop(Library::open(&app).unwrap());
    let db = Connection::open(our_file(&app)).unwrap();
    db.execute_batch(&format!("PRAGMA user_version = {};", FORMAT + 1)).unwrap();
    drop(db);
    let before = std::fs::read(our_file(&app)).unwrap();
    let e = Library::open(&app).err().expect("열려서는 안 된다");
    assert!(matches!(e, Error::NewerFormat { found, supported } if found == FORMAT + 1 && supported == FORMAT), "{e}");
    assert_eq!(std::fs::read(our_file(&app)).unwrap(), before);
    std::fs::remove_dir_all(&app).unwrap();
}
