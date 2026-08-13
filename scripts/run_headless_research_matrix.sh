#!/usr/bin/env bash
set -euo pipefail
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="${KYUUBIKI_REPO_DIR:-/Users/Shared/chroot/dev/kyuubiki}"
CLI_SCRIPT="${REPO_DIR}/scripts/kyuubiki"
WORK_ROOT="${WORKSPACE_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"

DEFAULT_WORKDIR="${WORK_ROOT}/results/headless-research-matrix-$(date '+%Y%m%d-%H%M%S')"
DEFAULT_REPORT_DIR="${WORK_ROOT}/reports"
DEFAULT_REPORT_BASENAME="headless-research-matrix"
DEFAULT_PRIMARY_API_URL="${SERVICE_PRIMARY_API_BASE_URL:-http://127.0.0.1:3000}"
DEFAULT_FALLBACK_API_URL="${SERVICE_FALLBACK_API_BASE_URL:-http://127.0.0.1:4000}"
DEFAULT_CONTROL_PLANE_API_URL="${SERVICE_CONTROL_PLANE_API_BASE_URL:-http://127.0.0.1:4000}"
DEFAULT_RUN_PIPELINE="${PIPELINE:-all}"

MAX_ATTEMPTS="${MAX_ATTEMPTS:-2}"
RETRY_DELAY_SECONDS=1
RUN_DRY="false"
RUN_MOCK="false"
RUN_SERVICE="false"
SERVICE_FALLBACK="1"

ALLOW_SENSITIVE="${HEADLESS_ALLOW_SENSITIVE:-0}"
ALLOW_SENSITIVE_FLAG=""
if [ "$ALLOW_SENSITIVE" = "1" ]; then
  ALLOW_SENSITIVE_FLAG="--allow-sensitive"
fi

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

WORKDIR="${WORKDIR:-$DEFAULT_WORKDIR}"
REPORT_DIR="${REPORT_DIR:-$DEFAULT_REPORT_DIR}"
REPORT_BASENAME="${REPORT_BASENAME:-$DEFAULT_REPORT_BASENAME}"
PRIMARY_API_URL="${DEFAULT_PRIMARY_API_URL}"
FALLBACK_API_URL="${DEFAULT_FALLBACK_API_URL}"
CONTROL_PLANE_API_URL="${DEFAULT_CONTROL_PLANE_API_URL}"
CUSTOM_TEMPLATES=()
TEMPLATE_SOURCE="default"
RUN_ID="$(date '+%Y%m%d-%H%M%S')"

if [ -n "${TEMPLATES:-${TEMPLATES_CSV:-}}" ]; then
  IFS=',' read -r -a CUSTOM_TEMPLATES <<< "${TEMPLATES:-${TEMPLATES_CSV:-}}"
  TEMPLATE_SOURCE="custom"
fi

resolve_template_alias() {
  case "$1" in
    material_dielectric_screening)
      echo "dielectric-screening"
      ;;
    material_structural_panel_screening)
      echo "structural-panel"
      ;;
    material_composite_thermo_electric_panel_screening)
      echo "composite-thermo-electric-panel"
      ;;
    material_thermo_shield_screening)
      echo "thermo-shield"
      ;;
    *)
      echo ""
      ;;
  esac
}

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --pipeline [dry|mock|service|all|dry,mock,service]
                                  Which run pipelines to execute. default: $DEFAULT_RUN_PIPELINE
  --templates <comma-separated>  Custom templates list, overrides defaults
  --template <name>              Append one template (can repeat)
  --workdir <path>               Output work directory. default: $DEFAULT_WORKDIR
  --report-dir <path>            Report output directory. default: $DEFAULT_REPORT_DIR
  --report-basename <name>        Report basename (without suffix). default: $DEFAULT_REPORT_BASENAME
  --service-primary-url <url>     Service executor primary API URL. default: $DEFAULT_PRIMARY_API_URL
  --service-fallback-url <url>    Service executor fallback API URL. default: $DEFAULT_FALLBACK_API_URL
  --service-control-plane-url <url> Service executor control-plane API URL for artifact/transport fallback. default: $DEFAULT_CONTROL_PLANE_API_URL
  --service-fallback <0|1>       Enable fallback execution (default: $SERVICE_FALLBACK)
  --retries <n>                  Retry count for each command (default: ${MAX_ATTEMPTS})
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
  local value
  value="$(jq -r "$query" "$file" 2>/dev/null || printf "n/a")"
  if [ -z "$value" ] || [ "$value" = "null" ]; then
    value="n/a"
  fi
  printf '%s' "$value"
}

