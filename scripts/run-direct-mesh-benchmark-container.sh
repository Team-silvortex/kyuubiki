#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
exec "$ROOT_DIR/scripts/kyuubiki" direct-mesh-benchmark-container "$@"
