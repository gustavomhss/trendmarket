use std::collections::HashSet;

use credit_engine_core::amm::error::AmmError;
use credit_engine_core::amm::error_catalog::AmmErrorCode;

fn extract_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let mut chars = json[start..].chars();
    let mut value = String::new();
    let mut escape = false;
    while let Some(ch) = chars.next() {
        if escape {
            match ch {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                'u' => {
                    let mut digits = String::new();
                    for _ in 0..4 {
                        if let Some(d) = chars.next() {
                            digits.push(d);
                        } else {
                            return None;
                        }
                    }
                    if let Ok(codepoint) = u16::from_str_radix(&digits, 16) {
                        if let Some(chr) = char::from_u32(codepoint as u32) {
                            value.push(chr);
                        }
                    }
                }
                other => value.push(other),
            }
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => return Some(value),
            _ => value.push(ch),
        }
    }
    None
}

fn extract_object_body(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":{{", key);
    let start = json.find(&pattern)? + pattern.len();
    let mut depth = 1i32;
    let mut in_string = false;
    let mut escape = false;
    let mut body = String::new();
    for ch in json[start..].chars() {
        if escape {
            body.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_string => {
                body.push(ch);
                escape = true;
            }
            '"' => {
                in_string = !in_string;
                body.push(ch);
            }
            '{' if !in_string => {
                depth += 1;
                body.push(ch);
            }
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(body);
                }
                body.push(ch);
            }
            _ => body.push(ch),
        }
    }
    None
}

#[test]
fn json_shape_per_code() {
    for code in AmmErrorCode::all() {
        let err = AmmError::new(*code);
        let json = err.to_log_json();
        assert!(json.starts_with('{'));
        assert_eq!(extract_string(&json, "code").as_deref(), Some(code.code()));
        assert_eq!(
            extract_string(&json, "title").as_deref(),
            Some(code.title())
        );
        assert_eq!(
            extract_string(&json, "message").as_deref(),
            Some(code.message_pt())
        );
        assert!(extract_object_body(&json, "context").is_some());
    }
}

#[test]
fn all_codes_seen() {
    let mut seen = HashSet::new();
    for code in AmmErrorCode::all() {
        let err = AmmError::new(*code);
        let json = err.to_log_json();
        if let Some(code_str) = extract_string(&json, "code") {
            seen.insert(code_str);
        }
    }
    assert_eq!(seen.len(), 5);
}