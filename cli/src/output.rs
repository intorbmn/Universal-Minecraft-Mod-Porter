//! Stable CLI JSON envelopes. Human-readable command output stays on stderr in JSON mode.

use serde::Serialize;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

static JSON_MODE: AtomicBool = AtomicBool::new(false);

thread_local! {
    static RESULT: RefCell<Option<Value>> = const { RefCell::new(None) };
}

pub fn set_json_mode(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::Relaxed);
}

#[must_use]
pub fn is_json_mode() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

pub fn set_result<T: Serialize>(value: T) {
    RESULT.with(|result| *result.borrow_mut() = serde_json::to_value(value).ok());
}

pub fn success(command: &str, exit_code: i32) {
    let result = RESULT.with(|result| result.borrow_mut().take())
        .unwrap_or_else(|| json!({ "exit_code": exit_code }));
    println!(
        "{}",
        json!({
            "schema": 1,
            "command": command,
            "result": result,
        })
    );
}

pub fn failure(kind: &str, message: &str) {
    println!(
        "{}",
        json!({
            "schema": 1,
            "error": {
                "kind": kind,
                "message": message,
            }
        })
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_envelope_has_stable_top_level_keys() {
        let value = json!({
            "schema": 1,
            "command": "scan",
            "result": {"level": "low"},
        });
        let mut keys = value.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
        keys.sort();
        assert_eq!(keys, vec!["command", "result", "schema"]);
    }

    #[test]
    fn error_envelope_has_stable_shape() {
        let value = json!({
            "schema": 1,
            "error": {"kind": "invalid_input", "message": "bad path"},
        });
        assert_eq!(value["schema"], 1);
        assert_eq!(value["error"]["kind"], "invalid_input");
        assert!(value["error"]["message"].is_string());
    }
}
