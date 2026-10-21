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
