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
