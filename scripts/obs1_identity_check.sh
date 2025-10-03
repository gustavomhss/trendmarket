#!/usr/bin/env sh
set -eu

SERVICE_NAME_REGEX='^[a-z0-9._-]{3,64}$'
SERVICE_VERSION_REGEX='^[A-Za-z0-9+._-]{2,64}$'
SEMVER_REGEX='^[0-9]+\.[0-9]+\.[0-9]+([A-Za-z0-9._-]*)$'
SEMVER_WITH_HASH_REGEX='^[0-9]+\.[0-9]+\.[0-9]+([A-Za-z0-9._-]*)\+[0-9a-fA-F]{7}[A-Za-z0-9._-]*$'

if [ -z "${GIT_COMMIT:-}" ]; then
  echo "error: GIT_COMMIT is not set. export GIT_COMMIT in CI before building." >&2
  exit 1
fi

echo "GIT_COMMIT=${GIT_COMMIT}"

git_sha=$(git rev-parse HEAD)
echo "git rev-parse HEAD => ${git_sha}"

if [ -n "${SERVICE_NAME:-}" ]; then
  if ! printf '%s' "${SERVICE_NAME}" | grep -Eq "${SERVICE_NAME_REGEX}"; then
    echo "error: SERVICE_NAME '${SERVICE_NAME}' does not match ${SERVICE_NAME_REGEX}" >&2
    exit 1
  fi
else
  echo "SERVICE_NAME not set -> default 'ce-amm' will be used"
fi

if [ -n "${SERVICE_VERSION:-}" ]; then
  if ! printf '%s' "${SERVICE_VERSION}" | grep -Eq "${SERVICE_VERSION_REGEX}"; then
    echo "error: SERVICE_VERSION '${SERVICE_VERSION}' does not match ${SERVICE_VERSION_REGEX}" >&2
    exit 1
  fi
  if printf '%s' "${SERVICE_VERSION}" | grep -Eq "${SEMVER_WITH_HASH_REGEX}"; then
    :
  elif printf '%s' "${SERVICE_VERSION}" | grep -Eq "${SEMVER_REGEX}"; then
    :
  else
    echo "error: SERVICE_VERSION '${SERVICE_VERSION}' must follow semver MAJOR.MINOR.PATCH or include '+<git hash>'" >&2
    exit 1
  fi
else
  echo "SERVICE_VERSION not set -> version will be composed from build metadata"
fi

echo "Identity checks passed."
