use std::env;
use std::str::FromStr;
use std::sync::{Mutex, MutexGuard, OnceLock};

use credit_engine_core::telemetry_identity::{DeployEnv, IdentityError, ServiceIdentityBuilder};
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

const FULL_SHA: &str = "4fd0c2a64b7f1a3e9c0b2e1d5a6c7b8f4fd0c2a6";
const SHORT_SHA: &str = "4fd0c2a";
const BUILD_TIME: &str = "2025-10-03T12:34:56Z";

#[test]
fn deploy_env_parsing_is_case_insensitive() {
    assert_eq!(DeployEnv::from_str("DEV").unwrap(), DeployEnv::Dev);
    assert_eq!(DeployEnv::from_str("StG").unwrap(), DeployEnv::Stg);
    assert_eq!(DeployEnv::from_str("prod").unwrap(), DeployEnv::Prod);
    let err = DeployEnv::from_str("production").unwrap_err();
    assert!(matches!(err, IdentityError::InvalidDeployEnv { .. }));
}

#[test]
fn invalid_service_name_is_rejected() {
    let _guard = lock_env();
    let err = ServiceIdentityBuilder::new()
        .with_service_name("CE AMM")
        .with_service_version("1.0.0")
        .with_deploy_env(DeployEnv::Dev)
        .with_build_time_utc(BUILD_TIME)
        .with_git_sha(FULL_SHA)
        .with_git_sha_short(SHORT_SHA)
        .build()
        .expect_err("builder should reject invalid service name");
    match err {
        IdentityError::InvalidServiceName { reason, .. } => {
            assert!(reason.contains("allowed characters"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn invalid_service_version_is_rejected() {
    let _guard = lock_env();
    let err = ServiceIdentityBuilder::new()
        .with_service_name("ce-amm")
        .with_service_version("dev-local")
        .with_deploy_env(DeployEnv::Dev)
        .with_build_time_utc(BUILD_TIME)
        .with_git_sha(FULL_SHA)
        .with_git_sha_short(SHORT_SHA)
        .build()
        .expect_err("builder should reject invalid service version");
    match err {
        IdentityError::InvalidServiceVersion { reason, .. } => {
            assert!(reason.contains("semver"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn builder_overrides_environment() {
    let _guard = lock_env();
    let mut env_guard = EnvGuard::default();
    env_guard.set("SERVICE_NAME", "ce-env");

    let identity = ServiceIdentityBuilder::new()
        .with_service_name("ce-builder")
        .with_service_version("2.3.1")
        .with_deploy_env(DeployEnv::Prod)
        .with_build_time_utc(BUILD_TIME)
        .with_git_sha(FULL_SHA)
        .with_git_sha_short(SHORT_SHA)
        .build()
        .expect("builder should succeed");

    assert_eq!(identity.service_name, "ce-builder");
    assert_eq!(identity.deploy_env, DeployEnv::Prod);
    assert_eq!(identity.service_version, "2.3.1");
}

#[test]
fn environment_overrides_defaults() {
    let _guard = lock_env();
    let mut env_guard = EnvGuard::default();
    env_guard.set("SERVICE_NAME", "ce-env");
    env_guard.set("SERVICE_VERSION", "1.4.0");
    env_guard.set("DEPLOY_ENV", "stg");

    let identity = ServiceIdentityBuilder::new()
        .with_build_time_utc(BUILD_TIME)
        .with_git_sha(FULL_SHA)
        .with_git_sha_short(SHORT_SHA)
        .build()
        .expect("builder should consume env vars");

    assert_eq!(identity.service_name, "ce-env");
    assert_eq!(identity.service_version, "1.4.0");
    assert_eq!(identity.deploy_env, DeployEnv::Stg);
}

#[test]
fn version_is_composed_from_git_metadata() {
    let _guard = lock_env();
    let mut env_guard = EnvGuard::default();
    env_guard.remove("SERVICE_VERSION");
    env_guard.set("CE_GIT_SHA_SHORT", "1a2b3c4");

    let identity = ServiceIdentityBuilder::new()
        .with_build_time_utc(BUILD_TIME)
        .with_git_sha(FULL_SHA)
        .build()
        .expect("builder should compose version");

    assert_eq!(
        identity.service_version,
        format!("{}+1a2b3c4", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn resource_pairs_are_canonical() {
    let _guard = lock_env();
    let identity = ServiceIdentityBuilder::new()
        .with_service_name("ce-amm")
        .with_service_version("1.2.3")
        .with_deploy_env(DeployEnv::Dev)
        .with_build_time_utc(BUILD_TIME)
        .with_git_sha(FULL_SHA)
        .with_git_sha_short(SHORT_SHA)
        .build()
        .expect("builder should succeed");

    let pairs = identity.resource_pairs();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0], ("service.name", "ce-amm".to_string()));
    assert_eq!(pairs[1], ("service.version", "1.2.3".to_string()));
    assert_eq!(pairs[2], ("deployment.environment", "dev".to_string()));
}

#[derive(Default)]
struct EnvGuard {
    entries: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn set(&mut self, key: &str, value: &str) {
        self.entries.push((key.to_string(), env::var(key).ok()));
        env::set_var(key, value);
    }

    fn remove(&mut self, key: &str) {
        self.entries.push((key.to_string(), env::var(key).ok()));
        env::remove_var(key);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.entries.drain(..).rev() {
            match value {
                Some(v) => env::set_var(&key, v),
                None => env::remove_var(&key),
            }
        }
    }
}

fn env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_env<'a>() -> MutexGuard<'a, ()> {
    env_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}