read_error_code() {
  local report_path="$1"
  if [ ! -f "$report_path" ]; then
    printf "n/a"
    return 0
  fi

  local err_code
  err_code="$(read_json_field "$report_path" '.execution_summary.failure.error_code')"
  if [ "$err_code" = "n/a" ]; then
    err_code="$(read_json_field "$report_path" '.error.code // .failure.code // .error_code // "n/a"')"
  fi
  printf '%s' "$err_code"
}

is_artifact_limit_failure() {
  local report_path="$1"
  local err_log="${2:-}"
  local err_code
  local message

  err_code="$(read_error_code "$report_path")"
  if [ "$err_code" = "frontend_proxy_artifact_limit" ]; then
    return 0
  fi
  if [ "$err_code" = "kyuubiki.headless.transport_failure" ]; then
    return 0
  fi

  message="$(read_json_field "$report_path" '.message // ""')"
  if [ "$err_log" != "" ] && [ -f "$err_log" ]; then
    message="$message\n$(cat "$err_log")"
  fi

  if printf '%s' "$message" | grep -Eqi "frontend_proxy_artifact_limit|artifact transport|body limit|413|Payload Too Large|artifacts upload failed|transport failure|connection refused|econnrefused|failed to connect"; then
    return 0
  fi
  return 1
}

run_cmd_with_retry() {
  local logfile="$1"
  shift
  local attempt=1

  while true; do
    echo "[attempt $attempt/${MAX_ATTEMPTS}] $*" >>"${logfile}"
    set +e
    (cd "$REPO_DIR" && "$@") >>"${logfile}" 2>>"${logfile}.err"
    local rc=$?
    set -e
    printf '%s' "$rc" > "${logfile}.status"
    if [ "$rc" -eq 0 ]; then
      return 0
    fi
    if [ "$attempt" -ge "$MAX_ATTEMPTS" ]; then
      return "$rc"
    fi
    echo "retrying after failure (rc=$rc) ..." >>"${logfile}"
    sleep "$RETRY_DELAY_SECONDS"
    attempt=$((attempt + 1))
  done
}

run_pipeline() {
  local label="$1"
  shift
  local out="$1"
  shift
  local cmd=("$@")
  local out_file="${out}/${label}.out"
  local err_file="${out}/${label}.err"
  run_cmd_with_retry "$out_file" "${cmd[@]}"
}

parse_winner() {
  local report_file="$1"
  local winner_var="$2"
  local score_var="$3"
  local field_var="$4"
  local safety_var="$5"

  local winner
  local score
  local max_field
  local safety

  winner="$(read_json_field "$report_file" '.winner_candidate_id // "n/a"')"
  if [ "$winner" = "n/a" ] || [ "$winner" = "null" ]; then
    printf -v "$winner_var" '%s' "n/a"
    printf -v "$score_var" '%s' "n/a"
    printf -v "$field_var" '%s' "n/a"
    printf -v "$safety_var" '%s' "n/a"
    return 0
  fi

  score="$(jq -r --arg id "$winner" '.candidates[] | select(.candidate_id == $id) | .score // "n/a"' "$report_file" 2>/dev/null || printf 'n/a')"
  max_field="$(jq -r --arg id "$winner" '.candidates[] | select(.candidate_id == $id) | .max_electric_field_v_m // "n/a"' "$report_file" 2>/dev/null || printf 'n/a')"
  safety="$(jq -r --arg id "$winner" '.candidates[] | select(.candidate_id == $id) | .breakdown_safety_factor // "n/a"' "$report_file" 2>/dev/null || printf 'n/a')"

  score="${score:-n/a}"
  max_field="${max_field:-n/a}"
  safety="${safety:-n/a}"
  if [ "$score" = "null" ] || [ -z "$score" ]; then
    score="n/a"
  fi
  if [ "$max_field" = "null" ] || [ -z "$max_field" ]; then
    max_field="n/a"
  fi
  if [ "$safety" = "null" ] || [ -z "$safety" ]; then
    safety="n/a"
  fi

  printf -v "$winner_var" '%s' "$winner"
  printf -v "$score_var" '%s' "$score"
  printf -v "$field_var" '%s' "$max_field"
  printf -v "$safety_var" '%s' "$safety"
}

