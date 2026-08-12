#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="headless-research-matrix"
MODULE_DESCRIPTION="多模板 headless 矩阵（dry/mock/service/3000/4000）闭环研发探索"
MODULE_LEGACY_SCRIPT="run_headless_research_matrix.sh"
MODULE_OUTPUTS="results/headless-research-matrix-* reports/headless-research-matrix-*.md"
MODULE_REQUIRES="none"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}
