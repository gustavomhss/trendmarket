use std::env;
use std::fmt;
use std::str::FromStr;

const SERVICE_NAME_DEFAULT: &str = "ce-amm";
const GIT_SHA_LENGTH: usize = 40;
const GIT_SHA_SHORT_LENGTH: usize = 7;

#[derive(Debug)]
pub enum IdentityError {
    InvalidServiceName { value: String, reason: &'static str },
    InvalidServiceVersion { value: String, reason: &'static str },
    InvalidDeployEnv { value: String },
    MissingBuildTime,
    InvalidBuildTime { value: String },
    MissingGitSha,
    InvalidGitSha { value: String },
    InvalidGitShaShort { value: String },
    MissingVersion,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::InvalidServiceName { value, reason } => {
                write!(f, "service.name `{value}` is invalid: {reason}")
            }
            IdentityError::InvalidServiceVersion { value, reason } => {
                write!(f, "service.version `{value}` is invalid: {reason}")
            }
            IdentityError::InvalidDeployEnv { value } => {
                write!(f, "deployment.environment `{value}` is invalid. accepted values: dev, stg, prod")
            }
            IdentityError::MissingBuildTime => write!(
                f,
                "missing build.time.utc (CE_BUILD_TIME_RFC3339). ensure the build script runs during compilation."
            ),
            IdentityError::InvalidBuildTime { value } => write!(
                f,
                "build.time.utc `{value}` is not a valid RFC3339 UTC timestamp ending with 'Z'."
            ),
            IdentityError::MissingGitSha => write!(
                f,
                "missing git sha (CE_GIT_SHA). set GIT_COMMIT in CI or ensure git metadata is available."
            ),
            IdentityError::InvalidGitSha { value } => write!(
                f,
                "git sha `{value}` is invalid. expected 40 lowercase hexadecimal characters."
            ),
            IdentityError::InvalidGitShaShort { value } => write!(
                f,
                "git sha short `{value}` is invalid. expected 7 lowercase hexadecimal characters."
            ),
            IdentityError::MissingVersion => write!(
                f,
                "service.version could not be determined. provide SERVICE_VERSION or ensure CE_GIT_SHA_SHORT is available."
            ),
        }
    }
}

impl std::error::Error for IdentityError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployEnv {
    Dev,
    Stg,
    Prod,
}

impl DeployEnv {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Stg => "stg",
            Self::Prod => "prod",
        }
    }
}

