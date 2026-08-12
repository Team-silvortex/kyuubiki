#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="material-explore"
MODULE_DESCRIPTION="电磁材料探索基线链路（初筛 -> 描述 -> 计划 -> 下轮计划 -> 链路）"
MODULE_LEGACY_SCRIPT="run_dielectric_screening.sh"
MODULE_OUTPUTS="initial-exploration.json study-description.json study-plan.json next-round-plan.json chain.json"
MODULE_REQUIRES="none"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

