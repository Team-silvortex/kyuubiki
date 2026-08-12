#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="headless-fault-injection"
MODULE_DESCRIPTION="Headless 故障注入回归（非法 action / alias / schema 缺失）"
MODULE_LEGACY_SCRIPT="run_headless_fault_injection_regression.sh"
MODULE_OUTPUTS="headless-fault-injection/*"
MODULE_REQUIRES="none"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