run_case() {
  local tpl="$1"
  local out="$2"

  mkdir -p "$out"

  local init_s validate_s plan_s render_s
  local validate_ok="false"
  local validate_issue_count=0

  local dry_s=-1
  local dry_status="skipped"
  local dry_mode="skipped"
  local dry_steps="n/a"
  local dry_error="n/a"

  local mock_s=-1
  local mock_status="skipped"
  local mock_mode="skipped"
  local mock_steps="n/a"
  local mock_error="n/a"
  local mock_validation_ok="false"
  local mock_validation_issue_count=0
  local mock_report_out=""
  local winner_candidate="n/a"
  local winner_score="n/a"
  local winner_field="n/a"
  local winner_safety="n/a"

  local service_primary_s=-1
  local service_primary_status="skipped"
  local service_primary_mode="skipped"
  local service_primary_steps="n/a"
  local service_primary_error="n/a"
  local service_primary_validation_ok="false"
  local service_primary_validation_issue_count=0
  local service_fallback_s=-1
  local service_fallback_status="skipped"
  local service_fallback_mode="skipped"
  local service_fallback_steps="n/a"
  local service_fallback_error="n/a"
  local service_fallback_validation_ok="false"
  local service_fallback_validation_issue_count=0
  local service_fallback_api="$FALLBACK_API_URL"

  local material_alias
  local material_report_name="service-material-report"
  local used_report=""

  material_alias="${MATERIAL_REPORT_ALIAS:-$(resolve_template_alias "$tpl")}"
  if [ -n "${MATERIAL_REPORT_ALIAS:-}" ]; then
    material_report_name="service-material-report-${MATERIAL_REPORT_ALIAS}"
  fi

  run_pipeline "init" "$out" "$CLI_SCRIPT" headless init --template "$tpl" --out "$out/input.json" --json
  init_s="$(cat "$out/init.out.status")"

  run_pipeline "validate" "$out" "$CLI_SCRIPT" headless validate "$out/input.json" --json
  validate_s="$(cat "$out/validate.out.status")"
  if [ -f "$out/validate.out" ]; then
    local validate_json
    validate_json="$(extract_json "$out/validate.out")"
    if [ -n "$validate_json" ]; then
      validate_ok="$(printf '%s' "$validate_json" | jq -r '.ok // false' 2>/dev/null || printf 'false')"
      validate_issue_count="$(printf '%s' "$validate_json" | jq -r '.issue_count // 0' 2>/dev/null || printf '0')"
    fi
  fi

  run_pipeline "plan" "$out" "$CLI_SCRIPT" headless plan "$out/input.json" --json --out "$out/plan.json"
  plan_s="$(cat "$out/plan.out.status")"

  run_pipeline "render" "$out" "$CLI_SCRIPT" headless render "$out/input.json" --json --out "$out/batch.json"
  render_s="$(cat "$out/render.out.status")"

  if [ "$RUN_DRY" = "true" ]; then
    if run_pipeline "run_dry" "$out" "$CLI_SCRIPT" headless run "$out/batch.json" --json --report-out "$out/dry-report.json"; then
      dry_s="$(cat "$out/run_dry.out.status")"
      dry_status="$(read_json_field "$out/dry-report.json" '.status // "n/a"')"
      dry_mode="$(read_json_field "$out/dry-report.json" '.mode // "n/a"')"
      dry_steps="$(read_json_field "$out/dry-report.json" '.executed_step_count // "n/a"')"
      dry_error="$(read_error_code "$out/dry-report.json")"
    else
      dry_s="$(cat "$out/run_dry.out.status")"
      dry_status="$(read_json_field "$out/dry-report.json" '.status // "failed"')"
      dry_mode="$(read_json_field "$out/dry-report.json" '.mode // "n/a"')"
      dry_steps="$(read_json_field "$out/dry-report.json" '.executed_step_count // "n/a"')"
      dry_error="$(read_error_code "$out/dry-report.json")"
    fi
  fi

  if [ "$RUN_MOCK" = "true" ] && [ "$dry_status" != "blocked" ]; then
    mock_report_out="$out/mock-report.json"
    if run_pipeline "run_mock" "$out" "$CLI_SCRIPT" headless run "$out/batch.json" --json --report-out "$mock_report_out" --execute --executor mock ${ALLOW_SENSITIVE_FLAG:+$ALLOW_SENSITIVE_FLAG}; then
      mock_s="$(cat "$out/run_mock.out.status")"
      mock_status="$(read_json_field "$mock_report_out" '.status // "n/a"')"
      mock_mode="$(read_json_field "$mock_report_out" '.mode // "n/a"')"
      mock_steps="$(read_json_field "$mock_report_out" '.executed_step_count // "n/a"')"
      mock_validation_ok="$(read_json_field "$mock_report_out" '.validation.ok // false')"
      mock_validation_issue_count="$(read_json_field "$mock_report_out" '.validation.issue_count // 0')"
      mock_error="$(read_error_code "$mock_report_out")"
    else
      mock_s="$(cat "$out/run_mock.out.status")"
      mock_status="$(read_json_field "$mock_report_out" '.status // "failed"')"
      mock_mode="$(read_json_field "$mock_report_out" '.mode // "n/a"')"
      mock_steps="$(read_json_field "$mock_report_out" '.executed_step_count // "n/a"')"
      mock_validation_ok="$(read_json_field "$mock_report_out" '.validation.ok // false')"
      mock_validation_issue_count="$(read_json_field "$mock_report_out" '.validation.issue_count // 0')"
      mock_error="$(read_error_code "$mock_report_out")"
      mock_report_out=""
    fi
  fi

  if [ "$RUN_SERVICE" = "true" ] && [ "$dry_status" != "blocked" ]; then
    local -a material_args=()
    if [ -n "$material_alias" ]; then
      local alias_report="$out/${material_report_name}-primary.json"
      material_args=(--material-report "$material_alias" --material-report-out "$alias_report")
    fi

    local primary_report="$out/service-primary-report.json"
    if run_pipeline "run_service_primary" "$out" "$CLI_SCRIPT" headless run "$out/batch.json" --json --report-out "$primary_report" --execute --executor service ${ALLOW_SENSITIVE_FLAG:+$ALLOW_SENSITIVE_FLAG} "${material_args[@]+"${material_args[@]}"}" --api-base-url "$PRIMARY_API_URL"; then
      service_primary_s="$(cat "$out/run_service_primary.out.status")"
      service_primary_status="$(read_json_field "$primary_report" '.status // "n/a"')"
      service_primary_mode="$(read_json_field "$primary_report" '.mode // "n/a"')"
      service_primary_steps="$(read_json_field "$primary_report" '.executed_step_count // "n/a"')"
      service_primary_validation_ok="$(read_json_field "$primary_report" '.validation.ok // false')"
      service_primary_validation_issue_count="$(read_json_field "$primary_report" '.validation.issue_count // 0')"
      service_primary_error="$(read_error_code "$primary_report")"
    else
      service_primary_s="$(cat "$out/run_service_primary.out.status")"
      service_primary_status="$(read_json_field "$primary_report" '.status // "failed"')"
      service_primary_mode="$(read_json_field "$primary_report" '.mode // "n/a"')"
      service_primary_steps="$(read_json_field "$primary_report" '.executed_step_count // "n/a"')"
      service_primary_validation_ok="$(read_json_field "$primary_report" '.validation.ok // false')"
      service_primary_validation_issue_count="$(read_json_field "$primary_report" '.validation.issue_count // 0')"
      service_primary_error="$(read_error_code "$primary_report")"
    fi

  if [ "$SERVICE_FALLBACK" = "1" ] && [ "$PRIMARY_API_URL" != "$FALLBACK_API_URL" ] && \
      { [ "$service_primary_s" -ne 0 ] || is_artifact_limit_failure "$primary_report" "$out/run_service_primary.err"; }; then
      if is_artifact_limit_failure "$primary_report" "$out/run_service_primary.err"; then
        service_fallback_api="$CONTROL_PLANE_API_URL"
      fi
      local fb_alias_report="$out/${material_report_name}-fallback.json"
      material_args=()
      if [ -n "$material_alias" ]; then
        material_args=(--material-report "$material_alias" --material-report-out "$fb_alias_report")
      fi

      local fallback_report="$out/service-fallback-report.json"
      if run_pipeline "run_service_fallback" "$out" "$CLI_SCRIPT" headless run "$out/batch.json" --json --report-out "$fallback_report" --execute --executor service ${ALLOW_SENSITIVE_FLAG:+$ALLOW_SENSITIVE_FLAG} "${material_args[@]+"${material_args[@]}"}" --api-base-url "$service_fallback_api"; then
        service_fallback_s="$(cat "$out/run_service_fallback.out.status")"
        service_fallback_status="$(read_json_field "$fallback_report" '.status // "n/a"')"
        service_fallback_mode="$(read_json_field "$fallback_report" '.mode // "n/a"')"
        service_fallback_steps="$(read_json_field "$fallback_report" '.executed_step_count // "n/a"')"
        service_fallback_validation_ok="$(read_json_field "$fallback_report" '.validation.ok // false')"
        service_fallback_validation_issue_count="$(read_json_field "$fallback_report" '.validation.issue_count // 0')"
        service_fallback_error="$(read_error_code "$fallback_report")"
      else
        service_fallback_s="$(cat "$out/run_service_fallback.out.status")"
        service_fallback_status="$(read_json_field "$fallback_report" '.status // "failed"')"
        service_fallback_mode="$(read_json_field "$fallback_report" '.mode // "n/a"')"
        service_fallback_steps="$(read_json_field "$fallback_report" '.executed_step_count // "n/a"')"
        service_fallback_validation_ok="$(read_json_field "$fallback_report" '.validation.ok // false')"
        service_fallback_validation_issue_count="$(read_json_field "$fallback_report" '.validation.issue_count // 0')"
        service_fallback_error="$(read_error_code "$fallback_report")"
      fi
      if [ "$service_fallback_s" -ne 0 ] && [ "$service_fallback_api" != "$FALLBACK_API_URL" ]; then
        service_fallback_status="${service_fallback_status}(fallback-control-plane-attempted)"
      fi
    fi
  fi

  if [ -f "$out/${material_report_name}-primary.json" ]; then
    used_report="$out/${material_report_name}-primary.json"
  elif [ -f "$out/${material_report_name}-fallback.json" ]; then
    used_report="$out/${material_report_name}-fallback.json"
  fi
  if [ -f "$used_report" ]; then
    parse_winner "$used_report" winner_candidate winner_score winner_field winner_safety
  fi

    jq -n \
    --arg tpl "$tpl" \
    --argjson init "$init_s" \
    --argjson validate "$validate_s" \
    --argjson plan "$plan_s" \
    --argjson render "$render_s" \
    --argjson dry "$dry_s" \
    --arg dry_status "$dry_status" \
    --arg dry_mode "$dry_mode" \
    --arg dry_error "$dry_error" \
    --argjson dry_steps "$(printf '%s' "$dry_steps" | jq -R 'if . == "n/a" then "n/a" else tonumber end')" \
    --argjson mock "$mock_s" \
    --arg mock_status "$mock_status" \
    --arg mock_mode "$mock_mode" \
    --arg mock_error "$mock_error" \
    --argjson mock_steps "$(printf '%s' "$mock_steps" | jq -R 'if . == "n/a" then "n/a" else tonumber end')" \
    --argjson mock_validation_ok "$mock_validation_ok" \
    --argjson mock_validation_issue_count "$mock_validation_issue_count" \
    --argjson service_primary "$service_primary_s" \
    --arg service_primary_status "$service_primary_status" \
    --arg service_primary_mode "$service_primary_mode" \
    --arg service_primary_error "$service_primary_error" \
    --argjson service_primary_steps "$(printf '%s' "$service_primary_steps" | jq -R 'if . == "n/a" then "n/a" else tonumber end')" \
    --argjson service_primary_validation_ok "$service_primary_validation_ok" \
    --argjson service_primary_validation_issue_count "$service_primary_validation_issue_count" \
    --argjson service_fallback "$service_fallback_s" \
    --arg service_fallback_status "$service_fallback_status" \
    --arg service_fallback_mode "$service_fallback_mode" \
    --arg service_primary_api "$PRIMARY_API_URL" \
    --arg service_fallback_api "$service_fallback_api" \
    --arg service_control_plane_api "$CONTROL_PLANE_API_URL" \
    --arg service_fallback_error "$service_fallback_error" \
    --argjson service_fallback_steps "$(printf '%s' "$service_fallback_steps" | jq -R 'if . == "n/a" then "n/a" else tonumber end')" \
    --argjson service_fallback_validation_ok "$service_fallback_validation_ok" \
    --argjson service_fallback_validation_issue_count "$service_fallback_validation_issue_count" \
    --arg winner "$winner_candidate" \
    --arg score "$winner_score" \
    --arg field "$winner_field" \
    --arg safety "$winner_safety" \
    --arg validate_ok "$validate_ok" \
    --argjson validate_issue_count "$validate_issue_count" \
    '{
      template: $tpl,
      init_exit: $init,
      validate_exit: $validate,
      plan_exit: $plan,
      render_exit: $render,
      validate_ok: ($validate_ok == "true"),
      validate_issue_count: $validate_issue_count,
      run_dry_exit: $dry,
      dry_status: $dry_status,
      dry_mode: $dry_mode,
      dry_error_code: $dry_error,
      dry_steps: $dry_steps,
      run_mock_exit: $mock,
      mock_status: $mock_status,
      mock_mode: $mock_mode,
      mock_error_code: $mock_error,
      mock_steps: $mock_steps,
      mock_validation_ok: $mock_validation_ok,
      mock_validation_issue_count: $mock_validation_issue_count,
      run_service_primary_exit: $service_primary,
      service_primary_status: $service_primary_status,
      service_primary_mode: $service_primary_mode,
      service_primary_api: $service_primary_api,
      service_primary_error_code: $service_primary_error,
      service_primary_steps: $service_primary_steps,
      service_primary_validation_ok: $service_primary_validation_ok,
      service_primary_validation_issue_count: $service_primary_validation_issue_count,
      run_service_fallback_exit: $service_fallback,
      service_fallback_status: $service_fallback_status,
      service_fallback_mode: $service_fallback_mode,
      service_fallback_api: $service_fallback_api,
      service_fallback_control_plane_api: $service_control_plane_api,
      service_fallback_error_code: $service_fallback_error,
      service_fallback_steps: $service_fallback_steps,
      service_fallback_validation_ok: $service_fallback_validation_ok,
      service_fallback_validation_issue_count: $service_fallback_validation_issue_count,
      winner_candidate_id: $winner,
      winner_score: $score,
      winner_max_electric_field_v_m: $field,
      winner_breakdown_safety_factor: $safety
    }' >> "$WORKDIR/summary.ndjson"
}

