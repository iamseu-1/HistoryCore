# P2-1 서재 저장소 · 판 · 엔진 질의 약속 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 서재의 원본 저장소(`library.db` — 문서 · 판 · 이름 바꾸기와 넘겨주기 · 지우기 표시)를 Rust 엔진에 짓고, 다리(`hc_call`)와 Dart API로 열어, 아카이브 앱이 실제 앱 자료 폴더에서 서재를 열어 문서 수를 띄우게 한다. 옛 앱의 같은 이름 파일은 바이트 하나도 건드리지 않는다.

**Architecture:** `crates/library`가 SQLite 파일 하나(`<앱 자료 폴더>/HistoryCore/library.db`)를 연다. 문서는 바뀌지 않는 UUIDv7, 판은 앞 판 · 기기 · HLC · zstd 본문을 갖고 지우지 않고 쌓인다. 다리는 열린 서재를 손잡이(handle) 표로 들고 `library.*` · `doc.*` · `rev.*` op를 JSON으로 받는다. Dart `Library`는 그 op를 감싼다. 엔진 질의 전체 약속(`spec/engine-api.md`)을 이 계획에서 먼저 적어, P3 화면 일이 나란히 갈 수 있게 한다(설계 9-1 "P2와 P3는 엔진 질의 API가 정해진 뒤 나란히").

**Tech Stack:** Rust 1.94.1 · rusqlite 0.37(bundled) · uuid 1(v7) · zstd 0.13 · unicode-normalization 0.1 · serde_json 1 · Flutter 3.47.7 / Dart ^3.13.0 · path_provider 2.1.6

**Spec:** `docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md` — 5-1(정체) · 5-2(판) · 5-7(파일 배치) · 9-1 · 9-3. 앞 계획: `docs/superpowers/plans/2026-10-21-p0-p1-foundation-bench.md`(P0 · P1 끝남).

## P2를 넷으로 나눈다

설계 9-1의 P2(저장 · 판 · Rust 나무마크 해석기 · 그리기 모델 · 색인 여덟 · 증분 색인 · 자전 질의 — 100만 건에서 9-2 통과)는 서로 따로 검사할 수 있는 체계 넷이라 계획을 넷으로 쓴다.

| 계획 | 하는 일 | 끝의 모습 |
| --- | --- | --- |
| **P2-1(이것)** | `library.db` · 판 · 다리 op · Dart API · 앱이 서재를 엶 · `spec/engine-api.md`(P2 전체 질의 약속) | 앱이 실제 자료 폴더에서 서재를 열어 `서재 N편`을 띄움, 옛 파일 그대로 |
| P2-2 | 나무마크 해석기(옛 Dart 정답지를 Rust로) · 그리기 모델 · `spec/namumark.md` | 정답지 · 퍼징 통과, `namumark.parse` op |
| P2-3 | `index.db` — 색인 여덟 · 증분 색인 · 통째 들이기 · 측정 판에 엔진 꽂기 | **100만 건 PC에서 9-2 통과**(설계 P2 끝), 폰 수치 보고 |
| P2-4 | 자전 질의(`kangxi.db`) | `hanja.*` op · 자전 카드 자료 |

## Global Constraints

- 저장소 `D:\HistoryCore`(iamseu-11/HistoryCore) · `D:\HistoryArchive`(iamseu-11/HistoryArchive), 나란히. 둘 다 `main`, 원격 있음. **밀기(push)는 감독이 허락한 뒤에만.**
- 도구: Rust 1.94.1(`rust-toolchain.toml`) · Flutter = `D:\flutter-sdk\flutter\bin\flutter.bat`(아래 `$flutter`) · Python은 `py -3 -I -X utf8` · Android SDK `%LOCALAPPDATA%\Android\Sdk` · NDK 28.2.13676358.
- **앱 자료 폴더 규칙**: 엔진은 `<앱 자료 폴더>/HistoryCore/` 아래에만 쓴다. 옛 앱(같은 정체 — Windows `%APPDATA%\HistoryArchive\WikiArchive`, Android 같은 패키지의 `files/`)은 자료 폴더 바로 아래에 `library.db` · `index.db` · `local.db`를 두었다 — 그 자리에는 아무것도 만들거나 고치지 않는다. 옛 자료를 들이는 일은 이 계획에 없다.
- 서재 파일 정체: `PRAGMA application_id = 0x48434C42`("HCLB"), `PRAGMA user_version = 1`(형식 1). 이 값이 아닌 SQLite 파일 · SQLite가 아닌 파일 · 더 새 형식은 **열지 않고 건드리지 않는다**.
- ID는 UUIDv7(16바이트 BLOB, 다리에서는 소문자 하이픈 글). HLC = `(벽시계 ms << 16) | 카운터(16비트)`, 기기 안에서 늘 커진다. 본문은 zstd 레벨 3. 제목은 NFC · 앞뒤 공백 없앰.
- `library.db`는 원본이라 `journal_mode=WAL`, `synchronous=FULL`.
- 다리 ABI는 1 그대로(op를 더하기만 한다). 답은 늘 `ok`, 실패면 `error`(사람이 읽는 글)와 `code`(짧은 이름).
- `.ps1`은 BOM 있는 UTF-8. 고친 뒤 그 저장소 뿌리에서:
  ```powershell
  Get-ChildItem -Recurse -Filter *.ps1 -Path tools | ForEach-Object { [IO.File]::WriteAllText($_.FullName, [IO.File]::ReadAllText($_.FullName), (New-Object Text.UTF8Encoding $true)) }
  ```
- PowerShell 5.1에서 바깥 명령(cargo · cmd · adb)을 부르는 스크립트는 부르는 동안만 `$ErrorActionPreference = 'Continue'`로 두고 종료 코드로 판정한다(P0의 `Invoke-Cargo` 꼴).
- Windows 빌드 전 `$env:CL = '/utf-8'`. 이 PC의 Android 빌드는 `--android-project-arg=kotlin.incremental=false`. 개발자 모드가 꺼져 있다 — 플러그인이 바뀌면 `tools/windows/plugin-junction.ps1`(Task 6에서 모든 플러그인으로 넓힌다).
- 엔진 DLL을 고친 뒤 플러그인 · 앱 검사 전에 `D:\HistoryCore\tools\build-native.ps1`을 다시 돌린다(검사는 `native/windows-x64/history_core.dll`을 연다).
- 주석 · 문서 · 화면 글은 한국어. 커밋은 `git -c user.name=Claude -c user.email=noreply@anthropic.com commit …`, 메시지 끝 줄 `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **옛 앱 파일이 같은 폴더에 있다** — 옛 앱 위에 덮어 설치한 사용자의 자료 폴더에는 옛 `library.db` · `index.db` · `local.db`가 있다. 새 엔진을 열고 문서를 만든 뒤에도 그 파일들은 바이트 하나 바뀌지 않아야 한다. → Task 2 검사.
2. **서재 자리에 남의 파일이 있다** — 옛 앱 DB를 손으로 옮겨 두었거나, 깨진 파일이거나, 더 새 판 엔진이 만든 서재. 열기를 거절하고 까닭을 말하며 파일은 그대로 둔다. 앱은 멈추지 않고 그 까닭을 보인다. → Task 2 · Task 4 · Task 6 검사.
3. **낡은 판 위에 저장** — 편집을 연 뒤 다른 곳(다른 창 · 이름 바꾸기 · 지우기)에서 먼저 저장했다. 덮어쓰지 않고 `stale_base`와 지금 머리 판을 돌려준다. → Task 3 · Task 4 · Task 5 검사.
4. **저장 도중 끊김** — 판을 넣은 뒤 문서 머리를 옮기기 전에 끊겨도, 판만 있고 머리는 그대로인 반쪽 상태가 남지 않는다(설계 9-3 실패 주입). → Task 3 검사.
5. **시계가 뒤로 감** — 시간대 바꿈 · 수동 조정 · 앱을 다시 켬. 판 차례(HLC)가 거꾸로 가지 않는다. → Task 1 · Task 3 검사.

---

## 파일 지도

```
D:\HistoryCore\
  crates\library\Cargo.toml                 rusqlite · uuid · zstd · unicode-normalization
  crates\library\src\lib.rs                 판 번호 + 모듈 · 다시 내보냄
  crates\library\src\error.rs               Error · code() · Result
  crates\library\src\ids.rs                 DocId · RevId · DeviceId · LibraryId(UUIDv7) + SQLite 변환
  crates\library\src\hlc.rs                 Hlc · Clock · WallClock
  crates\library\src\ns.rs                  Namespace · RESERVED_PREFIXES + SQLite 변환
  crates\library\src\title.rs               normalize_title
  crates\library\src\store.rs               Library 열기 · 정체 확인 · 스키마 · 결과 타입
  crates\library\src\docs.rs                문서 일(만들기 · 저장 · 이름 바꾸기 · 지우기 · 읽기) + 실패 주입 단위 검사
  crates\library\tests\{basics,open,docs}.rs
  crates\ffi\src\lib.rs                     op 나누기에 ops 잇기
  crates\ffi\src\ops.rs                     library.* · doc.* · rev.* op, 손잡이 표
  crates\ffi\tests\library.rs
  spec\library-format.md  spec\engine-api.md  spec\README.md
  packages\history_core\lib\src\engine.dart        HistoryCoreError에 code
  packages\history_core\lib\src\library.dart       Library · Namespace · 결과 타입 · StaleBaseError
  packages\history_core\lib\history_core.dart      export 더함
  packages\history_core\test\support.dart          builtLibrary() · windowsOnly
  packages\history_core\test\engine_test.dart      support.dart를 쓴다
  packages\history_core\test\library_test.dart
D:\HistoryArchive\
  pubspec.yaml                              path_provider
  lib\main.dart                             appDataDir · engineStatus가 서재를 연다
  test\app_test.dart
  tools\windows\smoke-test.ps1              자료 폴더를 임시로 · 상태 글에 서재
  tools\windows\plugin-junction.ps1         모든 Windows 플러그인
```

---

### Task 1: 바탕 타입 — ID · HLC · 이름공간 · 제목 · 오류

**Files:**
- Modify: `D:\HistoryCore\crates\library\Cargo.toml`, `crates\library\src\lib.rs`
- Create: `crates\library\src\error.rs`, `ids.rs`, `hlc.rs`, `ns.rs`, `title.rs`, `crates\library\tests\basics.rs`

**Interfaces:**
- Consumes: 없음(P0의 `ENGINE_VERSION` · `ABI_VERSION`은 그대로 둔다).
- Produces(`history_library`):
  - `enum Error { BadTitle(String), NotFound(String), Deleted(DocId), StaleBase { head: RevId }, NotALibrary(PathBuf), NewerFormat { found: i64, supported: i64 }, Corrupt(String), Sqlite(rusqlite::Error), Io(std::io::Error), Injected }` · `Error::code(&self) -> &'static str`(`bad_title` · `not_found` · `deleted` · `stale_base` · `not_a_library` · `newer_format` · `corrupt` · `sqlite` · `io` · `injected`) · `type Result<T>`
  - `struct DocId(pub Uuid)` · `RevId` · `DeviceId` · `LibraryId` — 모두 `new()`(UUIDv7) · `parse(&str) -> Option<Self>` · `from_slice(&[u8]) -> Option<Self>` · `as_bytes() -> &[u8; 16]` · `Display`(하이픈 글) · rusqlite `ToSql`/`FromSql`(16바이트 BLOB)
  - `struct Hlc(pub u64)` · `Hlc::new(wall_ms: u64, counter: u16)` · `wall_ms()` · `counter()` · `Ord` · rusqlite 변환(INTEGER)
  - `type WallClock = Box<dyn FnMut() -> u64 + Send>` · `struct Clock` · `Clock::system() -> WallClock` · `Clock::new(last: Hlc, wall: WallClock)` · `now(&mut self) -> Hlc` · `last(&self) -> Hlc`
  - `enum Namespace { Doc = 0, Category = 1, Template = 2, File = 3, Front = 4 }` · `ALL` · `code()`(`doc` · `category` · `template` · `file` · `front`) · `from_code(&str)` · `prefix()`(`문서` · `분류` · `틀` · `파일` · `대문`) · `from_i64(i64)` · `link_title(&str) -> String` · rusqlite 변환 · `const RESERVED_PREFIXES: [&str; 8]`
  - `fn normalize_title(raw: &str) -> Result<String>` · `const MAX_TITLE_CHARS: usize = 200`

- [ ] **Step 1: 의존성을 더하고 실패하는 검사를 쓴다**

`crates\library\Cargo.toml`(통째로):
```toml
[package]
name = "history_library"
version.workspace = true
edition.workspace = true
publish.workspace = true

[dependencies]
rusqlite = { version = "0.37", features = ["bundled"] }
unicode-normalization = "0.1"
uuid = { version = "1", features = ["v7"] }
zstd = "0.13"
```
(rusqlite 판은 `crates/bench`와 같아야 한다 — 다르면 `libsqlite3-sys`가 두 벌이라 빌드가 멈춘다.)

