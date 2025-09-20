//! Tipo de erro unificado do AMM com formatação estável.
use core::fmt;
use std::collections::BTreeMap;

use crate::amm::error_catalog::{default_locale_message, AmmErrorCode};

const CONTEXT_VALUE_MAX: usize = 256;

fn sanitize_value(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\n' | '\r' | '\t' => cleaned.push(' '),
            _ => cleaned.push(ch),
        }
    }
    if cleaned.len() > CONTEXT_VALUE_MAX {
        let mut truncated = cleaned
            .chars()
            .take(CONTEXT_VALUE_MAX - 1)
            .collect::<String>();
        truncated.push('…');
        truncated
    } else {
        cleaned
    }
}

fn escape_json(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            c if c.is_control() => {
                use core::fmt::Write as _;
                let _ = write!(&mut escaped, "\\u{:04x}", c as u32);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn render_template(template: &str, context: &BTreeMap<String, String>) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut key = String::new();
            while let Some(next) = chars.next() {
                if next == '}' {
                    break;
                }
                key.push(next);
            }
            if key.is_empty() || !context.contains_key(&key) {
                rendered.push('{');
                rendered.push_str(&key);
                rendered.push('}');
            } else if let Some(value) = context.get(&key) {
                rendered.push_str(value);
            }
        } else {
            rendered.push(ch);
        }
    }
    rendered
}

/// Erro do AMM com contexto estruturado.
#[derive(Debug, Clone)]
pub struct AmmError {
    pub code: AmmErrorCode,
    pub context: BTreeMap<String, String>,
}

impl AmmError {
    /// Cria um novo erro sem contexto adicional.
    pub fn new(code: AmmErrorCode) -> Self {
        Self {
            code,
            context: BTreeMap::new(),
        }
    }

    /// Adiciona um par chave/valor ao contexto.
    pub fn with_context<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: ToString,
    {
        let key_string = key.into();
        if !key_string.is_empty() {
            let sanitized = sanitize_value(&value.to_string());
            self.context.insert(key_string, sanitized);
        }
        self
    }

    fn resolved_message(&self) -> String {
        let template = default_locale_message(self.code);
        render_template(template, &self.context)
    }

    /// Mensagem curta para UI.
    pub fn to_user_string(&self) -> String {
        let message = self.resolved_message();
        format!("[{}] {}", self.code.code(), message)
    }

    /// Renderiza um template arbitrário usando o contexto atual.
    pub fn render_with_template(&self, template: &str) -> String {
        render_template(template, &self.context)
    }

    /// Serialização estável em JSON para logs.
    pub fn to_log_json(&self) -> String {
        let message = self.resolved_message();
        let mut json = String::from("{");
        json.push_str("\"code\":\"");
        json.push_str(&escape_json(self.code.code()));
        json.push_str("\",\"title\":\"");
        json.push_str(&escape_json(self.code.title()));
        json.push_str("\",\"message\":\"");
        json.push_str(&escape_json(&message));
        json.push_str("\",\"context\":{");
        let mut first = true;
        for (key, value) in &self.context {
            if !first {
                json.push(',');
            }
            first = false;
            json.push('"');
            json.push_str(&escape_json(key));
            json.push_str("\":\"");
            json.push_str(&escape_json(value));
            json.push('"');
        }
        json.push_str("}}");
        json
    }
}

impl fmt::Display for AmmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_user_string())
    }
}

impl std::error::Error for AmmError {}

/// Resultado padrão para operações do AMM.
pub type Result<T> = std::result::Result<T, AmmError>;

#[macro_export]
macro_rules! amm_err {
  ($code:expr) => {{
    $crate::amm::error::AmmError::new($code)
  }};
  ($code:expr, $($key:ident => $value:expr),+ $(,)?) => {{
    let mut err = $crate::amm::error::AmmError::new($code);
    $(
      err = err.with_context(stringify!($key), $value);
    )+
    err
  }};
  ($code:expr, { $($key:expr => $value:expr),+ $(,)? }) => {{
    let mut err = $crate::amm::error::AmmError::new($code);
    $(
      err = err.with_context($key, $value);
    )+
    err
  }};
  ($code:expr, $($key:expr => $value:expr),+ $(,)?) => {{
    let mut err = $crate::amm::error::AmmError::new($code);
    $(
      err = err.with_context($key, $value);
    )+
    err
  }};
}

#[macro_export]
macro_rules! amm_bail {
  ($($tt:tt)*) => {
    return Err($crate::amm_err!($($tt)*));
  };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_string_basic() {
        let err = AmmError::new(AmmErrorCode::ZeroAmount);
        assert_eq!(err.to_user_string(), "[AMM-0001] amount deve ser > 0");
    }

    #[test]
    fn placeholder_subst() {
        let err = AmmError::new(AmmErrorCode::OverflowNumeric).with_context("detalhe", "valor");
        let rendered = err.render_with_template("falha {detalhe}");
        assert_eq!(rendered, "falha valor");
    }

    #[test]
    fn log_json_shape() {
        let err = AmmError::new(AmmErrorCode::ZeroReserve).with_context("reserve", "0");
        let json = err.to_log_json();
        assert!(json.starts_with('{'));
        assert!(json.contains("\"code\":"));
        assert!(json.contains("\"title\":"));
        assert!(json.contains("\"message\":"));
        assert!(json.contains("\"context\":"));
    }

    #[test]
    fn macros_variants() {
        let err = amm_err!(AmmErrorCode::ZeroAmount, amount => 0);
        assert_eq!(err.code, AmmErrorCode::ZeroAmount);
        assert_eq!(err.context.get("amount").unwrap(), "0");

        let err_block = amm_err!(AmmErrorCode::ZeroReserve, { "reserve" => 0 });
        assert_eq!(err_block.code, AmmErrorCode::ZeroReserve);
        assert_eq!(err_block.context.get("reserve").unwrap(), "0");
    }
}