#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="headless-workflow"
MODULE_DESCRIPTION="Headless 闭环工作流回归（模板->validate/plan/render->dry/exec）"
MODULE_LEGACY_SCRIPT="run_headless_workflow_regression.sh"
MODULE_OUTPUTS="headless-loop/logs/* headless-loop/report.md headless-loop/loop-*.json"
MODULE_REQUIRES="none"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

