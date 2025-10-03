use std::env;
use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=GIT_COMMIT");

    let build_time = resolve_build_time()?;
    println!("cargo:rustc-env=CE_BUILD_TIME_RFC3339={build_time}");

    let git_sha = resolve_git_sha()?;
    println!("cargo:rustc-env=CE_GIT_SHA={git_sha}");

    let git_sha_short = &git_sha[..7];
    println!("cargo:rustc-env=CE_GIT_SHA_SHORT={git_sha_short}");

    Ok(())
}

fn resolve_build_time() -> Result<String, Box<dyn Error>> {
    let output = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map_err(|err| format!(
            "failed to execute `date -u +%Y-%m-%dT%H:%M:%SZ`: {err}. ensure GNU coreutils are available during the build"
        ))?;

    if !output.status.success() {
        return Err(format!(
            "`date -u +%Y-%m-%dT%H:%M:%SZ` exited with status {}. ensure GNU coreutils are available during the build",
            output.status
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.trim().to_string())
}

fn resolve_git_sha() -> Result<String, Box<dyn Error>> {
    if let Ok(value) = env::var("GIT_COMMIT") {
        validate_git_sha(&value)?;
        return Ok(value.to_lowercase());
    }

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|err| {
            format!(
                "failed to execute `git rev-parse HEAD`: {err}. set the GIT_COMMIT environment variable in CI to provide the commit hash"
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "`git rev-parse HEAD` exited with status {}. set the GIT_COMMIT environment variable in CI to provide the commit hash",
            output.status
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let sha = stdout.trim();
    validate_git_sha(sha)?;
    Ok(sha.to_lowercase())
}

fn validate_git_sha(value: &str) -> Result<(), Box<dyn Error>> {
    if value.len() != 40 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("expected a 40-character hexadecimal git sha, got `{value}`").into());
    }
    Ok(())
}
