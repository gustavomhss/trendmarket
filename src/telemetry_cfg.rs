use std::env::{self, VarError};
use std::fmt;
use std::time::Duration;

const DEFAULT_SERVICE_NAME: &str = "ce-amm";
const DEFAULT_SERVICE_VERSION: &str = "0.0.0-dev";
const DEFAULT_METRICS_HTTP_ADDR: &str = "0.0.0.0:9464";
const DEFAULT_LOG_LEVEL: &str = "info";

const SERVICE_NAME_EXPECTED: &str =
    "lowercase service identifier (3-64 chars) matching ^[a-z0-9._-]{3,64}$";
const SERVICE_VERSION_EXPECTED: &str = "non-empty version string up to 64 characters";
const METRICS_ADDR_EXPECTED: &str =
    "host:port with host as IPv4 or alphanumeric/._- and port between 10 and 65535";
const OTLP_ENDPOINT_EXPECTED: &str = "URL starting with http:// or https:// followed by host:port";
const OTLP_TIMEOUT_EXPECTED: &str =
    "positive integer number of milliseconds (OTLP exporter timeout)";
const LOG_LEVEL_EXPECTED: &str = "one of trace, debug, info, warn, error";
const BOOL_EXPECTED: &str = "one of on, off, true, false, 1, 0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObsLevel {
    Off,
    Min,
    Full,
}

impl ObsLevel {
    fn parse(input: &str) -> Result<Self, &'static str> {
        match input {
            x if x.eq_ignore_ascii_case("off") => Ok(ObsLevel::Off),
            x if x.eq_ignore_ascii_case("min") => Ok(ObsLevel::Min),
            x if x.eq_ignore_ascii_case("full") => Ok(ObsLevel::Full),
            _ => Err("expected values: off|min|full"),
        }
    }
}

impl fmt::Display for ObsLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObsLevel::Off => f.write_str("off"),
            ObsLevel::Min => f.write_str("min"),
            ObsLevel::Full => f.write_str("full"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployEnv {
    Dev,
    Stg,
    Prod,
}

impl DeployEnv {
    fn parse(input: &str) -> Result<Self, &'static str> {
        match input {
            x if x.eq_ignore_ascii_case("dev") => Ok(DeployEnv::Dev),
            x if x.eq_ignore_ascii_case("stg") => Ok(DeployEnv::Stg),
            x if x.eq_ignore_ascii_case("prod") => Ok(DeployEnv::Prod),
            _ => Err("expected values: dev|stg|prod"),
        }
    }
}