impl fmt::Display for DeployEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DeployEnv {
    type Err = IdentityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let lowered = trimmed.to_ascii_lowercase();
        match lowered.as_str() {
            "dev" => Ok(Self::Dev),
            "stg" => Ok(Self::Stg),
            "prod" => Ok(Self::Prod),
            _ => Err(IdentityError::InvalidDeployEnv {
                value: trimmed.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceIdentity {
    pub service_name: String,
    pub service_version: String,
    pub deploy_env: DeployEnv,
    pub build_time_utc: String,
    pub git_sha: String,
}

impl ServiceIdentity {
    pub fn resource_pairs(&self) -> [(&'static str, String); 3] {
        [
            ("service.name", self.service_name.clone()),
            ("service.version", self.service_version.clone()),
            ("deployment.environment", self.deploy_env.to_string()),
        ]
    }
}

#[derive(Debug, Default, Clone)]
pub struct ServiceIdentityBuilder {
    service_name: Option<String>,
    service_version: Option<String>,
    deploy_env: Option<DeployEnv>,
    build_time_utc: Option<String>,
    git_sha: Option<String>,
    git_sha_short: Option<String>,
}

impl ServiceIdentityBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_service_name(mut self, value: impl Into<String>) -> Self {
        self.service_name = Some(value.into());
        self
    }

    pub fn with_service_version(mut self, value: impl Into<String>) -> Self {
        self.service_version = Some(value.into());
        self
    }

    pub fn with_deploy_env(mut self, value: DeployEnv) -> Self {
        self.deploy_env = Some(value);
        self
    }

    pub fn with_build_time_utc(mut self, value: impl Into<String>) -> Self {
        self.build_time_utc = Some(value.into());
        self
    }

    pub fn with_git_sha(mut self, value: impl Into<String>) -> Self {
        self.git_sha = Some(value.into());
        self
    }

    pub fn with_git_sha_short(mut self, value: impl Into<String>) -> Self {
        self.git_sha_short = Some(value.into());
        self
    }

    pub fn build(self) -> Result<ServiceIdentity, IdentityError> {
        let service_name = self
            .service_name
            .or_else(|| env::var("SERVICE_NAME").ok())
            .unwrap_or_else(|| SERVICE_NAME_DEFAULT.to_string());
        validate_service_name(&service_name)?;

        let deploy_env = if let Some(value) = self.deploy_env {
            value
        } else if let Ok(raw) = env::var("DEPLOY_ENV") {
            DeployEnv::from_str(&raw)?
        } else {
            DeployEnv::Dev
        };

        let build_time_utc = if let Some(value) = self.build_time_utc {
            value
        } else if let Ok(value) = env::var("CE_BUILD_TIME_RFC3339") {
            value
        } else if let Some(value) = option_env!("CE_BUILD_TIME_RFC3339") {
            value.to_string()
        } else {
            return Err(IdentityError::MissingBuildTime);
        };
        validate_build_time(&build_time_utc)?;

        let git_sha_raw = if let Some(value) = self.git_sha {
            value
        } else if let Ok(value) = env::var("CE_GIT_SHA") {
            value
        } else if let Some(value) = option_env!("CE_GIT_SHA") {
            value.to_string()
        } else {
            return Err(IdentityError::MissingGitSha);
        };
        let git_sha = normalize_git_sha(&git_sha_raw)?;

        let git_sha_short_raw = if let Some(value) = self.git_sha_short {
            value
        } else if let Ok(value) = env::var("CE_GIT_SHA_SHORT") {
            value
        } else if let Some(value) = option_env!("CE_GIT_SHA_SHORT") {
            value.to_string()
        } else {
            git_sha[..GIT_SHA_SHORT_LENGTH].to_string()
        };
        let git_sha_short = normalize_git_sha_short(&git_sha_short_raw)?;

        let version_candidate = self
            .service_version
            .or_else(|| env::var("SERVICE_VERSION").ok());

        let service_version = if let Some(version) = version_candidate {
            validate_service_version(&version)?;
            version
        } else {
            let pkg_version = env!("CARGO_PKG_VERSION");
            let composed = if pkg_version.trim().is_empty() {
                format!("0.0.0+{git_sha_short}")
            } else {
                format!("{pkg_version}+{git_sha_short}")
            };
            validate_service_version(&composed)?;
            composed
        };

        Ok(ServiceIdentity {
            service_name,
            service_version,
            deploy_env,
            build_time_utc,
            git_sha,
        })
    }
}

fn validate_service_name(value: &str) -> Result<(), IdentityError> {
    if value.is_empty() {
        return Err(IdentityError::InvalidServiceName {
            value: value.to_string(),
            reason: "must not be empty",
        });
    }
    if value.len() < 3 || value.len() > 64 {
        return Err(IdentityError::InvalidServiceName {
            value: value.to_string(),
            reason: "length must be between 3 and 64 characters",
        });
    }
    if !value.chars().all(is_valid_service_name_char) {
        return Err(IdentityError::InvalidServiceName {
            value: value.to_string(),
            reason: "allowed characters: lowercase letters, digits, '.', '_' or '-'",
        });
    }
    Ok(())
}

fn validate_service_version(value: &str) -> Result<(), IdentityError> {
    if value.is_empty() {
        return Err(IdentityError::InvalidServiceVersion {
            value: value.to_string(),
            reason: "must not be empty",
        });
    }
    if value.len() < 2 || value.len() > 64 {
        return Err(IdentityError::InvalidServiceVersion {
            value: value.to_string(),
            reason: "length must be between 2 and 64 characters",
        });
    }
    if !value.chars().all(is_valid_service_version_char) {
        return Err(IdentityError::InvalidServiceVersion {
            value: value.to_string(),
            reason: "allowed characters: letters, digits, '+', '.', '_' or '-'",
        });
    }
    if let Some((prefix, suffix)) = value.split_once('+') {
        if prefix.is_empty() {
            return Err(IdentityError::InvalidServiceVersion {
                value: value.to_string(),
                reason: "prefix before '+' must follow semver core (MAJOR.MINOR.PATCH)",
            });
        }
        if !is_semver(prefix) {
            return Err(IdentityError::InvalidServiceVersion {
                value: value.to_string(),
                reason: "prefix before '+' must follow semver core (MAJOR.MINOR.PATCH)",
            });
        }
        if suffix.len() < GIT_SHA_SHORT_LENGTH
            || !suffix
                .chars()
                .take(GIT_SHA_SHORT_LENGTH)
                .all(|c| c.is_ascii_hexdigit())
        {
            return Err(IdentityError::InvalidServiceVersion {
                value: value.to_string(),
                reason: "suffix after '+' must start with at least 7 hexadecimal characters",
            });
        }
    } else if !is_semver(value) {
        return Err(IdentityError::InvalidServiceVersion {
            value: value.to_string(),
            reason: "must follow semver MAJOR.MINOR.PATCH or include '+<git hash>'",
        });
    }
    Ok(())
}

fn validate_build_time(value: &str) -> Result<(), IdentityError> {
    if value.len() != 20 {
        return Err(IdentityError::InvalidBuildTime {
            value: value.to_string(),
        });
    }

    let bytes = value.as_bytes();
    let separators = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];

    for &(idx, expected) in &separators {
        if bytes[idx] != expected {
            return Err(IdentityError::InvalidBuildTime {
                value: value.to_string(),
            });
        }
    }

    for (idx, byte) in bytes.iter().enumerate() {
        if separators.iter().any(|(sep_idx, _)| *sep_idx == idx) {
            continue;
        }
        if !byte.is_ascii_digit() {
            return Err(IdentityError::InvalidBuildTime {
                value: value.to_string(),
            });
        }
    }

    Ok(())
}

fn is_valid_service_name_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | '0'..='9' | '.' | '_' | '-')
}

fn is_valid_service_version_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '.' | '_' | '-')
}

