#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE_TAG="${IMAGE_TAG:-kyuubiki/direct-mesh-benchmark:local}"
BASE_IMAGE="${BASE_IMAGE:-elixir:1.19}"
REPEAT="${REPEAT:-3}"
HTTP_PROXY_VALUE="${HTTP_PROXY:-${http_proxy:-}}"
HTTPS_PROXY_VALUE="${HTTPS_PROXY:-${https_proxy:-}}"
NO_PROXY_VALUE="${NO_PROXY:-${no_proxy:-}}"
DOCKER_BUILD_NETWORK="${DOCKER_BUILD_NETWORK:-}"
DOCKER_RUN_NETWORK="${DOCKER_RUN_NETWORK:-}"
SKIP_BUILD=0
KEEP_CONTAINERS=0
TIMESTAMP="$(date -u +"%Y%m%dT%H%M%SZ")"
OUTPUT_DIR_DEFAULT="$ROOT_DIR/tmp/direct-mesh-benchmark-container/$TIMESTAMP"
OUTPUT_DIR="$OUTPUT_DIR_DEFAULT"

proxy_build_args=()
proxy_run_args=()
docker_build_network_args=()
docker_run_network_args=()

usage() {
  cat <<'EOF'
Usage:
  ./scripts/run-direct-mesh-benchmark-container.sh [options]

Options:
  --repeat <count>       Number of benchmark runs to execute. Default: 3
  --output-dir <path>    Host directory for logs and summaries.
  --image-tag <tag>      Docker image tag to build/run.
  --base-image <image>   Override the benchmark image base layer.
  --skip-build           Reuse an existing image instead of rebuilding it.
  --keep-containers      Keep per-run containers after completion.
  --help                 Show this message.

Environment:
  REPEAT=<count>
  IMAGE_TAG=<tag>
  BASE_IMAGE=<image>
  HTTP_PROXY=<url>
  HTTPS_PROXY=<url>
  NO_PROXY=<hosts>
  DOCKER_BUILD_NETWORK=<mode>
  DOCKER_RUN_NETWORK=<mode>

Output:
  Each run writes:
  - test.log
  - time.txt
  - subtests.tsv
  - summary.json

  The root output directory also includes:
  - summary.json
  - summary.md
EOF
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

metric_from_time_file() {
  local path="$1"
  local key="$2"
  awk -v key="$key" '{
    for (i = 1; i <= NF; i += 1) {
      split($i, kv, "=")
      if (kv[1] == key) {
        print kv[2]
        exit
      }
    }
  }' "$path"
}

extract_subtests() {
  local log_path="$1"
  local output_path="$2"

  awk '
    /^# Subtest: / {
      name = substr($0, 12)
      next
    }
    /^[[:space:]]+duration_ms: / {
      value = $2
      gsub(/[^0-9.]/, "", value)
      if (name != "") {
        printf "%s\t%s\n", name, value
        name = ""
      }
    }
  ' "$log_path" > "$output_path"
}

write_run_summary() {
  local run_dir="$1"
  local run_index="$2"
  local time_file="$run_dir/time.txt"
  local subtests_file="$run_dir/subtests.tsv"
  local summary_file="$run_dir/summary.json"
  local elapsed_s user_s sys_s max_rss_kib
  local first=1

  elapsed_s="$(metric_from_time_file "$time_file" "elapsed_s")"
  user_s="$(metric_from_time_file "$time_file" "user_s")"
  sys_s="$(metric_from_time_file "$time_file" "sys_s")"
  max_rss_kib="$(metric_from_time_file "$time_file" "max_rss_kib")"

  {
    printf '{\n'
    printf '  "run_index": %s,\n' "$run_index"
    printf '  "elapsed_s": %s,\n' "$elapsed_s"
    printf '  "user_s": %s,\n' "$user_s"
    printf '  "sys_s": %s,\n' "$sys_s"
    printf '  "max_rss_kib": %s,\n' "$max_rss_kib"
    printf '  "subtests": [\n'
    while IFS=$'\t' read -r name duration_ms; do
      [ -n "$name" ] || continue
      if [ "$first" -eq 0 ]; then
        printf ',\n'
      fi
      first=0
      printf '    {"name": "%s", "duration_ms": %s}' \
        "$(json_escape "$name")" \
        "$duration_ms"
    done < "$subtests_file"
    if [ "$first" -eq 0 ]; then
      printf '\n'
    fi
    printf '  ]\n'
    printf '}\n'
  } > "$summary_file"
}