append_report_row() {
  local tpl="$1"
  local row="$2"
  echo "| $tpl | $(printf '%s' "$row" | jq -r '.init_exit') | $(printf '%s' "$row" | jq -r '.validate_exit') | $(printf '%s' "$row" | jq -r '.plan_exit') | $(printf '%s' "$row" | jq -r '.render_exit') | $(printf '%s' "$row" | jq -r '.validate_ok') | $(printf '%s' "$row" | jq -r '.validate_issue_count') | $(printf '%s' "$row" | jq -r '.run_dry_exit') | $(printf '%s' "$row" | jq -r '.dry_status') | $(printf '%s' "$row" | jq -r '.dry_mode') | $(printf '%s' "$row" | jq -r '.dry_error_code') | $(printf '%s' "$row" | jq -r '.run_mock_exit') | $(printf '%s' "$row" | jq -r '.mock_status') | $(printf '%s' "$row" | jq -r '.mock_mode') | $(printf '%s' "$row" | jq -r '.mock_error_code') | $(printf '%s' "$row" | jq -r '.run_service_primary_exit') | $(printf '%s' "$row" | jq -r '.service_primary_status') | $(printf '%s' "$row" | jq -r '.service_primary_mode') | $(printf '%s' "$row" | jq -r '.service_primary_api') | $(printf '%s' "$row" | jq -r '.service_primary_error_code') | $(printf '%s' "$row" | jq -r '.run_service_fallback_exit') | $(printf '%s' "$row" | jq -r '.service_fallback_status') | $(printf '%s' "$row" | jq -r '.service_fallback_mode') | $(printf '%s' "$row" | jq -r '.service_fallback_api') | $(printf '%s' "$row" | jq -r '.service_fallback_error_code') | $(printf '%s' "$row" | jq -r '.winner_candidate_id') | $(printf '%s' "$row" | jq -r '.winner_score') | $(printf '%s' "$row" | jq -r '.winner_max_electric_field_v_m') | $(printf '%s' "$row" | jq -r '.winner_breakdown_safety_factor') |"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pipeline)
      IFS=',' read -r -a pipeline_items <<< "$2"
      unset RUN_DRY
      unset RUN_MOCK
      unset RUN_SERVICE
      RUN_DRY="false"
      RUN_MOCK="false"
      RUN_SERVICE="false"
      for item in "${pipeline_items[@]}"; do
        case "$item" in
          dry)
            RUN_DRY="true"
            ;;
          mock)
            RUN_MOCK="true"
            ;;
          service)
            RUN_SERVICE="true"
            ;;
          all)
            RUN_DRY="true"
            RUN_MOCK="true"
            RUN_SERVICE="true"
            ;;
          *)
            echo "Unsupported pipeline item: $item"
            exit 1
            ;;
        esac
      done
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
    --service-primary-url)
      PRIMARY_API_URL="$2"
      shift 2
      ;;
    --service-fallback-url)
      FALLBACK_API_URL="$2"
      shift 2
      ;;
    --service-control-plane-url)
      CONTROL_PLANE_API_URL="$2"
      shift 2
      ;;
    --service-fallback)
      SERVICE_FALLBACK="$2"
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

