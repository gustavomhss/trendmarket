#!/usr/bin/env bash
# -----------------------------------------------------------------------------
# sync_core_to_trendmarket.sh
# Sincroniza o diretório credit-engine-core para o repositório Trendmarket.
#
# Exemplos:
#   ./sync_core_to_trendmarket.sh
#   ./sync_core_to_trendmarket.sh --dry-run
#   ./sync_core_to_trendmarket.sh --tag latest
#   ./sync_core_to_trendmarket.sh --no-delete --lfs-threshold-mb 120
#   SRC_DIR="/Users/gustavoschneiter/Documents/credit-engine-core" \
#   REPO_URL="git@github.com:gustavomhss/trendmarket.git" \
#   ./sync_core_to_trendmarket.sh
#
# Opções:
#   --repo-url <url>           URL do repositório Git (padrão: env REPO_URL
#                              ou git@github.com:gustavomhss/trendmarket.git)
#   --branch <nome>            Nome da branch (padrão: env BRANCH ou main)
#   --src-dir <path>           Diretório-fonte (padrão: env SRC_DIR ou
#                              /Users/gustavoschneiter/Documents/credit-engine-core)
#   --lfs-threshold-mb <mb>    Tamanho (MB) a partir do qual usa Git LFS
#                              (padrão: env LFS_THRESHOLD_MB ou 95)
#   --no-delete                Desabilita rsync --delete (ou set NO_DELETE=1)
#   --tag <sufixo>             Cria tag core-sync-<sufixo>
#                              (padrão: timestamp core-sync-YYYYMMDD-HHMM)
#   --dry-run                  Simula (ou set DRY_RUN=1)
#   -h | --help                Mostra ajuda
# -----------------------------------------------------------------------------

set -euo pipefail
IFS=$'\n\t'

log() {
  printf '[%s] %s\n' "$(date '+%Y-%m-%dT%H:%M:%S%z')" "$*" >&2
}

die() {
  log "ERROR: $*"
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "Missing required command: $1"
}

usage() {
  cat <<'USAGE'
Usage: sync_core_to_trendmarket.sh [options]

Options:
  --repo-url <url>           Git repository URL (default env REPO_URL or git@github.com:gustavomhss/trendmarket.git)
  --branch <name>            Branch name (default env BRANCH or main)
  --src-dir <path>           Source directory to mirror (default env SRC_DIR or /Users/gustavoschneiter/Documents/credit-engine-core)
  --lfs-threshold-mb <mb>    Git LFS threshold in megabytes (default env LFS_THRESHOLD_MB or 95)
  --no-delete                Disable rsync --delete (or set NO_DELETE=1)
  --tag <suffix>             Use core-sync-<suffix> as annotated tag (default timestamp core-sync-YYYYMMDD-HHMM)
  --dry-run                  Perform a dry run (or set DRY_RUN=1)
  -h, --help                 Show this help message and exit

Examples:
  ./sync_core_to_trendmarket.sh
  ./sync_core_to_trendmarket.sh --dry-run
  ./sync_core_to_trendmarket.sh --tag latest
  ./sync_core_to_trendmarket.sh --no-delete --lfs-threshold-mb 120
  SRC_DIR="/Users/gustavoschneiter/Documents/credit-engine-core" \
  REPO_URL="git@github.com:gustavomhss/trendmarket.git" \
  ./sync_core_to_trendmarket.sh
USAGE
}

cleanup() {
  if [[ -n "${temp_dir:-}" && -d "${temp_dir}" ]]; then
    rm -rf "${temp_dir}"
  fi
}
trap cleanup EXIT INT TERM

# Defaults (podem ser sobrescritos por env ou flags)
repo_url="${REPO_URL:-git@github.com:gustavomhss/trendmarket.git}"
branch="${BRANCH:-main}"
src_dir="${SRC_DIR:-/Users/gustavoschneiter/Documents/credit-engine-core}"
lfs_threshold_mb="${LFS_THRESHOLD_MB:-95}"

