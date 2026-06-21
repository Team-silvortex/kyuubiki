#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_HOST="${KYUUBIKI_RELEASE_REMOTE_HOST:-kyuubiki-lab}"
REMOTE_BASE_DIR="${KYUUBIKI_RELEASE_REMOTE_DIR:-~/kyuubiki-downloads}"
REMOTE_PASSWORD="${KYUUBIKI_RELEASE_REMOTE_PASSWORD:-}"
SSH_BIN="${KYUUBIKI_RELEASE_REMOTE_SSH_BIN:-ssh}"
RSYNC_BIN="${KYUUBIKI_RELEASE_REMOTE_RSYNC_BIN:-rsync}"
PLATFORM="${1:-$(uname -s | awk '{ if ($0 == "Darwin") print "macos"; else if ($0 == "Linux") print "linux"; else print "windows"; }')}"
PURGE_LOCAL="${PURGE_LOCAL:-0}"
VERSION="${KYUUBIKI_RELEASE_VERSION:-$(node -e 'const fs=require("fs"); const data=JSON.parse(fs.readFileSync("deploy/update-channels.json","utf8")); process.stdout.write(String(data.shipping_version || "").trim());')}"
SSH_OPTS_STRING="${KYUUBIKI_RELEASE_REMOTE_SSH_OPTS:--o StrictHostKeyChecking=accept-new}"
read -r -a SSH_OPTS <<<"$SSH_OPTS_STRING"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/upload-desktop-release-remote.sh [macos|linux|windows|all]

Upload generated desktop release outputs to a remote download server so local
machines do not need to retain large bundle artifacts.

Environment:
  KYUUBIKI_RELEASE_REMOTE_HOST   SSH host or alias. Default: kyuubiki-lab
  KYUUBIKI_RELEASE_REMOTE_DIR    Remote root directory. Default: ~/kyuubiki-downloads
  KYUUBIKI_RELEASE_REMOTE_PASSWORD
                                 Optional password for sshpass-backed upload
  KYUUBIKI_RELEASE_VERSION       Override version folder. Default: deploy/update-channels.json shipping_version
  KYUUBIKI_RELEASE_REMOTE_SSH_OPTS
                                 Extra SSH options. Default: -o StrictHostKeyChecking=accept-new
  PURGE_LOCAL                    Set to 1 to delete uploaded local bundle/dist outputs after success

Examples:
  ./scripts/upload-desktop-release-remote.sh macos
  PURGE_LOCAL=1 ./scripts/upload-desktop-release-remote.sh all
  KYUUBIKI_RELEASE_REMOTE_HOST=kyuubiki-dev@192.168.1.12 ./scripts/upload-desktop-release-remote.sh macos
  KYUUBIKI_RELEASE_REMOTE_PASSWORD=secret ./scripts/upload-desktop-release-remote.sh macos
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

if [[ -z "$VERSION" ]]; then
  echo "unable to determine release version from deploy/update-channels.json" >&2
  exit 1
fi

case "$PLATFORM" in
  macos|linux|windows|all)
    ;;
  *)
    echo "unsupported platform: $PLATFORM" >&2
    usage >&2
    exit 1
    ;;
esac

REMOTE_VERSION_DIR="$REMOTE_BASE_DIR/releases/$VERSION"
REMOTE_METADATA_DIR="$REMOTE_VERSION_DIR/metadata"

declare -a EXISTING_PATHS=()
declare -a SSH_COMMAND
declare -a RSYNC_COMMAND

if [[ -n "$REMOTE_PASSWORD" ]]; then
  if ! command -v sshpass >/dev/null 2>&1; then
    echo "KYUUBIKI_RELEASE_REMOTE_PASSWORD was set but sshpass is not installed" >&2
    exit 1
  fi
  SSH_COMMAND=(sshpass -p "$REMOTE_PASSWORD" "$SSH_BIN" "${SSH_OPTS[@]}")
  RSYNC_COMMAND=(sshpass -p "$REMOTE_PASSWORD" "$RSYNC_BIN")
