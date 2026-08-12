#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="headless-template-matrix"
MODULE_DESCRIPTION="Headless 模板矩阵回归（多模板逐模板执行，复用基础闭环）"
MODULE_LEGACY_SCRIPT="run_headless_template_matrix.sh"
MODULE_OUTPUTS="results/headless-template-matrix-* reports/headless-template-matrix-*.md"
MODULE_REQUIRES="none"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

