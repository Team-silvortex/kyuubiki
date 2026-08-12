#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="headless-service-rerun"
MODULE_DESCRIPTION="服务执行重跑复验（service executor + material report）"
MODULE_LEGACY_SCRIPT="run_headless_service_rerun_fixed.sh"
MODULE_OUTPUTS="results/research-massive-closure-*/headless-stress/* reports/*service-rerun*.md"
MODULE_REQUIRES="existing batch inputs under results/"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