`crates\library\tests\basics.rs`:
```rust
use history_library::{normalize_title, Clock, DocId, Hlc, Namespace, RevId};
use std::sync::{Arc, Mutex};

#[test]
fn ids_round_trip_through_text_and_bytes() {
    let d = DocId::new();
    assert_eq!(DocId::parse(&d.to_string()), Some(d));
    assert_eq!(DocId::from_slice(d.as_bytes()), Some(d));
    assert_eq!(d.to_string().len(), 36);
    assert_eq!(d.0.get_version_num(), 7);
    assert_eq!(RevId::parse("UUID 아님"), None);
}

#[test]
fn hlc_packs_wall_time_and_counter() {
    let h = Hlc::new(1_700_000_000_000, 5);
    assert_eq!(h.wall_ms(), 1_700_000_000_000);
    assert_eq!(h.counter(), 5);
    assert!(Hlc::new(1, 0) > Hlc::new(0, u16::MAX));
}

#[test]
fn the_clock_never_goes_backwards() {
    // Review Focus 5 — 벽시계가 뒤로 가도(시간대 · 수동 조정) 판 차례는 앞으로만 간다.
    let walls = Arc::new(Mutex::new(vec![1_000u64, 1_000, 900, 2_000, 1_500]));
    let w = walls.clone();
    let mut clock = Clock::new(Hlc(0), Box::new(move || w.lock().unwrap().remove(0)));
    let seq: Vec<Hlc> = (0..5).map(|_| clock.now()).collect();
    assert_eq!(
        seq,
        vec![Hlc::new(1_000, 0), Hlc::new(1_000, 1), Hlc::new(1_000, 2), Hlc::new(2_000, 0), Hlc::new(2_000, 1)]
    );
    assert_eq!(clock.last(), Hlc::new(2_000, 1));
}

#[test]
fn a_full_counter_moves_on_to_the_next_millisecond() {
    let mut clock = Clock::new(Hlc::new(5, u16::MAX), Box::new(|| 5u64));
    assert_eq!(clock.now(), Hlc::new(6, 0));
}

#[test]
fn namespaces_have_stable_codes_prefixes_and_numbers() {
    for ns in Namespace::ALL {
        assert_eq!(Namespace::from_code(ns.code()), Some(ns));
        assert_eq!(Namespace::from_i64(ns as i64), Some(ns));
    }
    assert_eq!(Namespace::Category.prefix(), "분류");
    assert_eq!(Namespace::Doc.link_title("임진왜란"), "임진왜란");
    assert_eq!(Namespace::Template.link_title("정보 상자"), "틀:정보 상자");
    assert_eq!(Namespace::from_code("연표"), None);
    assert_eq!(Namespace::from_i64(9), None);
}

#[test]
fn titles_are_nfc_and_trimmed() {
    assert_eq!(normalize_title("  임진왜란 ").unwrap(), "임진왜란");
    assert_eq!(normalize_title("\u{1100}\u{1161}").unwrap(), "가"); // 풀어 쓴 한글 → 모아 쓴 한글
    assert_eq!(normalize_title("\u{F900}").unwrap(), "\u{8C48}"); // 호환 한자 → 통합 한자
    assert_eq!(normalize_title("鶴 𪄃 😀").unwrap(), "鶴 𪄃 😀");
    assert_eq!(normalize_title("1:1 회담").unwrap(), "1:1 회담");
    assert_eq!(normalize_title("대문").unwrap(), "대문");
}

#[test]
fn titles_that_would_break_links_are_refused() {
    let long = "가".repeat(201);
    for bad in ["", "   ", "[[임진왜란]]", "가|나", "가#나", "가\n나", ":분류", "분류:조선", "틀 : 정보 상자", "字:鶴", "연표:abc", long.as_str()] {
        let e = normalize_title(bad).err().unwrap_or_else(|| panic!("받아들임: {bad:?}"));
        assert_eq!(e.code(), "bad_title", "{bad:?}");
    }
}
```

`crates\library\src\lib.rs`(통째로 — 모듈은 아직 비어 있어 검사가 실패한다):
```rust
//! 서재 엔진(설계 4-1). 원본 저장소 `library.db` · 판 · 다리가 부르는 일.

/// 엔진 판 — 작업공간 판과 같다.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// 다리 · 창구 약속의 판. 요청 · 답의 모양이 바뀌면 올린다(op를 더하는 것은 바꿈이 아니다).
pub const ABI_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_three_numbers() {
        assert_eq!(super::ENGINE_VERSION.split('.').count(), 3);
        assert_eq!(super::ABI_VERSION, 1);
    }
}
```

- [ ] **Step 2: 검사가 실패하는지 본다**

Run: `cargo test -p history_library --test basics`
Expected: 컴파일 실패 — `unresolved imports history_library::normalize_title, Clock, DocId, Hlc, Namespace, RevId`.

- [ ] **Step 3: 구현한다**

`crates\library\src\error.rs`:
```rust
use crate::ids::{DocId, RevId};
use std::fmt;
use std::path::PathBuf;

/// 서재 엔진의 오류. `code()`는 다리(JSON)로 나가는 짧은 이름이다(spec/engine-api.md).
#[derive(Debug)]
pub enum Error {
    /// 제목으로 쓸 수 없음.
    BadTitle(String),
    /// 그런 문서 · 판이 없음.
    NotFound(String),
    /// 지운 문서에는 할 수 없는 일.
    Deleted(DocId),
    /// 고치기 시작한 판(base)이 지금 머리가 아님 — 그 사이 다른 저장이 있었다. 덮어쓰지 않았다.
    StaleBase { head: RevId },
    /// 서재 자리에 다른 파일이 있음(옛 앱 DB · 깨진 파일). 건드리지 않았다.
    NotALibrary(PathBuf),
    /// 이 엔진보다 새 형식의 서재. 건드리지 않았다.
    NewerFormat { found: i64, supported: i64 },
    /// 저장소 안의 값이 약속과 다름.
    Corrupt(String),
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    /// 시험용 실패 주입(`docs.rs` 단위 검사). 보통 실행에서는 생기지 않는다.
    Injected,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Error::BadTitle(_) => "bad_title",
            Error::NotFound(_) => "not_found",
            Error::Deleted(_) => "deleted",
            Error::StaleBase { .. } => "stale_base",
            Error::NotALibrary(_) => "not_a_library",
            Error::NewerFormat { .. } => "newer_format",
            Error::Corrupt(_) => "corrupt",
            Error::Sqlite(_) => "sqlite",
            Error::Io(_) => "io",
            Error::Injected => "injected",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadTitle(why) => write!(f, "제목으로 쓸 수 없음: {why}"),
            Error::NotFound(what) => write!(f, "없음: {what}"),
            Error::Deleted(doc) => write!(f, "지운 문서: {doc}"),
            Error::StaleBase { head } => write!(f, "그 사이 다른 판이 저장됨(지금 머리 {head})"),
            Error::NotALibrary(path) => write!(f, "서재 파일이 아님 — 건드리지 않음: {}", path.display()),
            Error::NewerFormat { found, supported } => {
                write!(f, "더 새 형식의 서재(형식 {found}, 이 엔진은 {supported}까지) — 건드리지 않음")
            }
            Error::Corrupt(what) => write!(f, "저장소 값이 이상함: {what}"),
            Error::Sqlite(e) => write!(f, "SQLite: {e}"),
            Error::Io(e) => write!(f, "파일: {e}"),
            Error::Injected => write!(f, "시험용 실패 주입"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Sqlite(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
```

`crates\library\src\ids.rs`:
```rust
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub Uuid);

        impl $name {
            /// 새 UUIDv7 — 만든 시각이 앞쪽에 들어 있다.
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
            pub fn parse(text: &str) -> Option<Self> {
                Uuid::parse_str(text).ok().map(Self)
            }
            pub fn from_slice(bytes: &[u8]) -> Option<Self> {
                Uuid::from_slice(bytes).ok().map(Self)
            }
            pub fn as_bytes(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(&self.0.as_bytes()[..]))
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                Uuid::from_slice(value.as_blob()?).map(Self).map_err(|e| FromSqlError::Other(Box::new(e)))
            }
        }
    };
}

id_type!(
    /// 문서 — 바뀌지 않는 정체(설계 5-1). 제목이 바뀌어도 그대로다.
    DocId
);
id_type!(
    /// 판(설계 5-2).
    RevId
);
id_type!(
    /// 이 서재를 쓰는 기기. 서재 파일마다 하나.
    DeviceId
);
id_type!(
    /// 서재 하나. 기기가 달라도 같은 서재면 같다(P4 동기화).
    LibraryId
);
```

`crates\library\src\hlc.rs`:
```rust
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use std::time::{SystemTime, UNIX_EPOCH};

/// 혼합 논리 시각(HLC) — `(벽시계 ms << 16) | 카운터`. 판의 차례를 정한다. 기기 안에서 늘 커진다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hlc(pub u64);

impl Hlc {
    pub fn new(wall_ms: u64, counter: u16) -> Hlc {
        Hlc((wall_ms << 16) | counter as u64)
    }
    pub fn wall_ms(self) -> u64 {
        self.0 >> 16
    }
    pub fn counter(self) -> u16 {
        self.0 as u16
    }
}

impl ToSql for Hlc {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for Hlc {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(Hlc(value.as_i64()? as u64))
    }
}

/// 벽시계(1970년부터 ms). 시험은 바꿔 끼운다.
pub type WallClock = Box<dyn FnMut() -> u64 + Send>;

/// 이 기기의 HLC 시계. 벽시계가 뒤로 가면 마지막 시각의 카운터를 올린다.
pub struct Clock {
    last: Hlc,
    wall: WallClock,
}

impl Clock {
    pub fn system() -> WallClock {
        Box::new(|| SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0))
    }

    /// `last`는 저장소에 남은 가장 늦은 판의 시각 — 앱을 다시 켜도 거꾸로 가지 않게.
    pub fn new(last: Hlc, wall: WallClock) -> Clock {
        Clock { last, wall }
    }

    pub fn now(&mut self) -> Hlc {
        let wall = (self.wall)();
        let next = if wall > self.last.wall_ms() {
            Hlc::new(wall, 0)
        } else if self.last.counter() < u16::MAX {
            Hlc::new(self.last.wall_ms(), self.last.counter() + 1)
        } else {
            Hlc::new(self.last.wall_ms() + 1, 0)
        };
        self.last = next;
        next
    }

    pub fn last(&self) -> Hlc {
        self.last
    }
}
```

`crates\library\src\ns.rs`:
```rust
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

/// 서재의 이름공간(설계 5-1). 숫자는 저장소 값, `code()`는 다리 값이다 — 둘 다 바꾸지 않는다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Namespace {
    Doc = 0,
    Category = 1,
    Template = 2,
    File = 3,
    Front = 4,
}

/// 링크에서 이름공간으로 읽히는 접두어 — 서재 밖 자료(연표 · 字 · 책)도 든다. 제목이 이것과 `:`로 시작하면 안 된다.
pub const RESERVED_PREFIXES: [&str; 8] = ["문서", "분류", "틀", "파일", "대문", "연표", "字", "책"];

impl Namespace {
    pub const ALL: [Namespace; 5] = [Namespace::Doc, Namespace::Category, Namespace::Template, Namespace::File, Namespace::Front];

    pub fn code(self) -> &'static str {
        match self {
            Namespace::Doc => "doc",
            Namespace::Category => "category",
            Namespace::Template => "template",
            Namespace::File => "file",
            Namespace::Front => "front",
        }
    }

    pub fn from_code(code: &str) -> Option<Namespace> {
        Self::ALL.into_iter().find(|ns| ns.code() == code)
    }

    pub fn prefix(self) -> &'static str {
        match self {
            Namespace::Doc => "문서",
            Namespace::Category => "분류",
            Namespace::Template => "틀",
            Namespace::File => "파일",
            Namespace::Front => "대문",
        }
    }

    pub fn from_i64(value: i64) -> Option<Namespace> {
        Self::ALL.into_iter().find(|ns| *ns as i64 == value)
    }

    /// 본문 링크에 적는 꼴 — 일반 문서는 제목만, 나머지는 `분류:제목`.
    pub fn link_title(self, title: &str) -> String {
        if self == Namespace::Doc {
            title.to_string()
        } else {
            format!("{}:{title}", self.prefix())
        }
    }
}

impl ToSql for Namespace {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i64))
    }
}

impl FromSql for Namespace {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let v = value.as_i64()?;
        Namespace::from_i64(v).ok_or(FromSqlError::OutOfRange(v))
    }
}
```

`crates\library\src\title.rs`:
```rust
use crate::error::{Error, Result};
use crate::ns::RESERVED_PREFIXES;
use unicode_normalization::UnicodeNormalization;

pub const MAX_TITLE_CHARS: usize = 200;

/// 제목을 저장할 꼴로 맞춘다 — NFC(호환 한자는 통합 한자로) · 앞뒤 공백 없앰.
/// 링크 문법을 깨는 글자(`[` `]` `|` `#` · 제어 문자), `:`로 시작, 이름공간 접두어로 시작하는 제목은 받지 않는다.
pub fn normalize_title(raw: &str) -> Result<String> {
    let nfc: String = raw.nfc().collect();
    let title = nfc.trim();
    if title.is_empty() {
        return Err(Error::BadTitle("비어 있음".into()));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(Error::BadTitle(format!("{MAX_TITLE_CHARS}자를 넘음")));
    }
    if let Some(c) = title.chars().find(|c| c.is_control() || matches!(c, '[' | ']' | '|' | '#')) {
        return Err(Error::BadTitle(format!("쓸 수 없는 글자 {c:?}")));
    }
    if title.starts_with(':') {
        return Err(Error::BadTitle("`:`로 시작함".into()));
    }
    if let Some((head, _)) = title.split_once(':') {
        if RESERVED_PREFIXES.contains(&head.trim()) {
            return Err(Error::BadTitle(format!("이름공간 접두어 «{}:»로 시작함", head.trim())));
        }
    }
    Ok(title.to_string())
}
```

`crates\library\src\lib.rs`의 `ABI_VERSION` 줄 아래에 더한다:
```rust
pub mod error;
pub mod hlc;
pub mod ids;
pub mod ns;
pub mod title;