if ! [ -x "$CLI_SCRIPT" ]; then
  echo "kyuubiki CLI not found or not executable: $CLI_SCRIPT"
  exit 1
fi

if [ "${RUN_DRY:-false}" = "false" ] && [ "${RUN_MOCK:-false}" = "false" ] && [ "${RUN_SERVICE:-false}" = "false" ]; then
  if [ "$DEFAULT_RUN_PIPELINE" = "all" ]; then
    RUN_DRY="true"
    RUN_MOCK="true"
    RUN_SERVICE="true"
  else
    for item in $(echo "$DEFAULT_RUN_PIPELINE" | tr ',' ' '); do
      case "$item" in
        dry)
          RUN_DRY="true"
          ;;
        mock)
          RUN_MOCK="true"
          ;;
        service)
          RUN_SERVICE="true"
          ;;
      esac
    done
  fi
fi

if [ "$SERVICE_FALLBACK" != "0" ] && [ "$SERVICE_FALLBACK" != "1" ]; then
  echo "Invalid --service-fallback value: $SERVICE_FALLBACK (expect 0|1)"
  exit 1
fi

if ! [[ "$MAX_ATTEMPTS" =~ ^[1-9][0-9]*$ ]]; then
  echo "--retries must be positive integer"
  exit 1
fi

