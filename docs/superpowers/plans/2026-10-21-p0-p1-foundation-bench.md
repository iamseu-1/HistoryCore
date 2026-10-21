# P0 바탕 · P1 측정 판 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 새 저장소 둘(HistoryCore · HistoryArchive)을 세우고, Android · Windows 앱이 Rust 엔진을 불러 판 번호를 띄우게 한 뒤(P0), 100만 건 더미 서재로 서재 구현을 재어 설계 9-2 예산표와 견주는 측정 판과 기준 수치를 만든다(P1).

**Architecture:** HistoryCore는 Rust 작업공간(`crates/library` 서재 엔진 · `crates/ffi` C 다리 · `crates/corpus` 더미 서재 · `crates/bench` 측정)과 Flutter FFI 플러그인(`packages/history_core`)이다. 엔진은 HistoryTime 엔진과 같은 꼴 — UTF-8 JSON을 주고받는 C 함수 `hc_call`/`hc_free`를 전용 isolate에서 부른다. HistoryArchive는 그 플러그인을 쓰는 Flutter 앱이다. 측정 판은 `Library` 약속 하나에 기준 구현(`NaiveSqlite`)을 꽂아 잰다 — P2의 서재 엔진도 같은 약속에 꽂는다.

**Tech Stack:** Rust 1.94.1 · rusqlite 0.37(bundled, FTS5) · serde_json 1 · Flutter 3.47.7 / Dart ^3.13.0 · ffi 2.2.0 · Android NDK 28.2.13676358 · PowerShell · Inno Setup 6(P6에서) · GitHub Actions

**Spec:** `docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md`(특히 4-1 · 8 · 9절). 이 계획은 그 9-1의 P0 · P1이다.

## Global Constraints

- 저장소: `D:\HistoryCore`(GitHub `iamseu-11/HistoryCore`) · `D:\HistoryArchive`(GitHub `iamseu-11/HistoryArchive`). **두 폴더는 나란히 있어야 한다** — 앱이 `../HistoryCore/packages/history_core`를 쓴다. 둘 다 ASCII 경로(한글 경로의 Android 빌드 문제를 피함).
- 옛 저장소(가져올 것의 원본): `D:\아카이브\HistoryArchive`(가지 `claude/desktop-modules` 작업본). 읽기만 한다.
- 도구: Rust 1.94.1(`rust-toolchain.toml`) · Flutter 3.47.7 = `D:\flutter-sdk\flutter\bin\flutter.bat`(PATH에 없음 — 아래에서 `$flutter`) · JDK 21 · Android SDK `%LOCALAPPDATA%\Android\Sdk` · NDK `28.2.13676358` · adb `%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe` · Python 3은 `py -3 -I`.
- 대상: Windows x64 · Android arm64-v8a(x86_64는 에뮬레이터 시험에만).
- **Windows에서 플러그인이 든 Flutter 앱을 빌드하려면 개발자 모드(symlink)가 필요하다.** 감독이 설정 › 시스템 › 개발자용 › 개발자 모드를 켠다. 못 켜면 Task 3 Step 9의 정션 절차를 쓴다(관리자 권한 불필요).
- Windows 빌드 전 `$env:CL = '/utf-8'`. 이 PC의 Android 빌드는 `--android-project-arg=kotlin.incremental=false`(D: 작업본과 C: pub 캐시의 Kotlin 증분 캐시 오류를 피함). CI는 옛 저장소처럼 Android를 ubuntu-latest, Windows를 windows-2025에서 짓는다.
- Rust 1.94.1은 이 PC에 Windows 대상만 깔려 있다. Android 대상은 HistoryCore 폴더에서 처음 `cargo`를 돌릴 때 `rust-toolchain.toml`을 보고 rustup이 받는다(안 받으면 `rustup target add aarch64-linux-android x86_64-linux-android --toolchain 1.94.1`).
- 앱 정체(옛 앱 위에 덮어 설치): Android `io.github.seugg.archive` · Windows 실행 파일 `WikiArchive.exe` · CompanyName `HistoryArchive` · ProductName `WikiArchive` · Inno AppId `{3AD6480F-263A-48A3-996F-B73B78A99DA3}`. 판 `0.0.1+13`.
- 엔진 다리: `hc_call(const char* 요청) -> char* 답`(UTF-8 JSON, 답은 실패해도 늘 `{"ok":false,"error":…}`) · `hc_free(char*)`. 라이브러리 파일 `history_core.dll` / `libhistory_core.so`. Dart는 전용 isolate에서 부른다.
- 주석 · 문서 · 화면 글은 한국어. 줄끝 LF(`.gitattributes`의 `* text=auto eol=lf`).
- **`.ps1`은 BOM 있는 UTF-8로 저장한다.** 이 PC에는 pwsh 7이 없고 Windows PowerShell 5.1은 BOM 없는 `.ps1`을 cp949로 읽어 한글 문자열을 깨뜨린다(따옴표가 먹혀 구문 오류가 나기도 한다). CI의 pwsh 7은 둘 다 읽는다. `.ps1`을 쓰거나 고친 뒤 그 저장소 뿌리에서:
  ```powershell
  Get-ChildItem -Recurse -Filter *.ps1 -Path tools | ForEach-Object { [IO.File]::WriteAllText($_.FullName, [IO.File]::ReadAllText($_.FullName), (New-Object Text.UTF8Encoding $true)) }
  ```
  (몇 번 돌려도 같다 — ReadAllText가 BOM을 알아보고 떼어 읽는다.)
- 커밋: `git -c user.name=Claude -c user.email=noreply@anthropic.com commit …`, 메시지 마지막 줄 `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. **GitHub 저장소 만들기와 밀기(push)는 감독 몫**(이 PC에 gh 없음) — Task 5에서 안내만 한다.
- 예산표(설계 9-2): 제목 찾기 p99 ≤ 10ms · 본문 찾기 첫 20건 p95 ≤ 10ms · 문서 열기 p95 ≤ 16ms · 저장 한 편(판 + 색인 + 그리기 모델) p95 ≤ 50ms · 분류 폴더 하나 펼치기 ≤ 16ms · 대문 · 통계 모음 ≤ 50ms · 앱 시작 → 대문 ≤ 1초 · 바뀐 내역 묶음 하나(문서 10편) 적용 ≤ 100ms · 새 기기에 통째 사본 들이기 PC ≤ 10분(폰은 보고) · 저장 공간은 비율 보고.
- 실행 중 네트워크를 쓰지 않는다(빌드 때 crates.io · pub.dev만).

## Review Focus

1. **한글 · 공백이 든 경로** — 감독이 포터블 ZIP을 `D:\내 프로그램\…`에 풀거나, 측정 작업 폴더에 한글이 있어도 엔진 라이브러리가 열리고 SQLite 파일이 만들어져야 한다. → Task 2(한글 폴더에서 DLL 열기) · Task 8(한글 작업 폴더) 검사.
2. **엔진 라이브러리가 없거나 열리지 않음** — 앱이 멈추거나 빈 화면이 되지 않고 까닭을 글로 보여 준다. → Task 2(`HistoryCoreLoadError`) · Task 3(상태 글) 검사.
3. **끊긴 측정을 다시 돌림** — 작업 폴더에 반쯤 찬 DB가 남아 있어도 처음부터 깨끗이 다시 들인다(수가 두 배가 되지 않는다). → Task 8 검사.
4. **여러 요청이 한꺼번에** — 여러 스레드 · 여러 Future가 동시에 엔진을 불러도 답이 섞이지 않는다. → Task 1(스레드 8개) · Task 2(Future 50개) 검사.
5. **4바이트 글자(확장 B 한자 𪄃 · 이모지)가 다리를 건넘** — 자전 자료에 실제로 있는 글자다. 깨지거나 잘리지 않아야 한다. → Task 1 · Task 2의 `echo` 검사.

---

## 파일 지도

```
D:\HistoryCore\
  Cargo.toml  Cargo.lock  rust-toolchain.toml  .gitignore  .gitattributes  README.md
  docs\superpowers\specs\2026-10-21-remake-ecosystem-design.md   (설계 사본)
  docs\superpowers\plans\2026-10-21-p0-p1-foundation-bench.md    (이 계획 사본)
  spec\README.md                         약속 문서 목록(P2부터 채움)
  crates\library\  src\lib.rs            서재 엔진 — P0은 판 번호만(ENGINE_VERSION · ABI_VERSION)
  crates\ffi\      src\lib.rs            C 다리 hc_call/hc_free · handle(요청) → 답
                   tests\call.rs
  crates\corpus\   src\{lib,rng,pool,tree,doc}.rs   더미 서재(정해진 씨앗 · 문서 i를 바로)
                   assets\{pool,words}.txt          康熙字典 번역층에서 뽑은 문장 · 낱말
                   tests\corpus.rs
  crates\bench\    src\{lib,library,stats,report,workload,run,naive,main}.rs
                   tests\{run,naive,cli}.rs
  packages\history_core\                 Flutter FFI 플러그인(미리 지은 엔진을 싣는다)
    pubspec.yaml  lib\history_core.dart  lib\src\engine.dart  test\engine_test.dart
    android\build.gradle(.kts)  windows\CMakeLists.txt
    native\                              (git 무시) build-native.ps1이 놓는 dll · so
  tools\build-native.ps1                 엔진 짓기(Windows dll · Android so)
  tools\bench-android.ps1                폰에서 hc-bench
  tools\make-pool.py                     corpus 재료 뽑기(한 번)
  bench\README.md  bench\results\*.{json,md}   기준 수치
  .github\workflows\ci.yml

D:\HistoryArchive\
  (flutter create로 만든 앱 — Android · Windows) pubspec.yaml  HISTORYCORE_REF
  lib\main.dart  lib\src\app.dart  test\app_test.dart  test\assets_test.dart
  assets\fonts\ (+ licenses\)            옛 저장소 packages/design_system/fonts
  design\brand\  tools\brand\            '펼친 기록' 심벌 · 아이콘 생성기(node_modules 빼고)
  data\kangxi\  tools\kangxi\  docs\kangxi\   康熙字典 자료 · DB 도구 · 문서
  tools\fonts\subset_hanamin.py
  tools\windows\{installer.iss,package.ps1,smoke-test.ps1,plugin-junction.ps1}
  docs\superpowers\specs\…  docs\superpowers\plans\…   (사본)
  .github\workflows\build.yml
```

---

## P0 — 바탕

### Task 1: HistoryCore 저장소 · Rust 작업공간 · 엔진 다리(`hc_call`)

**Files:**
- Create: `D:\HistoryCore\Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `.gitattributes`, `README.md`, `spec\README.md`
- Create: `crates\library\Cargo.toml`, `crates\library\src\lib.rs`
- Create: `crates\ffi\Cargo.toml`, `crates\ffi\src\lib.rs`, `crates\ffi\tests\call.rs`
- Create(사본): `docs\superpowers\specs\2026-10-21-remake-ecosystem-design.md`, `docs\superpowers\plans\2026-10-21-p0-p1-foundation-bench.md`

**Interfaces:**
- Produces: `history_library::ENGINE_VERSION: &str = "0.0.1"`, `history_library::ABI_VERSION: u32 = 1`; 상자 `history_core_ffi`(lib 이름 `history_core`)의 `pub fn handle(request: &str) -> serde_json::Value`, `#[no_mangle] unsafe extern "C" fn hc_call(*const c_char) -> *mut c_char`, `hc_free(*mut c_char)`. op: `version` → `{"ok":true,"engine":"0.0.1","abi":1}` · `echo` → `{"ok":true,"text":<받은 text>}` · 그 밖 → `{"ok":false,"error":"…"}`.

- [ ] **Step 1: 저장소와 작업공간을 만든다**

```powershell
New-Item -ItemType Directory -Force D:\HistoryCore | Out-Null
Set-Location D:\HistoryCore
git init -b main
New-Item -ItemType Directory -Force crates\library\src, crates\ffi\src, crates\ffi\tests, spec, docs\superpowers\specs, docs\superpowers\plans | Out-Null
Copy-Item 'D:\아카이브\HistoryArchive-remake-design\docs\superpowers\specs\2026-10-21-remake-ecosystem-design.md' docs\superpowers\specs\
Copy-Item 'D:\아카이브\HistoryArchive-remake-design\docs\superpowers\plans\2026-10-21-p0-p1-foundation-bench.md' docs\superpowers\plans\
```

`Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/library", "crates/ffi"]

[workspace.package]
version = "0.0.1"
edition = "2021"
publish = false

[profile.release]
lto = "thin"
codegen-units = 1
```

`rust-toolchain.toml`:
```toml
[toolchain]
channel = "1.94.1"
targets = ["x86_64-pc-windows-msvc", "aarch64-linux-android", "x86_64-linux-android"]
```

`.gitignore`:
```
/target
/packages/history_core/native/
/packages/history_core/.dart_tool/
/packages/history_core/build/
/bench/work/
```

`.gitattributes`:
```
* text=auto eol=lf
*.png binary
*.jpg binary
*.ico binary
*.ttf binary
*.otf binary
```

`README.md`:
```markdown
# HistoryCore

역사 앱 생태계(아카이브 · HistoryTime · 히스토리챗)가 함께 쓰는 셈과 그림. 설계: [docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md](docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md).

| 자리 | 하는 일 |
| --- | --- |
| `crates/library` | 서재 엔진 |
| `crates/ffi` | Flutter 다리 — C 함수 `hc_call`(UTF-8 JSON 요청 → 답) · `hc_free` |
| `crates/corpus` | 100만 건 측정용 더미 서재 |
| `crates/bench` | 측정 판 `hc-bench` |
| `packages/history_core` | Flutter FFI 플러그인(미리 지은 엔진을 앱에 싣는다) |
| `spec/` | 앱 사이의 약속 문서 |

    cargo test --locked --workspace
    ./tools/build-native.ps1 -Android          # 엔진을 packages/history_core/native에 놓는다
```

`spec\README.md`:
```markdown
# 약속 문서

설계 4-1의 `spec/`. P2부터 채운다: 서재 형식(`library-format.md`) · 드라이브 형식(`drive-format.md`) · 창구 질의(`windows-api.md`) · 나무마크 문법(`namumark.md`).

지금 정해진 것 — 엔진 다리: `hc_call(요청 JSON) → 답 JSON`. 답은 늘 `ok`를 가진다. `{"op":"version"}` → `{"ok":true,"engine":"0.0.1","abi":1}`.
```

`crates\library\Cargo.toml`:
```toml
[package]
name = "history_library"
version.workspace = true
edition.workspace = true
publish.workspace = true
```

`crates\ffi\Cargo.toml`:
```toml
[package]
name = "history_core_ffi"
version.workspace = true
edition.workspace = true
publish.workspace = true

[lib]
name = "history_core"
crate-type = ["cdylib", "rlib"]

[dependencies]
history_library = { path = "../library" }
serde_json = "1"
```

- [ ] **Step 2: 실패하는 검사를 쓴다**

`crates\library\src\lib.rs`(검사만 먼저):
```rust
//! 서재 엔진. P0에서는 판 번호만 낸다(설계 9-1 P0).

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_three_numbers() {
        assert_eq!(super::ENGINE_VERSION.split('.').count(), 3);
        assert_eq!(super::ABI_VERSION, 1);
    }
}
```

`crates\ffi\src\lib.rs`(빈 파일로 둔다):
```rust
//! Flutter 다리.
```

`crates\ffi\tests\call.rs`:
```rust
use history_core::{handle, hc_call, hc_free};
use serde_json::Value;
use std::ffi::{CStr, CString};

fn call(req: &str) -> Value {
    unsafe {
        let c = CString::new(req).unwrap();
        let out = hc_call(c.as_ptr());
        let v = serde_json::from_str(CStr::from_ptr(out).to_str().unwrap()).unwrap();
        hc_free(out);
        v
    }
}

#[test]
fn version_reports_engine_and_abi() {
    let v = call(r#"{"op":"version"}"#);
    assert_eq!(v["ok"], true);
    assert_eq!(v["engine"], "0.0.1");
    assert_eq!(v["abi"], 1);
}

#[test]
fn unknown_op_is_an_error_answer_not_a_crash() {
    let v = call(r#"{"op":"nope"}"#);
    assert_eq!(v["ok"], false);
    assert!(v["error"].as_str().unwrap().contains("nope"));
}

#[test]
fn broken_json_and_missing_op_are_error_answers() {
    assert_eq!(call("{op:")["ok"], false);
    assert_eq!(call(r#"{"text":"x"}"#)["ok"], false);
}

#[test]
fn null_request_is_an_error_answer() {
    unsafe {
        let out = hc_call(std::ptr::null());
        let v: Value = serde_json::from_str(CStr::from_ptr(out).to_str().unwrap()).unwrap();
        hc_free(out);
        assert_eq!(v["ok"], false);
    }
}

#[test]
fn four_byte_characters_cross_the_bridge_intact() {
    // Review Focus 5 — 확장 B 한자 · 이모지 · 이스케이프된 NUL
    let v = call(r#"{"op":"echo","text":"鶴 𪄃 😀 \u0000 끝"}"#);
    assert_eq!(v["ok"], true);
    assert_eq!(v["text"], "鶴 𪄃 😀 \u{0} 끝");
}

#[test]
fn many_threads_can_call_at_once() {
    // Review Focus 4
    let handles: Vec<_> = (0..8)
        .map(|t| {
            std::thread::spawn(move || {
                for i in 0..200 {
                    let text = format!("스레드{t}-{i}");
                    let v = call(&format!(r#"{{"op":"echo","text":"{text}"}}"#));
                    assert_eq!(v["text"], text);
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn handle_is_pure() {
    assert_eq!(handle(r#"{"op":"version"}"#), handle(r#"{"op":"version"}"#));
}
```