aggregate_metric() {
  local index_path="$1"
  local key="$2"

  awk -v key="$key" '
    BEGIN {
      min = ""
      max = ""
      sum = 0
      count = 0
    }
    {
      for (i = 1; i <= NF; i += 1) {
        split($i, kv, "=")
        if (kv[1] == key) {
          value = kv[2] + 0
          if (min == "" || value < min) min = value
          if (max == "" || value > max) max = value
          sum += value
          count += 1
        }
      }
    }
    END {
      if (count == 0) {
        printf "{\"min\":0,\"max\":0,\"mean\":0}"
        exit
      }
      printf "{\"min\":%.3f,\"max\":%.3f,\"mean\":%.3f}", min, max, sum / count
    }
  ' "$index_path"
}

write_root_summary() {
  local output_dir="$1"
  local repeat="$2"
  local time_index="$output_dir/time-index.txt"
  local summary_json="$output_dir/summary.json"
  local summary_md="$output_dir/summary.md"
  local git_sha
  local first=1

  git_sha="$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || printf 'unknown')"

  {
    printf '{\n'
    printf '  "generated_at": "%s",\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    printf '  "git_sha": "%s",\n' "$git_sha"
    printf '  "image_tag": "%s",\n' "$(json_escape "$IMAGE_TAG")"
    printf '  "repeat": %s,\n' "$repeat"
    printf '  "aggregate": {\n'
    printf '    "elapsed_s": %s,\n' "$(aggregate_metric "$time_index" "elapsed_s")"
    printf '    "user_s": %s,\n' "$(aggregate_metric "$time_index" "user_s")"
    printf '    "sys_s": %s,\n' "$(aggregate_metric "$time_index" "sys_s")"
    printf '    "max_rss_kib": %s\n' "$(aggregate_metric "$time_index" "max_rss_kib")"
    printf '  },\n'
    printf '  "runs": [\n'
    for run_summary in "$output_dir"/run-*/summary.json; do
      [ -f "$run_summary" ] || continue
      if [ "$first" -eq 0 ]; then
        printf ',\n'
      fi
      first=0
      sed 's/^/    /' "$run_summary"
    done
    if [ "$first" -eq 0 ]; then
      printf '\n'
    fi
    printf '  ]\n'
    printf '}\n'
  } > "$summary_json"

  {
    printf '# Direct-mesh Docker benchmark\n\n'
    printf -- '- Git SHA: `%s`\n' "$git_sha"
    printf -- '- Image: `%s`\n' "$IMAGE_TAG"
    printf -- '- Repeat: `%s`\n' "$repeat"
    printf -- '- Output: `%s`\n' "$output_dir"
    printf '\n'
    printf '| Metric | Min | Max | Mean |\n'
    printf '| --- | ---: | ---: | ---: |\n'
    printf '| Elapsed (s) | %s | %s | %s |\n' \
      "$(aggregate_metric "$time_index" "elapsed_s" | awk -F'[:,}]' '{print $2}')" \
      "$(aggregate_metric "$time_index" "elapsed_s" | awk -F'[:,}]' '{print $4}')" \
      "$(aggregate_metric "$time_index" "elapsed_s" | awk -F'[:,}]' '{print $6}')"
    printf '| User CPU (s) | %s | %s | %s |\n' \
      "$(aggregate_metric "$time_index" "user_s" | awk -F'[:,}]' '{print $2}')" \
      "$(aggregate_metric "$time_index" "user_s" | awk -F'[:,}]' '{print $4}')" \
      "$(aggregate_metric "$time_index" "user_s" | awk -F'[:,}]' '{print $6}')"
    printf '| System CPU (s) | %s | %s | %s |\n' \
      "$(aggregate_metric "$time_index" "sys_s" | awk -F'[:,}]' '{print $2}')" \
      "$(aggregate_metric "$time_index" "sys_s" | awk -F'[:,}]' '{print $4}')" \
      "$(aggregate_metric "$time_index" "sys_s" | awk -F'[:,}]' '{print $6}')"
    printf '| Peak RSS (KiB) | %s | %s | %s |\n' \
      "$(aggregate_metric "$time_index" "max_rss_kib" | awk -F'[:,}]' '{print $2}')" \
      "$(aggregate_metric "$time_index" "max_rss_kib" | awk -F'[:,}]' '{print $4}')" \
      "$(aggregate_metric "$time_index" "max_rss_kib" | awk -F'[:,}]' '{print $6}')"
  } > "$summary_md"
}

