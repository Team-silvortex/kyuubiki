#!/usr/bin/env bash
# Shared helpers for modular lab orchestration.

set -euo pipefail

LABCTL_ROOT="${LABCTL_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

labctl_workspace_dir() {
  local run_root="$1"
  printf "%s/workspace\n" "$run_root"
}

labctl_prepare_workspace() {
  local workspace_dir="$1"

  mkdir -p "$(dirname "$workspace_dir")"
  rm -rf "$workspace_dir"
  mkdir -p "$workspace_dir"
  ln -sfn "$LABCTL_ROOT/scripts" "$workspace_dir/scripts"
}

labctl_run_step() {
  local run_root="$1"
  local step="$2"
  shift 2
  local cmd=("$@")
  local step_dir="$run_root/steps"
  local out_file="$step_dir/${step}.out"
  local err_file="$step_dir/${step}.err"
  local status_file="$step_dir/${step}.status"

  mkdir -p "$step_dir"

  set +e
  ("${cmd[@]}" >"$out_file" 2>"$err_file")
  local rc=$?
  set -e

  echo "$rc" > "$status_file"
  return "$rc"
}

labctl_execute_module_script() {
  local run_root="$1"
  local workspace_dir="$2"
  local script_name="$3"

  local script_path="$workspace_dir/scripts/$script_name"
  if [ ! -f "$script_path" ]; then
    echo "module legacy script missing: $script_path"
    return 1
  fi

  if head -n 1 "$script_path" | grep -q "python3"; then
    labctl_run_step "$run_root" "$script_name" python3 "$script_path"
  else
    labctl_run_step "$run_root" "$script_name" bash "$script_path"
  fi
}

labctl_write_manifest() {
  local manifest_path="$1"
  local module_name="$2"
  local run_id="$3"
  local run_dir="$4"
  local workspace_dir="$5"
  local status="$6"
  local started_at="$7"
  local finished_at="$8"
  local label="$9"
  local cmd="${10}"
  local env_overrides="${11:-}"

  cat > "$manifest_path" <<EOF
{
  "module": "$module_name",
  "run_id": "$run_id",
  "label": "$label",
  "status": "$status",
  "started_at_utc": "$started_at",
  "finished_at_utc": "$finished_at",
  "run_dir": "$run_dir",
  "workspace_dir": "$workspace_dir",
  "command": "$cmd",
  "environment_overrides": "$env_overrides",
  "repo_dir": "${KYUUBIKI_REPO_DIR:-/Users/Shared/chroot/dev/kyuubiki}"
}
EOF
}
