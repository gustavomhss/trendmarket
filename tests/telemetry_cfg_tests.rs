use std::env;
use std::sync::Mutex;

use credit_engine_core::telemetry_cfg::{DeployEnv, ObsLevel, TelemetryConfig, TelemetryError};
use once_cell::sync::Lazy;

static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
const ALL_ENV_VARS: &[&str] = &[
    "SERVICE_NAME",
    "SERVICE_VERSION",
    "DEPLOY_ENV",
    "OBSERVABILITY_LEVEL",
    "PROM_SCRAPE",
    "METRICS_HTTP_ADDR",
    "OTEL_EXPORTER_OTLP_ENDPOINT",
    "LOG_LEVEL",
    "DENY_DYNAMIC_LABELS",
];

fn with_env(vars: &[(&str, Option<&str>)], test: impl FnOnce()) {
    let _guard = ENV_MUTEX.lock().expect("env mutex poisoned");
    let snapshot: Vec<(String, Option<String>)> = ALL_ENV_VARS
        .iter()
        .map(|key| ((*key).to_string(), env::var(key).ok()))
        .collect();

    for key in ALL_ENV_VARS {
        env::remove_var(key);
    }

    for (key, value) in vars {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));

    for (key, value) in snapshot {
        match value {
            Some(val) => env::set_var(&key, val),
            None => env::remove_var(&key),
        }
    }

    result.expect("test panicked");
}

#[test]
fn defaults_are_applied() {
    with_env(&[], || {
        let cfg = TelemetryConfig::from_env().expect("default config loads");
        assert_eq!(cfg.service_name, "ce-amm");
        assert_eq!(cfg.service_version, "0.0.0-dev");
        assert_eq!(cfg.deploy_env, DeployEnv::Dev);
        assert_eq!(cfg.level, ObsLevel::Min);
        assert!(!cfg.prom_scrape);
        assert_eq!(cfg.metrics_http_addr, "0.0.0.0:9464");
        assert!(cfg.otlp_endpoint.is_none());
        assert_eq!(cfg.log_level, "info");
        assert!(cfg.deny_dynamic_labels);
    });
}

#[test]
fn env_values_override_defaults_and_normalize() {
    with_env(
        &[
            ("SERVICE_NAME", Some("ce-amm-dev")),
            ("SERVICE_VERSION", Some("1.2.3")),
            ("DEPLOY_ENV", Some("STG")),
            ("OBSERVABILITY_LEVEL", Some("FULL")),
            ("PROM_SCRAPE", Some("true")),
            ("METRICS_HTTP_ADDR", Some("metrics.local:9550")),
            (
                "OTEL_EXPORTER_OTLP_ENDPOINT",
                Some("https://otel.example:4317/"),
            ),
            ("LOG_LEVEL", Some("DEBUG")),
            ("DENY_DYNAMIC_LABELS", Some("0")),
        ],
        || {
            let cfg = TelemetryConfig::from_env().expect("env config loads");
            assert_eq!(cfg.service_name, "ce-amm-dev");
            assert_eq!(cfg.service_version, "1.2.3");
            assert_eq!(cfg.deploy_env, DeployEnv::Stg);
            assert_eq!(cfg.level, ObsLevel::Full);
            assert!(cfg.prom_scrape);
            assert_eq!(cfg.metrics_http_addr, "metrics.local:9550");
            assert_eq!(
                cfg.otlp_endpoint.as_deref(),
                Some("https://otel.example:4317")
            );
            assert_eq!(cfg.log_level, "debug");
            assert!(!cfg.deny_dynamic_labels);
        },
    );
}