- [ ] **Step 3: 검사가 실패하는지 본다**

Run: `cargo test --workspace`
Expected: 컴파일 실패 — `cannot find value ENGINE_VERSION` · `unresolved imports history_core::handle, hc_call, hc_free`.

- [ ] **Step 4: 구현한다**

`crates\library\src\lib.rs`(통째로 — Step 2의 검사는 그대로 아래에 남는다):
```rust
//! 서재 엔진. P0에서는 판 번호만 낸다(설계 9-1 P0).

/// 엔진 판 — 작업공간 판과 같다.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// 다리 · 창구 약속의 판. 요청 · 답의 모양이 바뀌면 올린다.
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

`crates\ffi\src\lib.rs`:
```rust
//! Flutter 다리 — C 함수 둘(`hc_call` · `hc_free`). 요청과 답은 UTF-8 JSON이다.
//! 답은 실패해도 늘 `{"ok":false,"error":…}`이고 널을 돌려주지 않는다.
use serde_json::{json, Value};
use std::ffi::{c_char, CStr, CString};

/// 요청 하나를 처리한다. 상태를 갖지 않으므로 여러 스레드에서 동시에 불러도 된다.
pub fn handle(request: &str) -> Value {
    let req: Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": format!("요청 JSON을 읽지 못함: {e}")}),
    };
    match req.get("op").and_then(Value::as_str) {
        Some("version") => json!({
            "ok": true,
            "engine": history_library::ENGINE_VERSION,
            "abi": history_library::ABI_VERSION,
        }),
        // 다리 점검용 — 받은 글을 그대로 돌려준다(4바이트 글자 · 이스케이프 확인).
        Some("echo") => json!({"ok": true, "text": req.get("text").cloned().unwrap_or(Value::Null)}),
        Some(op) => json!({"ok": false, "error": format!("모르는 op: {op}")}),
        None => json!({"ok": false, "error": "op가 없음"}),
    }
}

fn to_c(value: Value) -> *mut c_char {
    // JSON 직렬화는 문자열 속 NUL을 \u0000으로 적으므로 결과에 NUL 바이트가 없다.
    CString::new(value.to_string()).expect("JSON 글에는 NUL 바이트가 없다").into_raw()
}

/// # Safety
/// `request`는 NUL로 끝나는 UTF-8 글이거나 널이다. 돌려준 포인터는 반드시 `hc_free`로 한 번 놓는다.
#[no_mangle]
pub unsafe extern "C" fn hc_call(request: *const c_char) -> *mut c_char {
    if request.is_null() {
        return to_c(json!({"ok": false, "error": "요청이 널"}));
    }
    let text = match CStr::from_ptr(request).to_str() {
        Ok(t) => t,
        Err(_) => return to_c(json!({"ok": false, "error": "요청이 UTF-8이 아님"})),
    };
    let answer = std::panic::catch_unwind(|| handle(text))
        .unwrap_or_else(|_| json!({"ok": false, "error": "엔진 안에서 멈춤(panic)"}));
    to_c(answer)
}

/// # Safety
/// `value`는 `hc_call`이 돌려준 포인터이거나 널이다. 두 번 놓지 않는다.
#[no_mangle]
pub unsafe extern "C" fn hc_free(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}
```

- [ ] **Step 5: 검사가 통과하는지 본다**

Run: `cargo test --workspace`
Expected: `history_library` 1 passed · `call.rs` 7 passed. `Cargo.lock`이 생긴다.

- [ ] **Step 6: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "HistoryCore 시작 — Rust 작업공간 · 엔진 판 번호 · C 다리 hc_call/hc_free" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: 엔진 짓기 스크립트 · Flutter FFI 플러그인 `history_core`

**Files:**
- Create: `D:\HistoryCore\tools\build-native.ps1`
- Create(flutter create 뒤 고침): `packages\history_core\pubspec.yaml`, `lib\history_core.dart`, `lib\src\engine.dart`, `test\engine_test.dart`, `android\build.gradle`(또는 `.kts`), `windows\CMakeLists.txt`, `analysis_options.yaml`
- Delete(생성물): `packages\history_core\src\`, `example\`, `ffigen.yaml`, `lib\history_core_bindings_generated.dart`

**Interfaces:**
- Consumes: Task 1의 `hc_call` · `hc_free`, 파일 이름 `history_core.dll` · `libhistory_core.so`.
- Produces(Dart, `package:history_core/history_core.dart`):
  - `String defaultLibraryPath()` — 환경 변수 `HISTORY_CORE_LIBRARY` → Android `libhistory_core.so` → Windows `history_core.dll`
  - `class HistoryCore { static Future<HistoryCore> start({String? libraryPath}); Future<Map<String, dynamic>> call(Map<String, Object?> request); Future<EngineVersion> version(); void close(); }`
  - `class EngineVersion { final String engine; final int abi; }`
  - `class HistoryCoreError implements Exception { final String message; }` · `class HistoryCoreLoadError extends HistoryCoreError`
- Produces(파일): `packages\history_core\native\windows-x64\history_core.dll`, `packages\history_core\native\android\<abi>\libhistory_core.so`

- [ ] **Step 1: 엔진 짓기 스크립트를 쓴다**

`tools\build-native.ps1`(Windows PowerShell 5.1 · pwsh 7 · Linux pwsh 모두에서 돈다 — CI의 Android 작업은 ubuntu):
```powershell
# HistoryCore 엔진을 짓고 Flutter 플러그인 자리(packages/history_core/native)에 놓는다.
#   ./tools/build-native.ps1                                Windows x64
#   ./tools/build-native.ps1 -Android                       + Android arm64-v8a
#   ./tools/build-native.ps1 -Android -Abi arm64-v8a,x86_64 + 에뮬레이터용
#   ./tools/build-native.ps1 -Android -SkipWindows          Android만(Linux에서는 늘 이렇게 돈다)
param(
  [switch]$Android,
  [ValidateSet('arm64-v8a', 'x86_64')][string[]]$Abi = @('arm64-v8a'),
  [switch]$SkipWindows
)
$ErrorActionPreference = 'Stop'
$onWindows = $env:OS -eq 'Windows_NT'
$repo = Split-Path $PSScriptRoot -Parent
$manifest = Join-Path $repo 'Cargo.toml'
$native = Join-Path (Join-Path $repo 'packages') (Join-Path 'history_core' 'native')
$release = Join-Path (Join-Path $repo 'target') 'release'
# rustup은 지금 폴더에서 rust-toolchain.toml을 찾는다 — 어디서 불러도 1.94.1로 짓도록 저장소 안에서 돈다.
Push-Location $repo
try {

if ($onWindows -and -not $SkipWindows) {
  & cargo build --locked --release -p history_core_ffi --manifest-path $manifest
  if ($LASTEXITCODE -ne 0) { throw 'Windows 엔진 빌드 실패' }
  $dest = Join-Path $native 'windows-x64'
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  Copy-Item -LiteralPath (Join-Path $release 'history_core.dll') -Destination $dest -Force
  Write-Output "Windows: $(Join-Path $dest 'history_core.dll')"
}

if ($Android) {
  $sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } elseif ($onWindows) { Join-Path $env:LOCALAPPDATA 'Android\Sdk' } else { throw 'ANDROID_HOME이 필요합니다' }
  $hostTag = if ($onWindows) { 'windows-x86_64' } else { 'linux-x86_64' }
  $ndk = [IO.Path]::Combine($sdk, 'ndk', '28.2.13676358', 'toolchains', 'llvm', 'prebuilt', $hostTag, 'bin')
  if (-not (Test-Path -LiteralPath $ndk)) { throw "Android NDK 28.2.13676358이 필요합니다: $ndk" }
  $exe = if ($onWindows) { '.cmd' } else { '' }
  $arExe = if ($onWindows) { 'llvm-ar.exe' } else { 'llvm-ar' }
  foreach ($a in $Abi) {
    $target = if ($a -eq 'arm64-v8a') { 'aarch64-linux-android' } else { 'x86_64-linux-android' }
    $key = $target.Replace('-', '_')
    $linker = Join-Path $ndk "$($target)24-clang$exe"
    [Environment]::SetEnvironmentVariable("CC_$key", $linker, 'Process')
    [Environment]::SetEnvironmentVariable("AR_$key", (Join-Path $ndk $arExe), 'Process')
    [Environment]::SetEnvironmentVariable("CARGO_TARGET_$($key.ToUpper())_LINKER", $linker, 'Process')
    & cargo build --locked --release -p history_core_ffi --target $target --manifest-path $manifest
    if ($LASTEXITCODE -ne 0) { throw "Android($a) 엔진 빌드 실패" }
    $dest = Join-Path (Join-Path $native 'android') $a
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Copy-Item -LiteralPath ([IO.Path]::Combine($repo, 'target', $target, 'release', 'libhistory_core.so')) -Destination $dest -Force
    Write-Output "Android: $(Join-Path $dest 'libhistory_core.so')"
  }
}

} finally {
  Pop-Location
}
```

BOM을 붙이고(Global Constraints의 한 줄) 돌린다.

Run: `./tools/build-native.ps1 -Android`
Expected: `Windows: …\history_core.dll` · `Android: …\arm64-v8a\libhistory_core.so` 두 줄.

- [ ] **Step 2: 플러그인 뼈대를 만들고 C 견본을 걷어 낸다**

```powershell
$flutter = 'D:\flutter-sdk\flutter\bin\flutter.bat'
Set-Location D:\HistoryCore
& $flutter create --template=plugin_ffi --platforms=android,windows --org io.github.seugg --project-name history_core packages\history_core
Set-Location packages\history_core
Remove-Item -Recurse -Force src, example
Remove-Item -Force ffigen.yaml, lib\history_core_bindings_generated.dart
```

`pubspec.yaml`(통째로 바꾼다):
```yaml
name: history_core
description: HistoryCore 엔진의 Flutter 다리 — 미리 지은 엔진을 싣고 전용 isolate에서 부른다.
version: 0.0.1
publish_to: none

environment:
  sdk: ^3.13.0
  flutter: ">=3.47.0"

dependencies:
  ffi: ^2.2.0
  flutter:
    sdk: flutter

dev_dependencies:
  flutter_test:
    sdk: flutter
  lints: ^6.1.0

flutter:
  plugin:
    platforms:
      android:
        ffiPlugin: true
      windows:
        ffiPlugin: true
```

`analysis_options.yaml`:
```yaml
include: package:lints/recommended.yaml
```

`windows\CMakeLists.txt`(통째로 바꾼다):
```cmake
cmake_minimum_required(VERSION 3.14)
set(PROJECT_NAME "history_core")
project(${PROJECT_NAME} LANGUAGES CXX)

# 엔진 DLL은 tools/build-native.ps1이 미리 지어 둔다(Rust). 여기서는 앱 묶음에 넣기만 한다.
set(history_core_bundled_libraries
  "${CMAKE_CURRENT_SOURCE_DIR}/../native/windows-x64/history_core.dll"
  PARENT_SCOPE
)
```

`android\build.gradle`(Groovy로 생성됐다면): `android { … }` 안의 `externalNativeBuild { cmake { … } }` 묶음을 통째로 지우고, `namespace = …` 줄 바로 아래에 넣는다:
```groovy
    // 엔진 .so는 tools/build-native.ps1이 미리 지어 둔다(Rust). 앱 APK의 lib/<abi>/에 실린다.
    sourceSets {
        main {
            jniLibs.srcDirs = ["../native/android"]
        }
    }
```
`android\build.gradle.kts`(Kotlin DSL로 생성됐다면) 같은 자리에:
```kotlin
    // 엔진 .so는 tools/build-native.ps1이 미리 지어 둔다(Rust). 앱 APK의 lib/<abi>/에 실린다.
    sourceSets["main"].jniLibs.srcDirs("../native/android")
```
`ndkVersion` 줄이 있으면 그대로 둔다.

- [ ] **Step 3: 실패하는 검사를 쓴다**

`lib\history_core.dart`:
```dart
/// HistoryCore 엔진 다리.
library;

export 'src/engine.dart';
```

`lib\src\engine.dart`(빈 파일로 둔다):
```dart
// 엔진 다리 — Step 5에서 채운다.
```

`test\engine_test.dart`:
```dart
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:history_core/history_core.dart';

/// `tools/build-native.ps1`이 놓은 DLL(또는 HISTORY_CORE_LIBRARY).
String builtLibrary() =>
    Platform.environment['HISTORY_CORE_LIBRARY'] ??
    '${Directory.current.path}${Platform.pathSeparator}native${Platform.pathSeparator}windows-x64${Platform.pathSeparator}history_core.dll';

void main() {
  final windowsOnly = Platform.isWindows ? false : 'Windows에서만 — 미리 지은 DLL로 돈다';

  test('엔진 판 번호를 isolate 너머로 받는다', () async {
    final core = await HistoryCore.start(libraryPath: builtLibrary());
    addTearDown(core.close);
    final v = await core.version();
    expect(v.engine, '0.0.1');
    expect(v.abi, 1);
  }, skip: windowsOnly);

  test('모르는 op는 오류 답으로 온다', () async {
    final core = await HistoryCore.start(libraryPath: builtLibrary());
    addTearDown(core.close);
    final r = await core.call({'op': 'nope'});
    expect(r['ok'], false);
    expect(r['error'], contains('nope'));
  }, skip: windowsOnly);

  test('4바이트 글자가 다리를 그대로 건넌다', () async {
    // Review Focus 5
    final core = await HistoryCore.start(libraryPath: builtLibrary());
    addTearDown(core.close);
    final r = await core.call({'op': 'echo', 'text': '鶴 𪄃 😀 끝'});
    expect(r['text'], '鶴 𪄃 😀 끝');
  }, skip: windowsOnly);

  test('여러 요청을 한꺼번에 보내도 각자 제 답을 받는다', () async {
    // Review Focus 4
    final core = await HistoryCore.start(libraryPath: builtLibrary());
    addTearDown(core.close);
    final answers = await Future.wait([
      for (var i = 0; i < 50; i++) core.call({'op': 'echo', 'text': '요청$i'}),
    ]);
    for (var i = 0; i < 50; i++) {
      expect(answers[i]['text'], '요청$i');
    }
  }, skip: windowsOnly);

  test('한글 · 공백이 든 폴더의 DLL도 연다', () async {
    // Review Focus 1
    final dir = Directory.systemTemp.createTempSync('엔진 경로 시험 ');
    addTearDown(() {
      try {
        dir.deleteSync(recursive: true);
      } on FileSystemException {
        // 한 번 불러온 DLL은 시험 프로세스가 끝날 때까지 잠겨 있다 — 임시 폴더라 남겨도 된다.
      }
    });
    final copy = File(builtLibrary()).copySync('${dir.path}${Platform.pathSeparator}history_core.dll');
    final core = await HistoryCore.start(libraryPath: copy.path);
    final v = await core.version();
    core.close();
    expect(v.engine, '0.0.1');
  }, skip: windowsOnly);

  test('라이브러리가 없으면 HistoryCoreLoadError — 멈추지 않는다', () async {
    // Review Focus 2
    await expectLater(
      HistoryCore.start(libraryPath: 'no_such_history_core_library.dll'),
      throwsA(isA<HistoryCoreLoadError>()),
    );
  });
}
```

- [ ] **Step 4: 검사가 실패하는지 본다**

Run(`packages\history_core`에서): `& $flutter test`
Expected: 컴파일 실패 — `Undefined name 'HistoryCore'` · `HistoryCoreLoadError`.

- [ ] **Step 5: 다리를 구현한다**

`lib\src\engine.dart`:
```dart
import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';

/// 엔진 라이브러리를 찾는 곳. 환경 변수 `HISTORY_CORE_LIBRARY`가 먼저다(시험 · 측정용).
/// Android는 APK의 jniLibs에서, Windows는 실행 파일 옆에서 이름으로 찾는다.
String defaultLibraryPath() {
  final env = Platform.environment['HISTORY_CORE_LIBRARY'];
  if (env != null && env.isNotEmpty) return env;
  if (Platform.isAndroid) return 'libhistory_core.so';
  if (Platform.isWindows) return 'history_core.dll';
  throw UnsupportedError('HistoryCore는 Android · Windows만 지원한다: ${Platform.operatingSystem}');
}

typedef _CallNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _FreeNative = Void Function(Pointer<Utf8>);
typedef _FreeDart = void Function(Pointer<Utf8>);

/// 엔진 판 번호.
class EngineVersion {
  const EngineVersion(this.engine, this.abi);
  final String engine;
  final int abi;
}

/// 엔진이 돌려준 오류.
class HistoryCoreError implements Exception {
  HistoryCoreError(this.message);
  final String message;
  @override
  String toString() => 'HistoryCoreError: $message';
}

