#!/usr/bin/env bash
set -euo pipefail

LAB_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODULE_DIR="$LAB_ROOT/scripts/modules"
RUN_ROOT_DEFAULT="$LAB_ROOT/runs"

if [ ! -d "$MODULE_DIR" ]; then
  echo "module directory missing: $MODULE_DIR"
  echo "expected: $MODULE_DIR"
  exit 1
fi

source "$LAB_ROOT/scripts/lib/labctl_common.sh"

usage() {
  cat <<'EOF'
Usage:
  ./labctl.sh list
  ./labctl.sh run <module_name> [--run-id <id>] [--workspace <path>] [--label <text>] [--set KEY=VALUE ...]

Examples:
  ./labctl.sh list
  ./labctl.sh run material-explore
  ./labctl.sh run headless-template-matrix --set HEADLESS_ROUNDS=2 --set HEADLESS_MAX_VOLTAGE=3500
  ./labctl.sh run headless-workflow --run-id research_001 --label "closed-loop smoke"

Note:
  - 模块脚本默认会在 runs/<module>/<run_id>/ 创建隔离工作区；
  - 复用历史数据输入的模块，请在运行前先提前准备 workspace 目录再用 --workspace 指定。
EOF
}

list_modules() {
  for module_file in "$MODULE_DIR"/*.sh; do
    MODULE_NAME="(unknown)"
    MODULE_DESCRIPTION="(missing metadata)"
    # shellcheck source=/dev/null
    source "$module_file"
    printf "%-24s %s\n" "$MODULE_NAME" "$MODULE_DESCRIPTION"
    unset -f run_module
    unset MODULE_NAME MODULE_DESCRIPTION MODULE_LEGACY_SCRIPT MODULE_OUTPUTS MODULE_REQUIRES
  done
}

execute_module() {
  local module_name="$1"
  shift

  local run_id="run-$(date -u +%Y%m%dT%H%M%SZ)"
  local run_label="manual"
  local run_workspace=""
  local -a env_overrides=()
  local -a module_args=()

  while [ $# -gt 0 ]; do
    case "$1" in
      --run-id|--run-id=*)
        if [ "$1" = "--run-id" ]; then
          shift
          if [ $# -eq 0 ]; then
            echo "missing value for --run-id"
            exit 1
          fi
          run_id="$1"
        else
          run_id="${1#--run-id=}"
        fi
        ;;
      --label|--label=*)
        if [ "$1" = "--label" ]; then
          shift
          if [ $# -eq 0 ]; then
            echo "missing value for --label"
            exit 1
          fi
          run_label="$1"
        else
          run_label="${1#--label=}"
        fi
        ;;
      --workspace|--workspace=*)
        if [ "$1" = "--workspace" ]; then
          shift
          if [ $# -eq 0 ]; then
            echo "missing value for --workspace"
            exit 1
          fi
          run_workspace="$1"
        else
          run_workspace="${1#--workspace=}"
        fi
        ;;
      --set|--set=*)
        if [ "$1" = "--set" ]; then
          shift
          if [ $# -eq 0 ]; then
            echo "missing value for --set"
            exit 1
          fi
          env_overrides+=("$1")
        else
          env_overrides+=("${1#--set=}")
        fi
        ;;
      --help|-h)
        usage
        return 0
        ;;
      --*)
        echo "unknown option: $1"
        usage
        exit 1
        ;;
      *)
        module_args+=("$1")
        ;;
    esac
    shift
  done

  local module_file="$MODULE_DIR/$module_name.sh"
  if [ ! -f "$module_file" ]; then
    echo "module not found: $module_name"
    echo "run './labctl.sh list' first."
    exit 1
  fi

  # shellcheck source=/dev/null
  source "$module_file"
  if ! declare -f run_module > /dev/null; then
    echo "module invalid, no run_module() in $module_file"
    exit 1
  fi

  local run_root="${LAB_ROOT%/}/runs/$module_name/$run_id"
  local workspace_dir="$run_workspace"
  if [ -z "$workspace_dir" ]; then
    workspace_dir="$run_root/workspace"
  fi
  mkdir -p "$run_root"

  local -a env_pairs=()
  for kv in ${env_overrides[@]+"${env_overrides[@]}"}; do
    if [[ "$kv" != *"="* ]]; then
      echo "invalid --set value, expected KEY=VALUE: $kv"
      exit 1
    fi
    env_pairs+=("$kv")
  done

  local started_at
  local finished_at
  local status=0
  local step_error=0
  local start_label
  local command_hint

  started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  command_hint="./labctl.sh run $module_name"
  if [ "${#module_args[@]}" -gt 0 ]; then
    command_hint="$command_hint $(printf '%s ' "${module_args[@]}")"
  fi

  labctl_prepare_workspace "$workspace_dir"
  for kv in ${env_pairs[@]+"${env_pairs[@]}"}; do
    export "$kv"
  done

  run_module "$run_root" "$run_id" "$workspace_dir" "${module_args[@]+${module_args[@]}}" || status=$?

  finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [ "$status" -eq 0 ]; then
    labctl_write_manifest \
      "$run_root/run-manifest.json" \
      "$MODULE_NAME" \
      "$run_id" \
      "$run_root" \
      "$workspace_dir" \
      "ok" \
      "$started_at" \
      "$finished_at" \
      "$run_label" \
      "$command_hint" \
      "${env_overrides[*]:-}"
  else
    labctl_write_manifest \
      "$run_root/run-manifest.json" \
      "$MODULE_NAME" \
      "$run_id" \
      "$run_root" \
      "$workspace_dir" \
      "failed" \
      "$started_at" \
      "$finished_at" \
      "$run_label" \
      "$command_hint" \
      "${env_overrides[*]:-}"
  fi

  cat <<EOF
module: $MODULE_NAME
status: $status
run_id: $run_id
run_root: $run_root
workspace: $workspace_dir
manifest: $run_root/run-manifest.json
EOF
  return "$status"
}

main() {
  if [ $# -eq 0 ]; then
    usage
    exit 1
  fi

  local action="$1"
  shift

  case "$action" in
    list)
      list_modules
      ;;
    run)
      if [ $# -eq 0 ]; then
        usage
        exit 1
      fi
  execute_module "$@"
      ;;
    help|-h|--help)
      usage
      ;;
    *)
      echo "unknown command: $action"
      usage
      exit 1
      ;;
  esac
}

run_entry() {
  if [ $# -eq 0 ]; then
    usage
    exit 1
  fi

  local action="$1"
  shift

  case "$action" in
    list)
      list_modules
      ;;
    run)
      if [ $# -eq 0 ]; then
        usage
        exit 1
      fi
      execute_module "$@"
      ;;
    help|-h|--help)
      usage
      ;;
    *)
      echo "unknown command: $action"
      usage
      exit 1
      ;;
  esac
}

run_entry "$@"
