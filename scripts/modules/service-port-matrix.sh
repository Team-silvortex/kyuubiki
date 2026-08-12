#!/usr/bin/env bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")/../lib" && pwd)/labctl_common.sh"

MODULE_NAME="service-port-matrix"
MODULE_DESCRIPTION="服务端口矩阵回归（3000/4000 payload 限制验证）"
MODULE_LEGACY_SCRIPT="run_service_port_matrix.sh"
MODULE_OUTPUTS="results/service-matrix-port-rotation-* reports/service-matrix-port-rotation-*.md"
MODULE_REQUIRES="requires input fixtures under results/sdk-large-mesh-1m/ and results/headless-all-dryrun-*/"

run_module() {
  local run_root="$1"
  local run_id="$2"
  local workspace_dir="$3"

  labctl_execute_module_script "$run_root" "$workspace_dir" "$MODULE_LEGACY_SCRIPT"
}