/// 엔진 라이브러리를 열지 못함(파일 없음 · 깨짐 · 다른 아키텍처).
class HistoryCoreLoadError extends HistoryCoreError {
  HistoryCoreLoadError(super.message);
}

/// 엔진을 따로 도는 isolate 하나에서 부른다. 메인 isolate는 답을 기다리기만 한다.
class HistoryCore {
  HistoryCore._(this._isolate, this._requests, this._replies);

  final Isolate _isolate;
  final SendPort _requests;
  final ReceivePort _replies;
  final _pending = <int, Completer<Map<String, dynamic>>>{};
  var _next = 0;

  /// [libraryPath]가 없으면 [defaultLibraryPath]. 열지 못하면 [HistoryCoreLoadError].
  static Future<HistoryCore> start({String? libraryPath}) async {
    final replies = ReceivePort();
    final first = Completer<Object?>();
    HistoryCore? core;
    replies.listen((msg) {
      if (!first.isCompleted) {
        first.complete(msg);
      } else {
        core?._onReply(msg);
      }
    });
    final isolate = await Isolate.spawn(_main, <Object?>[replies.sendPort, libraryPath ?? defaultLibraryPath()]);
    final hello = await first.future;
    if (hello is! SendPort) {
      isolate.kill(priority: Isolate.immediate);
      replies.close();
      throw HistoryCoreLoadError(hello.toString());
    }
    return core = HistoryCore._(isolate, hello, replies);
  }

  /// 요청 하나. 답은 늘 `ok`를 가진다.
  Future<Map<String, dynamic>> call(Map<String, Object?> request) {
    final id = _next++;
    final done = Completer<Map<String, dynamic>>();
    _pending[id] = done;
    _requests.send(<Object?>[id, jsonEncode(request)]);
    return done.future;
  }

  Future<EngineVersion> version() async {
    final r = await call({'op': 'version'});
    if (r['ok'] != true) throw HistoryCoreError(r['error'] as String? ?? '알 수 없는 오류');
    return EngineVersion(r['engine'] as String, r['abi'] as int);
  }

  void close() {
    _isolate.kill(priority: Isolate.immediate);
    _replies.close();
    for (final c in _pending.values) {
      c.completeError(StateError('엔진이 닫힘'));
    }
    _pending.clear();
  }

  void _onReply(Object? msg) {
    final list = msg as List<Object?>;
    final done = _pending.remove(list[0] as int);
    done?.complete(Map<String, dynamic>.from(jsonDecode(list[1] as String) as Map));
  }

  static void _main(List<Object?> boot) {
    final parent = boot[0] as SendPort;
    // 열기 · 함수 찾기 어느 쪽이 실패해도 부모에게 까닭을 보낸다(그러지 않으면 start가 영영 기다린다).
    final _CallNative call;
    final _FreeDart free;
    try {
      final lib = DynamicLibrary.open(boot[1] as String);
      call = lib.lookupFunction<_CallNative, _CallNative>('hc_call');
      free = lib.lookupFunction<_FreeNative, _FreeDart>('hc_free');
    } catch (e) {
      parent.send('엔진 라이브러리를 열지 못함(${boot[1]}): $e');
      return;
    }
    final requests = ReceivePort();
    parent.send(requests.sendPort);
    requests.listen((msg) {
      final list = msg as List<Object?>;
      final input = (list[1] as String).toNativeUtf8();
      try {
        final out = call(input);
        try {
          parent.send(<Object?>[list[0], out.toDartString()]);
        } finally {
          free(out);
        }
      } finally {
        malloc.free(input);
      }
    });
  }
}
```

- [ ] **Step 6: 검사가 통과하는지 본다**

Run(`packages\history_core`에서): `& $flutter pub get; & $flutter analyze; & $flutter test`
Expected: analyze `No issues found!` · 6 tests passed.

- [ ] **Step 7: 커밋한다**

```powershell
Set-Location D:\HistoryCore
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "엔진 짓기 스크립트와 Flutter FFI 플러그인 history_core — 전용 isolate에서 hc_call" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: HistoryArchive 앱 뼈대 — 엔진 판 번호 화면 · 앱 정체

**Files:**
- Create(flutter create 뒤 고침): `D:\HistoryArchive\` 전체, `pubspec.yaml`, `analysis_options.yaml`, `HISTORYCORE_REF`, `lib\main.dart`, `lib\src\app.dart`, `test\app_test.dart`
- Modify: `android\app\build.gradle.kts`, `android\app\src\main\AndroidManifest.xml`, `android\app\src\main\kotlin\…\MainActivity.kt`(자리 옮김), `windows\CMakeLists.txt`, `windows\runner\Runner.rc`, `windows\runner\main.cpp`
- Create: `tools\windows\plugin-junction.ps1`, `tools\windows\smoke-test.ps1`
- Create(사본): `docs\superpowers\specs\…`, `docs\superpowers\plans\…`

**Interfaces:**
- Consumes: Task 2의 `HistoryCore.start` · `version()` · `HistoryCoreError` · `HistoryCoreLoadError`.
- Produces: `Future<String> engineStatus({Future<HistoryCore> Function()? start})` (`package:history_archive/main.dart`) — 성공 `'엔진 0.0.1 · ABI 1'`, 실패 `'엔진을 불러오지 못했습니다 — <까닭>'`; `class ArchiveApp extends StatelessWidget { const ArchiveApp({super.key, required Future<String> status}); }` (`package:history_archive/src/app.dart`).

- [ ] **Step 1: 앱을 만든다**

```powershell
$flutter = 'D:\flutter-sdk\flutter\bin\flutter.bat'
Set-Location D:\
& $flutter create --org io.github.seugg --project-name history_archive --platforms android,windows HistoryArchive
Set-Location D:\HistoryArchive
git init -b main
New-Item -ItemType Directory -Force docs\superpowers\specs, docs\superpowers\plans, tools\windows | Out-Null
Copy-Item 'D:\HistoryCore\docs\superpowers\specs\2026-10-21-remake-ecosystem-design.md' docs\superpowers\specs\
Copy-Item 'D:\HistoryCore\docs\superpowers\plans\2026-10-21-p0-p1-foundation-bench.md' docs\superpowers\plans\
[IO.File]::WriteAllText((Join-Path (Get-Location) 'HISTORYCORE_REF'), 'main')
```
`HISTORYCORE_REF`는 CI가 받을 HistoryCore의 가지 · 태그 · 커밋이다. 첫 판을 낼 때 태그로 못박는다.

`.gitattributes`(새로):
```
* text=auto eol=lf
*.png binary
*.jpg binary
*.ico binary
*.ttf binary
*.otf binary
*.jar binary
```

- [ ] **Step 2: 앱 정체를 옛 앱과 맞춘다(덮어 설치)**

`android\app\build.gradle.kts`에서 `namespace = "io.github.seugg.history_archive"` → `namespace = "io.github.seugg.archive"`, `applicationId = "io.github.seugg.history_archive"` → `applicationId = "io.github.seugg.archive"`. 같은 파일 `defaultConfig { … }` 끝에 옛 앱의 ABI 거르기를 더하고, `buildTypes` 앞에 서명 설정을 넣는다(옛 `app/android/app/build.gradle.kts` 그대로):
```kotlin
        // 테스트용 x86_64 엔진이나 다른 플러그인의 ABI가 arm64 배포본에 섞이지 않게 한다.
        val platforms = (project.findProperty("target-platform") as? String ?: "android-arm64").split(",")
        val abis = mapOf("android-arm" to "armeabi-v7a", "android-arm64" to "arm64-v8a", "android-x64" to "x86_64")
        ndk {
            abiFilters.clear()
            abiFilters.addAll(platforms.map { abis[it] ?: error("지원하지 않는 Android 대상: $it") })
        }
```
```kotlin
    // 고정 서명키는 CI 비밀값으로만 받는다. 없으면 빌드 기기의 임시 디버그 키로 서명한다.
    val keystorePath = System.getenv("ARCHIVE_KEYSTORE_PATH")
    signingConfigs {
        if (keystorePath != null) {
            create("persistent") {
                storeFile = file(keystorePath)
                storePassword = System.getenv("ARCHIVE_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ARCHIVE_KEY_ALIAS")
                keyPassword = System.getenv("ARCHIVE_KEY_PASSWORD")
            }
        }
    }
```
그리고 `buildTypes { release { signingConfig = … } }`를 다음으로 바꾼다:
```kotlin
    buildTypes {
        release {
            signingConfig = if (keystorePath != null) signingConfigs.getByName("persistent") else signingConfigs.getByName("debug")
        }
    }
```

`MainActivity.kt`를 옮긴다:
```powershell
$kotlin = Join-Path (Get-Location) 'android\app\src\main\kotlin\io\github\seugg'
New-Item -ItemType Directory -Force (Join-Path $kotlin 'archive') | Out-Null
# Windows PowerShell 5.1의 Set-Content는 BOM 없는 UTF-8을 못 쓴다 — .NET으로 쓴다(WriteAllText 기본값이 BOM 없는 UTF-8).
$text = [IO.File]::ReadAllText((Join-Path $kotlin 'history_archive\MainActivity.kt')).Replace('package io.github.seugg.history_archive', 'package io.github.seugg.archive')
[IO.File]::WriteAllText((Join-Path $kotlin 'archive\MainActivity.kt'), $text)
Remove-Item -Recurse -Force (Join-Path $kotlin 'history_archive')
```
`android\app\src\main\AndroidManifest.xml`의 `android:label="history_archive"` → `android:label="아카이브"`.

`windows\CMakeLists.txt`의 `set(BINARY_NAME "history_archive")` → `set(BINARY_NAME "WikiArchive")`.

`windows\runner\Runner.rc`의 `VALUE` 다섯을 바꾼다: `"CompanyName", "HistoryArchive"` · `"FileDescription", "WikiArchive"` · `"InternalName", "WikiArchive"` · `"OriginalFilename", "WikiArchive.exe"` · `"ProductName", "WikiArchive"`(옛 `tools/windows/package.ps1`이 이 둘 — CompanyName · ProductName — 으로 사용자 서재 자리를 지킨다).

`windows\runner\main.cpp`의 `window.Create(L"history_archive", origin, size)` → `window.Create(L"WikiArchive", origin, size)`(한글 창 제목은 P3에서 채널로).

- [ ] **Step 3: 의존성을 잇는다**

`pubspec.yaml`(통째로 바꾼다):
```yaml
name: history_archive
description: 개인 역사 서재 — 위키 · 자전 · 동기화
publish_to: none
version: 0.0.1+13

environment:
  sdk: ^3.13.0

dependencies:
  flutter:
    sdk: flutter
  history_core:
    path: ../HistoryCore/packages/history_core

dev_dependencies:
  flutter_test:
    sdk: flutter
  lints: ^6.1.0

flutter:
  uses-material-design: false
```
`analysis_options.yaml`:
```yaml
include: package:lints/recommended.yaml
```

- [ ] **Step 4: 실패하는 검사를 쓴다**

`test\app_test.dart`(생성된 `widget_test.dart`는 지운다):
```dart
import 'dart:async';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:history_archive/main.dart';
import 'package:history_archive/src/app.dart';
import 'package:history_core/history_core.dart';

void main() {
  testWidgets('엔진 상태 한 줄을 보여 준다', (tester) async {
    await tester.pumpWidget(ArchiveApp(status: Future.value('엔진 0.0.1 · ABI 1')));
    await tester.pump();
    expect(find.text('엔진 0.0.1 · ABI 1'), findsOneWidget);
  });

  testWidgets('기다리는 동안에도 빈 화면이 아니다', (tester) async {
    await tester.pumpWidget(ArchiveApp(status: Completer<String>().future));
    expect(find.text('엔진을 여는 중…'), findsOneWidget);
  });

  test('엔진을 못 열면 까닭을 글로 돌려준다 — 앱은 멈추지 않는다', () async {
    // Review Focus 2
    final s = await engineStatus(start: () => Future.error(HistoryCoreLoadError('파일 없음')));
    expect(s, '엔진을 불러오지 못했습니다 — 파일 없음');
  });

  test('실제 엔진을 연다(Windows)', () async {
    final lib = '${Directory.current.parent.path}\\HistoryCore\\packages\\history_core\\native\\windows-x64\\history_core.dll';
    final s = await engineStatus(start: () => HistoryCore.start(libraryPath: lib));
    expect(s, '엔진 0.0.1 · ABI 1');
  }, skip: Platform.isWindows ? false : 'Windows에서만');
}
```
`lib\src\app.dart`·`lib\main.dart`는 아직 생성물 그대로 둔다.

- [ ] **Step 5: 검사가 실패하는지 본다**

Run: `& $flutter pub get; & $flutter test`
Expected: 실패 — `Undefined name 'engineStatus'` · `src/app.dart` 없음. (`flutter pub get`이 "requires symlink support"로 멈추면 Step 9를 먼저 한다.)

- [ ] **Step 6: 구현한다**

`lib\src\app.dart`:
```dart
import 'package:flutter/widgets.dart';

/// P0 화면: 돌빛 바탕 가운데 엔진 상태 한 줄. 화면은 P3에서 짓는다(설계 3절).
class ArchiveApp extends StatelessWidget {
  const ArchiveApp({super.key, required this.status});

  final Future<String> status;

  @override
  Widget build(BuildContext context) {
    return WidgetsApp(
      title: 'WikiArchive',
      color: const Color(0xFF0A0A0A),
      debugShowCheckedModeBanner: false,
      builder: (context, _) => ColoredBox(
        color: const Color(0xFFF4F3F1),
        child: Center(
          child: FutureBuilder<String>(
            future: status,
            builder: (context, snap) => Text(
              snap.data ?? '엔진을 여는 중…',
              textDirection: TextDirection.ltr,
              style: const TextStyle(fontSize: 16, color: Color(0xFF0A0A0A)),
            ),
          ),
        ),
      ),
    );
  }
}
```

`lib\main.dart`(통째로 바꾼다):
```dart
import 'package:flutter/widgets.dart';
import 'package:history_core/history_core.dart';

import 'src/app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(ArchiveApp(status: engineStatus()));
}

/// 엔진을 한 번 열어 판 번호를 읽고 닫는다. 실패하면 그 까닭을 글로 돌려준다(앱은 멈추지 않는다).
Future<String> engineStatus({Future<HistoryCore> Function()? start}) async {
  try {
    final core = await (start ?? HistoryCore.start)();
    try {
      final v = await core.version();
      return '엔진 ${v.engine} · ABI ${v.abi}';
    } finally {
      core.close();
    }
  } on HistoryCoreError catch (e) {
    return '엔진을 불러오지 못했습니다 — ${e.message}';
  }
}
```

- [ ] **Step 7: 검사가 통과하는지 본다**

Run: `& $flutter analyze; & $flutter test`
Expected: `No issues found!` · 4 tests passed.

- [ ] **Step 8: Windows · Android로 지어 본다**

`tools\windows\smoke-test.ps1`:
```powershell
# 앱을 켜서 창이 뜨고 몇 초 버티는지 본다(P0 연기 시험). 묶음에 엔진 DLL이 있는지도 본다.
param([Parameter(Mandatory)][string]$Exe, [int]$Seconds = 5)
$ErrorActionPreference = 'Stop'
$bundle = Split-Path -Parent (Resolve-Path -LiteralPath $Exe)
if (-not (Test-Path -LiteralPath (Join-Path $bundle 'history_core.dll'))) { throw "묶음에 history_core.dll이 없습니다: $bundle" }
$p = Start-Process -FilePath $Exe -PassThru
try {
  $deadline = (Get-Date).AddSeconds(20)
  while ((Get-Date) -lt $deadline -and -not $p.HasExited -and $p.MainWindowHandle -eq 0) { Start-Sleep -Milliseconds 250; $p.Refresh() }
  if ($p.HasExited) { throw "앱이 바로 꺼졌습니다(종료 코드 $($p.ExitCode))" }
  if ($p.MainWindowHandle -eq 0) { throw '창이 20초 안에 뜨지 않았습니다' }
  Start-Sleep -Seconds $Seconds
  $p.Refresh()
  if ($p.HasExited) { throw "앱이 $($Seconds)초 안에 꺼졌습니다" }
  Write-Output "연기 시험 통과: '$($p.MainWindowTitle)'"
} finally {
  if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
}
```

BOM을 붙인 뒤(Global Constraints):
```powershell
$env:CL = '/utf-8'
& $flutter build windows --release
./tools/windows/smoke-test.ps1 -Exe build\windows\x64\runner\Release\WikiArchive.exe
& $flutter build apk --release --target-platform android-arm64 --android-project-arg=kotlin.incremental=false
Add-Type -AssemblyName System.IO.Compression.FileSystem
$apk = [IO.Compression.ZipFile]::OpenRead((Resolve-Path build\app\outputs\flutter-apk\app-release.apk))
try { if (-not ($apk.Entries.FullName -contains 'lib/arm64-v8a/libhistory_core.so')) { throw 'APK에 엔진이 없습니다' } } finally { $apk.Dispose() }
Write-Output 'APK에 lib/arm64-v8a/libhistory_core.so 있음'
```
Expected: `연기 시험 통과: 'WikiArchive'` · `APK에 lib/arm64-v8a/libhistory_core.so 있음`. 창 가운데에 `엔진 0.0.1 · ABI 1`이 보인다 — 연기 시험은 글까지 보지 않으므로, 실행자가 화면을 볼 수 있으면 한 번 열어 확인하고, 볼 수 없으면 보고에 "창 글은 감독 확인 남음"이라 적는다.

- [ ] **Step 9: (개발자 모드를 켜지 못했을 때만) 플러그인 정션**

`tools\windows\plugin-junction.ps1`:
```powershell
# 개발자 모드 없이 Windows 빌드를 하려고, Flutter가 만들 플러그인 symlink 자리에 디렉터리 정션을 미리 만든다.
# 관리자 권한이 필요 없고 시스템 설정을 바꾸지 않는다. 플러그인 목록이 바뀌면 다시 돌린다.
$ErrorActionPreference = 'Stop'
$repo = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$links = Join-Path $repo 'windows\flutter\ephemeral\.plugin_symlinks'
New-Item -ItemType Directory -Force -Path $links | Out-Null
$target = Resolve-Path (Join-Path $repo '..\HistoryCore\packages\history_core')
$link = Join-Path $links 'history_core'
if (Test-Path -LiteralPath $link) { cmd /c rmdir "$link" | Out-Null }  # 정션만 지운다(/s 없이 — 대상은 건드리지 않는다)
cmd /c mklink /J "$link" "$target" | Out-Null
Write-Output "정션: $link → $target"
```
BOM을 붙이고(Global Constraints) `./tools/windows/plugin-junction.ps1`을 돌린 뒤 Step 5부터 다시. (Flutter는 플러그인 목록이 바뀐 첫 실행에서만 이 폴더를 지우고 새로 만들고, 그다음부터는 이미 있는 링크를 그대로 쓴다. 정션을 만든 뒤에도 `pub get`이 또 symlink 오류로 멈추면 멈추고 감독에게 개발자 모드를 부탁한다.)

- [ ] **Step 10: 커밋한다**

`.gitignore`에 생성물 외에 `/out/`을 더한 뒤:
```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "HistoryArchive 0.0.1 시작 — 엔진 판 번호 화면 · 옛 앱과 같은 정체(덮어 설치)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: 옛 저장소에서 옮기기 — 글꼴 · 심벌 · 아이콘 · 康熙字典 · Windows 설치 도구