#[test]
fn builder_precedence_and_normalization() {
    with_env(
        &[
            ("SERVICE_NAME", Some("env-name")),
            ("PROM_SCRAPE", Some("0")),
            (
                "OTEL_EXPORTER_OTLP_ENDPOINT",
                Some("http://env-collector:4317"),
            ),
        ],
        || {
            let cfg = TelemetryConfig::builder()
                .with_service_name("builder-name")
                .with_deploy_env(DeployEnv::Prod)
                .with_level(ObsLevel::Off)
                .with_prom_scrape(true)
                .with_metrics_http_addr("0.0.0.0:9999")
                .with_otlp_endpoint(Some("http://builder:4317/".to_string()))
                .with_log_level("WARN")
                .with_deny_dynamic_labels(false)
                .build()
                .expect("builder config loads");

            assert_eq!(cfg.service_name, "builder-name");
            assert_eq!(cfg.deploy_env, DeployEnv::Prod);
            assert_eq!(cfg.level, ObsLevel::Off);
            assert!(cfg.prom_scrape);
            assert_eq!(cfg.metrics_http_addr, "0.0.0.0:9999");
            assert_eq!(cfg.otlp_endpoint.as_deref(), Some("http://builder:4317"));
            assert_eq!(cfg.log_level, "warn");
            assert!(!cfg.deny_dynamic_labels);
        },
    );
}

#[test]
fn builder_can_disable_otlp_endpoint() {
    with_env(
        &[("OTEL_EXPORTER_OTLP_ENDPOINT", Some("http://env-only:4317"))],
        || {
            let cfg = TelemetryConfig::builder()
                .with_otlp_endpoint(None)
                .build()
                .expect("builder overrides env");
            assert!(cfg.otlp_endpoint.is_none());
        },
    );
}

#[test]
fn invalid_env_value_reports_error() {
    with_env(&[("DEPLOY_ENV", Some("production"))], || {
        let err = TelemetryConfig::from_env().expect_err("invalid env");
        match err {
            TelemetryError::InvalidEnvValue { var, message } => {
                assert_eq!(var, "DEPLOY_ENV");
                assert!(message.contains("dev|stg|prod"));
                assert!(message.contains("production"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    });
}

#[test]
fn invalid_bool_env_reports_error() {
    with_env(&[("PROM_SCRAPE", Some("maybe"))], || {
        let err = TelemetryConfig::from_env().expect_err("invalid bool");
        match err {
            TelemetryError::InvalidEnvValue { var, message } => {
                assert_eq!(var, "PROM_SCRAPE");
                assert!(message.contains("one of on"));
                assert!(message.contains("maybe"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    });
}

#[test]
fn invalid_metrics_addr_from_builder_errors() {
    with_env(&[], || {
        let err = TelemetryConfig::builder()
            .with_metrics_http_addr("127.0.0.1")
            .build()
            .expect_err("missing port must fail");
        match err {
            TelemetryError::InvalidBuilderValue { field, message } => {
                assert_eq!(field, "metrics_http_addr");
                assert!(message.contains("host:port"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    });
}

#[test]
fn invalid_service_name_from_builder_errors() {
    with_env(&[], || {
        let err = TelemetryConfig::builder()
            .with_service_name("CE-AMM")
            .build()
            .expect_err("uppercase should fail");
        match err {
            TelemetryError::InvalidBuilderValue { field, message } => {
                assert_eq!(field, "service_name");
                assert!(message.contains("^[a-z0-9._-]"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    });
}

#[test]
fn empty_env_strings_are_rejected() {
    with_env(&[("SERVICE_VERSION", Some("   "))], || {
        let err = TelemetryConfig::from_env().expect_err("blank version fails");
        match err {
            TelemetryError::InvalidEnvValue { var, message } => {
                assert_eq!(var, "SERVICE_VERSION");
                assert!(message.contains("non-empty version"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    });
}

#[test]
fn bool_parsing_accepts_all_aliases() {
    let cases = [
        ("on", true),
        ("true", true),
        ("1", true),
        ("off", false),
        ("false", false),
        ("0", false),
    ];

    for (value, expected) in cases {
        with_env(&[("PROM_SCRAPE", Some(value))], || {
            let cfg = TelemetryConfig::from_env().expect("valid bool");
            assert_eq!(cfg.prom_scrape, expected);
        });
    }
}
