#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="large-mesh-auto-fallback"
MODULE_DESCRIPTION="大网格自动回退回归（3200/2800 与 1M 网格边界）"
MODULE_LEGACY_SCRIPT="run_large_mesh_auto_fallback.sh"
MODULE_OUTPUTS="results/large-mesh-* reports/large-mesh-*"
MODULE_REQUIRES="none"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

