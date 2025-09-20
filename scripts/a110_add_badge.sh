#!/usr/bin/env bash
set -euo pipefail

read_remote() {
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git remote get-url origin 2>/dev/null || return 1
  else
    return 1
  fi
}

normalize_owner_repo() {
  local raw="$1"
  raw="${raw%.git}"
  case "$raw" in
    https://github.com/*)
      printf '%s' "${raw#https://github.com/}"
      ;;
    http://github.com/*)
      printf '%s' "${raw#http://github.com/}"
      ;;
    git@github.com:*)
      printf '%s' "${raw#git@github.com:}"
      ;;
    ssh://git@github.com/*)
      printf '%s' "${raw#ssh://git@github.com/}"
      ;;
    git://github.com/*)
      printf '%s' "${raw#git://github.com/}"
      ;;
    *)
      return 1
      ;;
  esac
}

main() {
  local remote_url owner_repo badge readme tmp_file
  remote_url="${GITHUB_REPOSITORY:-}"

  if remote_from_git=$(read_remote); then
    remote_url="$remote_from_git"
  fi

  owner_repo=""
  if [ -n "$remote_url" ]; then
    if parsed=$(normalize_owner_repo "$remote_url" 2>/dev/null); then
      owner_repo="$parsed"
    fi
  fi

  if [ -z "$owner_repo" ] && [ -n "${GITHUB_REPOSITORY:-}" ]; then
    owner_repo="$GITHUB_REPOSITORY"
  fi

  if [ -z "$owner_repo" ]; then
    exit 0
  fi

  badge="[![A110 — Invariants](https://github.com/${owner_repo}/actions/workflows/a110-invariants.yml/badge.svg)](https://github.com/${owner_repo}/actions/workflows/a110-invariants.yml)"
  readme="README.md"

  if [ ! -e "$readme" ]; then
    : > "$readme"
  fi

  if grep -Fqx "$badge" "$readme" 2>/dev/null; then
    exit 0
  fi

  tmp_file="$(mktemp)"
  trap 'rm -f "$tmp_file"' EXIT

  {
    if [ -s "$readme" ]; then
      printf '%s\n\n' "$badge"
      cat "$readme"
    else
      printf '%s\n' "$badge"
    fi
  } > "$tmp_file"

  mv "$tmp_file" "$readme"
  trap - EXIT
}

main "$@"
exit 0