pub use error::{Error, Result};
pub use hlc::{Clock, Hlc, WallClock};
pub use ids::{DeviceId, DocId, LibraryId, RevId};
pub use ns::{Namespace, RESERVED_PREFIXES};
pub use title::{normalize_title, MAX_TITLE_CHARS};
```

- [ ] **Step 4: 검사가 통과하는지 본다**

Run: `cargo test -p history_library`
Expected: basics 7 passed · lib 1 passed.

- [ ] **Step 5: 커밋한다**

```powershell
Set-Location D:\HistoryCore
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "서재 엔진 바탕 타입 — UUIDv7 ID · HLC 시계 · 이름공간 · 제목 맞춤 · 오류 코드" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `library.db` 열기 — 자기 자리 · 정체 확인 · 형식 판

**Files:**
- Create: `crates\library\src\store.rs`, `crates\library\tests\open.rs`, `spec\library-format.md`
- Modify: `crates\library\src\lib.rs`, `spec\README.md`

**Interfaces:**
- Consumes: Task 1 전부.
- Produces(`history_library`):
  - `const FOLDER: &str = "HistoryCore"` · `const FILE: &str = "library.db"` · `const FORMAT: i64 = 1`
  - `struct LibraryInfo { pub library: LibraryId, pub device: DeviceId, pub format: i64, pub folder: PathBuf }`
  - `struct Library`(필드 `pub(crate) db: Connection`, `pub(crate) info: LibraryInfo`, `pub(crate) clock: Clock`, `pub(crate) fail_after_rev: bool`) · `Library::open(app_dir: &Path) -> Result<Library>` · `Library::open_with_clock(app_dir: &Path, wall: WallClock) -> Result<Library>` · `info(&self) -> &LibraryInfo`
  - 결과 타입(Task 3이 채운다): `enum RevKind { Content, Deleted }` · `struct Saved { pub doc: DocId, pub rev: RevId, pub hlc: Hlc, pub changed: bool }` · `struct Renamed { pub rev: RevId, pub hlc: Hlc, pub redirect: Option<DocId> }` · `struct Doc { pub id, pub ns, pub title: String, pub head: RevId, pub created: Hlc, pub updated: Hlc, pub deleted: bool, pub body: String }` · `struct DocSummary { pub id, pub ns, pub title, pub head, pub created, pub updated }` · `struct RevInfo { pub id: RevId, pub doc: DocId, pub parent: Option<RevId>, pub parent2: Option<RevId>, pub device: DeviceId, pub hlc: Hlc, pub kind: RevKind, pub ns: Namespace, pub title: String, pub message: String, pub len: u64 }` · `struct Revision { pub info: RevInfo, pub body: String }` · `struct Counts { pub live: [u64; 5], pub deleted: u64 }` · `Counts::total()` · `Counts::of(ns)`

- [ ] **Step 1: 실패하는 검사를 쓴다**

`crates\library\tests\open.rs`:
```rust
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
    assert!(matches!(&e, Error::NotALibrary(p) if p == &our_file(&app)), "{e}");
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
```
(옛 앱 파일 검사는 여기서는 열기만 본다. Task 3 Step 1이 문서를 하나 만든 뒤에도 그대로인지까지 넓힌다.)

- [ ] **Step 2: 검사가 실패하는지 본다**

Run: `cargo test -p history_library --test open`
Expected: 컴파일 실패 — `unresolved imports history_library::Library, FILE, FOLDER, FORMAT`.

- [ ] **Step 3: 구현한다**

`crates\library\src\store.rs`:
```rust
//! 서재 원본 저장소 `library.db`(설계 5-7). 문서 · 판 · 서재 정체를 갖는다. 판은 지우지 않고 쌓는다(설계 5-2).
use crate::error::{Error, Result};
use crate::hlc::{Clock, Hlc, WallClock};
use crate::ids::{DeviceId, DocId, LibraryId, RevId};
use crate::ns::Namespace;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// 앱 자료 폴더 아래 엔진의 자리. 옛 앱은 자료 폴더 바로 아래에 `library.db` · `index.db` · `local.db`를 두었으므로
/// 새 엔진은 그 자리에 아무것도 쓰지 않고 이 하위 폴더에만 쓴다.
pub const FOLDER: &str = "HistoryCore";
pub const FILE: &str = "library.db";
/// 서재 형식 판(`PRAGMA user_version`). 모양이 바뀌면 올리고 옮기는 길을 함께 둔다.
pub const FORMAT: i64 = 1;
/// `PRAGMA application_id` — "HCLB". 이 값이 아닌 SQLite 파일은 서재로 열지 않는다.
const APPLICATION_ID: i64 = 0x4843_4C42;

const SCHEMA: &str = "
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE doc(
  id      BLOB PRIMARY KEY,
  ns      INTEGER NOT NULL,
  title   TEXT NOT NULL,
  head    BLOB NOT NULL,
  created INTEGER NOT NULL,
  updated INTEGER NOT NULL,
  deleted INTEGER NOT NULL
) WITHOUT ROWID;
CREATE INDEX doc_title ON doc(ns, title, created);
CREATE TABLE rev(
  seq     INTEGER PRIMARY KEY,
  id      BLOB NOT NULL UNIQUE,
  doc     BLOB NOT NULL,
  parent  BLOB,
  parent2 BLOB,
  device  BLOB NOT NULL,
  hlc     INTEGER NOT NULL,
  kind    INTEGER NOT NULL,
  ns      INTEGER NOT NULL,
  title   TEXT NOT NULL,
  message TEXT NOT NULL,
  len     INTEGER NOT NULL,
  body    BLOB NOT NULL
);
CREATE INDEX rev_doc ON rev(doc, hlc);
";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryInfo {
    pub library: LibraryId,
    pub device: DeviceId,
    pub format: i64,
    /// 엔진의 자리(`<앱 자료 폴더>/HistoryCore`).
    pub folder: PathBuf,
}

/// 판의 종류. 지우기는 본문이 빈 표시 판이다(설계 5-2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevKind {
    Content,
    Deleted,
}

impl RevKind {
    pub fn code(self) -> &'static str {
        match self {
            RevKind::Content => "content",
            RevKind::Deleted => "deleted",
        }
    }
}

impl ToSql for RevKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(match self {
            RevKind::Content => 0i64,
            RevKind::Deleted => 1i64,
        }))
    }
}

impl FromSql for RevKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_i64()? {
            0 => Ok(RevKind::Content),
            1 => Ok(RevKind::Deleted),
            v => Err(FromSqlError::OutOfRange(v)),
        }
    }
}

/// 저장 한 번의 결과. `changed`가 거짓이면 본문이 같아 판을 더하지 않았다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved {
    pub doc: DocId,
    pub rev: RevId,
    pub hlc: Hlc,
    pub changed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Renamed {
    pub rev: RevId,
    pub hlc: Hlc,
    /// 옛 제목에 남긴 넘겨주기 문서.
    pub redirect: Option<DocId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    pub id: DocId,
    pub ns: Namespace,
    pub title: String,
    pub head: RevId,
    pub created: Hlc,
    pub updated: Hlc,
    pub deleted: bool,
    /// 머리 판의 본문(지운 문서는 빈 글).
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocSummary {
    pub id: DocId,
    pub ns: Namespace,
    pub title: String,
    pub head: RevId,
    pub created: Hlc,
    pub updated: Hlc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevInfo {
    pub id: RevId,
    pub doc: DocId,
    pub parent: Option<RevId>,
    /// 합친 판의 둘째 앞 판(P4).
    pub parent2: Option<RevId>,
    pub device: DeviceId,
    pub hlc: Hlc,
    pub kind: RevKind,
    pub ns: Namespace,
    pub title: String,
    pub message: String,
    /// 본문의 UTF-8 바이트 수.
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub info: RevInfo,
    pub body: String,
}

/// 이름공간별 살아 있는 문서 수와 지운 문서 수.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub live: [u64; 5],
    pub deleted: u64,
}

impl Counts {
    pub fn total(&self) -> u64 {
        self.live.iter().sum()
    }
    pub fn of(&self, ns: Namespace) -> u64 {
        self.live[ns as usize]
    }
}

/// 열린 서재 하나. 한 스레드에서 쓴다(다리는 Mutex 안에 둔다).
pub struct Library {
    pub(crate) db: Connection,
    pub(crate) info: LibraryInfo,
    pub(crate) clock: Clock,
    /// 시험용 실패 주입 — 참이면 판을 넣은 뒤 문서 머리를 옮기기 전에 `Error::Injected`로 끊는다.
    pub(crate) fail_after_rev: bool,
}

impl Library {
    /// 앱 자료 폴더 `app_dir`의 `HistoryCore/library.db`를 연다(없으면 만든다). `app_dir` 바로 아래에는 쓰지 않는다.
    pub fn open(app_dir: &Path) -> Result<Library> {
        Self::open_with_clock(app_dir, Clock::system())
    }

    /// 벽시계를 바꿔 끼워 연다(시험 — 시계가 뒤로 가는 경우).
    pub fn open_with_clock(app_dir: &Path, wall: WallClock) -> Result<Library> {
        let folder = app_dir.join(FOLDER);
        std::fs::create_dir_all(&folder)?;
        let path = folder.join(FILE);
        let db = Connection::open(&path)?;
        let tables = check_identity(&db, &path)?;
        db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;")?;
        if tables == 0 {
            create_schema(&db)?;
        }
        let info = LibraryInfo {
            library: meta_id(&db, "library", LibraryId::parse)?,
            device: meta_id(&db, "device", DeviceId::parse)?,
            format: FORMAT,
            folder,
        };
        let last: i64 = db.query_row("SELECT COALESCE(MAX(hlc), 0) FROM rev", [], |r| r.get(0))?;
        Ok(Library { db, info, clock: Clock::new(Hlc(last as u64), wall), fail_after_rev: false })
    }

    pub fn info(&self) -> &LibraryInfo {
        &self.info
    }
}

/// 이 파일이 우리 서재인지 본다 — 아무것도 쓰지 않는다. 표 수를 돌려준다(0이면 빈 새 파일).
fn check_identity(db: &Connection, path: &Path) -> Result<i64> {
    let not_ours = || Error::NotALibrary(path.to_path_buf());
    let app_id: i64 = db.pragma_query_value(None, "application_id", |r| r.get(0)).map_err(|_| not_ours())?;
    let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0)).map_err(|_| not_ours())?;
    let tables: i64 = db.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0)).map_err(|_| not_ours())?;
    if tables == 0 && (app_id == 0 || app_id == APPLICATION_ID) {
        return Ok(0);
    }
    if app_id != APPLICATION_ID {
        return Err(not_ours());
    }
    if version > FORMAT {
        return Err(Error::NewerFormat { found: version, supported: FORMAT });
    }
    Ok(tables)
}

fn create_schema(db: &Connection) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch(&format!("PRAGMA application_id = {APPLICATION_ID}; PRAGMA user_version = {FORMAT};"))?;
    tx.execute_batch(SCHEMA)?;
    for (key, value) in [("library", LibraryId::new().to_string()), ("device", DeviceId::new().to_string()), ("format", FORMAT.to_string())] {
        tx.execute("INSERT INTO meta(key, value) VALUES (?1, ?2)", params![key, value])?;
    }
    tx.commit()?;
    Ok(())
}

fn meta_id<T>(db: &Connection, key: &str, parse: fn(&str) -> Option<T>) -> Result<T> {
    let value: String = db
        .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| r.get(0))
        .optional()?
        .ok_or_else(|| Error::Corrupt(format!("meta.{key} 없음")))?;
    parse(&value).ok_or_else(|| Error::Corrupt(format!("meta.{key} = {value}")))
}
```

`crates\library\src\lib.rs`의 `pub mod ns;` 아래에 `pub mod store;`를, 다시 내보내기 끝에 더한다:
```rust
pub use store::{Counts, Doc, DocSummary, Library, LibraryInfo, RevInfo, RevKind, Renamed, Revision, Saved, FILE, FOLDER, FORMAT};
```

`spec\library-format.md`:
```markdown
# 서재 형식 1

설계 5-1 · 5-2 · 5-7. 이 문서가 `crates/library/src/store.rs`의 약속이다.

## 자리

- 엔진은 앱이 주는 **앱 자료 폴더** 아래 `HistoryCore/`에만 쓴다: `HistoryCore/library.db`(원본). 나중 계획에서 같은 폴더에 `index.db`(파생) · `blobs/`(첨부) · `local.db`(기기 상태)가 든다.
- 옛 앱(같은 앱 정체)은 앱 자료 폴더 **바로 아래**에 `library.db` · `index.db` · `local.db`를 두었다. 그 자리는 읽지도 고치지도 않는다.
- Windows 앱 자료 폴더: `%APPDATA%\HistoryArchive\WikiArchive`. Android: 패키지의 `files/`.

## 정체 확인

열기 전에 아무것도 쓰지 않고 본다.

| 파일 상태 | 엔진 |
| --- | --- |
| 없음 · 표가 없는 빈 파일 | 새 서재를 만든다 |
| `application_id = 0x48434C42`("HCLB") · `user_version ≤ 1` | 연다 |
| `application_id`가 다른 SQLite · SQLite가 아닌 파일 | `not_a_library` — 건드리지 않는다 |
| `user_version > 1` | `newer_format` — 건드리지 않는다 |

열면 `journal_mode = WAL`, `synchronous = FULL`.

## 표

| 표 | 열 | 뜻 |
| --- | --- | --- |
| `meta` | `key` · `value` | `library`(서재 UUID) · `device`(이 기기 UUID) · `format`(`1`) |
| `doc` | `id` · `ns` · `title` · `head` · `created` · `updated` · `deleted` | 문서마다 한 줄 — 지금 제목과 머리 판. 같은 이름공간 · 같은 제목이 둘일 수 있다(설계 6-4) |
| `rev` | `seq` · `id` · `doc` · `parent` · `parent2` · `device` · `hlc` · `kind` · `ns` · `title` · `message` · `len` · `body` | 판마다 한 줄. 지우지 않는다. `body`는 맨 끝 열(긴 값이 다른 열 읽기를 막지 않게) |

- ID(`doc.id` · `doc.head` · `rev.id` · `rev.doc` · `rev.parent*` · `rev.device`)는 UUIDv7 16바이트 BLOB.
- `ns`: 0 문서 · 1 분류 · 2 틀 · 3 파일 · 4 대문.
- `hlc`: `(벽시계 ms << 16) | 카운터`. 한 기기 안에서 늘 커진다 — 열 때 `MAX(rev.hlc)`에서 잇는다. `doc.created` · `doc.updated`는 첫 판 · 머리 판의 `hlc`.
- `kind`: 0 내용 · 1 지움(본문 빈 표시 판). 지운 문서는 `doc.deleted = 1`이고 판은 남는다.
- `body`: UTF-8 나무마크를 zstd(레벨 3)로 누른 것. `len`은 누르기 전 바이트 수.

## 제목

NFC로 맞추고(호환 한자 → 통합 한자) 앞뒤 공백을 없앤다. 받지 않는 것: 빈 제목 · 200자 넘음 · `[` `]` `|` `#` · 제어 문자 · `:`로 시작 · `문서` `분류` `틀` `파일` `대문` `연표` `字` `책` 뒤에 `:`로 시작.