run_container_benchmark() {
  local run_index="$1"
  local run_dir="$OUTPUT_DIR/run-$(printf '%02d' "$run_index")"
  local container_name="kyuubiki-direct-mesh-bench-$(date +%s)-$run_index"
  local rm_flag="--rm"

  mkdir -p "$run_dir"

  if [ "$KEEP_CONTAINERS" -eq 1 ]; then
    rm_flag=""
  fi

  docker run \
    --name "$container_name" \
    $rm_flag \
    "${docker_run_network_args[@]}" \
    -v "$run_dir:/artifacts" \
    "${proxy_run_args[@]}" \
    "$IMAGE_TAG" \
    bash -lc '
      set -euo pipefail
      /usr/bin/time \
        -f "elapsed_s=%e user_s=%U sys_s=%S max_rss_kib=%M" \
        -o /artifacts/time.txt \
        bash -lc "make test-integration-direct-mesh" \
        2>&1 | tee /artifacts/test.log
      test "${PIPESTATUS[0]}" -eq 0
    '

  extract_subtests "$run_dir/test.log" "$run_dir/subtests.tsv"
  write_run_summary "$run_dir" "$run_index"
  cat "$run_dir/time.txt" >> "$OUTPUT_DIR/time-index.txt"
  printf '\n' >> "$OUTPUT_DIR/time-index.txt"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --repeat)
      REPEAT="$2"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --image-tag)
      IMAGE_TAG="$2"
      shift 2
      ;;
    --base-image)
      BASE_IMAGE="$2"
      shift 2
      ;;
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --keep-containers)
      KEEP_CONTAINERS=1
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

case "$REPEAT" in
  ''|*[!0-9]*)
    echo "--repeat must be a positive integer" >&2
    exit 1
    ;;
esac

if [ "$REPEAT" -lt 1 ]; then
  echo "--repeat must be at least 1" >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
: > "$OUTPUT_DIR/time-index.txt"

if [ -n "$HTTP_PROXY_VALUE" ]; then
  proxy_build_args+=("--build-arg" "HTTP_PROXY=$HTTP_PROXY_VALUE" "--build-arg" "http_proxy=$HTTP_PROXY_VALUE")
  proxy_run_args+=("-e" "HTTP_PROXY=$HTTP_PROXY_VALUE" "-e" "http_proxy=$HTTP_PROXY_VALUE")
fi

if [ -n "$HTTPS_PROXY_VALUE" ]; then
  proxy_build_args+=("--build-arg" "HTTPS_PROXY=$HTTPS_PROXY_VALUE" "--build-arg" "https_proxy=$HTTPS_PROXY_VALUE")
  proxy_run_args+=("-e" "HTTPS_PROXY=$HTTPS_PROXY_VALUE" "-e" "https_proxy=$HTTPS_PROXY_VALUE")
fi

if [ -n "$NO_PROXY_VALUE" ]; then
  proxy_build_args+=("--build-arg" "NO_PROXY=$NO_PROXY_VALUE" "--build-arg" "no_proxy=$NO_PROXY_VALUE")
  proxy_run_args+=("-e" "NO_PROXY=$NO_PROXY_VALUE" "-e" "no_proxy=$NO_PROXY_VALUE")
fi

if [ -n "$DOCKER_BUILD_NETWORK" ]; then
  docker_build_network_args+=("--network" "$DOCKER_BUILD_NETWORK")
fi

if [ -n "$DOCKER_RUN_NETWORK" ]; then
  docker_run_network_args+=("--network" "$DOCKER_RUN_NETWORK")
fi

if [ "$SKIP_BUILD" -eq 0 ]; then
  docker build \
    "${docker_build_network_args[@]}" \
    --build-arg "BASE_IMAGE=$BASE_IMAGE" \
    "${proxy_build_args[@]}" \
    -f "$ROOT_DIR/deploy/docker/direct-mesh-benchmark.Dockerfile" \
    -t "$IMAGE_TAG" \
    "$ROOT_DIR"
fi

for run_index in $(seq 1 "$REPEAT"); do
  echo "==> direct-mesh docker benchmark run $run_index/$REPEAT"
  run_container_benchmark "$run_index"
done

write_root_summary "$OUTPUT_DIR" "$REPEAT"

echo "wrote benchmark artifacts to $OUTPUT_DIR"
echo "summary: $OUTPUT_DIR/summary.json"