**Files:**
- Create(복사): `D:\HistoryArchive\assets\fonts\*`, `assets\fonts\licenses\*`, `tools\fonts\subset_hanamin.py`, `design\brand\*`, `tools\brand\*`(node_modules 빼고), `android\app\src\main\res\mipmap-*\*`(적응형 아이콘 포함), `android\app\src\main\res\values\colors.xml`(아이콘 배경색), `windows\runner\resources\app_icon.ico`, `data\kangxi\**`, `tools\kangxi\**`, `docs\kangxi\{KANGXI_SOURCE,KANGXI_TRANSLATION,KANGXI_TRANSLATION_NOTES}.md`, `tools\windows\installer.iss`, `tools\windows\package.ps1`
- Modify: `pubspec.yaml`(글꼴), `lib\src\app.dart`(프리텐다드), `.gitattributes`(康熙字典 줄), `tools\windows\installer.iss`(경로 · URL), `tools\windows\package.ps1`(SQLite 대신 엔진 DLL)
- Test: `test\assets_test.dart`

**Interfaces:**
- Consumes: Task 3의 앱.
- Produces: 글꼴 가족 `Pretendard`(400 · 600 · 700) · `NotoSerifKR`(400 · 700) · `D2Coding` · `HanaMinA` · `HanaMinB`; 康熙字典 DB를 짓는 명령 `py -3 -I tools/kangxi/app/build_db.py data/kangxi <출력.db>`; Windows 배포 `tools/windows/package.ps1 -Version 0.0.1 -BundleDirectory build/windows/x64/runner/Release`.

- [ ] **Step 1: 실패하는 검사를 쓴다**

`test\assets_test.dart`:
```dart
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

void main() {
  const fonts = [
    'Pretendard-Regular.otf', 'Pretendard-SemiBold.otf', 'Pretendard-Bold.otf',
    'NotoSerifKR-Regular.ttf', 'NotoSerifKR-Bold.ttf', 'D2Coding-Regular.ttf', 'HanaMinA.ttf', 'HanaMinB.ttf',
  ];

  test('글꼴 파일과 라이선스가 모두 있다', () {
    for (final f in fonts) {
      expect(File('assets/fonts/$f').existsSync(), isTrue, reason: f);
    }
    expect(Directory('assets/fonts/licenses').listSync().whereType<File>(), isNotEmpty);
  });

  test('pubspec이 글꼴을 모두 등록한다', () {
    final spec = File('pubspec.yaml').readAsStringSync();
    for (final f in fonts) {
      expect(spec, contains('assets/fonts/$f'), reason: f);
    }
  });

  test('康熙字典 자료가 다 왔다(조목 47,043)', () {
    final heads = File('data/kangxi/index/heads.tsv').readAsLinesSync();
    expect(heads.length - 1, 47043);
    expect(Directory('data/kangxi/translations/ko').listSync().length, 62);
  });

  test('앱 아이콘과 심벌이 있다', () {
    expect(File('design/brand/archive-mark.svg').existsSync(), isTrue);
    expect(File('windows/runner/resources/app_icon.ico').lengthSync(), greaterThan(1000));
    expect(File('android/app/src/main/res/mipmap-xxxhdpi/ic_launcher.png').existsSync(), isTrue);
    // 적응형 아이콘이 찾는 배경색이 함께 왔다(없으면 APK 빌드가 리소스 오류로 멈춘다).
    expect(File('android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml').readAsStringSync(), contains('@color/ic_launcher_background'));
    expect(File('android/app/src/main/res/values/colors.xml').readAsStringSync(), contains('name="ic_launcher_background"'));
  });
}
```

- [ ] **Step 2: 검사가 실패하는지 본다**

Run: `& $flutter test test/assets_test.dart`
Expected: 4 failed(파일 없음).

- [ ] **Step 3: 옮긴다**

```powershell
$old = 'D:\아카이브\HistoryArchive'
Set-Location D:\HistoryArchive
New-Item -ItemType Directory -Force assets\fonts, tools\fonts, design, tools\brand, data, docs\kangxi | Out-Null
Copy-Item "$old\packages\design_system\fonts\*.otf", "$old\packages\design_system\fonts\*.ttf" assets\fonts\
Copy-Item -Recurse "$old\packages\design_system\fonts\licenses" assets\fonts\
Copy-Item "$old\tools\fonts\*" tools\fonts\
Copy-Item -Recurse "$old\design\brand" design\
robocopy "$old\tools\brand" tools\brand /E /XD node_modules /NFL /NDL /NJH /NJS | Out-Null
Copy-Item -Recurse -Force "$old\app\android\app\src\main\res\mipmap-*" android\app\src\main\res\
# 적응형 아이콘(mipmap-anydpi-v26)이 배경색 @color/ic_launcher_background를 values/colors.xml에서 찾는다.
New-Item -ItemType Directory -Force android\app\src\main\res\values | Out-Null
Copy-Item -Force "$old\app\android\app\src\main\res\values\colors.xml" android\app\src\main\res\values\colors.xml
Copy-Item -Force "$old\app\windows\runner\resources\app_icon.ico" windows\runner\resources\app_icon.ico
robocopy "$old\data\kangxi" data\kangxi /E /NFL /NDL /NJH /NJS | Out-Null
robocopy "$old\tools\kangxi" tools\kangxi /E /XD __pycache__ /NFL /NDL /NJH /NJS | Out-Null
Copy-Item "$old\docs\KANGXI_SOURCE.md", "$old\docs\KANGXI_TRANSLATION.md", "$old\docs\KANGXI_TRANSLATION_NOTES.md" docs\kangxi\
Copy-Item "$old\tools\windows\installer.iss", "$old\tools\windows\package.ps1" tools\windows\
```
(robocopy는 성공해도 종료 코드 1을 낼 수 있다 — 8 이상만 실패다.)

`.gitattributes`에 더한다(옛 저장소 그대로):
```
# 강희자전 원문 자료. 원본 사본과 생성물은 diff를 보이지 않는다.
data/kangxi/source/** -diff linguist-vendored=true
data/kangxi/entries/** -diff linguist-generated=true
data/kangxi/index/** -diff linguist-generated=true
```

- [ ] **Step 4: 글꼴을 등록하고 앱에 프리텐다드를 쓴다**

`pubspec.yaml`의 `flutter:` 아래를 바꾼다:
```yaml
flutter:
  uses-material-design: false
  fonts:
    - family: Pretendard
      fonts:
        - asset: assets/fonts/Pretendard-Regular.otf
          weight: 400
        - asset: assets/fonts/Pretendard-SemiBold.otf
          weight: 600
        - asset: assets/fonts/Pretendard-Bold.otf
          weight: 700
    - family: NotoSerifKR
      fonts:
        - asset: assets/fonts/NotoSerifKR-Regular.ttf
          weight: 400
        - asset: assets/fonts/NotoSerifKR-Bold.ttf
          weight: 700
    - family: D2Coding
      fonts:
        - asset: assets/fonts/D2Coding-Regular.ttf
    # 花園明朝: 앱 글꼴에 없는 한자(확장 A · B 이후, 호환 한자)의 대체 글꼴. A는 BMP, B는 확장 B 이후.
    - family: HanaMinA
      fonts:
        - asset: assets/fonts/HanaMinA.ttf
    - family: HanaMinB
      fonts:
        - asset: assets/fonts/HanaMinB.ttf
  assets:
    - assets/fonts/licenses/
```

`lib\src\app.dart`의 `TextStyle`을 바꾼다:
```dart
              style: const TextStyle(
                fontFamily: 'Pretendard',
                fontFamilyFallback: ['NotoSerifKR', 'HanaMinA', 'HanaMinB'],
                fontSize: 16,
                color: Color(0xFF0A0A0A),
              ),
```

- [ ] **Step 5: Windows 설치 도구를 새 앱에 맞춘다**

`tools\windows\installer.iss`:
- `AppPublisherURL=https://github.com/nyomnyom1126-lang/HistoryArchive` → `AppPublisherURL=https://github.com/iamseu-11/HistoryArchive`
- `SetupIconFile={#SourcePath}\..\..\app\windows\runner\resources\app_icon.ico` → `SetupIconFile={#SourcePath}\..\..\windows\runner\resources\app_icon.ico`
(AppId · AppName · 설치 폴더 · 사용자 서재 줄은 그대로 — 옛 앱 위에 덮어 설치된다.)

`tools\windows\package.ps1`:
- `[string]$BundleDirectory = 'app/build/windows/x64/runner/Release'` → `[string]$BundleDirectory = 'build/windows/x64/runner/Release'`
- SQLite DLL 검사 두 줄
  ```powershell
  $sqlite = @(Get-ChildItem -LiteralPath $bundle -Recurse -File -Filter '*sqlite*.dll')
  if ($sqlite.Count -eq 0) { throw 'SQLite native DLL is missing from the Windows bundle.' }
  ```
  을 다음으로 바꾼다(SQLite는 이제 엔진 DLL 안에 든다):
  ```powershell
  if (-not (Test-Path -LiteralPath (Join-Path $bundle 'history_core.dll') -PathType Leaf)) { throw 'HistoryCore 엔진 DLL(history_core.dll)이 Windows 묶음에 없습니다.' }
  ```
- 옛 `package.ps1`도 한글이 든 BOM 없는 파일이다 — 고친 뒤 BOM을 붙인다(Global Constraints).

- [ ] **Step 6: 검사가 통과하고 康熙字典 DB가 지어지는지 본다**

Run:
```powershell
& $flutter pub get; & $flutter test
py -3 -I -X utf8 tools/kangxi/app/build_db.py data/kangxi build/kangxi.db
(Get-Item build\kangxi.db).Length
& $flutter build apk --release --target-platform android-arm64 --android-project-arg=kotlin.incremental=false
$env:CL = '/utf-8'; & $flutter build windows --release
./tools/windows/smoke-test.ps1 -Exe build\windows\x64\runner\Release\WikiArchive.exe
```
Expected: 8 tests passed(Task 3의 4 + 이 4) · DB 약 91MB(옛 CI 주석이 적은 크기) · 새 아이콘으로 APK와 Windows가 지어지고 연기 시험 통과. 창의 글이 프리텐다드로 보인다.

- [ ] **Step 7: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "옛 저장소에서 옮김 — 글꼴과 라이선스 · 펼친 기록 심벌과 아이콘 · 康熙字典 자료와 DB 도구 · Windows 설치 도구" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: CI 두 저장소 · GitHub 올리기(감독 몫)

**Files:**
- Create: `D:\HistoryCore\.github\workflows\ci.yml`
- Create: `D:\HistoryArchive\.github\workflows\build.yml`
- Modify: `D:\HistoryArchive\README.md`(생성된 것을 바꾼다)

**Interfaces:**
- Consumes: `tools/build-native.ps1`(Task 2), `HISTORYCORE_REF`(Task 3), `tools/windows/smoke-test.ps1`(Task 3).
- Produces: 저장소 비밀값 `HISTORYCORE_TOKEN`(HistoryArchive에 — HistoryCore 읽기 권한의 fine-grained 토큰, 감독이 만든다).

- [ ] **Step 1: HistoryCore CI를 쓴다**

`.github\workflows\ci.yml`:
```yaml
name: ci
on:
  push:
  pull_request:
  workflow_dispatch:

env:
  FLUTTER_VERSION: 3.47.7

permissions:
  contents: read

jobs:
  windows:
    name: Rust 검사 · Windows 엔진 · 플러그인
    runs-on: windows-2025
    timeout-minutes: 40
    defaults:
      run:
        shell: pwsh
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: '1.94.1'
      - uses: Swatinem/rust-cache@v2
      - name: Rust 검사
        run: cargo test --locked --workspace
      - name: 엔진 짓기(Windows)
        run: ./tools/build-native.ps1
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: ${{ env.FLUTTER_VERSION }}
          channel: stable
          cache: true
      - name: 플러그인 검사
        working-directory: packages/history_core
        run: |
          flutter pub get
          flutter analyze
          flutter test

  android:
    name: Android 엔진(arm64)
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: android-actions/setup-android@v4
      - name: NDK 28.2
        run: sdkmanager --install "ndk;28.2.13676358"
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: '1.94.1'
          targets: aarch64-linux-android
      - uses: Swatinem/rust-cache@v2
      - name: 엔진 짓기(Android)
        shell: pwsh
        run: ./tools/build-native.ps1 -Android -SkipWindows
      - run: test -f packages/history_core/native/android/arm64-v8a/libhistory_core.so
```

- [ ] **Step 2: HistoryArchive CI를 쓴다**

`.github\workflows\build.yml`:
```yaml
name: build
on:
  push:
  workflow_dispatch:

env:
  FLUTTER_VERSION: 3.47.7

permissions:
  contents: read

concurrency:
  group: build-${{ github.ref }}
  cancel-in-progress: true

# 두 작업 모두 HistoryArchive와 HistoryCore를 나란한 폴더로 받는다(pubspec의 ../HistoryCore).
jobs:
  android:
    name: Android APK
    runs-on: ubuntu-latest
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@v4
        with:
          path: HistoryArchive
      - id: core
        run: echo "ref=$(tr -d '[:space:]' < HistoryArchive/HISTORYCORE_REF)" >> "$GITHUB_OUTPUT"
      - name: HistoryCore 받기
        uses: actions/checkout@v4
        with:
          repository: iamseu-11/HistoryCore
          ref: ${{ steps.core.outputs.ref }}
          path: HistoryCore
          token: ${{ secrets.HISTORYCORE_TOKEN }}
      - uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: '21'
      - uses: android-actions/setup-android@v4
      - name: NDK 28.2
        run: sdkmanager --install "ndk;28.2.13676358"
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: '1.94.1'
          targets: aarch64-linux-android
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: HistoryCore
      - name: 엔진 짓기(Android)
        shell: pwsh
        run: ./HistoryCore/tools/build-native.ps1 -Android -SkipWindows
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: ${{ env.FLUTTER_VERSION }}
          channel: stable
          cache: true
      - name: 분석 · 검사
        working-directory: HistoryArchive
        run: |
          flutter pub get
          flutter analyze
          flutter test
      - name: APK 짓기 · 엔진 확인
        working-directory: HistoryArchive
        run: |
          flutter build apk --release --target-platform android-arm64
          unzip -l build/app/outputs/flutter-apk/app-release.apk | grep -q 'lib/arm64-v8a/libhistory_core.so'
      - uses: actions/upload-artifact@v4
        with:
          name: HistoryArchive-${{ github.sha }}-APK
          path: HistoryArchive/build/app/outputs/flutter-apk/app-release.apk
          if-no-files-found: error

  windows:
    name: Windows EXE
    runs-on: windows-2025
    timeout-minutes: 60
    env:
      CL: /utf-8
    defaults:
      run:
        shell: pwsh
    steps:
      - uses: actions/checkout@v4
        with:
          path: HistoryArchive
      - id: core
        run: '"ref=$((Get-Content HistoryArchive/HISTORYCORE_REF -Raw).Trim())" >> $env:GITHUB_OUTPUT'
      - name: HistoryCore 받기
        uses: actions/checkout@v4
        with:
          repository: iamseu-11/HistoryCore
          ref: ${{ steps.core.outputs.ref }}
          path: HistoryCore
          token: ${{ secrets.HISTORYCORE_TOKEN }}
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: '1.94.1'
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: HistoryCore
      - name: 엔진 짓기(Windows)
        run: ./HistoryCore/tools/build-native.ps1
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: ${{ env.FLUTTER_VERSION }}
          channel: stable
          cache: true
      - name: 분석 · 검사(실제 엔진 시험 포함)
        working-directory: HistoryArchive
        run: |
          flutter pub get
          flutter analyze
          flutter test
      - name: Windows 짓기 · 연기 시험
        working-directory: HistoryArchive
        run: |
          flutter build windows --release
          ./tools/windows/smoke-test.ps1 -Exe build/windows/x64/runner/Release/WikiArchive.exe
      - uses: actions/upload-artifact@v4
        with:
          name: HistoryArchive-${{ github.sha }}-Windows
          path: HistoryArchive/build/windows/x64/runner/Release/
          if-no-files-found: error
```

