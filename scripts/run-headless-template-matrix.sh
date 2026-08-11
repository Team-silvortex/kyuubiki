#!/usr/bin/env bash
set -euo pipefail
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="${REPO_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"

DEFAULT_WORKDIR="/tmp/kyuubiki-template-matrix-$(date '+%Y%m%d-%H%M%S')"
DEFAULT_REPORT_DIR="/Users/Shared/chroot/dev/kyuubiki/reports"
DEFAULT_REPORT_BASENAME="headless-template-matrix"
DEFAULT_EXECUTOR="mock"
DEFAULT_MODE="both"
MAX_ATTEMPTS="${MAX_ATTEMPTS:-2}"
RETRY_DELAY_SECONDS=1

TEMPLATES_DIRECT=(
  direct_acoustic_bar_1d
  direct_electrostatic_triangle
  direct_heat_triangle
  direct_plane_triangle
  direct_thermal_truss_3d
  direct_thermal_frame_3d
  direct_mesh_pipeline
)

TEMPLATES_MATERIAL=(
  material_dielectric_screening
  material_structural_panel_screening
  material_composite_thermo_electric_panel_screening
  material_thermo_shield_screening
)

MODE="$DEFAULT_MODE"
EXECUTOR="$DEFAULT_EXECUTOR"
TEMPLATE_SOURCE="default"
WORKDIR="$DEFAULT_WORKDIR"
REPORT_DIR="$DEFAULT_REPORT_DIR"
REPORT_BASENAME="$DEFAULT_REPORT_BASENAME"
REPORT_SUFFIX="md"
CUSTOM_TEMPLATES=()
RUN_DRY="true"
RUN_EXEC="true"

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --mode [dry|execute|both]         Which runs to execute. default: both
  --executor <name>                 executor for execute mode (mock or other kyuubiki executor). default: mock
  --templates <comma-separated>      Custom templates list, overrides defaults
  --template <name>                 Append one template (can repeat)
  --workdir <path>                  Output work directory. default: $DEFAULT_WORKDIR
  --report-dir <path>               Directory for report. default: $DEFAULT_REPORT_DIR
  --report-basename <name>           Report basename (without suffix). default: $DEFAULT_REPORT_BASENAME
  --retries <n>                     Command retry count (retry attempts). default: ${MAX_ATTEMPTS}
  --help
EOF
}

extract_json() {
  local file="$1"
  awk '
    BEGIN { capture = 0 }
    /^\{/ { capture = 1 }
    { if (capture) print }
    /^\}$/ { if (capture) exit }
  ' "$file"
}

read_json_field() {
  local file="$1"
  local query="$2"
  jq -r "$query" "$file" 2>/dev/null || printf "n/a"
}

run_cmd_with_retry() {
  local logfile="$1"
  shift

  local attempt=1
  while true; do
    echo "[attempt $attempt/${MAX_ATTEMPTS}] $*" >>"$logfile"
    set +e
    (cd "$REPO_DIR" && "$@") >>"$logfile" 2>&1
    local rc=$?
    set -e
    if [ "$rc" -eq 0 ]; then
      printf '%s' "$rc" > "${logfile}.status"
      return 0
    fi

    if [ "$attempt" -ge "$MAX_ATTEMPTS" ]; then
      printf '%s' "$rc" > "${logfile}.status"
      return "$rc"
    fi

    echo "retrying after failure (rc=$rc) ..." >>"$logfile"
    sleep "$RETRY_DELAY_SECONDS"
    attempt=$((attempt + 1))
  done
}

