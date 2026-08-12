#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="boundary-regression"
MODULE_DESCRIPTION="边界回归（边界条件与 material-report 别名/缺字段场景）"
MODULE_LEGACY_SCRIPT="run_material_explore_boundary_regression.sh"
MODULE_OUTPUTS="boundary-regression/*"
MODULE_REQUIRES="none"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