`README.md`(통째로 바꾼다):
```markdown
# HistoryArchive

개인 역사 서재 — 위키 · 자전 · 동기화. 역사 앱 생태계의 서재를 쓰는 앱이다. 설계: [docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md](docs/superpowers/specs/2026-10-21-remake-ecosystem-design.md).

엔진은 나란한 폴더의 [HistoryCore](https://github.com/iamseu-11/HistoryCore)에서 온다(`../HistoryCore`, CI는 `HISTORYCORE_REF`의 판).

    ../HistoryCore/tools/build-native.ps1 -Android
    D:\flutter-sdk\flutter\bin\flutter.bat test
    $env:CL='/utf-8'; D:\flutter-sdk\flutter\bin\flutter.bat build windows --release

Windows 빌드에는 개발자 모드가 필요하다. 없으면 `tools/windows/plugin-junction.ps1`.
```

- [ ] **Step 3: 커밋한다(두 저장소)**

```powershell
Set-Location D:\HistoryCore
git add -A; git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "CI — Rust 검사 · 엔진 짓기 · 플러그인 검사(windows-2025)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
Set-Location D:\HistoryArchive
git add -A; git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "CI — HistoryCore를 나란히 받아 엔진을 짓고 APK · Windows · 연기 시험" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 4: 감독에게 올리기를 안내한다(실행자는 하지 않는다)**

감독에게 그대로 전한다:
1. GitHub `iamseu-11` 계정에 비공개 저장소 `HistoryCore` · `HistoryArchive`를 빈 채로 만든다.
2. `HistoryCore` 읽기 권한의 fine-grained 토큰을 만들어 `HistoryArchive` 저장소 비밀값 `HISTORYCORE_TOKEN`으로 넣는다.
3. 두 폴더에서 원격을 잇고 민다:
   ```
   git -C D:\HistoryCore remote add origin https://iamseu-11@github.com/iamseu-11/HistoryCore.git
   git -C D:\HistoryCore push -u origin main
   git -C D:\HistoryArchive remote add origin https://iamseu-11@github.com/iamseu-11/HistoryArchive.git
   git -C D:\HistoryArchive push -u origin main
   ```
4. 두 저장소의 Actions가 초록이면 **P0 끝**(설계 9-1: "빈 앱이 Android · Windows에서 Rust 엔진을 불러 엔진 판 번호를 띄움, CI 통과").

---

## P1 — 측정 판

### Task 6: 더미 서재 재료와 생성기 `history_corpus`

**Files:**
- Create: `D:\HistoryCore\tools\make-pool.py`
- Create(생성물, 커밋): `crates\corpus\assets\pool.txt`, `crates\corpus\assets\words.txt`
- Create: `crates\corpus\Cargo.toml`, `src\lib.rs`, `src\rng.rs`, `src\pool.rs`, `src\tree.rs`, `src\doc.rs`, `tests\corpus.rs`
- Modify: `Cargo.toml`(작업공간 members)

**Interfaces:**
- Produces(`history_corpus`):
  - `struct CorpusSpec { pub docs: u32, pub seed: u64 }`
  - `struct Corpus<'a>`: `new(CorpusSpec, &'a TextPool) -> Self` · `spec() -> CorpusSpec` · `tree() -> &CategoryTree` · `pool() -> &TextPool` · `title(i: u32) -> &str`(일반 문서 i) · `len() -> u32`(분류 문서 + 일반 문서) · `doc(idx: u32) -> GenDoc`(0..len, 분류 문서가 앞) · `docs() -> impl Iterator<Item = GenDoc>`
  - `struct GenDoc { pub id: u32, pub ns: Ns, pub title: String, pub body: String, pub home: Option<String>, pub group: Option<String> }` · `enum Ns { Doc, Category }`
  - `struct TextPool { pub sentences: Vec<&'static str>, pub words: Vec<&'static str> }` · `TextPool::embedded()`
  - `struct CategoryTree`: `categories() -> &[Category]` · `leaves() -> &[u32]` · `themes() -> &[u32]` · `groups() -> &[u32]` · `get(u32) -> &Category` · `path(u32) -> Vec<u32>`; `struct Category { pub name: String, pub parent: Option<u32>, pub group: u32, pub depth: u8, pub leaf: bool }`
  - `struct Rng`: `new(u64)` · `for_item(seed, salt, i)` · `next_u64()` · `below(n)` · `unit()` · `chance(p)` · `skewed(n)` · `pick(&[T]) -> &T`
  - `const SOURCES: [&str; 24]`(각주 사료)

- [ ] **Step 1: 재료를 뽑는다**

`tools\make-pool.py`:
```python
"""《康熙字典》 한국어 번역층에서 더미 서재의 재료(문장 · 낱말)를 뽑는다. 한 번 돌려 결과를 커밋한다.

  py -3 -I -X utf8 tools/make-pool.py <번역층 폴더> crates/corpus/assets
  예) py -3 -I -X utf8 tools/make-pool.py "D:/HistoryArchive/data/kangxi/translations/ko" crates/corpus/assets

늘 같은 결과가 나오도록 해시 차례로 고른다. 나무마크에서 뜻이 있는 글자(대괄호 · | · = · * · 중괄호 · ~ · ^ · _ · ' · -- · ,,)가 든 문장은 뺀다.
"""
import hashlib
import io
import json
import re
import sys
from collections import Counter
from pathlib import Path

SRC, OUT = Path(sys.argv[1]), Path(sys.argv[2])
SENTENCES, WORDS = 20000, 5000
BAD = re.compile(r"[\[\]|=*⟦⟧{}~^_'\n\r\t]|--|,,")
SUP = re.compile(r'[¹²³⁴⁵⁶⁷⁸⁹⁰]+')
STOP = {'이다', '하였다', '라고', '하는', '있다', '것이다', '같다', '이며', '으로', '에서', '모두',
        '또한', '이라', '하고', '한다', '되었다', '그리고', '이니', '이고', '것이', '한다고'}

sents, words = set(), Counter()
for p in sorted(SRC.glob('*.jsonl')):
    for line in io.open(p, encoding='utf-8'):
        for para in json.loads(line).get('paragraphs', []):
            ko = para.get('ko')
            if not ko:
                continue
            for s in re.split(r'(?<=다\.)\s+', SUP.sub('', ko)):
                s = s.strip()
                if 15 <= len(s) <= 160 and s.endswith('다.') and not BAD.search(s):
                    sents.add(s)
                    for w in re.findall(r'[가-힣]{2,4}', s):
                        if w not in STOP:
                            words[w] += 1

key = lambda s: hashlib.sha256(s.encode('utf-8')).hexdigest()
picked = sorted(sorted(sents, key=key)[:SENTENCES])
top = [w for w, _ in sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))][:WORDS]
OUT.mkdir(parents=True, exist_ok=True)
(OUT / 'pool.txt').write_text('\n'.join(picked) + '\n', encoding='utf-8', newline='\n')
(OUT / 'words.txt').write_text('\n'.join(top) + '\n', encoding='utf-8', newline='\n')
print(f'문장 {len(picked)} / 후보 {len(sents)} · 낱말 {len(top)}')
```

Run: `py -3 -I -X utf8 tools/make-pool.py "D:/HistoryArchive/data/kangxi/translations/ko" crates/corpus/assets`
Expected: `문장 20000 / 후보 …(10만 넘음) · 낱말 5000`. `pool.txt` 약 3~4MB.

- [ ] **Step 2: 상자를 만들고 실패하는 검사를 쓴다**

`Cargo.toml`의 members를 `["crates/library", "crates/ffi", "crates/corpus"]`로.

`crates\corpus\Cargo.toml`:
```toml
[package]
name = "history_corpus"
version.workspace = true
edition.workspace = true
publish.workspace = true

[dev-dependencies]
sha2 = "0.10"
```

`crates\corpus\src\lib.rs`(빈 뼈대):
```rust
//! 100만 건 측정용 더미 서재 — Step 4에서 채운다.
```

`crates\corpus\tests\corpus.rs`:
```rust
use history_corpus::{Corpus, CorpusSpec, Ns, TextPool};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

fn digest(c: &Corpus) -> Vec<u8> {
    let mut h = Sha256::new();
    for d in c.docs() {
        h.update(d.title.as_bytes());
        h.update(d.body.as_bytes());
    }
    h.finalize().to_vec()
}

#[test]
fn embedded_pool_is_large_enough() {
    let p = TextPool::embedded();
    assert!(p.sentences.len() >= 10_000, "문장 {}", p.sentences.len());
    assert!(p.words.len() >= 2_000, "낱말 {}", p.words.len());
}

#[test]
fn same_seed_same_library_other_seed_other_library() {
    let pool = TextPool::embedded();
    let a = Corpus::new(CorpusSpec { docs: 2_000, seed: 7 }, &pool);
    let b = Corpus::new(CorpusSpec { docs: 2_000, seed: 7 }, &pool);
    let c = Corpus::new(CorpusSpec { docs: 2_000, seed: 8 }, &pool);
    assert_eq!(digest(&a), digest(&b));
    assert_ne!(digest(&a), digest(&c));
}

#[test]
fn doc_i_does_not_depend_on_earlier_docs() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 3_000, seed: 1 }, &pool);
    let from_stream: Vec<_> = c.docs().skip(2_500).take(3).collect();
    let direct: Vec<_> = (2_500..2_503).map(|i| c.doc(i)).collect();
    assert_eq!(from_stream, direct);
}

#[test]
fn counts_match_and_titles_are_unique_per_namespace() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 5_000, seed: 3 }, &pool);
    let docs: Vec<_> = c.docs().collect();
    assert_eq!(docs.len() as u32, c.len());
    assert_eq!(docs.iter().filter(|d| d.ns == Ns::Doc).count(), 5_000);
    let mut seen = HashSet::new();
    for d in &docs {
        assert!(seen.insert((d.ns == Ns::Doc, d.title.clone())), "제목 겹침: {}", d.title);
    }
}

#[test]
fn first_category_is_home_and_every_category_reaches_a_root() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 5_000, seed: 3 }, &pool);
    let tree = c.tree();
    let index: HashMap<&str, u32> = tree.categories().iter().enumerate().map(|(i, k)| (k.name.as_str(), i as u32)).collect();
    let mut unfiled = 0;
    for d in c.docs().filter(|d| d.ns == Ns::Doc) {
        match &d.home {
            None => {
                unfiled += 1;
                assert!(!d.body.starts_with("[[분류:"));
                assert!(d.group.is_none());
            }
            Some(h) => {
                assert!(d.body.starts_with(&format!("[[분류:{h}]]")), "{}", d.title);
                let path = tree.path(index[h.as_str()]);
                assert!(path.len() <= 4 && tree.get(*path.last().unwrap()).parent.is_none());
                assert!(d.group.is_some());
            }
        }
    }
    assert!((25..=80).contains(&unfiled), "분류 안 됨 {unfiled}편 — 1% 안팎이어야 함");
}

#[test]
fn brackets_and_footnotes_close() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 2_000, seed: 5 }, &pool);
    for d in c.docs() {
        assert_eq!(d.body.matches("[[").count(), d.body.matches("]]").count(), "{}", d.title);
        assert_eq!(d.body.matches("[*").count(), d.body.matches("권.]").count(), "{}", d.title);
    }
}

#[test]
fn bodies_carry_hanja_including_beyond_bmp() {
    // 설계 9-1 P1: "실제 같은 … 한자". 재료(康熙字典 번역층)의 문단 열에 아홉에 한자, 일곱에 하나꼴로 확장 B 이후 한자가 있다.
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 2_000, seed: 8 }, &pool);
    let is_han = |ch: char| ('\u{4E00}'..='\u{9FFF}').contains(&ch) || ('\u{3400}'..='\u{4DBF}').contains(&ch) || ch >= '\u{20000}';
    let (mut han, mut beyond) = (0, 0);
    for d in c.docs().filter(|d| d.ns == Ns::Doc) {
        if d.body.chars().any(is_han) {
            han += 1;
        }
        if d.body.chars().any(|ch| ch >= '\u{20000}') {
            beyond += 1;
        }
    }
    assert!(han >= 1_000, "한자가 든 문서 {han}/2000");
    assert!(beyond >= 1, "확장 B 이후 한자가 든 문서가 없음");
}

#[test]
fn a_few_links_point_to_missing_docs() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 5_000, seed: 9 }, &pool);
    let titles: HashSet<&str> = (0..5_000).map(|i| c.title(i)).collect();
    let (mut all, mut missing) = (0usize, 0usize);
    for d in c.docs().filter(|d| d.ns == Ns::Doc) {
        for part in d.body.split("[[").skip(1) {
            let target = part.split("]]").next().unwrap();
            if target.starts_with("분류:") {
                continue;
            }
            all += 1;
            if !titles.contains(target) {
                missing += 1;
            }
        }
    }
    let share = missing as f64 / all as f64;
    assert!((0.02..0.09).contains(&share), "없는 문서 링크 비율 {share:.3}");
}
```

- [ ] **Step 3: 검사가 실패하는지 본다**

Run: `cargo test -p history_corpus`
Expected: 컴파일 실패 — `unresolved imports history_corpus::Corpus …`.

- [ ] **Step 4: 생성기를 구현한다**

`crates\corpus\src\rng.rs`:
```rust
/// SplitMix64 — 외부 상자 없이 플랫폼 · 판에 무관하게 늘 같은 수열을 낸다.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    /// 씨앗 · 갈래(salt) · 번호에서 갈라 낸 독립 수열 — 문서 i를 앞 문서 없이 만들 때 쓴다.
    pub fn for_item(seed: u64, salt: u64, i: u64) -> Self {
        let mut r = Rng(seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ i.wrapping_mul(0xD1B5_4A32_D192_ED03));
        r.next_u64();
        r
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0..n (n > 0). 이 쓰임에서 나머지 치우침은 무시할 만하다.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// [0, 1)
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    /// 앞쪽으로 쏠린 0..n — 앞 번호(인기 문서)가 자주 뽑힌다.
    pub fn skewed(&mut self, n: u64) -> u64 {
        let u = self.unit();
        ((u * u * u * n as f64) as u64).min(n.saturating_sub(1))
    }

    pub fn pick<'b, T>(&mut self, items: &'b [T]) -> &'b T {
        &items[self.below(items.len() as u64) as usize]
    }
}
```

`crates\corpus\src\pool.rs`:
```rust
/// 더미 문서의 재료 — 《康熙字典》 한국어 번역층에서 뽑은 문장과 낱말(`assets/`, `tools/make-pool.py`).
pub struct TextPool {
    pub sentences: Vec<&'static str>,
    pub words: Vec<&'static str>,
}