impl fmt::Display for DeployEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeployEnv::Dev => f.write_str("dev"),
            DeployEnv::Stg => f.write_str("stg"),
            DeployEnv::Prod => f.write_str("prod"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TelemetryConfig {
    pub service_name: String,
    pub service_version: String,
    pub deploy_env: DeployEnv,
    pub level: ObsLevel,
    pub prom_scrape: bool,
    pub metrics_http_addr: String,
    pub otlp_endpoint: Option<String>,
    pub otlp_timeout: Duration,
    pub log_level: String,
    pub deny_dynamic_labels: bool,
}

impl TelemetryConfig {
    pub fn from_env() -> Result<Self, TelemetryError> {
        TelemetryConfigBuilder::default().build()
    }

    pub fn builder() -> TelemetryConfigBuilder {
        TelemetryConfigBuilder::default()
    }
}

#[derive(Debug, Default, Clone)]
pub struct TelemetryConfigBuilder {
    service_name: Option<String>,
    service_version: Option<String>,
    deploy_env: Option<DeployEnv>,
    level: Option<ObsLevel>,
    prom_scrape: Option<bool>,
    metrics_http_addr: Option<String>,
    otlp_endpoint: Option<Option<String>>,
    otlp_timeout: Option<Duration>,
    log_level: Option<String>,
    deny_dynamic_labels: Option<bool>,
}

impl TelemetryConfigBuilder {
    pub fn with_service_name<S: Into<String>>(mut self, value: S) -> Self {
        self.service_name = Some(value.into());
        self
    }

    pub fn with_service_version<S: Into<String>>(mut self, value: S) -> Self {
        self.service_version = Some(value.into());
        self
    }

    pub fn with_deploy_env(mut self, value: DeployEnv) -> Self {
        self.deploy_env = Some(value);
        self
    }

    pub fn with_level(mut self, value: ObsLevel) -> Self {
        self.level = Some(value);
        self
    }

    pub fn with_prom_scrape(mut self, value: bool) -> Self {
        self.prom_scrape = Some(value);
        self
    }

    pub fn with_metrics_http_addr<S: Into<String>>(mut self, value: S) -> Self {
        self.metrics_http_addr = Some(value.into());
        self
    }

    pub fn with_otlp_endpoint(mut self, value: Option<String>) -> Self {
        self.otlp_endpoint = Some(value);
        self
    }

    pub fn with_otlp_timeout(mut self, value: Duration) -> Self {
        self.otlp_timeout = Some(value);
        self
    }

    pub fn with_log_level<S: Into<String>>(mut self, value: S) -> Self {
        self.log_level = Some(value.into());
        self
    }

    pub fn with_deny_dynamic_labels(mut self, value: bool) -> Self {
        self.deny_dynamic_labels = Some(value);
        self
    }

    pub fn build(self) -> Result<TelemetryConfig, TelemetryError> {
        let service_name = resolve_service_name(self.service_name)?;
        let service_version = resolve_service_version(self.service_version)?;
        let deploy_env = resolve_deploy_env(self.deploy_env)?;
        let level = resolve_obs_level(self.level)?;
        let prom_scrape = resolve_bool(self.prom_scrape, EnvKey::new("PROM_SCRAPE", false))?;
        let metrics_http_addr = resolve_metrics_http_addr(self.metrics_http_addr)?;
        let otlp_endpoint = resolve_otlp_endpoint(self.otlp_endpoint)?;
        let otlp_timeout = resolve_otlp_timeout(self.otlp_timeout)?;
        let log_level = resolve_log_level(self.log_level)?;
        let deny_dynamic_labels = resolve_bool(
            self.deny_dynamic_labels,
            EnvKey::new("DENY_DYNAMIC_LABELS", true),
        )?;

        Ok(TelemetryConfig {
            service_name,
            service_version,
            deploy_env,
            level,
            prom_scrape,
            metrics_http_addr,
            otlp_endpoint,
            otlp_timeout,
            log_level,
            deny_dynamic_labels,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TelemetryError {
    InvalidEnvValue {
        var: &'static str,
        message: String,
    },
    InvalidBuilderValue {
        field: &'static str,
        message: String,
    },
    InvalidUnicode {
        var: &'static str,
    },
}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelemetryError::InvalidEnvValue { var, message } => {
                write!(f, "invalid value for environment variable {var}: {message}")
            }
            TelemetryError::InvalidBuilderValue { field, message } => {
                write!(f, "invalid value for builder field {field}: {message}")
            }
            TelemetryError::InvalidUnicode { var } => {
                write!(f, "environment variable {var} is not valid UTF-8")
            }
        }
    }
}

impl std::error::Error for TelemetryError {}

#[derive(Clone, Copy)]
struct EnvKey {
    name: &'static str,
    default: bool,
}

impl EnvKey {
    const fn new(name: &'static str, default: bool) -> Self {
        Self { name, default }
    }
}

fn resolve_service_name(builder_value: Option<String>) -> Result<String, TelemetryError> {
    if let Some(value) = builder_value {
        let trimmed = value.trim();
        validate_service_name(trimmed).map_err(|msg| TelemetryError::InvalidBuilderValue {
            field: "service_name",
            message: format!("{msg}; received '{trimmed}'"),
        })?;
        return Ok(trimmed.to_string());
    }

    match read_env("SERVICE_NAME")? {
        Some(value) => {
            validate_service_name(&value).map_err(|msg| TelemetryError::InvalidEnvValue {
                var: "SERVICE_NAME",
                message: format!("{msg}; received '{value}'"),
            })?;
            Ok(value)
        }
        None => Ok(DEFAULT_SERVICE_NAME.to_string()),
    }
}

fn resolve_service_version(builder_value: Option<String>) -> Result<String, TelemetryError> {
    if let Some(value) = builder_value {
        let trimmed = value.trim();
        validate_service_version(trimmed).map_err(|msg| TelemetryError::InvalidBuilderValue {
            field: "service_version",
            message: format!("{msg}; received '{trimmed}'"),
        })?;
        return Ok(trimmed.to_string());
    }

    match read_env("SERVICE_VERSION")? {
        Some(value) => {
            validate_service_version(&value).map_err(|msg| TelemetryError::InvalidEnvValue {
                var: "SERVICE_VERSION",
                message: format!("{msg}; received '{value}'"),
            })?;
            Ok(value)
        }
        None => Ok(DEFAULT_SERVICE_VERSION.to_string()),
    }
}

fn resolve_deploy_env(builder_value: Option<DeployEnv>) -> Result<DeployEnv, TelemetryError> {
    if let Some(value) = builder_value {
        return Ok(value);
    }

    match read_env("DEPLOY_ENV")? {
        Some(value) => DeployEnv::parse(&value).map_err(|msg| TelemetryError::InvalidEnvValue {
            var: "DEPLOY_ENV",
            message: format!("{msg}; received '{value}'"),
        }),
        None => Ok(DeployEnv::Dev),
    }
}

fn resolve_obs_level(builder_value: Option<ObsLevel>) -> Result<ObsLevel, TelemetryError> {
    if let Some(value) = builder_value {
        return Ok(value);
    }

    match read_env("OBSERVABILITY_LEVEL")? {
        Some(value) => ObsLevel::parse(&value).map_err(|msg| TelemetryError::InvalidEnvValue {
            var: "OBSERVABILITY_LEVEL",
            message: format!("{msg}; received '{value}'"),
        }),
        None => Ok(ObsLevel::Min),
    }
}

fn resolve_bool(builder_value: Option<bool>, env_key: EnvKey) -> Result<bool, TelemetryError> {
    if let Some(value) = builder_value {
        return Ok(value);
    }

    match read_env(env_key.name)? {
        Some(value) => parse_bool(&value).map_err(|_| TelemetryError::InvalidEnvValue {
            var: env_key.name,
            message: format!("expected {BOOL_EXPECTED}; received '{value}'"),
        }),
        None => Ok(env_key.default),
    }
}

fn resolve_metrics_http_addr(builder_value: Option<String>) -> Result<String, TelemetryError> {
    if let Some(value) = builder_value {
        let trimmed = value.trim();
        validate_metrics_http_addr(trimmed).map_err(|msg| TelemetryError::InvalidBuilderValue {
            field: "metrics_http_addr",
            message: format!("{msg}; received '{trimmed}'"),
        })?;
        return Ok(trimmed.to_string());
    }

    match read_env("METRICS_HTTP_ADDR")? {
        Some(value) => {
            validate_metrics_http_addr(&value).map_err(|msg| TelemetryError::InvalidEnvValue {
                var: "METRICS_HTTP_ADDR",
                message: format!("{msg}; received '{value}'"),
            })?;
            Ok(value)
        }
        None => Ok(DEFAULT_METRICS_HTTP_ADDR.to_string()),
    }
}

fn resolve_otlp_endpoint(
    builder_value: Option<Option<String>>,
) -> Result<Option<String>, TelemetryError> {
    if let Some(value) = builder_value {
        return match value {
            Some(endpoint) => {
                let trimmed = endpoint.trim();
                let normalized = normalize_otlp_endpoint(trimmed).map_err(|msg| {
                    TelemetryError::InvalidBuilderValue {
                        field: "otlp_endpoint",
                        message: format!("{msg}; received '{trimmed}'"),
                    }
                })?;
                Ok(Some(normalized))
            }
            None => Ok(None),
        };
    }

    match read_env("OTEL_EXPORTER_OTLP_ENDPOINT")? {
        Some(value) => {
            let normalized =
                normalize_otlp_endpoint(&value).map_err(|msg| TelemetryError::InvalidEnvValue {
                    var: "OTEL_EXPORTER_OTLP_ENDPOINT",
                    message: format!("{msg}; received '{value}'"),
                })?;
            Ok(Some(normalized))
        }
        None => Ok(None),
    }
}

fn resolve_otlp_timeout(builder_value: Option<Duration>) -> Result<Duration, TelemetryError> {
    if let Some(value) = builder_value {
        if value.is_zero() {
            return Err(TelemetryError::InvalidBuilderValue {
                field: "otlp_timeout",
                message: format!("{OTLP_TIMEOUT_EXPECTED}; received '0'"),
            });
        }
        return Ok(value);
    }

    match read_env("OTEL_EXPORTER_OTLP_TIMEOUT")? {
        Some(raw) => {
            let duration =
                parse_timeout_ms(&raw).map_err(|msg| TelemetryError::InvalidEnvValue {
                    var: "OTEL_EXPORTER_OTLP_TIMEOUT",
                    message: format!("{msg}; received '{raw}'"),
                })?;
            Ok(duration)
        }
        None => Ok(Duration::from_secs(10)),
    }
}

fn resolve_log_level(builder_value: Option<String>) -> Result<String, TelemetryError> {
    if let Some(value) = builder_value {
        let trimmed = value.trim();
        let canonical = trimmed.to_ascii_lowercase();
        validate_log_level(&canonical).map_err(|msg| TelemetryError::InvalidBuilderValue {
            field: "log_level",
            message: format!("{msg}; received '{trimmed}'"),
        })?;
        return Ok(canonical);
    }

    match read_env("LOG_LEVEL")? {
        Some(value) => {
            let canonical = value.to_ascii_lowercase();
            validate_log_level(&canonical).map_err(|msg| TelemetryError::InvalidEnvValue {
                var: "LOG_LEVEL",
                message: format!("{msg}; received '{value}'"),
            })?;
            Ok(canonical)
        }
        None => Ok(DEFAULT_LOG_LEVEL.to_string()),
    }
}

fn read_env(name: &'static str) -> Result<Option<String>, TelemetryError> {
    match env::var(name) {
        Ok(value) => {
            let trimmed = value.trim().to_string();
            Ok(Some(trimmed))
        }
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(TelemetryError::InvalidUnicode { var: name }),
    }
}

fn validate_service_name(value: &str) -> Result<(), &'static str> {
    let is_valid = value.len() >= 3
        && value.len() <= 64
        && value
            .chars()
            .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '.' | '_' | '-'));
    if is_valid {
        Ok(())
    } else {
        Err(SERVICE_NAME_EXPECTED)
    }
}