run_case() {
  local tpl="$1"
  local out="$2"

  mkdir -p "$out"

  local init_s validate_s plan_s render_s run_dry_s run_exec_s
  local validate_ok="false"
  local validate_issue_count=0
  local dry_status="skipped"
  local dry_mode="skipped"
  local dry_steps="n/a"
  local exec_status="skipped"
  local exec_mode="skipped"
  local exec_steps="n/a"
  local exec_validation_ok="false"
  local exec_validation_issue_count=0
  local validate_json=""

  run_cmd_with_retry "$out/init.out" scripts/kyuubiki headless init --template "$tpl" --out "$out/input.json" --json
  init_s="$(cat "$out/init.out.status")"

  run_cmd_with_retry "$out/validate.out" scripts/kyuubiki headless validate "$out/input.json" --json
  validate_s="$(cat "$out/validate.out.status")"
  validate_json="$(extract_json "$out/validate.out")"
  if [ -n "$validate_json" ]; then
    validate_ok="$(printf '%s' "$validate_json" | jq -r '.ok // false' 2>/dev/null || printf 'false')"
    validate_issue_count="$(printf '%s' "$validate_json" | jq -r '.issue_count // 0' 2>/dev/null || printf '0')"
  fi

  run_cmd_with_retry "$out/plan.out" scripts/kyuubiki headless plan "$out/input.json" --json --out "$out/plan.json"
  plan_s="$(cat "$out/plan.out.status")"

  run_cmd_with_retry "$out/render.out" scripts/kyuubiki headless render "$out/input.json" --json --out "$out/batch.json"
  render_s="$(cat "$out/render.out.status")"

  if [ "$RUN_DRY" = "true" ]; then
    if run_cmd_with_retry "$out/run_dry.out" scripts/kyuubiki headless run "$out/batch.json" --json --report-out "$out/dry-report.json"; then
      run_dry_s="$(cat "$out/run_dry.out.status")"
      dry_status="$(read_json_field "$out/dry-report.json" '.status // "n/a"')"
      dry_mode="$(read_json_field "$out/dry-report.json" '.mode // "n/a"')"
      dry_steps="$(read_json_field "$out/dry-report.json" '.executed_step_count // "n/a"')"
    else
      run_dry_s="$(cat "$out/run_dry.out.status")"
      dry_status="$(read_json_field "$out/dry-report.json" '.status // "failed"')"
      dry_mode="$(read_json_field "$out/dry-report.json" '.mode // "n/a"')"
      dry_steps="$(read_json_field "$out/dry-report.json" '.executed_step_count // "n/a"')"
    fi
  else
    run_dry_s=-1
  fi

  if [ "$RUN_EXEC" = "true" ]; then
    if run_cmd_with_retry "$out/run_exec.out" scripts/kyuubiki headless run "$out/batch.json" --json --report-out "$out/exec-report.json" --execute --executor "$EXECUTOR"; then
      run_exec_s="$(cat "$out/run_exec.out.status")"
      exec_status="$(read_json_field "$out/exec-report.json" '.status // "n/a"')"
      exec_mode="$(read_json_field "$out/exec-report.json" '.mode // "n/a"')"
      exec_steps="$(read_json_field "$out/exec-report.json" '.executed_step_count // "n/a"')"
      exec_validation_ok="$(read_json_field "$out/exec-report.json" '.validation.ok // false')"
      exec_validation_issue_count="$(read_json_field "$out/exec-report.json" '.validation.issue_count // 0')"
    else
      run_exec_s="$(cat "$out/run_exec.out.status")"
      exec_status="$(read_json_field "$out/exec-report.json" '.status // "failed"')"
      exec_mode="$(read_json_field "$out/exec-report.json" '.mode // "n/a"')"
      exec_steps="$(read_json_field "$out/exec-report.json" '.executed_step_count // "n/a"')"
      exec_validation_ok="$(read_json_field "$out/exec-report.json" '.validation.ok // false')"
      exec_validation_issue_count="$(read_json_field "$out/exec-report.json" '.validation.issue_count // 0')"
    fi
  else
    run_exec_s=-1
  fi

  jq -n \
    --arg tpl "$tpl" \
    --argjson init "$init_s" \
    --argjson validate "$validate_s" \
    --argjson plan "$plan_s" \
    --argjson render "$render_s" \
    --argjson run_dry "$run_dry_s" \
    --argjson run_exec "$run_exec_s" \
    --arg ok "$validate_ok" \
    --argjson validate_issue_count "$validate_issue_count" \
    --arg dry_status "$dry_status" \
    --arg dry_mode "$dry_mode" \
    --argjson dry_steps "$(printf '%s' "$dry_steps" | jq -R 'if . == "n/a" then "n/a" else tonumber end')" \
    --arg exec_status "$exec_status" \
    --arg exec_mode "$exec_mode" \
    --argjson exec_steps "$(printf '%s' "$exec_steps" | jq -R 'if . == "n/a" then "n/a" else tonumber end')" \
    --arg exec_validation_ok "$exec_validation_ok" \
    --argjson exec_validation_issue_count "$exec_validation_issue_count" \
    '{
      template: $tpl,
      init_exit: $init,
      validate_exit: $validate,
      plan_exit: $plan,
      render_exit: $render,
      run_dry_exit: $run_dry,
      run_exec_exit: $run_exec,
      validate_ok: ($ok == "true"),
      validate_issue_count: $validate_issue_count,
      dry_status: $dry_status,
      dry_mode: $dry_mode,
      dry_steps: $dry_steps,
      exec_status: $exec_status,
      exec_mode: $exec_mode,
      exec_steps: $exec_steps,
      exec_validation_ok: ($exec_validation_ok == "true"),
      exec_validation_issue_count: $exec_validation_issue_count
    }' >> "$WORKDIR/summary.ndjson"
}