impl TextPool {
    pub fn embedded() -> Self {
        fn lines(s: &'static str) -> Vec<&'static str> {
            s.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
        }
        TextPool { sentences: lines(include_str!("../assets/pool.txt")), words: lines(include_str!("../assets/words.txt")) }
    }
}
```

`crates\corpus\src\tree.rs`:
```rust
use crate::{pool::TextPool, rng::Rng};
use std::collections::HashSet;

/// 분류 하나. `parent`가 없으면 뿌리. `group`은 큰 갈래(한국사 · 중국사 · …, 깊이 1)의 번호.
#[derive(Clone, Debug)]
pub struct Category {
    pub name: String,
    pub parent: Option<u32>,
    pub group: u32,
    pub depth: u8,
    pub leaf: bool,
}

/// 분류 나무 — 첫 분류가 집(설계 5-3). 뿌리 둘과 큰 갈래 여섯은 이름이 정해져 있고, 그 아래는 문서 수에 맞춰 는다.
/// 주제 분류(전쟁 · 인물 …)는 문서의 둘째 · 셋째 분류로만 쓰인다.
pub struct CategoryTree {
    cats: Vec<Category>,
    leaves: Vec<u32>,
    themes: Vec<u32>,
    groups: Vec<u32>,
}

pub const ROOTS: [(&str, [&str; 3]); 2] = [("동양사", ["한국사", "중국사", "일본사"]), ("서양사", ["고대", "중세", "근대"])];
pub const THEME_ROOT: &str = "주제";
pub const THEMES: [&str; 8] = ["전쟁", "인물", "제도", "외교", "문화", "경제", "종교", "과학"];
const MID_SUFFIX: [&str; 4] = ["시대", "왕조", "지방", "세력"];

impl CategoryTree {
    pub fn build(docs: u32, seed: u64, pool: &TextPool) -> Self {
        let mut cats = Vec::new();
        let mut groups = Vec::new();
        for (root, kids) in ROOTS {
            let r = cats.len() as u32;
            cats.push(Category { name: root.into(), parent: None, group: r, depth: 0, leaf: false });
            for k in kids {
                let g = cats.len() as u32;
                cats.push(Category { name: k.into(), parent: Some(r), group: g, depth: 1, leaf: false });
                groups.push(g);
            }
        }
        let t = cats.len() as u32;
        cats.push(Category { name: THEME_ROOT.into(), parent: None, group: t, depth: 0, leaf: false });
        let mut themes = Vec::new();
        for th in THEMES {
            themes.push(cats.len() as u32);
            cats.push(Category { name: th.into(), parent: Some(t), group: t, depth: 1, leaf: true });
        }
        let mut names: HashSet<String> = cats.iter().map(|c| c.name.clone()).collect();
        let mut leaves = Vec::new();
        // 문서 25편에 분류 하나꼴. 큰 갈래마다 둘째 층(√) · 셋째 층(잎).
        let per_group = ((docs / 25).max(groups.len() as u32 * 4)) / groups.len() as u32;
        let mids = ((per_group as f64).sqrt().ceil() as u32).max(2);
        let lows = (per_group / mids).max(1);
        let mut r = Rng::for_item(seed, 1, 0);
        for &g in &groups {
            for _ in 0..mids {
                let m = cats.len() as u32;
                let name = unique(&mut names, format!("{} {}", r.pick(&pool.words), r.pick(&MID_SUFFIX)));
                cats.push(Category { name, parent: Some(g), group: g, depth: 2, leaf: false });
                for _ in 0..lows {
                    let l = cats.len() as u32;
                    let name = unique(&mut names, format!("{} {}", r.pick(&pool.words), r.pick(&pool.words)));
                    cats.push(Category { name, parent: Some(m), group: g, depth: 3, leaf: true });
                    leaves.push(l);
                }
            }
        }
        CategoryTree { cats, leaves, themes, groups }
    }

    pub fn categories(&self) -> &[Category] {
        &self.cats
    }
    pub fn leaves(&self) -> &[u32] {
        &self.leaves
    }
    pub fn themes(&self) -> &[u32] {
        &self.themes
    }
    pub fn groups(&self) -> &[u32] {
        &self.groups
    }
    pub fn get(&self, i: u32) -> &Category {
        &self.cats[i as usize]
    }
    /// 뿌리까지 올라가는 길(자기 포함, 아래 → 위).
    pub fn path(&self, mut i: u32) -> Vec<u32> {
        let mut p = vec![i];
        while let Some(up) = self.cats[i as usize].parent {
            p.push(up);
            i = up;
        }
        p
    }
}

fn unique(seen: &mut HashSet<String>, base: String) -> String {
    if seen.insert(base.clone()) {
        return base;
    }
    let mut n = 2;
    loop {
        let t = format!("{base} {n}");
        if seen.insert(t.clone()) {
            return t;
        }
        n += 1;
    }
}
```

`crates\corpus\src\doc.rs`:
```rust
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
        b.push_str(r.pick(&pool.sentences));
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
```

`crates\corpus\src\lib.rs`(통째로):
```rust
//! 100만 건 측정용 더미 서재(설계 9-1 P1). 정해진 씨앗에서 늘 같은 문서가 나오고,
//! 문서 i는 앞 문서 없이 바로 만들 수 있다(파일로 두지 않고 흘려 낸다).
mod doc;
mod pool;
mod rng;
mod tree;

pub use doc::{GenDoc, Ns, SOURCES};
pub use pool::TextPool;
pub use rng::Rng;
pub use tree::{Category, CategoryTree};

use std::collections::HashSet;

/// 더미 서재의 크기와 씨앗.
#[derive(Clone, Copy, Debug)]
pub struct CorpusSpec {
    /// 일반 문서 수(분류 문서는 따로 더해진다).
    pub docs: u32,
    pub seed: u64,
}

pub struct Corpus<'a> {
    spec: CorpusSpec,
    pool: &'a TextPool,
    tree: CategoryTree,
    titles: Vec<String>,
}

impl<'a> Corpus<'a> {
    pub fn new(spec: CorpusSpec, pool: &'a TextPool) -> Self {
        let tree = CategoryTree::build(spec.docs, spec.seed, pool);
        let mut seen: HashSet<String> = tree.categories().iter().map(|c| c.name.clone()).collect();
        let mut titles = Vec::with_capacity(spec.docs as usize);
        for i in 0..spec.docs {
            let base = doc::title_for(spec.seed, i, pool);
            let mut t = base.clone();
            let mut n = 2;
            while !seen.insert(t.clone()) {
                t = format!("{base} ({n})");
                n += 1;
            }
            titles.push(t);
        }
        Corpus { spec, pool, tree, titles }
    }

