#!/usr/bin/env bash
set -Eeuo pipefail
set +H

SCRIPT_NAME="obs4_env_bootstrap"

log_ok() {
  local step="$1"
  shift || true
  echo "STEP=${step} OK $*"
}

log_fail() {
  local step="$1"
  shift || true
  echo "STEP=${step} FAIL $*" >&2
}

on_error() {
  local exit_code=$?
  log_fail "${CURRENT_STEP:-unknown}" "Script failed with exit code ${exit_code}"
  exit "$exit_code"
}
trap on_error ERR

CURRENT_STEP="init"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${REPO_ROOT}/out/obs_gatecheck"
LOG_DIR="${OUT_DIR}/logs"
EVIDENCE_DIR="${OUT_DIR}/evidence"
JIRA_DIR="${OUT_DIR}/jira"
TMP_DIR="${OUT_DIR}/tmp"
DIAG_DIR="${OUT_DIR}/diag"
TOOLS_DIR="${REPO_ROOT}/.tools"
VENV_DIR="${REPO_ROOT}/.venv"
OTEL_VERSION="0.97.0"
OTEL_DIR="${TOOLS_DIR}/otelcol-contrib"

mkdir -p "${LOG_DIR}" "${EVIDENCE_DIR}" "${JIRA_DIR}" "${TMP_DIR}" "${DIAG_DIR}"
log_ok "init" "Output directories ensured"