fn validate_service_version(value: &str) -> Result<(), &'static str> {
    if value.is_empty() || value.len() > 64 {
        Err(SERVICE_VERSION_EXPECTED)
    } else {
        Ok(())
    }
}

fn validate_metrics_http_addr(value: &str) -> Result<(), &'static str> {
    if let Some((host, port_str)) = value.rsplit_once(':') {
        if !validate_port(port_str) {
            return Err(METRICS_ADDR_EXPECTED);
        }
        if validate_ipv4(host) || validate_hostname(host) {
            return Ok(());
        }
    }
    Err(METRICS_ADDR_EXPECTED)
}

fn normalize_otlp_endpoint(value: &str) -> Result<String, &'static str> {
    let cleaned = value.trim_end_matches('/');
    let scheme_split = cleaned.split_once("//");
    let (scheme, rest) = match scheme_split {
        Some((scheme, rest)) => (scheme, rest),
        None => return Err(OTLP_ENDPOINT_EXPECTED),
    };
    let scheme_lower = scheme.to_ascii_lowercase();
    if scheme_lower != "http:" && scheme_lower != "https:" {
        return Err(OTLP_ENDPOINT_EXPECTED);
    }
    if rest.is_empty() || rest.contains('/') {
        return Err(OTLP_ENDPOINT_EXPECTED);
    }
    if let Some((host, port)) = rest.rsplit_once(':') {
        if host.is_empty() {
            return Err(OTLP_ENDPOINT_EXPECTED);
        }
        if !validate_port(port) {
            return Err(OTLP_ENDPOINT_EXPECTED);
        }
        if validate_hostname(host) || validate_ipv4(host) {
            return Ok(cleaned.to_string());
        }
    }
    Err(OTLP_ENDPOINT_EXPECTED)
}