## 이름 바꾸기 · 넘겨주기

이름을 바꾸면 같은 본문 · 새 제목의 판이 더해진다. 넘겨주기를 남기면(기본) 옛 제목에 새 문서가 생기고 본문은 `#넘겨주기 <새 제목>`(일반 문서가 아니면 `분류:새 제목`처럼 접두어를 붙인다) 한 줄이다. 넘겨주기를 따라가는 일은 해석기(P2-2) · 색인(P2-3)이 한다.
```

`spec\README.md`의 "P2부터 채운다" 문장 바로 아래에 더한다:
```markdown
- [서재 형식](library-format.md) — `library.db`(P2-1)
- [엔진 질의](engine-api.md) — 다리 op 전체(P2-1에서 정함, 몇은 뒤 계획에서 채움)
```

- [ ] **Step 4: 검사가 통과하는지 본다**

Run: `cargo test -p history_library`
Expected: open 5 · basics 7 · lib 1 passed.

- [ ] **Step 5: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "library.db 열기 — HistoryCore 하위 폴더에만 · 남의 파일 · 새 형식은 열지 않고 그대로 · 서재 형식 문서" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: 문서 일 — 만들기 · 저장 · 이름 바꾸기 · 지우기 · 읽기

**Files:**
- Create: `crates\library\src\docs.rs`, `crates\library\tests\docs.rs`
- Modify: `crates\library\src\lib.rs`

**Interfaces:**
- Consumes: Task 1 · 2(`Library`의 `pub(crate)` 필드, 결과 타입).
- Produces(`impl Library`):
  - `create(&mut self, ns: Namespace, title: &str, body: &str, message: &str) -> Result<Saved>`
  - `save(&mut self, doc: DocId, base: RevId, body: &str, message: &str) -> Result<Saved>` — `base`가 머리가 아니면 `StaleBase { head }`. 본문이 같으면 `changed: false`(판을 더하지 않는다). 지운 문서도 머리 판 위에 저장하면 되살아난다.
  - `rename(&mut self, doc: DocId, base: RevId, title: &str, keep_redirect: bool, message: &str) -> Result<Renamed>` — 지운 문서는 `Deleted`, 같은 제목은 `BadTitle`.
  - `delete(&mut self, doc: DocId, base: RevId, message: &str) -> Result<Saved>` — 이미 지웠으면 `changed: false`.
  - `get(&self, doc: DocId) -> Result<Option<Doc>>` · `by_title(&self, ns: Namespace, title: &str) -> Result<Vec<DocSummary>>`(지운 것 빼고, 만든 차례) · `history(&self, doc: DocId, limit: usize, before: Option<Hlc>) -> Result<Vec<RevInfo>>`(새 것부터) · `revision(&self, rev: RevId) -> Result<Option<Revision>>` · `counts(&self) -> Result<Counts>`

- [ ] **Step 1: 실패하는 검사를 쓴다**

`crates\library\tests\docs.rs`:
```rust
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
```

`crates\library\src\docs.rs`(실패 주입 단위 검사만 먼저):
```rust
//! 문서 일 — 만들기 · 저장 · 이름 바꾸기 · 지우기 · 읽기(설계 5-1 · 5-2). Step 3에서 채운다.

#[cfg(test)]
mod tests {
    use crate::{Error, Library, Namespace};

