//! Flutter 다리 — C 함수 둘(`hc_call` · `hc_free`). 요청과 답은 UTF-8 JSON이다.
//! 답은 실패해도 늘 `{"ok":false,"error":…}`이고 널을 돌려주지 않는다.
mod ops;

use serde_json::{json, Value};
use std::ffi::{c_char, CStr, CString};

/// 요청 하나를 처리해 답 JSON 글을 낸다. 그리기 모델은 풀지 않고 글 그대로 끼운다(깊은 중첩도 그대로 — P2-2 미룬 것).
pub fn handle(request: &str) -> String {
    match handle_plain(request).unwrap_or_else(ops::Reply::Json) {
        ops::Reply::Json(v) => v.to_string(),
        ops::Reply::WithModel(v, model) => {
            // `v`는 늘 열쇠가 있는 객체다 — 닫는 괄호 앞에 `"model":<글>`을 넣는다.
            let mut s = v.to_string();
            s.pop();
            s.push_str(",\"model\":");
            s.push_str(&model);
            s.push('}');
            s
        }
    }
}

/// 요청 하나를 처리한다. version · echo는 상태가 없고, 서재 op는 손잡이 표(Mutex) 하나를 나눠 쓴다 — 여러 스레드에서 불러도 된다.
fn handle_plain(request: &str) -> Result<ops::Reply, Value> {
    let req: Value = serde_json::from_str(request).map_err(|e| json!({"ok": false, "error": format!("요청 JSON을 읽지 못함: {e}")}))?;
    Ok(match req.get("op").and_then(Value::as_str) {
        Some("version") => ops::Reply::Json(json!({
            "ok": true,
            "engine": history_library::ENGINE_VERSION,
            "abi": history_library::ABI_VERSION,
        })),
        // 다리 점검용 — 받은 글을 그대로 돌려준다(4바이트 글자 · 이스케이프 확인).
        Some("echo") => ops::Reply::Json(json!({"ok": true, "text": req.get("text").cloned().unwrap_or(Value::Null)})),
        Some(op) => ops::handle_op(op, &req).unwrap_or_else(|| ops::Reply::Json(json!({"ok": false, "error": format!("모르는 op: {op}")}))),
        None => ops::Reply::Json(json!({"ok": false, "error": "op가 없음"})),
    })
}

fn to_c(text: String) -> *mut c_char {
    // JSON 직렬화는 문자열 속 NUL을 \u0000으로 적으므로 결과에 NUL 바이트가 없다(모델 JSON도 serde_json이 적은 글이다).
    CString::new(text).expect("JSON 글에는 NUL 바이트가 없다").into_raw()
}

/// # Safety
/// `request`는 NUL로 끝나는 UTF-8 글이거나 널이다. 돌려준 포인터는 반드시 `hc_free`로 한 번 놓는다.
#[no_mangle]
pub unsafe extern "C" fn hc_call(request: *const c_char) -> *mut c_char {
    if request.is_null() {
        return to_c(json!({"ok": false, "error": "요청이 널"}).to_string());
    }
    let text = match CStr::from_ptr(request).to_str() {
        Ok(t) => t,
        Err(_) => return to_c(json!({"ok": false, "error": "요청이 UTF-8이 아님"}).to_string()),
    };
    let answer = std::panic::catch_unwind(|| handle(text))
        .unwrap_or_else(|_| json!({"ok": false, "error": "엔진 안에서 멈춤(panic)"}).to_string());
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