    pub fn spec(&self) -> CorpusSpec {
        self.spec
    }
    pub fn tree(&self) -> &CategoryTree {
        &self.tree
    }
    pub fn pool(&self) -> &TextPool {
        self.pool
    }
    /// 일반 문서 i의 제목.
    pub fn title(&self, i: u32) -> &str {
        &self.titles[i as usize]
    }
    /// 분류 문서 + 일반 문서의 수.
    pub fn len(&self) -> u32 {
        self.tree.categories().len() as u32 + self.spec.docs
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// 문서 하나(0..len). 분류 문서가 앞, 그다음 일반 문서.
    pub fn doc(&self, idx: u32) -> GenDoc {
        let cats = self.tree.categories().len() as u32;
        if idx < cats {
            doc::category_doc(self, idx)
        } else {
            doc::article(self, idx - cats, idx)
        }
    }
    /// 모든 문서를 차례로 흘려 낸다(한꺼번에 메모리에 두지 않는다).
    pub fn docs(&self) -> impl Iterator<Item = GenDoc> + '_ {
        (0..self.len()).map(move |i| self.doc(i))
    }
}
```

- [ ] **Step 5: 검사가 통과하는지 본다**

Run: `cargo test -p history_corpus`
Expected: 8 passed. 실패하면 비율 · 개수 단언의 까닭을 고친다(단언을 느슨하게 하지 않는다 — 생성기의 확률을 맞춘다).

- [ ] **Step 6: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "측정용 더미 서재 history_corpus — 康熙字典 번역층 재료 · 첫 분류가 집인 나무 · 링크와 각주 · 정해진 씨앗" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: 측정 판 핵심 `history_bench` — Library 약속 · 일감 · 잰 값 · 예산표 · 보고

**Files:**
- Create: `crates\bench\Cargo.toml`, `src\lib.rs`, `src\library.rs`, `src\stats.rs`, `src\report.rs`, `src\workload.rs`, `src\run.rs`, `tests\run.rs`
- Modify: `Cargo.toml`(members)

**Interfaces:**
- Consumes: Task 6의 `Corpus` · `CorpusSpec` · `GenDoc` · `TextPool` · `Rng`.
- Produces(`history_bench`):
  - `type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>`
  - `trait Library { fn name(&self) -> &'static str; fn load(&mut self, docs: &mut dyn Iterator<Item = GenDoc>) -> Result<()>; fn find_title(&self, prefix: &str, limit: usize) -> Result<Vec<u32>>; fn search_body(&self, query: &str, limit: usize) -> Result<Vec<u32>>; fn open(&self, id: u32) -> Result<Option<String>>; fn save(&mut self, id: u32, body: &str) -> Result<()>; fn category_children(&self, category: &str) -> Result<Vec<u32>>; fn home_aggregates(&self) -> Result<Aggregates>; fn reopen(&mut self) -> Result<()>; fn bytes_on_disk(&self) -> Result<u64>; }`
  - `struct Aggregates { pub groups: Vec<(String, u64)>, pub recent: Vec<u32> }`
  - `enum Op { FindTitle, SearchBody, Open, Save, CategoryChildren, HomeAggregates, Startup }` · `struct OpStats { op, count, p50_us, p95_us, p99_us, max_us }` · `fn nearest_rank(&[Duration], f64) -> Duration`
  - `struct RunPlan { seed, find_title, search_body, open, save, categories, aggregates, startups }` · `RunPlan::standard(seed)` · `RunPlan::quick(seed)`
  - `struct Workload { title_prefixes, body_queries, open_ids, saves, categories }` · `Workload::new(&Corpus, &RunPlan)`
  - `fn run(lib: &mut dyn Library, corpus: &Corpus, plan: &RunPlan, device: &str) -> Result<Report>`
  - `struct Report { library, device, docs, seed, load_ms, bytes_on_disk, ops, verdicts }` · `Report::judge(&[OpStats]) -> Vec<Verdict>` · `to_json()` · `to_markdown()`; `struct Verdict { label, limit_us, measured_us, pass }`; `const BUDGETS: [Budget; 7]`

- [ ] **Step 1: 상자를 만들고 실패하는 검사를 쓴다**

`Cargo.toml` members에 `"crates/bench"`를 더한다.

`crates\bench\Cargo.toml`:
```toml
[package]
name = "history_bench"
version.workspace = true
edition.workspace = true
publish.workspace = true

[[bin]]
name = "hc-bench"
path = "src/main.rs"

[dependencies]
history_corpus = { path = "../corpus" }
rusqlite = { version = "0.37", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`crates\bench\src\lib.rs`(빈 뼈대): `//! 측정 판 — Step 3에서 채운다.`
`crates\bench\src\main.rs`(빈 뼈대): `fn main() {}`

`crates\bench\tests\run.rs`:
```rust
use history_bench::{nearest_rank, run, Aggregates, Library, Op, Report, Result, RunPlan, Workload};
use history_corpus::{Corpus, CorpusSpec, GenDoc, TextPool};
use std::time::Duration;

/// 메모리 속 흉내 서재 — 판이 일을 다 돌리고 보고를 만드는지만 본다.
#[derive(Default)]
struct Fake {
    docs: Vec<GenDoc>,
}

impl Library for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }
    fn load(&mut self, docs: &mut dyn Iterator<Item = GenDoc>) -> Result<()> {
        self.docs = docs.collect();
        Ok(())
    }
    fn find_title(&self, p: &str, limit: usize) -> Result<Vec<u32>> {
        Ok(self.docs.iter().filter(|d| d.title.starts_with(p)).take(limit).map(|d| d.id).collect())
    }
    fn search_body(&self, q: &str, limit: usize) -> Result<Vec<u32>> {
        Ok(self.docs.iter().filter(|d| d.body.contains(q)).take(limit).map(|d| d.id).collect())
    }
    fn open(&self, id: u32) -> Result<Option<String>> {
        Ok(self.docs.get(id as usize).map(|d| d.body.clone()))
    }
    fn save(&mut self, id: u32, body: &str) -> Result<()> {
        self.docs[id as usize].body = body.into();
        Ok(())
    }
    fn category_children(&self, c: &str) -> Result<Vec<u32>> {
        Ok(self.docs.iter().filter(|d| d.home.as_deref() == Some(c)).map(|d| d.id).collect())
    }
    fn home_aggregates(&self) -> Result<Aggregates> {
        Ok(Aggregates::default())
    }
    fn reopen(&mut self) -> Result<()> {
        Ok(())
    }
    fn bytes_on_disk(&self) -> Result<u64> {
        Ok(0)
    }
}

#[test]
fn nearest_rank_percentiles() {
    let v: Vec<Duration> = (1..=100).map(Duration::from_millis).collect();
    assert_eq!(nearest_rank(&v, 50.0), Duration::from_millis(50));
    assert_eq!(nearest_rank(&v, 95.0), Duration::from_millis(95));
    assert_eq!(nearest_rank(&v, 99.0), Duration::from_millis(99));
    assert_eq!(nearest_rank(&[Duration::from_millis(7)], 99.0), Duration::from_millis(7));
}

#[test]
fn same_seed_same_workload() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 1_000, seed: 2 }, &pool);
    let a = Workload::new(&c, &RunPlan::quick(5));
    let b = Workload::new(&c, &RunPlan::quick(5));
    assert_eq!(a.title_prefixes, b.title_prefixes);
    assert_eq!(a.body_queries, b.body_queries);
    assert_eq!(a.open_ids, b.open_ids);
    assert!(a.body_queries.iter().all(|q| (2..=4).contains(&q.chars().count())));
}

#[test]
fn run_measures_every_op_and_judges_every_budget() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 1_000, seed: 2 }, &pool);
    let mut f = Fake::default();
    let r = run(&mut f, &c, &RunPlan::quick(2), "시험").unwrap();
    assert_eq!(r.ops.len(), 7);
    assert_eq!(r.verdicts.len(), 7);
    assert!(r.ops.iter().any(|o| o.op == Op::Startup));
    let md = r.to_markdown();
    assert!(md.contains("| 제목 찾기 p99 ≤ 10ms |"));
    assert!(md.contains("fake · 시험 · 문서 1000편"));
    let j: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
    assert_eq!(j["docs"], 1000);
    assert_eq!(j["verdicts"].as_array().unwrap().len(), 7);
}

#[test]
fn verdict_fails_when_over_budget() {
    let ops = vec![history_bench::OpStats { op: Op::FindTitle, count: 1, p50_us: 5, p95_us: 9_000, p99_us: 12_000, max_us: 12_000 }];
    let v = Report::judge(&ops);
    assert_eq!(v.len(), 1);
    assert!(!v[0].pass);
    assert_eq!(v[0].measured_us, 12_000);
}
```

- [ ] **Step 2: 검사가 실패하는지 본다**

Run: `cargo test -p history_bench --test run`
Expected: 컴파일 실패 — `unresolved imports history_bench::nearest_rank …`.

- [ ] **Step 3: 구현한다**

`crates\bench\src\library.rs`:
```rust
use history_corpus::GenDoc;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 대문 · 통계가 한 번에 읽는 모음: 큰 갈래별 문서 수와 최근 바뀐 문서 10.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Aggregates {
    pub groups: Vec<(String, u64)>,
    pub recent: Vec<u32>,
}

/// 잴 서재 구현. P1의 기준 구현은 `naive::NaiveSqlite`, P2의 서재 엔진도 이 약속에 꽂는다.
pub trait Library {
    fn name(&self) -> &'static str;
    /// 빈 자리에서 모두 들인다. 앞서 남은 것(끊긴 측정)은 지우고 시작한다.
    fn load(&mut self, docs: &mut dyn Iterator<Item = GenDoc>) -> Result<()>;
    /// 제목이 prefix로 시작하는 일반 문서(제목 차례), 많아야 limit.
    fn find_title(&self, prefix: &str, limit: usize) -> Result<Vec<u32>>;
    /// 본문에 query가 든 문서, 많아야 limit.
    fn search_body(&self, query: &str, limit: usize) -> Result<Vec<u32>>;
    /// 문서 열기 — 화면에 그릴 것을 꺼낸다(기준 구현은 본문 글, 엔진은 그리기 모델).
    fn open(&self, id: u32) -> Result<Option<String>>;
    /// 한 편 저장(판 + 색인 + 그리기 모델).
    fn save(&mut self, id: u32, body: &str) -> Result<()>;
    /// 분류 폴더 하나에 바로 든 문서.
    fn category_children(&self, category: &str) -> Result<Vec<u32>>;
    fn home_aggregates(&self) -> Result<Aggregates>;
    /// 닫았다 다시 연다(앱 시작의 엔진 몫).
    fn reopen(&mut self) -> Result<()>;
    fn bytes_on_disk(&self) -> Result<u64>;
}
```

`crates\bench\src\stats.rs`:
```rust
use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Op {
    FindTitle,
    SearchBody,
    Open,
    Save,
    CategoryChildren,
    HomeAggregates,
    Startup,
}

impl Op {
    pub fn label(self) -> &'static str {
        match self {
            Op::FindTitle => "제목 찾기",
            Op::SearchBody => "본문 찾기 첫 20건",
            Op::Open => "문서 열기",
            Op::Save => "저장 한 편",
            Op::CategoryChildren => "분류 폴더 펼치기",
            Op::HomeAggregates => "대문 · 통계 모음",
            Op::Startup => "다시 열기 → 대문 모음",
        }
    }
}

/// 일 하나를 여러 번 잰 값(마이크로초).
#[derive(Clone, Debug, Serialize)]
pub struct OpStats {
    pub op: Op,
    pub count: usize,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub max_us: u64,
}

impl OpStats {
    pub fn from_samples(op: Op, mut samples: Vec<Duration>) -> Self {
        assert!(!samples.is_empty(), "잰 값이 없음: {op:?}");
        samples.sort();
        let us = |d: Duration| d.as_micros() as u64;
        OpStats {
            op,
            count: samples.len(),
            p50_us: us(nearest_rank(&samples, 50.0)),
            p95_us: us(nearest_rank(&samples, 95.0)),
            p99_us: us(nearest_rank(&samples, 99.0)),
            max_us: us(*samples.last().expect("비지 않음")),
        }
    }
}

/// 정렬된 값의 nearest-rank 백분위(0 < p ≤ 100).
pub fn nearest_rank(sorted: &[Duration], p: f64) -> Duration {
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}
```

`crates\bench\src\report.rs`:
```rust
use crate::stats::{Op, OpStats};
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub enum Stat {
    P95,
    P99,
}

/// 설계 9-2의 예산 한 줄.
pub struct Budget {
    pub label: &'static str,
    pub op: Op,
    pub stat: Stat,
    pub limit_us: u64,
}

/// 설계 9-2 가운데 이 판이 재는 일곱. 묶음 적용은 P4에서 재고, 통째 사본 들이기 · 저장 공간은 보고만 한다(Report의 load_ms · bytes_on_disk).
pub const BUDGETS: [Budget; 7] = [
    Budget { label: "제목 찾기 p99 ≤ 10ms", op: Op::FindTitle, stat: Stat::P99, limit_us: 10_000 },
    Budget { label: "본문 찾기 첫 20건 p95 ≤ 10ms", op: Op::SearchBody, stat: Stat::P95, limit_us: 10_000 },
    Budget { label: "문서 열기 p95 ≤ 16ms", op: Op::Open, stat: Stat::P95, limit_us: 16_000 },
    Budget { label: "저장 한 편 p95 ≤ 50ms", op: Op::Save, stat: Stat::P95, limit_us: 50_000 },
    Budget { label: "분류 폴더 하나 펼치기 ≤ 16ms", op: Op::CategoryChildren, stat: Stat::P95, limit_us: 16_000 },
    Budget { label: "대문 · 통계 모음 ≤ 50ms", op: Op::HomeAggregates, stat: Stat::P95, limit_us: 50_000 },
    Budget { label: "앱 시작 → 대문 ≤ 1초(엔진 몫)", op: Op::Startup, stat: Stat::P95, limit_us: 1_000_000 },
];

#[derive(Clone, Debug, Serialize)]
pub struct Verdict {
    pub label: String,
    pub limit_us: u64,
    pub measured_us: u64,
    pub pass: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub library: String,
    pub device: String,
    pub docs: u32,
    pub seed: u64,
    pub load_ms: u64,
    pub bytes_on_disk: u64,
    pub ops: Vec<OpStats>,
    pub verdicts: Vec<Verdict>,
}

impl Report {
    pub fn judge(ops: &[OpStats]) -> Vec<Verdict> {
        BUDGETS
            .iter()
            .filter_map(|b| {
                let s = ops.iter().find(|o| o.op == b.op)?;
                let measured = match b.stat {
                    Stat::P95 => s.p95_us,
                    Stat::P99 => s.p99_us,
                };
                Some(Verdict { label: b.label.into(), limit_us: b.limit_us, measured_us: measured, pass: measured <= b.limit_us })
            })
            .collect()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("보고는 늘 JSON이 된다")
    }

    pub fn to_markdown(&self) -> String {
        let ms = |us: u64| format!("{:.2}ms", us as f64 / 1000.0);
        let mut s = format!(
            "# 측정 — {} · {} · 문서 {}편\n\n들이기 {:.1}초 · 저장 공간 {:.1}MB · 씨앗 {}\n\n## 예산표(설계 9-2)\n\n| 예산 | 잰 값 | 통과 |\n| --- | --- | --- |\n",
            self.library,
            self.device,
            self.docs,
            self.load_ms as f64 / 1000.0,
            self.bytes_on_disk as f64 / 1_048_576.0,
            self.seed
        );
        for v in &self.verdicts {
            s += &format!("| {} | {} | {} |\n", v.label, ms(v.measured_us), if v.pass { "통과" } else { "못 미침" });
        }
        s += "\n## 잰 값\n\n| 일 | 횟수 | p50 | p95 | p99 | 최대 |\n| --- | --- | --- | --- | --- | --- |\n";
        for o in &self.ops {
            s += &format!("| {} | {} | {} | {} | {} | {} |\n", o.op.label(), o.count, ms(o.p50_us), ms(o.p95_us), ms(o.p99_us), ms(o.max_us));
        }
        s
    }
}
```

`crates\bench\src\workload.rs`:
```rust
use crate::run::RunPlan;
use history_corpus::{Corpus, Rng};

/// 정해진 씨앗에서 뽑은 일감 — 어떤 구현이든 같은 일감을 받는다.
pub struct Workload {
    pub title_prefixes: Vec<String>,
    pub body_queries: Vec<String>,
    pub open_ids: Vec<u32>,
    pub saves: Vec<(u32, String)>,
    pub categories: Vec<String>,
}

impl Workload {
    pub fn new(c: &Corpus, plan: &RunPlan) -> Self {
        let mut r = Rng::for_item(plan.seed, 21, 0);
        let docs = c.spec().docs;
        let first_article = c.len() - docs; // 분류 문서가 앞 번호를 쓴다
        let title_prefixes = (0..plan.find_title)
            .map(|_| {
                let t = c.title(r.below(docs as u64) as u32);
                t.chars().take(1 + r.below(3) as usize).collect()
            })
            .collect();
        let body_queries = (0..plan.search_body).map(|_| query_from(&mut r, c)).collect();
        let open_ids = (0..plan.open).map(|_| first_article + r.below(docs as u64) as u32).collect();
        let saves = (0..plan.save)
            .map(|_| {
                let id = first_article + r.below(docs as u64) as u32;
                let mut body = c.doc(id).body;
                body.push_str(r.pick(&c.pool().sentences));
                body.push('\n');
                (id, body)
            })
            .collect();
        let categories = (0..plan.categories).map(|_| c.tree().get(*r.pick(c.tree().leaves())).name.clone()).collect();
        Workload { title_prefixes, body_queries, open_ids, saves, categories }
    }
}

/// 문장 속 한글 2~4글자 덩이 — 사람이 칠 만한 검색어.
fn query_from(r: &mut Rng, c: &Corpus) -> String {
    loop {
        let s = r.pick(&c.pool().sentences);
        let words: Vec<&str> = s.split(|ch: char| !('가'..='힣').contains(&ch)).filter(|w| (2..=4).contains(&w.chars().count())).collect();
        if !words.is_empty() {
            return r.pick(&words).to_string();
        }
    }
}
```

`crates\bench\src\run.rs`:
```rust
use crate::{
    report::Report,
    stats::{Op, OpStats},
    workload::Workload,
    Library, Result,
};
use history_corpus::Corpus;
use std::time::Instant;

/// 일마다 몇 번 잴지.
#[derive(Clone, Debug)]
pub struct RunPlan {
    pub seed: u64,
    pub find_title: usize,
    pub search_body: usize,
    pub open: usize,
    pub save: usize,
    pub categories: usize,
    pub aggregates: usize,
    pub startups: usize,
}

impl RunPlan {
    pub fn standard(seed: u64) -> Self {
        RunPlan { seed, find_title: 2_000, search_body: 1_000, open: 2_000, save: 300, categories: 500, aggregates: 50, startups: 5 }
    }
    pub fn quick(seed: u64) -> Self {
        RunPlan { seed, find_title: 50, search_body: 50, open: 50, save: 20, categories: 20, aggregates: 5, startups: 2 }
    }
}

fn measure(ops: &mut Vec<OpStats>, op: Op, n: usize, mut f: impl FnMut(usize) -> Result<()>) -> Result<()> {
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let t = Instant::now();
        f(i)?;
        samples.push(t.elapsed());
    }
    ops.push(OpStats::from_samples(op, samples));
    Ok(())
}

/// 들이고, 일감을 돌리고, 예산표와 견준 보고를 만든다.
pub fn run(lib: &mut dyn Library, corpus: &Corpus, plan: &RunPlan, device: &str) -> Result<Report> {
    let work = Workload::new(corpus, plan);
    let mut docs = corpus.docs();
    let t = Instant::now();
    lib.load(&mut docs)?;
    let load = t.elapsed();
    let mut ops = Vec::new();
    measure(&mut ops, Op::FindTitle, work.title_prefixes.len(), |i| lib.find_title(&work.title_prefixes[i], 20).map(drop))?;
    measure(&mut ops, Op::SearchBody, work.body_queries.len(), |i| lib.search_body(&work.body_queries[i], 20).map(drop))?;
    measure(&mut ops, Op::Open, work.open_ids.len(), |i| lib.open(work.open_ids[i]).map(drop))?;
    measure(&mut ops, Op::CategoryChildren, work.categories.len(), |i| lib.category_children(&work.categories[i]).map(drop))?;
    measure(&mut ops, Op::HomeAggregates, plan.aggregates, |_| lib.home_aggregates().map(drop))?;
    measure(&mut ops, Op::Save, work.saves.len(), |i| {
        let (id, body) = &work.saves[i];
        lib.save(*id, body)
    })?;
    measure(&mut ops, Op::Startup, plan.startups, |_| {
        lib.reopen()?;
        lib.home_aggregates().map(drop)
    })?;
    let verdicts = Report::judge(&ops);
    Ok(Report {
        library: lib.name().into(),
        device: device.into(),
        docs: corpus.spec().docs,
        seed: corpus.spec().seed,
        load_ms: load.as_millis() as u64,
        bytes_on_disk: lib.bytes_on_disk()?,
        ops,
        verdicts,
    })
}
```

`crates\bench\src\lib.rs`(통째로):
```rust
//! 100만 건 측정 판 — 서재 구현 하나(`Library`)를 같은 일감으로 재고 예산표(설계 9-2)와 견준다.
pub mod library;
pub mod report;
pub mod run;
pub mod stats;
pub mod workload;

pub use library::{Aggregates, Library, Result};
pub use report::{Budget, Report, Stat, Verdict, BUDGETS};
pub use run::{run, RunPlan};
pub use stats::{nearest_rank, Op, OpStats};
pub use workload::Workload;
```

- [ ] **Step 4: 검사가 통과하는지 본다**

Run: `cargo test -p history_bench --test run`
Expected: 4 passed.

- [ ] **Step 5: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "측정 판 history_bench — Library 약속 · 정해진 일감 · 백분위 · 설계 9-2 예산표 · JSON과 표 보고" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: 기준 서재 `NaiveSqlite`

**Files:**
- Create: `crates\bench\src\naive.rs`, `crates\bench\tests\naive.rs`
- Modify: `crates\bench\src\lib.rs`(`pub mod naive;`)

**Interfaces:**
- Consumes: Task 7의 `Library` · `Aggregates` · `Result`; Task 6의 `GenDoc` · `Ns`.
- Produces: `pub struct NaiveSqlite`; `NaiveSqlite::open(dir: &Path) -> Result<NaiveSqlite>`(파일 `naive.db`, WAL). `Library`를 구현한다(이름 `"naive-sqlite"`).

- [ ] **Step 1: 실패하는 검사를 쓴다**

`crates\bench\tests\naive.rs`:
```rust
use history_bench::{naive::NaiveSqlite, Library};
use history_corpus::{Corpus, CorpusSpec, TextPool};
use std::path::PathBuf;

/// 한글 · 공백이 든 작업 폴더(Review Focus 1).
fn workdir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("측정 시험 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// 이어진 한글 n글자(본문에 실제로 있는 검색어).
fn hangul_run(text: &str, n: usize) -> String {
    for w in text.split(|ch: char| !('가'..='힣').contains(&ch)) {
        if w.chars().count() >= n {
            return w.chars().take(n).collect();
        }
    }
    panic!("한글 {n}글자 이어진 곳이 없음");
}

#[test]
fn answers_like_a_library() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 2_000, seed: 4 }, &pool);
    let dir = workdir("기본");
    let mut lib = NaiveSqlite::open(&dir).unwrap();
    lib.load(&mut c.docs()).unwrap();
    let first = c.len() - 2_000;
    let id = first + 10;
    let body = c.doc(id).body;

    // 제목
    let prefix: String = c.title(10).chars().take(2).collect();
    let hits = lib.find_title(&prefix, 50).unwrap();
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|h| c.doc(*h).title.starts_with(&prefix)));

    // 본문 — 3글자(trigram) · 2글자(훑기)
    let q3 = hangul_run(&body, 3);
    let found = lib.search_body(&q3, 20).unwrap();
    assert!(!found.is_empty());
    assert!(found.iter().all(|h| c.doc(*h).body.contains(&q3)));
    let q2 = hangul_run(&body, 2);
    assert!(!lib.search_body(&q2, 20).unwrap().is_empty());

    // 열기 · 저장 · 다시 찾기
    assert_eq!(lib.open(id).unwrap().unwrap(), body);
    let new = format!("{body}새로 넣은 문장 가나다라마바사.\n");
    lib.save(id, &new).unwrap();
    assert_eq!(lib.open(id).unwrap().unwrap(), new);
    assert!(lib.search_body("가나다라마바사", 20).unwrap().contains(&id));

    // 분류 · 모음
    if let Some(home) = &c.doc(id).home {
        assert!(lib.category_children(home).unwrap().contains(&id));
    }
    let agg = lib.home_aggregates().unwrap();
    assert_eq!(agg.groups.iter().map(|g| g.1).sum::<u64>(), 2_000);
    assert_eq!(agg.recent[0], id);

    // 다시 열기 · 크기
    lib.reopen().unwrap();
    assert_eq!(lib.open(id).unwrap().unwrap(), new);
    assert!(lib.bytes_on_disk().unwrap() > 0);
    drop(lib);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn loading_again_starts_clean() {
    // Review Focus 3 — 끊긴 측정이 남긴 DB 위에 다시 들인다
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 500, seed: 6 }, &pool);
    let dir = workdir("다시");
    let mut lib = NaiveSqlite::open(&dir).unwrap();
    lib.load(&mut c.docs().take(100)).unwrap();
    drop(lib);
    let mut lib = NaiveSqlite::open(&dir).unwrap();
    lib.load(&mut c.docs()).unwrap();
    let agg = lib.home_aggregates().unwrap();
    assert_eq!(agg.groups.iter().map(|g| g.1).sum::<u64>(), 500);
    drop(lib);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn saving_a_missing_doc_is_an_error_not_a_crash() {
    let pool = TextPool::embedded();
    let c = Corpus::new(CorpusSpec { docs: 100, seed: 1 }, &pool);
    let dir = workdir("없는 문서");
    let mut lib = NaiveSqlite::open(&dir).unwrap();
    lib.load(&mut c.docs()).unwrap();
    assert!(lib.save(9_999_999, "x").is_err());
    assert_eq!(lib.open(9_999_999).unwrap(), None);
    drop(lib);
    std::fs::remove_dir_all(&dir).unwrap();
}
```

- [ ] **Step 2: 검사가 실패하는지 본다**

Run: `cargo test -p history_bench --test naive`
Expected: 컴파일 실패 — `could not find naive in history_bench`.

- [ ] **Step 3: 구현한다**

`crates\bench\src\lib.rs`에 `pub mod naive;`를 더한다.

`crates\bench\src\naive.rs`:
```rust
//! P1의 기준 구현 — SQLite 하나에 본문 그대로 + FTS5 trigram. 서재 엔진이 넘어야 할 바닥 수치를 낸다.
use crate::{Aggregates, Library, Result};
use history_corpus::{GenDoc, Ns};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

const FILE: &str = "naive.db";
const PRAGMAS: &str = "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;";
const SCHEMA: &str = "
CREATE TABLE doc(id INTEGER PRIMARY KEY, ns INTEGER NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL, grp TEXT, updated INTEGER NOT NULL, UNIQUE(ns, title));
CREATE INDEX doc_updated ON doc(updated DESC);
CREATE TABLE cat_member(cat TEXT NOT NULL, doc INTEGER NOT NULL, PRIMARY KEY(cat, doc)) WITHOUT ROWID;
CREATE VIRTUAL TABLE fts USING fts5(body, content='doc', content_rowid='id', tokenize='trigram');
";

pub struct NaiveSqlite {
    dir: PathBuf,
    db: Connection,
    clock: i64,
}

fn connect(path: &Path) -> Result<Connection> {
    let db = Connection::open(path)?;
    db.execute_batch(PRAGMAS)?;
    Ok(db)
}

/// 본문 첫 줄의 [[분류:…]]들 — 첫 것이 집.
fn categories(body: &str) -> Vec<&str> {
    let first = body.lines().next().unwrap_or("");
    first.split("[[분류:").skip(1).filter_map(|p| p.split("]]").next()).collect()
}

fn ids(rows: impl Iterator<Item = rusqlite::Result<u32>>) -> Result<Vec<u32>> {
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

impl NaiveSqlite {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let db = connect(&dir.join(FILE))?;
        let clock = db
            .query_row("SELECT COALESCE(MAX(updated), 0) FROM doc", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(NaiveSqlite { dir: dir.to_path_buf(), db, clock })
    }

    /// 옛 파일을 모두 지우고 빈 표로 다시 연다(끊긴 측정이 남긴 것 포함).
    fn reset(&mut self) -> Result<()> {
        self.db = Connection::open_in_memory()?; // 옛 파일을 놓는다
        for f in [FILE, "naive.db-wal", "naive.db-shm"] {
            let p = self.dir.join(f);
            if p.exists() {
                std::fs::remove_file(p)?;
            }
        }
        let db = connect(&self.dir.join(FILE))?;
        db.execute_batch(SCHEMA)?;
        self.db = db;
        self.clock = 0;
        Ok(())
    }
}

impl Library for NaiveSqlite {
    fn name(&self) -> &'static str {
        "naive-sqlite"
    }

    fn load(&mut self, docs: &mut dyn Iterator<Item = GenDoc>) -> Result<()> {
        self.reset()?;
        let mut n = 0i64;
        self.db.execute_batch("BEGIN")?;
        {
            let mut ins = self.db.prepare("INSERT INTO doc(id, ns, title, body, grp, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
            let mut mem = self.db.prepare("INSERT OR IGNORE INTO cat_member(cat, doc) VALUES (?1, ?2)")?;
            for d in docs {
                n += 1;
                let ns = if d.ns == Ns::Category { 1 } else { 0 };
                ins.execute(params![d.id, ns, d.title, d.body, d.group, n])?;
                if d.ns == Ns::Doc {
                    for c in categories(&d.body) {
                        mem.execute(params![c, d.id])?;
                    }
                }
                if n % 20_000 == 0 {
                    self.db.execute_batch("COMMIT; BEGIN")?;
                }
            }
        }
        self.db.execute_batch("COMMIT")?;
        self.db.execute_batch("INSERT INTO fts(fts) VALUES('rebuild'); PRAGMA wal_checkpoint(TRUNCATE);")?;
        self.clock = n;
        Ok(())
    }

    fn find_title(&self, prefix: &str, limit: usize) -> Result<Vec<u32>> {
        let upper = format!("{prefix}\u{10FFFF}");
        let mut st = self.db.prepare_cached("SELECT id FROM doc WHERE ns = 0 AND title >= ?1 AND title < ?2 ORDER BY title LIMIT ?3")?;
        let rows = st.query_map(params![prefix, upper, limit as i64], |r| r.get::<_, u32>(0))?;
        ids(rows)
    }

    fn search_body(&self, query: &str, limit: usize) -> Result<Vec<u32>> {
        if query.chars().count() >= 3 {
            let phrase = format!("\"{}\"", query.replace('"', "\"\""));
            let mut st = self.db.prepare_cached("SELECT rowid FROM fts WHERE fts MATCH ?1 LIMIT ?2")?;
            let rows = st.query_map(params![phrase, limit as i64], |r| r.get::<_, u32>(0))?;
            ids(rows)
        } else {
            // trigram은 3글자 미만을 못 찾는다 — 기준 구현은 통째로 훑는다(엔진이 2글자 조각 색인으로 넘을 곳).
            let mut st = self.db.prepare_cached("SELECT id FROM doc WHERE body LIKE ?1 LIMIT ?2")?;
            let rows = st.query_map(params![format!("%{query}%"), limit as i64], |r| r.get::<_, u32>(0))?;
            ids(rows)
        }
    }

    fn open(&self, id: u32) -> Result<Option<String>> {
        let mut st = self.db.prepare_cached("SELECT body FROM doc WHERE id = ?1")?;
        let body = st.query_row([id], |r| r.get(0)).optional()?;
        Ok(body)
    }

    fn save(&mut self, id: u32, body: &str) -> Result<()> {
        let now = self.clock + 1;
        let tx = self.db.transaction()?;
        let old: Option<String> = tx.query_row("SELECT body FROM doc WHERE id = ?1", [id], |r| r.get(0)).optional()?;
        let Some(old) = old else {
            return Err(format!("없는 문서 {id}").into());
        };
        tx.execute("INSERT INTO fts(fts, rowid, body) VALUES('delete', ?1, ?2)", params![id, old])?;
        tx.execute("UPDATE doc SET body = ?2, updated = ?3 WHERE id = ?1", params![id, body, now])?;
        tx.execute("INSERT INTO fts(rowid, body) VALUES (?1, ?2)", params![id, body])?;
        tx.commit()?;
        self.clock = now;
        Ok(())
    }

    fn category_children(&self, category: &str) -> Result<Vec<u32>> {
        let mut st = self.db.prepare_cached("SELECT doc FROM cat_member WHERE cat = ?1")?;
        let rows = st.query_map([category], |r| r.get::<_, u32>(0))?;
        ids(rows)
    }

    fn home_aggregates(&self) -> Result<Aggregates> {
        let mut st = self.db.prepare_cached("SELECT COALESCE(grp, '분류 안 됨'), COUNT(*) FROM doc WHERE ns = 0 GROUP BY 1 ORDER BY 2 DESC")?;
        let groups = st
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut st = self.db.prepare_cached("SELECT id FROM doc WHERE ns = 0 ORDER BY updated DESC LIMIT 10")?;
        let recent = ids(st.query_map([], |r| r.get::<_, u32>(0))?)?;
        Ok(Aggregates { groups, recent })
    }

    fn reopen(&mut self) -> Result<()> {
        self.db = Connection::open_in_memory()?;
        self.db = connect(&self.dir.join(FILE))?;
        Ok(())
    }

    fn bytes_on_disk(&self) -> Result<u64> {
        let mut total = 0;
        for f in [FILE, "naive.db-wal", "naive.db-shm"] {
            if let Ok(m) = std::fs::metadata(self.dir.join(f)) {
                total += m.len();
            }
        }
        Ok(total)
    }
}
```
`open`이 빈 폴더(표 없음)에서 `MAX(updated)`를 물으면 오류가 나므로 `unwrap_or(0)`로 받는다.

- [ ] **Step 4: 검사가 통과하는지 본다**

Run: `cargo test -p history_bench`
Expected: run 4 · naive 3 passed. FTS5 trigram이 없다는 오류(`no such tokenizer: trigram`)가 나면 rusqlite `bundled`가 FTS5를 켜는지 확인한다 — `features = ["bundled", "modern_sqlite"]`로 바꾸고 다시 돌린다(둘 다 실패하면 멈추고 보고한다).

- [ ] **Step 5: 커밋한다**

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "기준 서재 NaiveSqlite — 본문 그대로 + FTS5 trigram, 끊긴 측정 위에서도 새로 들임" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: `hc-bench` 명령 · PC 기준 수치(10만 · 100만) · 폰 측정

**Files:**
- Modify: `crates\bench\src\main.rs`
- Create: `crates\bench\tests\cli.rs`, `tools\bench-android.ps1`, `bench\README.md`
- Create(측정 결과, 커밋): `bench\results\pc-naive-sqlite-100000.{json,md}`, `bench\results\pc-naive-sqlite-1000000.{json,md}`, (폰이 있으면) `bench\results\<기종>-naive-sqlite-100000.{json,md}`

**Interfaces:**
- Consumes: Task 7 · 8 전부.
- Produces: 명령 `hc-bench run --docs <수> --dir <작업 폴더> [--seed <수>(기본 20261021)] [--device <이름>(기본 pc)] [--out <결과 폴더>(기본 bench/results)] [--quick]` → `<out>/<device>-<library>-<docs>.{json,md}`, 표준 출력에 표.

- [ ] **Step 1: 실패하는 검사를 쓴다**

`crates\bench\tests\cli.rs`:
```rust
use std::process::Command;

#[test]
fn runs_quick_and_writes_both_reports() {
    let work = std::env::temp_dir().join(format!("측정 명령 {}", std::process::id()));
    let out = work.join("결과");
    let _ = std::fs::remove_dir_all(&work);
    let status = Command::new(env!("CARGO_BIN_EXE_hc-bench"))
        .args(["run", "--docs", "300", "--quick", "--device", "시험", "--dir"])
        .arg(work.join("자리"))
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    let md = std::fs::read_to_string(out.join("시험-naive-sqlite-300.md")).unwrap();
    assert!(md.contains("## 예산표(설계 9-2)"));
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out.join("시험-naive-sqlite-300.json")).unwrap()).unwrap();
    assert_eq!(json["docs"], 300);
    std::fs::remove_dir_all(&work).unwrap();
}

#[test]
fn bad_arguments_exit_nonzero_with_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_hc-bench")).arg("nope").output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("쓰는 법"));
}
```

- [ ] **Step 2: 검사가 실패하는지 본다**

Run: `cargo test -p history_bench --test cli`
Expected: 2 failed(빈 `main`이라 파일이 없고 종료 코드가 0).

- [ ] **Step 3: 명령을 구현한다**

`crates\bench\src\main.rs`:
```rust
//! hc-bench — 더미 서재를 들이고 일감을 재어 예산표(설계 9-2)와 견준다.
//!   hc-bench run --docs 100000 --dir <작업 폴더> [--seed 20261021] [--device pc] [--out bench/results] [--quick]
use history_bench::{naive::NaiveSqlite, run, Result, RunPlan};
use history_corpus::{Corpus, CorpusSpec, TextPool};
use std::path::PathBuf;

const USAGE: &str = "쓰는 법: hc-bench run --docs <수> --dir <작업 폴더> [--seed <수>] [--device <이름>] [--out <결과 폴더>] [--quick]";

fn main() {
    if let Err(e) = real_main() {
        eprintln!("hc-bench: {e}");
        std::process::exit(1);
    }
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn real_main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("run") {
        return Err(USAGE.into());
    }
    let docs: u32 = arg(&args, "--docs").ok_or(USAGE)?.parse()?;
    let dir = PathBuf::from(arg(&args, "--dir").ok_or(USAGE)?);
    let seed: u64 = arg(&args, "--seed").map(|s| s.parse()).transpose()?.unwrap_or(20_261_021);
    let device = arg(&args, "--device").unwrap_or_else(|| "pc".into());
    let out = PathBuf::from(arg(&args, "--out").unwrap_or_else(|| "bench/results".into()));
    let plan = if args.iter().any(|a| a == "--quick") { RunPlan::quick(seed) } else { RunPlan::standard(seed) };

    let pool = TextPool::embedded();
    eprintln!("더미 서재 {docs}편을 준비한다(씨앗 {seed})…");
    let corpus = Corpus::new(CorpusSpec { docs, seed }, &pool);
    let mut lib = NaiveSqlite::open(&dir)?;
    eprintln!("들이고 재는 중 — {}", dir.display());
    let report = run(&mut lib, &corpus, &plan, &device)?;
    std::fs::create_dir_all(&out)?;
    let stem = format!("{}-{}-{}", device, report.library, docs);
    std::fs::write(out.join(format!("{stem}.json")), report.to_json())?;
    std::fs::write(out.join(format!("{stem}.md")), report.to_markdown())?;
    println!("{}", report.to_markdown());
    Ok(())
}
```

- [ ] **Step 4: 검사가 통과하는지 본다**

Run: `cargo test --locked --workspace`
Expected: 모두 통과(library 1 · ffi 7 · corpus 8 · bench run 4 · naive 3 · cli 2).

- [ ] **Step 5: PC 기준 수치를 잰다**

```powershell
cargo run --locked --release -p history_bench --bin hc-bench -- run --docs 100000 --dir D:\hc-bench-work --device pc
cargo run --locked --release -p history_bench --bin hc-bench -- run --docs 1000000 --dir D:\hc-bench-work --device pc
```
Expected: `bench\results\pc-naive-sqlite-100000.{json,md}` · `pc-naive-sqlite-1000000.{json,md}`. 더미 문서는 한 편에 약 7KB라 100만은 본문만 7GB 안팎이고 trigram 색인이 그 몇 배다 — **먼저 D: 여유 공간이 40GB 넘는지 본다**(계획을 쓸 때 약 98GB). 모자라면 멈추고 감독에게 묻는다. 100만은 들이기에 한 시간 가까이 걸릴 수 있다(백그라운드로 돌리고 끝 알림을 기다린다 — 2분 넘는 명령을 앞에서 붙잡고 있지 않는다). 작업 폴더 `D:\hc-bench-work`는 저장소 밖이다 — 다 잰 뒤 지운다(`Remove-Item -Recurse -Force D:\hc-bench-work`). 기준 구현은 예산을 못 넘는 줄이 있는 것이 정상이다(엔진이 넘을 바닥을 재는 것이 목적).

- [ ] **Step 6: 폰 측정 스크립트를 쓰고(폰이 연결돼 있으면) 잰다**

`tools\bench-android.ps1`:
```powershell
# 폰에서 hc-bench를 돌린다(USB 디버깅으로 연결된 폰 한 대).
#   ./tools/bench-android.ps1 -Docs 100000
#   ./tools/bench-android.ps1 -Docs 1000 -Quick      # 길 확인만
param([Parameter(Mandatory)][int]$Docs, [switch]$Quick)
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
$adb = Join-Path $sdk 'platform-tools\adb.exe'
$ndk = Join-Path $sdk 'ndk\28.2.13676358\toolchains\llvm\prebuilt\windows-x86_64\bin'
if (-not (Test-Path -LiteralPath $ndk)) { throw "Android NDK 28.2.13676358이 필요합니다: $ndk" }
$target = 'aarch64-linux-android'
$key = $target.Replace('-', '_')
$linker = Join-Path $ndk "$($target)24-clang.cmd"
[Environment]::SetEnvironmentVariable("CC_$key", $linker, 'Process')
[Environment]::SetEnvironmentVariable("AR_$key", (Join-Path $ndk 'llvm-ar.exe'), 'Process')
[Environment]::SetEnvironmentVariable("CARGO_TARGET_$($key.ToUpper())_LINKER", $linker, 'Process')
Push-Location $repo  # rust-toolchain.toml(1.94.1)을 따르도록 저장소 안에서 짓는다
try { & cargo build --locked --release -p history_bench --bin hc-bench --target $target } finally { Pop-Location }
if ($LASTEXITCODE -ne 0) { throw 'hc-bench(Android) 빌드 실패' }

$devices = @(& $adb devices | Select-String -Pattern "`tdevice$")
if ($devices.Count -ne 1) { throw "연결된 폰이 한 대여야 합니다(지금 $($devices.Count)대). USB 디버깅을 켜고 다시." }
$model = ((& $adb shell getprop ro.product.model) | Out-String).Trim() -replace '[^\w-]', '_'
& $adb push (Join-Path $repo "target\$target\release\hc-bench") /data/local/tmp/hc-bench | Out-Null
& $adb shell chmod 755 /data/local/tmp/hc-bench
$quickArg = if ($Quick) { '--quick' } else { '' }
& $adb shell "cd /data/local/tmp && ./hc-bench run --docs $Docs --dir /data/local/tmp/hcb-work --device $model --out /data/local/tmp/hcb-out $quickArg"
if ($LASTEXITCODE -ne 0) { throw '폰 측정 실패' }
$dest = Join-Path $repo 'bench\results'
New-Item -ItemType Directory -Force -Path $dest | Out-Null
& $adb pull /data/local/tmp/hcb-out/. $dest | Out-Null
& $adb shell rm -rf /data/local/tmp/hcb-work /data/local/tmp/hcb-out
Write-Output "결과: $dest"
```

BOM을 붙인다(Global Constraints).

Run(감독의 폰이 USB 디버깅으로 연결돼 있을 때만): `./tools/bench-android.ps1 -Docs 1000 -Quick` 뒤 `./tools/bench-android.ps1 -Docs 100000`. 폰에 10만 편이 들 자리(약 3GB)가 있어야 한다.
Expected: `bench\results\<기종>-naive-sqlite-100000.{json,md}`. 폰이 없으면 이 Step의 실행만 건너뛰고 보고에 "폰 측정 남음"으로 적는다(스크립트는 커밋한다).

- [ ] **Step 7: 결과를 적고 커밋한다**

`bench\README.md`:
```markdown
# 측정 판

설계 9-1 P1 · 9-2 예산표. 같은 더미 서재(씨앗 20261021)와 같은 일감으로 서재 구현을 재어 견준다. 구현은 `history_bench::Library`에 꽂는다 — P1은 기준 구현 `naive-sqlite`(본문 그대로 + FTS5 trigram), P2부터 서재 엔진.

    cargo run --locked --release -p history_bench --bin hc-bench -- run --docs 1000000 --dir D:\hc-bench-work --device pc
    ./tools/bench-android.ps1 -Docs 100000

| 결과 | 무엇 |
| --- | --- |
| `results/pc-naive-sqlite-100000.md` | PC · 10만 |
| `results/pc-naive-sqlite-1000000.md` | PC · 100만 — P2 엔진이 넘어야 할 바닥 |
| `results/<기종>-naive-sqlite-100000.md` | 폰 · 10만(잰 경우) |

예산표에서 아직 재지 않는 것: 바뀐 내역 묶음 적용(P4). 통째 사본 들이기는 표의 「들이기」 시간으로, 저장 공간은 「저장 공간」으로 보고한다.
```
그 아래에 PC 100만의 예산표 줄(통과 · 못 미침)을 표로 옮겨 적는다(`results/pc-naive-sqlite-1000000.md`에서 복사).

```powershell
git add -A
git -c user.name=Claude -c user.email=noreply@anthropic.com commit -m "hc-bench 명령과 기준 수치 — PC 10만 · 100만(+폰), P2 엔진이 넘을 바닥" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 8: P1 끝을 확인한다**

설계 9-1 P1의 끝: "측정기가 돌고 9-2와 견줄 기준 수치가 나옴". `bench\results`에 PC 10만 · 100만 보고가 있고, `cargo test --locked --workspace`가 통과하면 끝. 감독에게 PC 100만 예산표를 그대로 보여 주고, 폰 측정이 남았으면 그 사실을 함께 적는다. HistoryCore의 새 커밋은 감독이 민다(Task 5 Step 4와 같은 길).