CURRENT_STEP="pid_guard"
shopt -s nullglob
for pid_file in "${LOG_DIR}"/*.pid; do
  if [[ -f "${pid_file}" ]]; then
    pid="$(<"${pid_file}")"
    if [[ -n "${pid}" ]] && [[ ! -d "/proc/${pid}" ]]; then
      rm -f "${pid_file}"
      log_ok "pid_guard" "Removed stale PID file $(basename "${pid_file}")"
    fi
  fi
done
log_ok "pid_guard" "PID guard completed"
shopt -u nullglob

CURRENT_STEP="tools_dir"
mkdir -p "${TOOLS_DIR}"
log_ok "tools_dir" "Tools directory ensured at ${TOOLS_DIR}"

CURRENT_STEP="venv_setup"
if [[ ! -d "${VENV_DIR}" ]]; then
  python3 -m venv "${VENV_DIR}"
  log_ok "venv_setup" "Created Python virtualenv at ${VENV_DIR}"
else
  log_ok "venv_setup" "Virtualenv already exists at ${VENV_DIR}"
fi

# shellcheck disable=SC1091
source "${VENV_DIR}/bin/activate"

CURRENT_STEP="pip_packages"
PIP_BIN="${VENV_DIR}/bin/pip"
PYTHON_BIN="${VENV_DIR}/bin/python"
SITE_PACKAGES="$(${PYTHON_BIN} - <<'PY'
import sysconfig
print(sysconfig.get_paths()["purelib"])
PY
)"

ensure_package() {
  local package="$1"
  local vendor_dir="$2"
  local module_name="$3"
  local version_label="$4"
  if "${PIP_BIN}" show "$package" >/dev/null 2>&1; then
    log_ok "pip_packages" "${package} already installed"
    return
  fi
  if [[ "${OBS4_ALLOW_PYPI:-0}" == "1" ]]; then
    if "${PIP_BIN}" install --quiet "$package" >/dev/null 2>&1; then
      log_ok "pip_packages" "${package} installed via pip"
      return
    fi
  fi
  if [[ -n "${vendor_dir}" ]] && [[ -d "${vendor_dir}/${module_name}" ]]; then
    local target_dir="${SITE_PACKAGES}/${module_name}"
    rm -rf "${target_dir}"
    mkdir -p "${target_dir%/*}"
    cp -R "${vendor_dir}/${module_name}" "${target_dir}"
    local dist_info_dir="${SITE_PACKAGES}/${package}-${version_label}.dist-info"
    rm -rf "${dist_info_dir}"
    mkdir -p "${dist_info_dir}"
    {
      echo "Metadata-Version: 2.1"
      echo "Name: ${package}"
      echo "Version: ${version_label}"
    } > "${dist_info_dir}/METADATA"
    echo "manual" > "${dist_info_dir}/INSTALLER"
    echo "${module_name}" > "${dist_info_dir}/top_level.txt"
    : > "${dist_info_dir}/RECORD"
    if "${PYTHON_BIN}" -c "import ${module_name}" >/dev/null 2>&1; then
      log_ok "pip_packages" "${package} installed from vendor snapshot"
      return
    fi
  fi
  log_fail "pip_packages" "Failed to install ${package}. Please install manually inside ${VENV_DIR}."
  exit 3
}

ensure_package "jsonschema" "${REPO_ROOT}/vendor/python/jsonschema_stub" "jsonschema" "0.0.0-local"
ensure_package "PyYAML" "${REPO_ROOT}/vendor/python/pyyaml_stub" "yaml" "0.0.0-local"

if declare -F deactivate >/dev/null; then deactivate; fi

CURRENT_STEP="jq_check"
if command -v jq >/dev/null 2>&1; then
  log_ok "jq_check" "jq present at $(command -v jq)"
else
  log_fail "jq_check" "jq not found. Install via your package manager (e.g., sudo apt-get install jq or brew install jq)."
fi

CURRENT_STEP="otelcol_check"
if [[ -x "${OTEL_DIR}/otelcol-contrib" ]]; then
  log_ok "otelcol_check" "otelcol-contrib already present"
else
  mkdir -p "${OTEL_DIR}"
  if [[ -f "${REPO_ROOT}/otelcol-contrib/otelcol-contrib" ]]; then
    cp "${REPO_ROOT}/otelcol-contrib/otelcol-contrib" "${OTEL_DIR}/"
    chmod +x "${OTEL_DIR}/otelcol-contrib"
    log_ok "otelcol_check" "otelcol-contrib copied from bundled archive"
  elif [[ -f "${REPO_ROOT}/otelcol-contrib.tgz" ]]; then
    TMP_DIR_DL="$(mktemp -d)"
    tar -xzf "${REPO_ROOT}/otelcol-contrib.tgz" -C "${TMP_DIR_DL}"
    cp "${TMP_DIR_DL}/otelcol-contrib" "${OTEL_DIR}/"
    chmod +x "${OTEL_DIR}/otelcol-contrib"
    rm -rf "${TMP_DIR_DL}"
    log_ok "otelcol_check" "otelcol-contrib extracted from local archive"
  else
    OS_NAME="$(uname -s)"
    ARCH_NAME="$(uname -m)"
    case "${OS_NAME}" in
      Linux) OS_TOKEN="linux" ;;
      Darwin) OS_TOKEN="darwin" ;;
      *) log_fail "otelcol_check" "Unsupported OS ${OS_NAME}"; exit 3 ;;
    esac
    case "${ARCH_NAME}" in
      x86_64|amd64) ARCH_TOKEN="amd64" ;;
      arm64|aarch64) ARCH_TOKEN="arm64" ;;
      *) log_fail "otelcol_check" "Unsupported architecture ${ARCH_NAME}"; exit 3 ;;
    esac
    TARBALL="otelcol-contrib_${OTEL_VERSION}_${OS_TOKEN}_${ARCH_TOKEN}.tar.gz"
    URL="https://github.com/open-telemetry/opentelemetry-collector-releases/releases/download/v${OTEL_VERSION}/${TARBALL}"
    TMP_DIR_DL="$(mktemp -d)"
    CURRENT_STEP="otelcol_download"
    if command -v curl >/dev/null 2>&1; then
      if curl -fsSL "${URL}" -o "${TMP_DIR_DL}/${TARBALL}"; then
        log_ok "otelcol_download" "Downloaded ${TARBALL}"
        CURRENT_STEP="otelcol_extract"
        tar -xzf "${TMP_DIR_DL}/${TARBALL}" -C "${TMP_DIR_DL}"
        cp -f "${TMP_DIR_DL}/otelcol-contrib" "${OTEL_DIR}/"
        chmod +x "${OTEL_DIR}/otelcol-contrib"
        rm -rf "${TMP_DIR_DL}"
        log_ok "otelcol_extract" "otelcol-contrib v${OTEL_VERSION} installed"
      else
        rm -rf "${TMP_DIR_DL}"
        log_fail "otelcol_download" "Failed to download otelcol-contrib from ${URL}"
        exit 3
      fi
    elif command -v wget >/dev/null 2>&1; then
      if wget -q "${URL}" -O "${TMP_DIR_DL}/${TARBALL}"; then
        log_ok "otelcol_download" "Downloaded ${TARBALL}"
        CURRENT_STEP="otelcol_extract"
        tar -xzf "${TMP_DIR_DL}/${TARBALL}" -C "${TMP_DIR_DL}"
        cp -f "${TMP_DIR_DL}/otelcol-contrib" "${OTEL_DIR}/"
        chmod +x "${OTEL_DIR}/otelcol-contrib"
        rm -rf "${TMP_DIR_DL}"
        log_ok "otelcol_extract" "otelcol-contrib v${OTEL_VERSION} installed"
      else
        rm -rf "${TMP_DIR_DL}"
        log_fail "otelcol_download" "Failed to download otelcol-contrib from ${URL}"
        exit 3
      fi
    else
      rm -rf "${TMP_DIR_DL}"
      log_fail "otelcol_download" "curl or wget required to download otelcol-contrib"
      exit 3
    fi
  fi
fi

CURRENT_STEP="env_vars"
set_default() {
  local var_name="$1"
  local default_value="$2"
  if [[ -z "${!var_name+x}" ]] || [[ -z "${!var_name}" ]]; then
    export "${var_name}=${default_value}"
    log_ok "env_vars" "${var_name} set to default ${default_value}"
  else
    log_ok "env_vars" "${var_name} preserved (${!var_name})"
  fi
}

set_default "SERVICE_NAME" "credit-engine-core"
set_default "SERVICE_VERSION" "0.0.0-dev"
set_default "DEPLOY_ENV" "dev"
set_default "OTEL_TRACES_SAMPLER" "parentbased_traceidratio"
set_default "OTEL_TRACES_SAMPLER_ARG" "0.1"
set_default "OTEL_EXPORTER_OTLP_PROTOCOL" "http/protobuf"
set_default "OTELCOL_LISTEN_ADDR" "127.0.0.1"
set_default "OTELCOL_LISTEN_PORT" "8888"
set_default "OTLP_GRPC_PORT" "4317"
set_default "OTLP_HTTP_PORT" "4318"

if [[ -n "${TEMPO_HTTP_URL:-}" ]]; then
  export TEMPO_HTTP_URL
  log_ok "env_vars" "TEMPO_HTTP_URL preserved (${TEMPO_HTTP_URL})"
elif [[ -n "${JAEGER_HTTP_URL:-}" ]]; then
  export JAEGER_HTTP_URL
  log_ok "env_vars" "JAEGER_HTTP_URL preserved (${JAEGER_HTTP_URL})"
else
  log_ok "env_vars" "WARNING: TEMPO_HTTP_URL and JAEGER_HTTP_URL not set"
fi

CURRENT_STEP="port_guard"
check_port() {
  local port="$1"
  python3 - "$port" <<'PY'
import errno
import socket
import sys
port = int(sys.argv[1])
address = ('127.0.0.1', port)
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.settimeout(0.5)
    try:
        sock.connect(address)
    except OSError as exc:
        if exc.errno in (errno.ECONNREFUSED, errno.ETIMEDOUT, errno.EHOSTUNREACH):
            print("FREE")
        else:
            print("INUSE")
    else:
        print("INUSE")
PY
}

for port in "${OTLP_GRPC_PORT}" "${OTLP_HTTP_PORT}" "${OTELCOL_LISTEN_PORT}"; do
  status="$(check_port "${port}")"
  if [[ "${status}" == "INUSE" ]]; then
    log_fail "port_guard" "Port ${port} appears to be in use"
  else
    log_ok "port_guard" "Port ${port} available"
  fi
done

if declare -F deactivate >/dev/null; then deactivate; fi
source "${VENV_DIR}/bin/activate"

CURRENT_STEP="diag_snapshot"
SNAPSHOT_FILE="${DIAG_DIR}/obs4_env_$(date -u +%Y%m%d-%H%M%SZ).txt"
{
  echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Uname: $(uname -a)"
  echo "Python: $("${VENV_DIR}/bin/python" --version 2>&1)"
  echo "Pip packages:"
  "${VENV_DIR}/bin/pip" list
  echo "otelcol-contrib version:"
  "${OTEL_DIR}/otelcol-contrib" --version || echo "otelcol-contrib --version failed"
  echo "Environment variables:"
  env | grep -E '^(SERVICE_NAME|SERVICE_VERSION|DEPLOY_ENV|OTEL_TRACES_SAMPLER|OTEL_TRACES_SAMPLER_ARG|OTEL_EXPORTER_OTLP_PROTOCOL|TEMPO_HTTP_URL|JAEGER_HTTP_URL|OTELCOL_LISTEN_ADDR|OTELCOL_LISTEN_PORT|OTLP_GRPC_PORT|OTLP_HTTP_PORT)='
} > "${SNAPSHOT_FILE}"
log_ok "diag_snapshot" "Snapshot saved to ${SNAPSHOT_FILE}"

if declare -F deactivate >/dev/null; then deactivate; fi

CURRENT_STEP="summary"
cat <<SUM
STEP=summary OK Created/verified paths:
- ${LOG_DIR}
- ${EVIDENCE_DIR}
- ${JIRA_DIR}
- ${TMP_DIR}
- ${DIAG_DIR}
- ${TOOLS_DIR}
- ${VENV_DIR}
SUM

log_ok "${SCRIPT_NAME}" "Completed successfully"