delete_enabled=1
[[ "${NO_DELETE:-0}" == "1" ]] && delete_enabled=0

dry_run=0
[[ "${DRY_RUN:-0}" == "1" ]] && dry_run=1

custom_tag_suffix="${TAG:-}"

# Parse de argumentos
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-url)
      shift || die "--repo-url requires a value"
      repo_url="$1"
      ;;
    --branch)
      shift || die "--branch requires a value"
      branch="$1"
      ;;
    --src-dir)
      shift || die "--src-dir requires a value"
      src_dir="$1"
      ;;
    --lfs-threshold-mb)
      shift || die "--lfs-threshold-mb requires a value"
      lfs_threshold_mb="$1"
      ;;
    --no-delete)
      delete_enabled=0
      ;;
    --tag)
      shift || die "--tag requires a value"
      custom_tag_suffix="$1"
      ;;
    --dry-run)
      dry_run=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    *)
      die "Unknown option: $1"
      ;;
  esac
  shift || true
done

# Validações
[[ -n "$repo_url" ]] || die "Repository URL cannot be empty"
[[ -n "$branch"   ]] || die "Branch name cannot be empty"
[[ -n "$src_dir"  ]] || die "Source directory cannot be empty"
[[ -n "$lfs_threshold_mb" ]] || die "LFS threshold cannot be empty"
[[ "$lfs_threshold_mb" =~ ^[0-9]+$ ]] || die "LFS threshold must be an integer"

require_cmd git
require_cmd rsync
require_cmd find
require_cmd date
require_cmd grep

if ! git lfs version >/dev/null 2>&1; then
  die "git lfs is required. Please install Git LFS."
fi
git lfs install >/dev/null 2>&1 || die "Failed to run git lfs install"

[[ -d "$src_dir" ]] || die "Source directory does not exist: $src_dir"

log "Calculating source file statistics"
# Ignora qualquer .git (em qualquer nível) dentro do src_dir
total_files="$(
  find "$src_dir" -type f -not -path '*/.git/*' -print | wc -l | tr -d '[:space:]'
)"
[[ -n "$total_files" ]] || total_files=0

log "Preparing temporary clone"
# mktemp portável no macOS/BSD
temp_dir="$(mktemp -d 2>/dev/null || mktemp -d -t tm-sync)"
repo_dir="${temp_dir}/repo"

log "Cloning repository $repo_url (branch: $branch)"
if ! git clone --depth 1 --branch "$branch" "$repo_url" "$repo_dir" >/dev/null 2>&1; then
  rm -rf "$repo_dir"
  log "Branch $branch not found remotely. Cloning default branch and creating $branch."
  git clone --depth 1 "$repo_url" "$temp_dir/repo" >/dev/null
  pushd "$repo_dir" >/dev/null || die "Failed to enter repo dir"
  if git checkout "$branch" >/dev/null 2>&1; then
    log "Checked out existing branch $branch"
  else
    git checkout -b "$branch" >/dev/null 2>&1
    log "Created new branch $branch"
  fi
  popd >/dev/null || true
else
  log "Successfully cloned branch $branch"
fi

# Avisos sobre identidade git
if [[ -z "$(git -C "$repo_dir" config user.name || true)" ]]; then
  log "Warning: git user.name is not configured. Commits may fail."
fi
if [[ -z "$(git -C "$repo_dir" config user.email || true)" ]]; then
  log "Warning: git user.email is not configured. Commits may fail."
fi

# Rsync (espelha tudo, exceto .git e lixo do mac)
rsync_opts=(-a --exclude='.git' --exclude='.DS_Store')
[[ $delete_enabled -eq 1 ]] && rsync_opts+=(--delete)
[[ $dry_run -eq 1     ]] && rsync_opts+=(-n -v)

log "Running rsync from $src_dir to repository"
rsync "${rsync_opts[@]}" "$src_dir"/ "$repo_dir"/