    #[test]
    fn a_write_cut_after_the_revision_leaves_no_half_state() {
        // Review Focus 4 — 판을 넣은 뒤 문서 머리를 옮기기 전에 끊긴다.
        let app = std::env::temp_dir().join(format!("서재 시험 끊긴 저장 {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&app);
        let mut lib = Library::open(&app).unwrap();
        let s = lib.create(Namespace::Doc, "임진왜란", "첫 판", "").unwrap();
        lib.fail_after_rev = true;
        assert!(matches!(lib.save(s.doc, s.rev, "둘째 판", ""), Err(Error::Injected)));
        assert!(matches!(lib.delete(s.doc, s.rev, ""), Err(Error::Injected)));
        assert!(matches!(lib.rename(s.doc, s.rev, "임진 전쟁", true, ""), Err(Error::Injected)));
        assert!(matches!(lib.create(Namespace::Doc, "정유재란", "x", ""), Err(Error::Injected)));
        lib.fail_after_rev = false;
        let d = lib.get(s.doc).unwrap().unwrap();
        assert_eq!((d.head, d.body.as_str(), d.title.as_str()), (s.rev, "첫 판", "임진왜란"));
        let revs: i64 = lib.db.query_row("SELECT count(*) FROM rev", [], |r| r.get(0)).unwrap();
        let docs: i64 = lib.db.query_row("SELECT count(*) FROM doc", [], |r| r.get(0)).unwrap();
        assert_eq!((revs, docs), (1, 1));
        drop(lib);
        std::fs::remove_dir_all(&app).unwrap();
    }
}
```
`crates\library\src\lib.rs`의 `pub mod error;` 위에 `mod docs;`를 더한다.

`crates\library\tests\open.rs`의 옛 앱 파일 검사(Review Focus 1)를 문서를 쓴 뒤까지 넓힌다 — 다음 두 줄을
```rust
    let lib = Library::open(&app).unwrap();
    drop(lib);
```
이렇게 바꾼다:
```rust
    let mut lib = Library::open(&app).unwrap();
    lib.create(history_library::Namespace::Doc, "임진왜란", "새 서재의 본문", "").unwrap();
    drop(lib);
```

- [ ] **Step 2: 검사가 실패하는지 본다**

Run: `cargo test -p history_library`
Expected: 컴파일 실패 — `no method named create found for struct Library`(tests/docs.rs · tests/open.rs · docs.rs 단위 검사).

- [ ] **Step 3: 구현한다**

`crates\library\src\docs.rs`(검사 모듈 위에 더한다 — 맨 위 `//!` 줄은 바꾼다):
```rust
//! 문서 일 — 만들기 · 저장 · 이름 바꾸기 · 지우기 · 읽기(설계 5-1 · 5-2). 쓰는 일은 모두 SQLite 트랜잭션 하나다.
use crate::error::{Error, Result};
use crate::hlc::Hlc;
use crate::ids::{DeviceId, DocId, RevId};
use crate::ns::Namespace;
use crate::store::{Counts, Doc, DocSummary, Library, Renamed, RevInfo, RevKind, Revision, Saved};
use crate::title::normalize_title;
use rusqlite::{params, Connection, OptionalExtension};

const ZSTD_LEVEL: i32 = 3;
const REV_COLUMNS: &str = "id, doc, parent, parent2, device, hlc, kind, ns, title, message, len";

struct Head {
    ns: Namespace,
    title: String,
    head: RevId,
    created: Hlc,
    updated: Hlc,
    deleted: bool,
}

struct NewRev<'a> {
    id: RevId,
    doc: DocId,
    parent: Option<RevId>,
    kind: RevKind,
    ns: Namespace,
    title: &'a str,
    message: &'a str,
    body: &'a str,
}

fn load_head(db: &Connection, doc: DocId) -> Result<Head> {
    db.query_row("SELECT ns, title, head, created, updated, deleted FROM doc WHERE id = ?1", params![doc], |r| {
        Ok(Head { ns: r.get(0)?, title: r.get(1)?, head: r.get(2)?, created: r.get(3)?, updated: r.get(4)?, deleted: r.get(5)? })
    })
    .optional()?
    .ok_or_else(|| Error::NotFound(format!("문서 {doc}")))
}

fn encode(body: &str) -> Result<Vec<u8>> {
    Ok(zstd::bulk::compress(body.as_bytes(), ZSTD_LEVEL)?)
}

fn decode(blob: &[u8]) -> Result<String> {
    let bytes = zstd::stream::decode_all(blob).map_err(|e| Error::Corrupt(format!("본문 풀기: {e}")))?;
    String::from_utf8(bytes).map_err(|_| Error::Corrupt("본문이 UTF-8이 아님".into()))
}

fn read_body(db: &Connection, rev: RevId) -> Result<String> {
    let blob: Vec<u8> = db
        .query_row("SELECT body FROM rev WHERE id = ?1", params![rev], |r| r.get(0))
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("판 {rev}")))?;
    decode(&blob)
}

fn insert_rev(db: &Connection, device: DeviceId, hlc: Hlc, r: &NewRev) -> Result<()> {
    db.execute(
        "INSERT INTO rev(id, doc, parent, parent2, device, hlc, kind, ns, title, message, len, body)
         VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![r.id, r.doc, r.parent, device, hlc, r.kind, r.ns, r.title, r.message, r.body.len() as i64, encode(r.body)?],
    )?;
    Ok(())
}

fn insert_doc(db: &Connection, doc: DocId, ns: Namespace, title: &str, head: RevId, hlc: Hlc) -> Result<()> {
    db.execute(
        "INSERT INTO doc(id, ns, title, head, created, updated, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)",
        params![doc, ns, title, head, hlc],
    )?;
    Ok(())
}

fn rev_info(r: &rusqlite::Row<'_>) -> rusqlite::Result<RevInfo> {
    Ok(RevInfo {
        id: r.get(0)?,
        doc: r.get(1)?,
        parent: r.get(2)?,
        parent2: r.get(3)?,
        device: r.get(4)?,
        hlc: r.get(5)?,
        kind: r.get(6)?,
        ns: r.get(7)?,
        title: r.get(8)?,
        message: r.get(9)?,
        len: r.get::<_, i64>(10)? as u64,
    })
}

impl Library {
    pub fn create(&mut self, ns: Namespace, title: &str, body: &str, message: &str) -> Result<Saved> {
        let title = normalize_title(title)?;
        let (doc, rev, hlc, device) = (DocId::new(), RevId::new(), self.clock.now(), self.info.device);
        let tx = self.db.transaction()?;
        insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: None, kind: RevKind::Content, ns, title: &title, message, body })?;
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        insert_doc(&tx, doc, ns, &title, rev, hlc)?;
        tx.commit()?;
        Ok(Saved { doc, rev, hlc, changed: true })
    }

    pub fn save(&mut self, doc: DocId, base: RevId, body: &str, message: &str) -> Result<Saved> {
        let device = self.info.device;
        let tx = self.db.transaction()?;
        let h = load_head(&tx, doc)?;
        if h.head != base {
            return Err(Error::StaleBase { head: h.head });
        }
        if !h.deleted && read_body(&tx, h.head)? == body {
            return Ok(Saved { doc, rev: h.head, hlc: h.updated, changed: false });
        }
        let (rev, hlc) = (RevId::new(), self.clock.now());
        insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: Some(h.head), kind: RevKind::Content, ns: h.ns, title: &h.title, message, body })?;
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        tx.execute("UPDATE doc SET head = ?2, updated = ?3, deleted = 0 WHERE id = ?1", params![doc, rev, hlc])?;
        tx.commit()?;
        Ok(Saved { doc, rev, hlc, changed: true })
    }

    pub fn rename(&mut self, doc: DocId, base: RevId, title: &str, keep_redirect: bool, message: &str) -> Result<Renamed> {
        let new_title = normalize_title(title)?;
        let device = self.info.device;
        let tx = self.db.transaction()?;
        let h = load_head(&tx, doc)?;
        if h.head != base {
            return Err(Error::StaleBase { head: h.head });
        }
        if h.deleted {
            return Err(Error::Deleted(doc));
        }
        if h.title == new_title {
            return Err(Error::BadTitle("지금 제목과 같음".into()));
        }
        let body = read_body(&tx, h.head)?;
        let (rev, hlc) = (RevId::new(), self.clock.now());
        insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: Some(h.head), kind: RevKind::Content, ns: h.ns, title: &new_title, message, body: &body })?;
        tx.execute("UPDATE doc SET title = ?2, head = ?3, updated = ?4 WHERE id = ?1", params![doc, new_title, rev, hlc])?;
        let redirect = if keep_redirect {
            let (rdoc, rrev, rhlc) = (DocId::new(), RevId::new(), self.clock.now());
            let rbody = format!("#넘겨주기 {}\n", h.ns.link_title(&new_title));
            let r = NewRev { id: rrev, doc: rdoc, parent: None, kind: RevKind::Content, ns: h.ns, title: &h.title, message: "이름을 바꾸며 남긴 넘겨주기", body: &rbody };
            insert_rev(&tx, device, rhlc, &r)?;
            insert_doc(&tx, rdoc, h.ns, &h.title, rrev, rhlc)?;
            Some(rdoc)
        } else {
            None
        };
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        tx.commit()?;
        Ok(Renamed { rev, hlc, redirect })
    }

    pub fn delete(&mut self, doc: DocId, base: RevId, message: &str) -> Result<Saved> {
        let device = self.info.device;
        let tx = self.db.transaction()?;
        let h = load_head(&tx, doc)?;
        if h.head != base {
            return Err(Error::StaleBase { head: h.head });
        }
        if h.deleted {
            return Ok(Saved { doc, rev: h.head, hlc: h.updated, changed: false });
        }
        let (rev, hlc) = (RevId::new(), self.clock.now());
        insert_rev(&tx, device, hlc, &NewRev { id: rev, doc, parent: Some(h.head), kind: RevKind::Deleted, ns: h.ns, title: &h.title, message, body: "" })?;
        if self.fail_after_rev {
            return Err(Error::Injected);
        }
        tx.execute("UPDATE doc SET head = ?2, updated = ?3, deleted = 1 WHERE id = ?1", params![doc, rev, hlc])?;
        tx.commit()?;
        Ok(Saved { doc, rev, hlc, changed: true })
    }

    pub fn get(&self, doc: DocId) -> Result<Option<Doc>> {
        let h = match load_head(&self.db, doc) {
            Ok(h) => h,
            Err(Error::NotFound(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        let body = read_body(&self.db, h.head)?;
        Ok(Some(Doc { id: doc, ns: h.ns, title: h.title, head: h.head, created: h.created, updated: h.updated, deleted: h.deleted, body }))
    }

    /// 이름공간 · 제목이 같은 살아 있는 문서들(만든 차례). 쓸 수 없는 제목이면 빈 목록.
    pub fn by_title(&self, ns: Namespace, title: &str) -> Result<Vec<DocSummary>> {
        let Ok(title) = normalize_title(title) else {
            return Ok(Vec::new());
        };
        let mut st = self.db.prepare_cached(
            "SELECT id, ns, title, head, created, updated FROM doc WHERE ns = ?1 AND title = ?2 AND deleted = 0 ORDER BY created, id",
        )?;
        let rows = st.query_map(params![ns, title], |r| {
            Ok(DocSummary { id: r.get(0)?, ns: r.get(1)?, title: r.get(2)?, head: r.get(3)?, created: r.get(4)?, updated: r.get(5)? })
        })?;
        let out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(out)
    }

    /// 판 기록, 새 것부터. `before`가 있으면 그 시각보다 앞선 판만(이어 받기).
    pub fn history(&self, doc: DocId, limit: usize, before: Option<Hlc>) -> Result<Vec<RevInfo>> {
        let mut st = self.db.prepare_cached(&format!("SELECT {REV_COLUMNS} FROM rev WHERE doc = ?1 AND hlc < ?2 ORDER BY hlc DESC LIMIT ?3"))?;
        let rows = st.query_map(params![doc, before.unwrap_or(Hlc(i64::MAX as u64)), limit as i64], rev_info)?;
        let out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(out)
    }

    pub fn revision(&self, rev: RevId) -> Result<Option<Revision>> {
        let row = self
            .db
            .query_row(&format!("SELECT {REV_COLUMNS}, body FROM rev WHERE id = ?1"), params![rev], |r| Ok((rev_info(r)?, r.get::<_, Vec<u8>>(11)?)))
            .optional()?;
        match row {
            None => Ok(None),
            Some((info, blob)) => Ok(Some(Revision { info, body: decode(&blob)? })),
        }
    }

    pub fn counts(&self) -> Result<Counts> {
        let mut counts = Counts::default();
        let mut st = self.db.prepare_cached("SELECT ns, deleted, count(*) FROM doc GROUP BY ns, deleted")?;
        let mut rows = st.query([])?;
        while let Some(r) = rows.next()? {
            let (ns, deleted, n): (Namespace, bool, i64) = (r.get(0)?, r.get(1)?, r.get(2)?);
            if deleted {
                counts.deleted += n as u64;
            } else {
                counts.live[ns as usize] += n as u64;
            }
        }
        Ok(counts)
    }
}
```

- [ ] **Step 4: 검사가 통과하는지 본다**

Run: `cargo test -p history_library`
Expected: basics 7 · open 5(옛 앱 파일 검사 포함) · docs 7 · 단위 2(판 번호 · 끊긴 저장) passed. 경고 없음.

- [ ] **Step 5: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "서재 문서 일 — 만들기 · 저장(낡은 판은 거절) · 이름 바꾸기와 넘겨주기 · 지우기 표시 · 판 기록, 한 트랜잭션" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: 다리 op · 엔진 질의 약속(`spec/engine-api.md`)

**Files:**
- Create: `crates\ffi\src\ops.rs`, `crates\ffi\tests\library.rs`, `spec\engine-api.md`
- Modify: `crates\ffi\src\lib.rs`

**Interfaces:**
- Consumes: Task 3의 `Library` 일 전부 · `Error::code()` · ID `parse`/`Display` · `Namespace::from_code`/`code` · `RevKind::code`.
- Produces(다리 JSON — `spec/engine-api.md`가 정본):
  - `library.open {dir}` → `{handle, library, device, format, folder}` (같은 폴더를 다시 열면 같은 `handle`)
  - `library.close {handle}` → `{}` · `library.counts {handle}` → `{doc, category, template, file, front, deleted, total}`
  - `doc.create {handle, ns, title, body, message?}` · `doc.save {handle, doc, base, body, message?}` · `doc.delete {handle, doc, base, message?}` → `{doc, rev, hlc, changed}`
  - `doc.rename {handle, doc, base, title, keep_redirect?(기본 참), message?}` → `{rev, hlc, redirect}`
  - `doc.get {handle, doc}` → `{doc: {id, ns, title, head, created, updated, deleted, body}}` · `doc.by_title {handle, ns, title}` → `{docs: [{id, ns, title, head, created, updated}]}`
  - `doc.history {handle, doc, limit?(기본 50, 많아야 1000), before?}` → `{revisions: [{id, doc, parent, parent2, device, hlc, kind, ns, title, message, len}]}` · `rev.get {handle, rev}` → `{revision: {…, body}}`
  - 실패: `{ok: false, error, code}` — 엔진 코드(Task 1) + `bad_request` · `bad_handle`. `stale_base`는 `head`를 더한다.

- [ ] **Step 1: 실패하는 검사를 쓴다**

`crates\ffi\tests\library.rs`:
```rust
use history_core::{hc_call, hc_free};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::path::PathBuf;

fn call(req: Value) -> Value {
    unsafe {
        let c = CString::new(req.to_string()).unwrap();
        let out = hc_call(c.as_ptr());
        let v = serde_json::from_str(CStr::from_ptr(out).to_str().unwrap()).unwrap();
        hc_free(out);
        v
    }
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("다리 서재 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn open(dir: &PathBuf) -> Value {
    let o = call(json!({"op": "library.open", "dir": dir.to_str().unwrap()}));
    assert_eq!(o["ok"], true, "{o}");
    o
}

#[test]
fn the_library_works_through_the_bridge() {
    let dir = temp("기본");
    let o = open(&dir);
    let h = o["handle"].clone();
    assert!(o["folder"].as_str().unwrap().ends_with("HistoryCore"));
    let c = call(json!({"op": "doc.create", "handle": h, "ns": "doc", "title": "鶴", "body": "𪄃 😀 학", "message": "처음"}));
    assert_eq!((c["ok"].clone(), c["changed"].clone()), (json!(true), json!(true)), "{c}");
    let g = call(json!({"op": "doc.get", "handle": h, "doc": c["doc"]}));
    assert_eq!(g["doc"]["body"], "𪄃 😀 학");
    assert_eq!(g["doc"]["head"], c["rev"]);
    assert_eq!(g["doc"]["ns"], "doc");
    assert_eq!(g["doc"]["deleted"], false);
    let s = call(json!({"op": "doc.save", "handle": h, "doc": c["doc"], "base": c["rev"], "body": "둘째"}));
    assert_eq!(s["changed"], true);
    let hist = call(json!({"op": "doc.history", "handle": h, "doc": c["doc"]}));
    let revs = hist["revisions"].as_array().unwrap();
    assert_eq!(revs.len(), 2);
    assert_eq!(revs[0]["id"], s["rev"]);
    assert_eq!(revs[0]["parent"], c["rev"]);
    assert_eq!(revs[1]["kind"], "content");
    let r = call(json!({"op": "rev.get", "handle": h, "rev": c["rev"]}));
    assert_eq!(r["revision"]["body"], "𪄃 😀 학");
    let rn = call(json!({"op": "doc.rename", "handle": h, "doc": c["doc"], "base": s["rev"], "title": "두루미"}));
    assert!(rn["redirect"].is_string(), "{rn}");
    let by = call(json!({"op": "doc.by_title", "handle": h, "ns": "doc", "title": "鶴"}));
    assert_eq!(by["docs"][0]["id"], rn["redirect"]);
    let d = call(json!({"op": "doc.delete", "handle": h, "doc": c["doc"], "base": rn["rev"]}));
    assert_eq!(d["changed"], true);
    let counts = call(json!({"op": "library.counts", "handle": h}));
    assert_eq!((counts["doc"].clone(), counts["deleted"].clone(), counts["total"].clone()), (json!(1), json!(1), json!(1)), "{counts}");
    assert_eq!(call(json!({"op": "library.close", "handle": h}))["ok"], true);
    assert_eq!(call(json!({"op": "doc.get", "handle": h, "doc": c["doc"]}))["code"], "bad_handle");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_stale_save_answers_with_the_current_head() {
    // Review Focus 3
    let dir = temp("낡은 판");
    let h = open(&dir)["handle"].clone();
    let c = call(json!({"op": "doc.create", "handle": h, "ns": "doc", "title": "정유재란", "body": "첫"}));
    let s = call(json!({"op": "doc.save", "handle": h, "doc": c["doc"], "base": c["rev"], "body": "다른 창"}));
    let late = call(json!({"op": "doc.save", "handle": h, "doc": c["doc"], "base": c["rev"], "body": "늦은 저장"}));
    assert_eq!(late["ok"], false);
    assert_eq!(late["code"], "stale_base");
    assert_eq!(late["head"], s["rev"]);
    assert_eq!(call(json!({"op": "doc.get", "handle": h, "doc": c["doc"]}))["doc"]["body"], "다른 창");
    call(json!({"op": "library.close", "handle": h}));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bad_requests_are_error_answers_not_crashes() {
    let dir = temp("나쁜 요청");
    let h = open(&dir)["handle"].clone();
    let codes = [
        call(json!({"op": "doc.get", "doc": "x"}))["code"].clone(),
        call(json!({"op": "doc.get", "handle": 987654, "doc": "x"}))["code"].clone(),
        call(json!({"op": "doc.get", "handle": h, "doc": "UUID 아님"}))["code"].clone(),
        call(json!({"op": "doc.create", "handle": h, "ns": "연표", "title": "가", "body": ""}))["code"].clone(),
        call(json!({"op": "doc.create", "handle": h, "ns": "doc", "title": "가|나", "body": ""}))["code"].clone(),
        call(json!({"op": "doc.get", "handle": h, "doc": "00000000-0000-7000-8000-000000000000"}))["code"].clone(),
        call(json!({"op": "library.open"}))["code"].clone(),
    ];
    assert_eq!(codes.to_vec(), vec![json!("bad_request"), json!("bad_handle"), json!("bad_request"), json!("bad_request"), json!("bad_title"), json!("not_found"), json!("bad_request")]);
    call(json!({"op": "library.close", "handle": h}));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn opening_twice_gives_the_same_handle_and_a_foreign_file_is_refused() {
    // Review Focus 2
    let dir = temp("두 번");
    let a = open(&dir)["handle"].clone();
    let b = open(&dir)["handle"].clone();
    assert_eq!(a, b);
    call(json!({"op": "library.close", "handle": a}));
    let other = temp("남의 파일");
    std::fs::create_dir_all(other.join("HistoryCore")).unwrap();
    std::fs::write(other.join("HistoryCore").join("library.db"), "SQLite 아님").unwrap();
    let r = call(json!({"op": "library.open", "dir": other.to_str().unwrap()}));
    assert_eq!(r["code"], "not_a_library");
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&other).unwrap();
}
```

- [ ] **Step 2: 검사가 실패하는지 본다**

Run: `cargo test -p history_core_ffi --test library`
Expected: 4 failed — 답이 `{"ok":false,"error":"모르는 op: library.open"}`(`assert_eq!(o["ok"], true)`에서 멈춤).

- [ ] **Step 3: 구현한다**

`crates\ffi\src\ops.rs`:
```rust
//! 서재 일의 다리 — `library.*` · `doc.*` · `rev.*`(spec/engine-api.md). 열린 서재는 손잡이 표에 든다.
use history_library::{Doc, DocId, DocSummary, Error, Hlc, Library, Namespace, RevId, RevInfo, Saved};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

struct Open {
    next: u64,
    libraries: HashMap<u64, Library>,
}

fn table() -> MutexGuard<'static, Open> {
    static TABLE: OnceLock<Mutex<Open>> = OnceLock::new();
    TABLE
        .get_or_init(|| Mutex::new(Open { next: 1, libraries: HashMap::new() }))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

type Answer = Result<Value, Value>;

const LIBRARY_OPS: [&str; 9] = ["library.counts", "doc.create", "doc.save", "doc.rename", "doc.delete", "doc.get", "doc.by_title", "doc.history", "rev.get"];

/// 서재 op면 답을, 아니면 None(모르는 op).
pub fn handle_op(op: &str, req: &Value) -> Option<Value> {
    let answer = match op {
        "library.open" => open(req),
        "library.close" => close(req),
        op if LIBRARY_OPS.contains(&op) => with_library(req, |lib| run(op, lib, req)),
        _ => return None,
    };
    Some(answer.unwrap_or_else(|e| e))
}

fn bad(message: impl Into<String>) -> Value {
    json!({"ok": false, "error": message.into(), "code": "bad_request"})
}

fn fail(e: Error) -> Value {
    let mut v = json!({"ok": false, "error": e.to_string(), "code": e.code()});
    if let Error::StaleBase { head } = &e {
        v["head"] = json!(head.to_string());
    }
    v
}

fn text<'a>(req: &'a Value, key: &str) -> Result<&'a str, Value> {
    req.get(key).and_then(Value::as_str).ok_or_else(|| bad(format!("`{key}`(글)가 없음")))
}

fn optional_text<'a>(req: &'a Value, key: &str) -> &'a str {
    req.get(key).and_then(Value::as_str).unwrap_or("")
}

fn doc_id(req: &Value, key: &str) -> Result<DocId, Value> {
    DocId::parse(text(req, key)?).ok_or_else(|| bad(format!("`{key}`가 UUID가 아님")))
}

fn rev_id(req: &Value, key: &str) -> Result<RevId, Value> {
    RevId::parse(text(req, key)?).ok_or_else(|| bad(format!("`{key}`가 UUID가 아님")))
}

fn namespace(req: &Value) -> Result<Namespace, Value> {
    let code = text(req, "ns")?;
    Namespace::from_code(code).ok_or_else(|| bad(format!("모르는 ns: {code}")))
}

fn handle_of(req: &Value) -> Result<u64, Value> {
    req.get("handle").and_then(Value::as_u64).ok_or_else(|| bad("`handle`(수)이 없음"))
}

fn bad_handle(h: u64) -> Value {
    json!({"ok": false, "error": format!("열린 서재가 아님: {h}"), "code": "bad_handle"})
}

fn same_folder(a: &Path, b: &Path) -> bool {
    matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
}

fn opened(h: u64, lib: &Library) -> Value {
    let i = lib.info();
    json!({"ok": true, "handle": h, "library": i.library.to_string(), "device": i.device.to_string(), "format": i.format, "folder": i.folder.to_string_lossy()})
}

fn open(req: &Value) -> Answer {
    let dir = PathBuf::from(text(req, "dir")?);
    let folder = dir.join(history_library::FOLDER);
    let mut t = table();
    if let Some((&h, lib)) = t.libraries.iter().find(|(_, lib)| same_folder(&lib.info().folder, &folder)) {
        return Ok(opened(h, lib));
    }
    let lib = Library::open(&dir).map_err(fail)?;
    let h = t.next;
    t.next += 1;
    let answer = opened(h, &lib);
    t.libraries.insert(h, lib);
    Ok(answer)
}

fn close(req: &Value) -> Answer {
    let h = handle_of(req)?;
    table().libraries.remove(&h).map(|_| json!({"ok": true})).ok_or_else(|| bad_handle(h))
}

fn with_library(req: &Value, f: impl FnOnce(&mut Library) -> Answer) -> Answer {
    let h = handle_of(req)?;
    let mut t = table();
    let lib = t.libraries.get_mut(&h).ok_or_else(|| bad_handle(h))?;
    f(lib)
}

fn saved(s: &Saved) -> Value {
    json!({"ok": true, "doc": s.doc.to_string(), "rev": s.rev.to_string(), "hlc": s.hlc.0, "changed": s.changed})
}

fn doc_json(d: &Doc) -> Value {
    json!({"id": d.id.to_string(), "ns": d.ns.code(), "title": d.title, "head": d.head.to_string(), "created": d.created.0, "updated": d.updated.0, "deleted": d.deleted, "body": d.body})
}

fn summary_json(d: &DocSummary) -> Value {
    json!({"id": d.id.to_string(), "ns": d.ns.code(), "title": d.title, "head": d.head.to_string(), "created": d.created.0, "updated": d.updated.0})
}

fn rev_json(r: &RevInfo) -> Value {
    json!({
        "id": r.id.to_string(), "doc": r.doc.to_string(),
        "parent": r.parent.map(|p| p.to_string()), "parent2": r.parent2.map(|p| p.to_string()),
        "device": r.device.to_string(), "hlc": r.hlc.0, "kind": r.kind.code(), "ns": r.ns.code(),
        "title": r.title, "message": r.message, "len": r.len,
    })
}

fn not_found(what: &str) -> Value {
    json!({"ok": false, "error": format!("없음: {what}"), "code": "not_found"})
}

fn run(op: &str, lib: &mut Library, req: &Value) -> Answer {
    match op {
        "library.counts" => {
            let c = lib.counts().map_err(fail)?;
            let mut v = json!({"ok": true, "deleted": c.deleted, "total": c.total()});
            for ns in Namespace::ALL {
                v[ns.code()] = json!(c.of(ns));
            }
            Ok(v)
        }
        "doc.create" => {
            let s = lib.create(namespace(req)?, text(req, "title")?, text(req, "body")?, optional_text(req, "message")).map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.save" => {
            let s = lib.save(doc_id(req, "doc")?, rev_id(req, "base")?, text(req, "body")?, optional_text(req, "message")).map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.delete" => {
            let s = lib.delete(doc_id(req, "doc")?, rev_id(req, "base")?, optional_text(req, "message")).map_err(fail)?;
            Ok(saved(&s))
        }
        "doc.rename" => {
            let keep = req.get("keep_redirect").and_then(Value::as_bool).unwrap_or(true);
            let r = lib.rename(doc_id(req, "doc")?, rev_id(req, "base")?, text(req, "title")?, keep, optional_text(req, "message")).map_err(fail)?;
            Ok(json!({"ok": true, "rev": r.rev.to_string(), "hlc": r.hlc.0, "redirect": r.redirect.map(|d| d.to_string())}))
        }
        "doc.get" => {
            let id = doc_id(req, "doc")?;
            match lib.get(id).map_err(fail)? {
                Some(d) => Ok(json!({"ok": true, "doc": doc_json(&d)})),
                None => Err(not_found(&format!("문서 {id}"))),
            }
        }
        "doc.by_title" => {
            let docs = lib.by_title(namespace(req)?, text(req, "title")?).map_err(fail)?;
            Ok(json!({"ok": true, "docs": docs.iter().map(summary_json).collect::<Vec<_>>()}))
        }
        "doc.history" => {
            let limit = req.get("limit").and_then(Value::as_u64).unwrap_or(50).min(1000) as usize;
            let before = req.get("before").and_then(Value::as_u64).map(Hlc);
            let revs = lib.history(doc_id(req, "doc")?, limit, before).map_err(fail)?;
            Ok(json!({"ok": true, "revisions": revs.iter().map(rev_json).collect::<Vec<_>>()}))
        }
        "rev.get" => {
            let id = rev_id(req, "rev")?;
            match lib.revision(id).map_err(fail)? {
                Some(r) => {
                    let mut v = rev_json(&r.info);
                    v["body"] = json!(r.body);
                    Ok(json!({"ok": true, "revision": v}))
                }
                None => Err(not_found(&format!("판 {id}"))),
            }
        }
        other => Err(bad(format!("모르는 서재 op: {other}"))),
    }
}
```

`crates\ffi\src\lib.rs`:
- 맨 위 `use serde_json::…` 줄 위에 `mod ops;`를 더한다.
- `handle`의 doc 주석을 `/// 요청 하나를 처리한다. version · echo는 상태가 없고, 서재 op는 손잡이 표(Mutex) 하나를 나눠 쓴다 — 여러 스레드에서 불러도 된다.`로 바꾼다.
- `Some(op) => json!({"ok": false, "error": format!("모르는 op: {op}")}),` 줄을 다음으로 바꾼다:
```rust
        Some(op) => ops::handle_op(op, &req).unwrap_or_else(|| json!({"ok": false, "error": format!("모르는 op: {op}")})),
```

`spec\engine-api.md`:
```markdown
# 엔진 질의 약속

다리 `hc_call(요청 JSON) → 답 JSON`(UTF-8). 앱은 이 문서만 보고 엔진을 부른다. P2-1이 정한 것은 **됨**, 뒤 계획이 채울 것은 **예정**(모양은 그 계획에서 굳히고 이 문서를 고친다 — 아직 출시 전이라 ABI 판은 1 그대로).

## 약속

- 요청은 `{"op": "<이름>", …}`. 답은 늘 `ok`를 갖는다.
- 실패: `{"ok": false, "error": "<사람이 읽는 글>", "code": "<짧은 이름>"}`.
- ID는 UUID 글(소문자 · 하이픈). 시각(`hlc`)은 수 — `(ms << 16) | 카운터`.
- 이름공간 `ns`: `doc`(문서) · `category`(분류) · `template`(틀) · `file`(파일) · `front`(대문).
- 서재 op는 `library.open`이 준 `handle`을 받는다.

| code | 뜻 |
| --- | --- |
| `bad_request` | 요청 모양이 틀림(빠진 값 · UUID 아님 · 모르는 ns) |
| `bad_handle` | 열린 서재가 아님(닫았거나 없음) |
| `not_found` | 그런 문서 · 판이 없음 |
| `bad_title` | 제목으로 쓸 수 없음(spec/library-format.md「제목」) |
| `stale_base` | 고치기 시작한 판이 지금 머리가 아님 — 덮어쓰지 않았다. 답에 `head`(지금 머리 판) |
| `deleted` | 지운 문서에는 할 수 없음 |
| `not_a_library` | 서재 자리에 다른 파일 — 건드리지 않았다 |
| `newer_format` | 더 새 형식의 서재 — 건드리지 않았다 |
| `corrupt` · `sqlite` · `io` | 저장소 · 파일 문제 |

## 됨 (P2-1)

| op | 요청 | 답 |
| --- | --- | --- |
| `version` | — | `engine` · `abi` |
| `library.open` | `dir`(앱 자료 폴더) | `handle` · `library` · `device` · `format` · `folder`(`<dir>/HistoryCore`). 같은 폴더를 다시 열면 같은 `handle` |
| `library.close` | `handle` | — |
| `library.counts` | `handle` | `doc` · `category` · `template` · `file` · `front`(살아 있는 수) · `deleted` · `total` |
| `doc.create` | `handle` · `ns` · `title` · `body` · `message`? | `doc` · `rev` · `hlc` · `changed`(참) |
| `doc.save` | `handle` · `doc` · `base` · `body` · `message`? | `doc` · `rev` · `hlc` · `changed`(본문이 같으면 거짓, 판을 더하지 않음) |
| `doc.rename` | `handle` · `doc` · `base` · `title` · `keep_redirect`?(기본 참) · `message`? | `rev` · `hlc` · `redirect`(옛 제목의 넘겨주기 문서 ID 또는 null) |
| `doc.delete` | `handle` · `doc` · `base` · `message`? | `doc` · `rev` · `hlc` · `changed` |
| `doc.get` | `handle` · `doc` | `doc`: `id` · `ns` · `title` · `head` · `created` · `updated` · `deleted` · `body` |
| `doc.by_title` | `handle` · `ns` · `title` | `docs`: [`id` · `ns` · `title` · `head` · `created` · `updated`] — 만든 차례, 지운 것 뺌 |
| `doc.history` | `handle` · `doc` · `limit`?(50, 많아야 1000) · `before`?(hlc) | `revisions`: [`id` · `doc` · `parent` · `parent2` · `device` · `hlc` · `kind`(`content`/`deleted`) · `ns` · `title` · `message` · `len`] — 새 것부터 |
| `rev.get` | `handle` · `rev` | `revision`: 위 판 정보 + `body` |

## 예정

| op | 계획 | 하는 일 |
| --- | --- | --- |
| `namumark.parse` | P2-2 | 저장하지 않은 글의 그리기 모델(편집기 미리보기) |
| `doc.open` | P2-3 | 저장 때 굳힌 그리기 모델 · 분류 · 각주 · 넘겨주기 따라가기(한 번) |
| `titles.exist` | P2-3 | 제목 목록 → 있는 문서 ID(설계 5-6, 링크 색) |
| `search.titles` · `search.body` | P2-3 | 제목 앞부분 · 본문 찾기(한글 · 한자 2글자 조각), 첫 n건 |
| `tree.children` | P2-3 | 분류 폴더 하나: 하위 분류(아래 문서 합) · 문서 |
| `links.back` · `links.wanted` | P2-3 | 가리키는 문서 · 만들 문서 |
| `home.aggregates` · `stats.*` | P2-3 | 대문 · 서재 통계 모음(설계 5-5) |
| `timeline.docs` | P2-3 | 연표 항목ID → 이어진 문서(창구 `docs_for_entity`) |
| `hanja.get` · `hanja.search` | P2-4 | 자전 조목 · 찾기 |
```

- [ ] **Step 4: 검사가 통과하는지 본다**

Run: `cargo test --locked --workspace`
Expected: 모두 통과 — ffi `library.rs` 4 · `call.rs` 7 포함. `cargo test`가 `Cargo.lock`을 고치므로 `--locked` 앞에 한 번 `cargo test --workspace`를 돌린다.

- [ ] **Step 5: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "다리: library.* · doc.* · rev.* op와 손잡이 표 · 엔진 질의 약속(spec/engine-api.md)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Dart `Library` — 플러그인이 서재 op를 감싼다

**Files:**
- Create: `packages\history_core\lib\src\library.dart`, `packages\history_core\test\support.dart`, `packages\history_core\test\library_test.dart`
- Modify: `packages\history_core\lib\src\engine.dart`, `packages\history_core\lib\history_core.dart`, `packages\history_core\test\engine_test.dart`

**Interfaces:**
- Consumes: Task 4의 op · 답 모양.
- Produces(`package:history_core/history_core.dart`):
  - `class HistoryCoreError implements Exception { HistoryCoreError(String message, {String? code}); final String message; final String? code; }` (생성자에 `code`가 더해짐 — 앞 쓰임 그대로 맞음)
  - `class StaleBaseError extends HistoryCoreError { final String head; }`
  - `enum Namespace { doc, category, template, file, front }`
  - `class LibraryInfo { library, device, folder: String; format: int }` · `class Saved { doc, rev: String; hlc: int; changed: bool }` · `class Renamed { rev: String; hlc: int; redirect: String? }` · `class DocRecord { id, title, head: String; ns: Namespace; created, updated: int; deleted: bool; body: String }` · `class DocSummary { id, title, head; ns; created, updated }` · `class RevisionInfo { id, doc, device, title, message: String; parent, parent2: String?; hlc, length: int; deleted: bool; ns: Namespace }` · `class Counts { live: Map<Namespace, int>; deleted, total: int }`
  - `class Library { static Future<Library> open(HistoryCore core, String appDataDir); LibraryInfo info; create(Namespace, String title, String body, {String message}) → Saved; save(String doc, String base, String body, {String message}) → Saved; rename(String doc, String base, String title, {bool keepRedirect = true, String message}) → Renamed; delete(String doc, String base, {String message}) → Saved; get(String doc) → DocRecord?; byTitle(Namespace, String) → List<DocSummary>; history(String doc, {int limit = 50, int? before}) → List<RevisionInfo>; revisionBody(String rev) → String?; counts() → Counts; close() }`

- [ ] **Step 1: 시험 도우미를 나누고 실패하는 검사를 쓴다**

`packages\history_core\test\support.dart`:
```dart
import 'dart:io';

/// `tools/build-native.ps1`이 놓은 DLL(또는 HISTORY_CORE_LIBRARY).
String builtLibrary() =>
    Platform.environment['HISTORY_CORE_LIBRARY'] ??
    [Directory.current.path, 'native', 'windows-x64', 'history_core.dll'].join(Platform.pathSeparator);

/// 미리 지은 DLL로 도는 검사는 Windows에서만.
final Object windowsOnly = Platform.isWindows ? false : 'Windows에서만 — 미리 지은 DLL로 돈다';
```

`packages\history_core\test\engine_test.dart`: 맨 위 import 아래에 `import 'support.dart';`를 더하고, 파일 안의 `String builtLibrary() => …;` 함수 정의와 `main()` 안의 `final windowsOnly = …;` 줄을 지운다(이제 support.dart의 것을 쓴다).

`packages\history_core\test\library_test.dart`:
```dart
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:history_core/history_core.dart';

import 'support.dart';

void main() {
  late Directory dir;
  late HistoryCore core;

  setUp(() async {
    dir = Directory.systemTemp.createTempSync('서재 다리 시험 ');
    if (Platform.isWindows) core = await HistoryCore.start(libraryPath: builtLibrary());
  });

  tearDown(() {
    if (Platform.isWindows) core.close();
    try {
      dir.deleteSync(recursive: true);
    } on FileSystemException {
      // 엔진이 연 파일이 잠깐 잠겨 있을 수 있다 — 임시 폴더라 남겨도 된다.
    }
  });

  test('만들고 읽고 고치고 판 기록을 본다', () async {
    final lib = await Library.open(core, dir.path);
    expect(lib.info.folder, endsWith('HistoryCore'));
    expect(lib.info.format, 1);
    final a = await lib.create(Namespace.doc, '鶴', '𪄃 😀 학은 물새다.', message: '처음');
    final d = await lib.get(a.doc);
    expect(d!.body, '𪄃 😀 학은 물새다.');
    expect(d.head, a.rev);
    expect(d.ns, Namespace.doc);
    final b = await lib.save(a.doc, a.rev, '둘째 판', message: '고침');
    expect(b.changed, isTrue);
    expect((await lib.save(a.doc, b.rev, '둘째 판')).changed, isFalse);
    final h = await lib.history(a.doc);
    expect(h.map((r) => r.id), [b.rev, a.rev]);
    expect(h.first.parent, a.rev);
    expect(await lib.revisionBody(a.rev), '𪄃 😀 학은 물새다.');
    final counts = await lib.counts();
    expect((counts.total, counts.live[Namespace.doc], counts.deleted), (1, 1, 0));
    expect(await lib.get('00000000-0000-7000-8000-000000000000'), isNull);
    await lib.close();
  }, skip: windowsOnly);

  test('낡은 판 위 저장은 StaleBaseError — 덮어쓰지 않는다', () async {
    // Review Focus 3
    final lib = await Library.open(core, dir.path);
    final a = await lib.create(Namespace.doc, '정유재란', '첫 판');
    final b = await lib.save(a.doc, a.rev, '다른 창의 저장');
    await expectLater(
      lib.save(a.doc, a.rev, '늦은 저장'),
      throwsA(isA<StaleBaseError>().having((e) => e.head, 'head', b.rev).having((e) => e.code, 'code', 'stale_base')),
    );
    expect((await lib.get(a.doc))!.body, '다른 창의 저장');
    await lib.close();
  }, skip: windowsOnly);

  test('이름을 바꾸면 옛 제목에 넘겨주기가 남고, 지우면 표시만 남는다', () async {
    final lib = await Library.open(core, dir.path);
    final a = await lib.create(Namespace.category, '조선 중기', '');
    final r = await lib.rename(a.doc, a.rev, '조선 중엽');
    final old = await lib.byTitle(Namespace.category, '조선 중기');
    expect(old.single.id, r.redirect);
    expect((await lib.get(r.redirect!))!.body, '#넘겨주기 분류:조선 중엽\n');
    final d = await lib.delete(a.doc, r.rev);
    expect(d.changed, isTrue);
    expect((await lib.get(a.doc))!.deleted, isTrue);
    expect((await lib.history(a.doc)).first.deleted, isTrue);
    await lib.close();
  }, skip: windowsOnly);

  test('서재 자리에 남의 파일이 있으면 not_a_library — 파일은 그대로', () async {
    // Review Focus 2
    final ours = File([dir.path, 'HistoryCore', 'library.db'].join(Platform.pathSeparator))
      ..createSync(recursive: true)
      ..writeAsStringSync('SQLite 아님');
    await expectLater(
      Library.open(core, dir.path),
      throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'not_a_library')),
    );
    expect(ours.readAsStringSync(), 'SQLite 아님');
  }, skip: windowsOnly);

  test('쓸 수 없는 제목은 bad_title', () async {
    final lib = await Library.open(core, dir.path);
    await expectLater(
      lib.create(Namespace.doc, '분류:조선', ''),
      throwsA(isA<HistoryCoreError>().having((e) => e.code, 'code', 'bad_title')),
    );
    await lib.close();
  }, skip: windowsOnly);
}
```

- [ ] **Step 2: 엔진을 다시 짓고 검사가 실패하는지 본다**

Run:
```powershell
Set-Location D:\HistoryCore; ./tools/build-native.ps1
Set-Location packages\history_core; & $flutter test
```
Expected: 컴파일 실패 — `Undefined name 'Library'` · `Namespace` · `StaleBaseError`.

- [ ] **Step 3: 구현한다**

`packages\history_core\lib\src\engine.dart`의 `HistoryCoreError`를 바꾼다:
```dart
/// 엔진이 돌려준 오류. [code]는 다리의 짧은 이름(spec/engine-api.md) — 없을 수 있다.
class HistoryCoreError implements Exception {
  HistoryCoreError(this.message, {this.code});
  final String message;
  final String? code;
  @override
  String toString() => code == null ? 'HistoryCoreError: $message' : 'HistoryCoreError($code): $message';
}
```

`packages\history_core\lib\src\library.dart`:
```dart
import 'engine.dart';

/// 서재 이름공간(설계 5-1). 이름이 다리의 `ns` 값이다.
enum Namespace { doc, category, template, file, front }

/// 고치기 시작한 판이 지금 머리가 아니다 — 그 사이 다른 저장이 있었다. 덮어쓰지 않았다. [head]가 지금 머리 판.
class StaleBaseError extends HistoryCoreError {
  StaleBaseError(super.message, {required this.head}) : super(code: 'stale_base');
  final String head;
}

class LibraryInfo {
  LibraryInfo._(Map<String, dynamic> j)
      : library = j['library'] as String,
        device = j['device'] as String,
        format = j['format'] as int,
        folder = j['folder'] as String;
  final String library;
  final String device;
  final int format;

  /// 엔진의 자리(`<앱 자료 폴더>/HistoryCore`).
  final String folder;
}

class Saved {
  Saved._(Map<String, dynamic> j)
      : doc = j['doc'] as String,
        rev = j['rev'] as String,
        hlc = j['hlc'] as int,
        changed = j['changed'] as bool;
  final String doc;
  final String rev;
  final int hlc;

  /// 거짓이면 본문이 같아 판을 더하지 않았다.
  final bool changed;
}

class Renamed {
  Renamed._(Map<String, dynamic> j)
      : rev = j['rev'] as String,
        hlc = j['hlc'] as int,
        redirect = j['redirect'] as String?;
  final String rev;
  final int hlc;

  /// 옛 제목에 남긴 넘겨주기 문서.
  final String? redirect;
}

class DocRecord {
  DocRecord._(Map<String, dynamic> j)
      : id = j['id'] as String,
        ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        head = j['head'] as String,
        created = j['created'] as int,
        updated = j['updated'] as int,
        deleted = j['deleted'] as bool,
        body = j['body'] as String;
  final String id;
  final Namespace ns;
  final String title;
  final String head;
  final int created;
  final int updated;
  final bool deleted;
  final String body;
}

class DocSummary {
  DocSummary._(Map<String, dynamic> j)
      : id = j['id'] as String,
        ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        head = j['head'] as String,
        created = j['created'] as int,
        updated = j['updated'] as int;
  final String id;
  final Namespace ns;
  final String title;
  final String head;
  final int created;
  final int updated;
}

class RevisionInfo {
  RevisionInfo._(Map<String, dynamic> j)
      : id = j['id'] as String,
        doc = j['doc'] as String,
        parent = j['parent'] as String?,
        parent2 = j['parent2'] as String?,
        device = j['device'] as String,
        hlc = j['hlc'] as int,
        deleted = j['kind'] == 'deleted',
        ns = Namespace.values.byName(j['ns'] as String),
        title = j['title'] as String,
        message = j['message'] as String,
        length = j['len'] as int;
  final String id;
  final String doc;
  final String? parent;
  final String? parent2;
  final String device;
  final int hlc;

  /// 지우기 표시 판.
  final bool deleted;
  final Namespace ns;
  final String title;
  final String message;

  /// 본문의 UTF-8 바이트 수.
  final int length;
}

class Counts {
  Counts._(Map<String, dynamic> j)
      : live = {for (final ns in Namespace.values) ns: j[ns.name] as int},
        deleted = j['deleted'] as int,
        total = j['total'] as int;

  /// 이름공간별 살아 있는 문서 수.
  final Map<Namespace, int> live;
  final int deleted;
  final int total;
}

/// 열린 서재 하나(spec/engine-api.md의 `library.*` · `doc.*` · `rev.*`).
class Library {
  Library._(this._core, this._handle, this.info);

  final HistoryCore _core;
  final int _handle;
  final LibraryInfo info;

  /// 앱 자료 폴더 [appDataDir] 아래 `HistoryCore/`에 서재를 연다(없으면 만든다). 그 폴더 바로 아래 옛 앱 파일은 건드리지 않는다.
  static Future<Library> open(HistoryCore core, String appDataDir) async {
    final r = _check(await core.call({'op': 'library.open', 'dir': appDataDir}));
    return Library._(core, r['handle'] as int, LibraryInfo._(r));
  }

  Future<Map<String, dynamic>> _call(String op, [Map<String, Object?> args = const {}]) async =>
      _check(await _core.call({'op': op, 'handle': _handle, ...args}));

  Future<Saved> create(Namespace ns, String title, String body, {String message = ''}) async =>
      Saved._(await _call('doc.create', {'ns': ns.name, 'title': title, 'body': body, 'message': message}));

  Future<Saved> save(String doc, String base, String body, {String message = ''}) async =>
      Saved._(await _call('doc.save', {'doc': doc, 'base': base, 'body': body, 'message': message}));

  Future<Renamed> rename(String doc, String base, String title, {bool keepRedirect = true, String message = ''}) async =>
      Renamed._(await _call('doc.rename', {'doc': doc, 'base': base, 'title': title, 'keep_redirect': keepRedirect, 'message': message}));

  Future<Saved> delete(String doc, String base, {String message = ''}) async =>
      Saved._(await _call('doc.delete', {'doc': doc, 'base': base, 'message': message}));

  /// 없는 문서면 null.
  Future<DocRecord?> get(String doc) async {
    try {
      return DocRecord._((await _call('doc.get', {'doc': doc}))['doc'] as Map<String, dynamic>);
    } on HistoryCoreError catch (e) {
      if (e.code == 'not_found') return null;
      rethrow;
    }
  }

  Future<List<DocSummary>> byTitle(Namespace ns, String title) async => [
        for (final d in (await _call('doc.by_title', {'ns': ns.name, 'title': title}))['docs'] as List)
          DocSummary._(d as Map<String, dynamic>),
      ];

  /// 판 기록, 새 것부터. [before](hlc)를 주면 그보다 앞선 판만.
  Future<List<RevisionInfo>> history(String doc, {int limit = 50, int? before}) async => [
        for (final r in (await _call('doc.history', {'doc': doc, 'limit': limit, 'before': before}))['revisions'] as List)
          RevisionInfo._(r as Map<String, dynamic>),
      ];

  /// 판 하나의 본문. 없는 판이면 null.
  Future<String?> revisionBody(String rev) async {
    try {
      return ((await _call('rev.get', {'rev': rev}))['revision'] as Map<String, dynamic>)['body'] as String;
    } on HistoryCoreError catch (e) {
      if (e.code == 'not_found') return null;
      rethrow;
    }
  }

  Future<Counts> counts() async => Counts._(await _call('library.counts'));

  Future<void> close() async {
    await _call('library.close');
  }
}

Map<String, dynamic> _check(Map<String, dynamic> r) {
  if (r['ok'] == true) return r;
  final message = r['error'] as String? ?? '알 수 없는 오류';
  final code = r['code'] as String?;
  if (code == 'stale_base') throw StaleBaseError(message, head: r['head'] as String);
  throw HistoryCoreError(message, code: code);
}
```
(`before`가 null이면 JSON `null`로 간다 — 엔진은 `as_u64()`가 None이라 "앞선 판 제한 없음"으로 본다.)

`packages\history_core\lib\history_core.dart`의 `export 'src/engine.dart';` 아래에 `export 'src/library.dart';`를 더한다.

- [ ] **Step 4: 검사가 통과하는지 본다**

Run(`packages\history_core`에서): `& $flutter analyze; & $flutter test`
Expected: `No issues found!` · engine 6 · library 5 · ci_workflow 2 = 13 passed.

- [ ] **Step 5: 커밋한다**

```powershell
Set-Location D:\HistoryCore
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "Dart Library — 서재 op를 감싼 API · StaleBaseError · 오류 code" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: 아카이브 앱이 실제 자료 폴더에서 서재를 연다

**Files:**
- Modify: `D:\HistoryArchive\pubspec.yaml`, `lib\main.dart`, `test\app_test.dart`, `tools\windows\smoke-test.ps1`, `tools\windows\plugin-junction.ps1`

**Interfaces:**
- Consumes: Task 5의 `Library.open` · `counts()` · `HistoryCoreError`.
- Produces: `Future<String> appDataDir({Map<String, String>? environment})`(환경 변수 `HISTORY_ARCHIVE_DATA_DIR`가 먼저, 없으면 `getApplicationSupportDirectory()`) · `Future<String> engineStatus({Future<HistoryCore> Function()? start, Future<String> Function()? dataDir})` — 성공 `엔진 <판> · ABI <n> · 서재 <N>편`, 서재 실패 `엔진 <판> · ABI <n> · 서재를 열지 못했습니다 — <까닭>`, 엔진 실패 `엔진을 불러오지 못했습니다 — <까닭>`.

- [ ] **Step 1: 실패하는 검사를 쓴다**

`test\app_test.dart`의 `'실제 엔진을 연다(Windows)'` 검사를 다음 셋으로 바꾸고(나머지 검사는 그대로), 맨 위 import에 변화 없음:
```dart
  test('실제 엔진을 열고 서재 문서 수를 띄운다(Windows)', () async {
    final data = Directory.systemTemp.createTempSync('앱 자료 ');
    addTearDown(() => data.deleteSync(recursive: true));
    final s = await engineStatus(start: () => HistoryCore.start(libraryPath: engineDll()), dataDir: () async => data.path);
    expect(s, '엔진 0.0.1 · ABI 1 · 서재 0편');
    expect(Directory('${data.path}${Platform.pathSeparator}HistoryCore').existsSync(), isTrue);
  }, skip: Platform.isWindows ? false : 'Windows에서만');

  test('서재를 못 열면 엔진 판 번호와 함께 까닭을 보인다(Windows)', () async {
    // Review Focus 2
    final data = Directory.systemTemp.createTempSync('앱 자료 ');
    addTearDown(() => data.deleteSync(recursive: true));
    File([data.path, 'HistoryCore', 'library.db'].join(Platform.pathSeparator))
      ..createSync(recursive: true)
      ..writeAsStringSync('SQLite 아님');
    final s = await engineStatus(start: () => HistoryCore.start(libraryPath: engineDll()), dataDir: () async => data.path);
    expect(s, startsWith('엔진 0.0.1 · ABI 1 · 서재를 열지 못했습니다 — 서재 파일이 아님'));
  }, skip: Platform.isWindows ? false : 'Windows에서만');

  test('HISTORY_ARCHIVE_DATA_DIR가 있으면 그 폴더를 앱 자료 폴더로 쓴다', () async {
    expect(await appDataDir(environment: {'HISTORY_ARCHIVE_DATA_DIR': r'D:\시험 자료'}), r'D:\시험 자료');
  });
```
그리고 `main()` 위에 도우미를 더한다:
```dart
String engineDll() => '${Directory.current.parent.path}\\HistoryCore\\packages\\history_core\\native\\windows-x64\\history_core.dll';
```

- [ ] **Step 2: 검사가 실패하는지 본다**

Run(`D:\HistoryArchive`에서): `& $flutter test test/app_test.dart`
Expected: 컴파일 실패 — `No named parameter with the name 'dataDir'` · `Method not found: 'appDataDir'`.

- [ ] **Step 3: 구현한다**

`pubspec.yaml`의 `history_core:` 묶음 위에 더한다:
```yaml
  path_provider: ^2.1.6
```

`lib\main.dart`(통째로):
```dart
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:history_core/history_core.dart';
import 'package:path_provider/path_provider.dart';

import 'src/app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final status = engineStatus();
  runApp(ArchiveApp(status: status));
  await reportStatus(status);
}

/// 앱 자료 폴더. 시험 · 연기 시험은 `HISTORY_ARCHIVE_DATA_DIR`로 바꿔 끼운다(사용자의 자료 폴더를 건드리지 않게).
Future<String> appDataDir({Map<String, String>? environment}) async {
  final override = (environment ?? Platform.environment)['HISTORY_ARCHIVE_DATA_DIR'];
  if (override != null && override.isNotEmpty) return override;
  return (await getApplicationSupportDirectory()).path;
}

/// 엔진을 열어 판 번호를 읽고, 서재를 열어 문서 수를 센다. 실패하면 그 까닭을 글로 돌려준다(앱은 멈추지 않는다).
Future<String> engineStatus({Future<HistoryCore> Function()? start, Future<String> Function()? dataDir}) async {
  final HistoryCore core;
  try {
    core = await (start ?? HistoryCore.start)();
  } on HistoryCoreError catch (e) {
    return '엔진을 불러오지 못했습니다 — ${e.message}';
  }
  try {
    final v = await core.version();
    final head = '엔진 ${v.engine} · ABI ${v.abi}';
    try {
      final library = await Library.open(core, await (dataDir ?? appDataDir)());
      try {
        return '$head · 서재 ${(await library.counts()).total}편';
      } finally {
        await library.close();
      }
    } on HistoryCoreError catch (e) {
      return '$head · 서재를 열지 못했습니다 — ${e.message}';
    }
  } on HistoryCoreError catch (e) {
    return '엔진을 불러오지 못했습니다 — ${e.message}';
  } finally {
    core.close();
  }
}

/// 엔진 상태 한 줄을 로그(Android logcat)에 남기고, 환경 변수 `HISTORY_ARCHIVE_STATUS_FILE`이 있으면
/// 그 파일에도 쓴다 — 연기 시험이 창 안의 글을 읽는 대신 이 파일을 본다.
Future<void> reportStatus(Future<String> status, {Map<String, String>? environment}) async {
  final line = await status;
  debugPrint('HistoryArchive: $line');
  final path = (environment ?? Platform.environment)['HISTORY_ARCHIVE_STATUS_FILE'];
  if (path != null && path.isNotEmpty) {
    await File(path).writeAsString(line, flush: true);
  }
}
```

`tools\windows\smoke-test.ps1`:
- `[string]$ExpectStatus = '^엔진 \d+\.\d+\.\d+ · ABI \d+$'` → `[string]$ExpectStatus = '^엔진 \d+\.\d+\.\d+ · ABI \d+ · 서재 \d+편$'`
- 첫 줄 주석 아래에 한 줄 더한다: `# 앱 자료 폴더도 임시 폴더(HISTORY_ARCHIVE_DATA_DIR)로 바꿔 — 사용자의 서재를 건드리지 않는다.`
- `$env:HISTORY_ARCHIVE_STATUS_FILE = $statusFile` 줄 아래에 `$env:HISTORY_ARCHIVE_DATA_DIR = Join-Path $work '자료'`를, `Remove-Item Env:\HISTORY_ARCHIVE_STATUS_FILE` 줄 아래에 `Remove-Item Env:\HISTORY_ARCHIVE_DATA_DIR`를 더한다.

`tools\windows\plugin-junction.ps1`(통째로 — 이제 Windows 플러그인 전부):
```powershell
# 개발자 모드 없이 Windows 빌드를 하려고, Flutter가 만들 플러그인 symlink 자리에 디렉터리 정션을 미리 만든다.
# 관리자 권한이 필요 없고 시스템 설정을 바꾸지 않는다. 플러그인 목록이 바뀌면 다시 돌린다.
# 먼저 `flutter pub get`을 한 번 돌려 .flutter-plugins-dependencies가 있어야 한다(symlink 오류로 멈춰도 그 파일은 생긴다).
$ErrorActionPreference = 'Stop'

function Invoke-Cmd {
  # cmd의 오류 글(stderr)을 PowerShell 5.1이 멈춤 오류로 바꾸지 않게, 부르는 동안만 Continue로 두고 종료 코드로 본다.
  # 인자는 배열로 넘긴다 — 따옴표를 품은 글 하나로 넘기면 5.1이 따옴표를 망가뜨린다.
  param([string]$What, [string[]]$Arguments)
  $ErrorActionPreference = 'Continue'
  & cmd /c @Arguments | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "$What 실패(종료 코드 $LASTEXITCODE): $($Arguments -join ' ')" }
}

$repo = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$depsFile = Join-Path $repo '.flutter-plugins-dependencies'
if (-not (Test-Path -LiteralPath $depsFile)) { throw "먼저 flutter pub get을 돌리세요 — 없음: $depsFile" }
$deps = [IO.File]::ReadAllText($depsFile, [Text.Encoding]::UTF8) | ConvertFrom-Json
$links = Join-Path $repo 'windows\flutter\ephemeral\.plugin_symlinks'
New-Item -ItemType Directory -Force -Path $links | Out-Null
foreach ($plugin in @($deps.plugins.windows)) {
  $link = Join-Path $links $plugin.name
  if (Test-Path -LiteralPath $link) { Invoke-Cmd '정션 지우기(진짜 폴더면 멈춘다)' @('rmdir', $link) }
  Invoke-Cmd '정션 만들기' @('mklink', '/J', $link, $plugin.path)
  Write-Output "정션: $($plugin.name) → $($plugin.path)"
}
```
두 `.ps1`에 BOM을 붙인다(Global Constraints).

- [ ] **Step 4: 플러그인 정션을 고치고 검사가 통과하는지 본다**

Run(`D:\HistoryArchive`에서):
```powershell
& $flutter pub get          # path_provider_windows 정션이 없어 symlink 오류로 멈출 수 있다 — 괜찮다
./tools/windows/plugin-junction.ps1
& $flutter pub get; & $flutter analyze; & $flutter test
```
Expected: 정션 줄이 플러그인마다(history_core · path_provider_windows) · `No issues found!` · 17 passed(앞 15 + 이번에 바뀐 하나 · 더한 둘).

- [ ] **Step 5: Windows · Android로 지어 본다**

```powershell
Set-Location D:\HistoryCore; ./tools/build-native.ps1 -Android
Set-Location D:\HistoryArchive
$env:CL = '/utf-8'; & $flutter build windows --release
./tools/windows/smoke-test.ps1 -Exe build\windows\x64\runner\Release\WikiArchive.exe
& $flutter build apk --release --target-platform android-arm64 --android-project-arg=kotlin.incremental=false
```
Expected: `연기 시험 통과: 'WikiArchive' · 엔진 0.0.1 · ABI 1 · 서재 0편 · …\연기 시험 …\내 프로그램` · APK 지어짐. 연기 시험은 `%APPDATA%\HistoryArchive\WikiArchive`(사용자 자료 폴더)에 아무것도 만들지 않는다 — 돌린 뒤 그 폴더에 `HistoryCore`가 새로 생기지 않았는지 본다(`Test-Path "$env:APPDATA\HistoryArchive\WikiArchive\HistoryCore"`가 돌리기 전과 같음).

(에뮬레이터가 있으면 P0 때처럼 `hc-test` AVD를 `-read-only -no-snapshot-save`로 띄워 x86_64 APK를 깔고 logcat에서 `HistoryArchive: 엔진 0.0.1 · ABI 1 · 서재 0편`을 본다 — 없으면 건너뛰고 보고에 적는다.)

- [ ] **Step 6: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "앱이 자료 폴더의 HistoryCore/에서 서재를 열어 문서 수를 띄운다 · 연기 시험은 임시 자료 폴더 · 정션은 모든 Windows 플러그인" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
밀기는 감독이 허락한 뒤에(`git -C D:\HistoryCore push` 먼저, 그다음 `git -C D:\HistoryArchive push` — 앱 CI가 HistoryCore `main`을 받는다).