fn validate_log_level(value: &str) -> Result<(), &'static str> {
    match value {
        v if v.eq_ignore_ascii_case("trace")
            || v.eq_ignore_ascii_case("debug")
            || v.eq_ignore_ascii_case("info")
            || v.eq_ignore_ascii_case("warn")
            || v.eq_ignore_ascii_case("error") =>
        {
            Ok(())
        }
        _ => Err(LOG_LEVEL_EXPECTED),
    }
}

fn validate_port(value: &str) -> bool {
    if value.len() < 2 || value.len() > 5 || !value.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    match value.parse::<u16>() {
        Ok(port) => port >= 10,
        Err(_) => false,
    }
}

fn validate_ipv4(value: &str) -> bool {
    let mut segments = value.split('.');
    let mut count = 0;
    while let Some(segment) = segments.next() {
        if segment.is_empty() || segment.len() > 3 || !segment.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        if let Ok(num) = segment.parse::<u8>() {
            let str_num = num.to_string();
            if str_num != segment && segment.len() > 1 && segment.starts_with('0') {
                return false;
            }
        } else {
            return false;
        }
        count += 1;
    }
    count == 4
}

fn validate_hostname(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn parse_timeout_ms(value: &str) -> Result<Duration, &'static str> {
    if value.is_empty() {
        return Err(OTLP_TIMEOUT_EXPECTED);
    }

    let parsed: u64 = value.parse().map_err(|_| OTLP_TIMEOUT_EXPECTED)?;

    if parsed == 0 {
        return Err(OTLP_TIMEOUT_EXPECTED);
    }

    Ok(Duration::from_millis(parsed))
}

fn parse_bool(value: &str) -> Result<bool, ()> {
    match value {
        v if v.eq_ignore_ascii_case("on")
            || v.eq_ignore_ascii_case("true")
            || v.eq_ignore_ascii_case("1") =>
        {
            Ok(true)
        }
        v if v.eq_ignore_ascii_case("off")
            || v.eq_ignore_ascii_case("false")
            || v.eq_ignore_ascii_case("0") =>
        {
            Ok(false)
        }
        _ => Err(()),
    }
}
