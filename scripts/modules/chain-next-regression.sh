#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="chain-next-regression"
MODULE_DESCRIPTION="chain-next 回归（baseline/replay/缺参/缺文件异常）"
MODULE_LEGACY_SCRIPT="run_chain_next_regression.sh"
MODULE_OUTPUTS="chain-next-regression/ chain-next-regression/report..."
MODULE_REQUIRES="none"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