fn is_semver(value: &str) -> bool {
    let mut parts = value.splitn(3, '.');
    let major = parts.next().unwrap_or("");
    let minor = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    if major.is_empty() || minor.is_empty() || rest.is_empty() {
        return false;
    }
    if !major.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if !minor.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    let mut chars = rest.chars();
    let mut seen_digit = false;
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            seen_digit = true;
            continue;
        }
        // once we hit a non-digit character, remaining characters must be valid identifier pieces
        if !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.') {
            return false;
        }
        break;
    }

    // ensure rest contains at least one digit at the start
    if !seen_digit {
        return false;
    }

    for ch in chars {
        if !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.') {
            return false;
        }
    }

    true
}

fn normalize_git_sha(value: &str) -> Result<String, IdentityError> {
    if value.len() != GIT_SHA_LENGTH || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(IdentityError::InvalidGitSha {
            value: value.to_string(),
        });
    }
    Ok(value.to_ascii_lowercase())
}

fn normalize_git_sha_short(value: &str) -> Result<String, IdentityError> {
    if value.len() != GIT_SHA_SHORT_LENGTH || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(IdentityError::InvalidGitShaShort {
            value: value.to_string(),
        });
    }
    Ok(value.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_builder_uses_defaults() {
        let identity = ServiceIdentityBuilder::new()
            .with_build_time_utc("2025-10-03T12:34:56Z")
            .with_git_sha("4fd0c2a64b7f1a3e9c0b2e1d5a6c7b8f4fd0c2a6")
            .with_git_sha_short("4fd0c2a")
            .build()
            .expect("identity should build");

        assert_eq!(identity.service_name, "ce-amm");
        assert_eq!(identity.deploy_env, DeployEnv::Dev);
        assert_eq!(identity.service_version, format!("{}+4fd0c2a", env!("CARGO_PKG_VERSION")));
    }
}