else
  SSH_COMMAND=("$SSH_BIN" "${SSH_OPTS[@]}")
  RSYNC_COMMAND=("$RSYNC_BIN")
fi

append_if_exists() {
  local path="$1"
  if [[ -e "$ROOT_DIR/$path" ]]; then
    EXISTING_PATHS+=("$path")
  fi
}

collect_platform_paths() {
  local platform="$1"
  append_if_exists "dist/$platform"
}

collect_bundle_paths() {
  append_if_exists "apps/hub-gui/src-tauri/target/release/bundle"
  append_if_exists "apps/workbench-gui/src-tauri/target/release/bundle"
  append_if_exists "apps/installer-gui/src-tauri/target/release/bundle"
}

append_if_exists "releases/index.json"
append_if_exists "releases/update-catalog.json"
append_if_exists "releases/snapshots/$VERSION.json"
append_if_exists "deploy/update-channels.json"
append_if_exists "deploy/installation-integrity-contract.json"
append_if_exists "docs/update-catalog.html"
append_if_exists "docs/installation-integrity-contract.html"
append_if_exists "apps/hub-gui/ui/docs/update-catalog.html"
append_if_exists "apps/hub-gui/ui/docs/installation-integrity.html"

if [[ "$PLATFORM" == "all" ]]; then
  collect_platform_paths "macos"
  collect_platform_paths "linux"
  collect_platform_paths "windows"
else
  collect_platform_paths "$PLATFORM"
fi
collect_bundle_paths

if [[ "${#EXISTING_PATHS[@]}" -eq 0 ]]; then
  echo "no local release outputs were found to upload" >&2
  exit 1
fi

resolve_remote_version_dir() {
  "${SSH_COMMAND[@]}" "$REMOTE_HOST" sh -s -- "$REMOTE_BASE_DIR" "$VERSION" <<'EOF'
base_dir="$1"
version="$2"

case "$base_dir" in
  "~")
    base_dir="$HOME"
    ;;
  "~/"*)
    base_dir="$HOME/${base_dir#~/}"
    ;;
esac

printf '%s/releases/%s\n' "$base_dir" "$version"
EOF
}

build_rsync_ssh_command() {
  printf '%q ' "$SSH_BIN" "${SSH_OPTS[@]}"
}

REMOTE_VERSION_DIR="$(resolve_remote_version_dir)"
REMOTE_METADATA_DIR="$REMOTE_VERSION_DIR/metadata"
RSYNC_SSH_COMMAND="$(build_rsync_ssh_command)"

"${SSH_COMMAND[@]}" "$REMOTE_HOST" "mkdir -p \"$REMOTE_METADATA_DIR\" \"$REMOTE_VERSION_DIR\""

for relative_path in "${EXISTING_PATHS[@]}"; do
  (
    cd "$ROOT_DIR"
    "${RSYNC_COMMAND[@]}" -az --relative -e "$RSYNC_SSH_COMMAND" "./$relative_path" "$REMOTE_HOST:$REMOTE_VERSION_DIR/"
  )
done

if [[ "$PURGE_LOCAL" == "1" ]]; then
  if [[ "$PLATFORM" == "all" ]]; then
    rm -rf \
      "$ROOT_DIR/dist/macos" \
      "$ROOT_DIR/dist/linux" \
      "$ROOT_DIR/dist/windows"
  else
    rm -rf "$ROOT_DIR/dist/$PLATFORM"
  fi

  rm -rf \
    "$ROOT_DIR/apps/hub-gui/src-tauri/target/release/bundle" \
    "$ROOT_DIR/apps/workbench-gui/src-tauri/target/release/bundle" \
    "$ROOT_DIR/apps/installer-gui/src-tauri/target/release/bundle"
fi

echo "uploaded release artifacts for version $VERSION"
echo "remote host: $REMOTE_HOST"
echo "remote dir: $REMOTE_VERSION_DIR"
if [[ "$PURGE_LOCAL" == "1" ]]; then
  echo "local generated bundle outputs were removed after upload"
fi