append_report_row() {
  local tpl="$1"
  local row="$2"
  echo "| $tpl | $(printf '%s' "$row" | jq -r '.init_exit') | $(printf '%s' "$row" | jq -r '.validate_exit') | $(printf '%s' "$row" | jq -r '.plan_exit') | $(printf '%s' "$row" | jq -r '.render_exit') | $(printf '%s' "$row" | jq -r '.run_dry_exit') | $(printf '%s' "$row" | jq -r '.run_exec_exit') | $(printf '%s' "$row" | jq -r '.dry_status') | $(printf '%s' "$row" | jq -r '.dry_mode') | $(printf '%s' "$row" | jq -r '.dry_steps') | $(printf '%s' "$row" | jq -r '.exec_status') | $(printf '%s' "$row" | jq -r '.exec_mode') | $(printf '%s' "$row" | jq -r '.exec_steps') | $(printf '%s' "$row" | jq -r '.validate_ok') | $(printf '%s' "$row" | jq -r '.validate_issue_count') | $(printf '%s' "$row" | jq -r '.exec_validation_ok') | $(printf '%s' "$row" | jq -r '.exec_validation_issue_count') |"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      MODE="$2"
      shift 2
      ;;
    --executor)
      EXECUTOR="$2"
      shift 2
      ;;
    --templates)
      IFS=',' read -r -a CUSTOM_TEMPLATES <<< "$2"
      TEMPLATE_SOURCE="custom"
      shift 2
      ;;
    --template)
      CUSTOM_TEMPLATES+=("$2")
      TEMPLATE_SOURCE="custom"
      shift 2
      ;;
    --workdir)
      WORKDIR="$2"
      shift 2
      ;;
    --report-dir)
      REPORT_DIR="$2"
      shift 2
      ;;
    --report-basename)
      REPORT_BASENAME="$2"
      shift 2
      ;;
    --retries)
      MAX_ATTEMPTS="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [ "$MODE" = "both" ]; then
  RUN_DRY="true"
  RUN_EXEC="true"
elif [ "$MODE" = "dry" ]; then
  RUN_DRY="true"
  RUN_EXEC="false"
elif [ "$MODE" = "execute" ]; then
  RUN_DRY="false"
  RUN_EXEC="true"
else
  echo "Invalid mode: $MODE (must be dry|execute|both)" >&2
  exit 1
fi

if ! [[ "$MAX_ATTEMPTS" =~ ^[0-9]+$ ]] || [ "$MAX_ATTEMPTS" -lt 1 ]; then
  echo "--retries must be a positive integer" >&2
  exit 1
fi

if [ "${#CUSTOM_TEMPLATES[@]:-0}" -gt 0 ]; then
  TEMPLATES=("${CUSTOM_TEMPLATES[@]}")
else
  TEMPLATES=("${TEMPLATES_DIRECT[@]}" "${TEMPLATES_MATERIAL[@]}")
fi

mkdir -p "$WORKDIR" "$REPORT_DIR"

RUN_TS="$(date '+%Y%m%d-%H%M%S')"
REPORT_PATH="${REPORT_DIR}/${REPORT_BASENAME}-${RUN_TS}.md"

: > "$WORKDIR/summary.ndjson"

for tpl in "${TEMPLATES[@]}"; do
  echo "===== $tpl ====="
  run_case "$tpl" "$WORKDIR/$tpl"
done

jq -s '.' "$WORKDIR/summary.ndjson" > "$WORKDIR/summary.json"

{
  echo "# Headless template matrix"
  echo "- 时间: $(date '+%Y-%m-%d %H:%M:%S %z')"
  echo "- 执行模式: $MODE"
  echo "- 执行器: $EXECUTOR"
  echo "- 模板源: $TEMPLATE_SOURCE"
  echo "- 输出目录: $WORKDIR"
  echo ""
  echo "| template | init | validate | plan | render | run_dry | run_exec | dry_status | dry_mode | dry_steps | exec_status | exec_mode | exec_steps | validate_ok | validate_issue_count | exec_validation_ok | exec_validation_issue_count |"
  echo "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
  for tpl in "${TEMPLATES[@]}"; do
    row="$(jq -r --arg tpl "$tpl" '.[] | select(.template == $tpl)' "$WORKDIR/summary.json")"
    append_report_row "$tpl" "$row"
  done
} > "$REPORT_PATH"

echo "done"
echo "$WORKDIR/summary.json"
echo "$REPORT_PATH"