if [ "${#CUSTOM_TEMPLATES[@]:-0}" -gt 0 ]; then
  TEMPLATES=("${CUSTOM_TEMPLATES[@]}")
else
  TEMPLATES=("${TEMPLATES_DIRECT[@]}" "${TEMPLATES_MATERIAL[@]}")
fi

mkdir -p "$WORKDIR" "$REPORT_DIR"
: > "$WORKDIR/summary.ndjson"

for tpl in "${TEMPLATES[@]}"; do
  echo "===== $tpl ====="
  run_case "$tpl" "$WORKDIR/$tpl"
done

jq -s '.' "$WORKDIR/summary.ndjson" > "$WORKDIR/summary.json"
REPORT_PATH="${REPORT_DIR}/${REPORT_BASENAME}-${RUN_ID}.md"

{
  echo "# Headless research matrix"
  echo "- 时间: $(date '+%Y-%m-%d %H:%M:%S %z')"
  echo "- 执行模式: dry=$RUN_DRY mock=$RUN_MOCK service=$RUN_SERVICE"
  echo "- 模板源: $TEMPLATE_SOURCE"
  echo "- 输出目录: $WORKDIR"
  echo "- Service 主/兜底: $PRIMARY_API_URL / $FALLBACK_API_URL, fallback=$SERVICE_FALLBACK"
  echo ""
  echo "| template | init | validate | plan | render | validate_ok | validate_issue_count | dry_exit | dry_status | dry_mode | dry_error | mock_exit | mock_status | mock_mode | mock_error | service_primary_exit | service_primary_status | service_primary_mode | service_primary_api | service_primary_error | service_fallback_exit | service_fallback_status | service_fallback_mode | service_fallback_api | service_fallback_error | winner_id | winner_score | winner_field(V/m) | winner_safety |"
  echo "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
  for tpl in "${TEMPLATES[@]}"; do
    row="$(jq -r --arg tpl "$tpl" '.[] | select(.template == $tpl)' "$WORKDIR/summary.json")"
    append_report_row "$tpl" "$row"
  done
} > "$REPORT_PATH"

echo "done"
echo "$WORKDIR/summary.json"
echo "$REPORT_PATH"