pushd "$repo_dir" >/dev/null || die "Failed to enter repo dir"

# Scan de arquivos grandes para LFS
log "Scanning for large files (threshold ${lfs_threshold_mb}MB)"
large_files=()

if [[ $dry_run -eq 1 ]]; then
  # Em dry-run não copiamos de fato, então checamos direto no src_dir
  while IFS= read -r -d '' file; do
    rel_path="${file#$src_dir/}"
    large_files+=("$rel_path")
  done < <(find "$src_dir" -type f -not -path '*/.git/*' -size +"${lfs_threshold_mb}"M -print0)
else
  # Já estamos dentro do repo_dir
  while IFS= read -r -d '' file; do
    rel_path="${file#./}"
    large_files+=("$rel_path")
  done < <(find . -type f -not -path './.git/*' -size +"${lfs_threshold_mb}"M -print0)
fi

large_file_count=${#large_files[@]}

if [[ $dry_run -eq 1 ]]; then
  mirror_state=$([[ $delete_enabled -eq 1 ]] && echo "ON" || echo "OFF")
  echo "Dry-run summary:"
  echo " Repository: $repo_url"
  echo " Branch: $branch"
  echo " Mirror delete: $mirror_state"
  echo " Total files in source: $total_files"
  echo " Files >= ${lfs_threshold_mb}MB: ${large_file_count}"
  if [[ $large_file_count -gt 0 ]]; then
    echo " Sample files to LFS-track (up to 10):"
    for lf in "${large_files[@]:0:10}"; do
      echo " - $lf"
    done
  fi
  popd >/dev/null || true
  exit 0
fi

# Track com Git LFS (se houver)
if [[ $large_file_count -gt 0 ]]; then
  for lf in "${large_files[@]}"; do
    git lfs track -- "$lf"
  done
  [[ -f .gitattributes ]] && git add .gitattributes
  log "Tracked ${large_file_count} files with Git LFS"
else
  log "No files exceed ${lfs_threshold_mb}MB"
fi

# Commit
log "Staging changes"
git add -A

if git diff --cached --quiet; then
  log "No changes detected after rsync"
else
  mirror_state=$([[ $delete_enabled -eq 1 ]] && echo "ON" || echo "OFF")
  commit_message="chore(sync): import credit-engine-core (LFS ≥ ${lfs_threshold_mb}MB, mirror=${mirror_state})"
  git commit -m "$commit_message"
  log "Created commit"
fi

# Push branch
log "Pushing branch $branch"
git push -u origin "$branch"

# Tag
if [[ -n "$custom_tag_suffix" ]]; then
  tag_name="core-sync-${custom_tag_suffix}"
else
  tag_name="core-sync-$(date '+%Y%m%d-%H%M')"
fi
tag_message="Core sync on $(date '+%Y-%m-%d %H:%M:%S %Z')"

git tag -f -a "$tag_name" -m "$tag_message"

if git ls-remote --tags origin "$tag_name" | grep -qE "[[:space:]]refs/tags/${tag_name}$"; then
  log "Remote tag $tag_name exists. Deleting before pushing."
  git push origin ":refs/tags/${tag_name}" || true
fi

log "Pushing tag $tag_name"
git push origin "$tag_name"

# Resumo
current_commit="$(git rev-parse HEAD)"
mirror_state=$([[ $delete_enabled -eq 1 ]] && echo "ON" || echo "OFF")

echo "Sync summary:"
echo " Repository: $repo_url"
echo " Branch: $branch"
echo " Commit: $current_commit"
echo " Tag: $tag_name"
echo " Mirror delete: $mirror_state"
echo " LFS threshold: ${lfs_threshold_mb}MB"
echo " Total files in source: $total_files"
echo " Files tracked with LFS: ${large_file_count}"
if [[ $large_file_count -gt 0 ]]; then
  echo " Sample LFS files (up to 10):"
  for lf in "${large_files[@]:0:10}";    echo " - $lf"
  done
fi

popd >/dev/null || true
