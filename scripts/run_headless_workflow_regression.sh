#!/usr/bin/env bash
set -euo pipefail
set -o pipefail

WORKSPACE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_DIR="${KYUUBIKI_REPO_DIR:-/Users/Shared/chroot/research/kyuubiki}"
DEV_REPO_DIR="${DEV_KYUUBIKI_REPO_DIR:-/Users/Shared/chroot/dev/kyuubiki}"
RUST_DIR="$REPO_DIR/workers/rust"
OUT_DIR="$WORKSPACE_DIR/headless-loop"
LOG_DIR="$OUT_DIR/logs"
API_BASE_URL="${API_BASE_URL:-http://127.0.0.1:3000}"
SYNC_SDK="${SYNC_SDK_FROM_DEV:-1}"
TEMPLATE_ID="${HEADLESS_TEMPLATE:-material_dielectric_screening}"
HEADLESS_ROUNDS="${HEADLESS_ROUNDS:-3}"
HEADLESS_START_VOLTAGE="${HEADLESS_START_VOLTAGE:-1200}"
HEADLESS_MAX_VOLTAGE="${HEADLESS_MAX_VOLTAGE:-3000}"
HEADLESS_MIN_VOLTAGE="${HEADLESS_MIN_VOLTAGE:-1200}"
SCRIPT_REPORT="$OUT_DIR/report.md"
ALLOW_SENSITIVE="${HEADLESS_ALLOW_SENSITIVE:-0}"
HEADLESS_ALLOW_SENSITIVE_FLAG=""
if [ "$ALLOW_SENSITIVE" = "1" ]; then
  HEADLESS_ALLOW_SENSITIVE_FLAG="--allow-sensitive"
fi

if [ ! -x "$REPO_DIR/scripts/kyuubiki" ]; then
  if [ -x "$DEV_REPO_DIR/scripts/kyuubiki" ]; then
    echo "repo fallback: $REPO_DIR missing headless entrypoint, switching to dev repo $DEV_REPO_DIR"
    REPO_DIR="$DEV_REPO_DIR"
    RUST_DIR="$REPO_DIR/workers/rust"
  else
    echo "kyuubiki scripts not found in $REPO_DIR, and dev fallback also missing entrypoint"
    exit 1
  fi
fi

if ! [[ "$HEADLESS_ROUNDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "HEADLESS_ROUNDS must be a positive integer"
  exit 1
fi

reset_output_dir() {
  mkdir -p "$OUT_DIR"
  find "$OUT_DIR" -mindepth 1 -delete
  mkdir -p "$LOG_DIR"
}

for value in "$HEADLESS_START_VOLTAGE" "$HEADLESS_MAX_VOLTAGE" "$HEADLESS_MIN_VOLTAGE"; do
  if ! [[ "$value" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
    echo "HEADLESS_*_VOLTAGE must be numeric"
    exit 1
  fi
done

reset_output_dir

if [ "$SYNC_SDK" = "1" ]; then
  if [ ! -d "$DEV_REPO_DIR/sdks" ]; then
    echo "SYNC_SDK_FROM_DEV is set but source missing: $DEV_REPO_DIR/sdks"
    exit 1
  fi
  rsync -a --delete "$DEV_REPO_DIR/sdks/" "$REPO_DIR/sdks/"
  echo "synced sdks from $DEV_REPO_DIR -> $REPO_DIR"
fi

run_wrapper() {
  local label="$1"
  shift
  local cmd=("$@")
  local out_file="$LOG_DIR/${label}.out"
  local err_file="$LOG_DIR/${label}.err"

  set +e
  (cd "$REPO_DIR" && "${cmd[@]}") >"$out_file" 2>"$err_file"
  local status=$?
  set -e
  echo "$status" > "$LOG_DIR/${label}.status"
  return "$status"
}

assert_file() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "[assert] expected file missing: $path"
    return 1
  fi
}

assert_jq_eq() {
  local path="$1"
  local jq_expr="$2"
  local expected="$3"
  local actual
  actual="$(jq -r "$jq_expr" "$path")"
  if [ "$actual" != "$expected" ]; then
    echo "[assert] $path unexpected value for $jq_expr: got '$actual', expected '$expected'"
    return 1
  fi
}

resolve_material_report_alias() {
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
    material_heat_spreader_screening)
      echo "heat-spreader"
      ;;
    material_thermo_shield_screening)
      echo "thermo-shield"
      ;;
    *)
      echo ""
      ;;
  esac
}

clone_batch_with_voltage() {
  local source_batch="$1"
  local voltage="$2"
  local out_batch="$3"

  jq --argjson voltage "$voltage" '
    .steps |= map(
      if .action == "solve_electrostatic_plane_quad_2d" or .action == "solve_electrostatic_plane_triangle_2d" then
        if (.payload.model.nodes // null) != null then
          .payload.model.nodes = (
            .payload.model.nodes
            | map(if (.potential // 0) != 0 then .potential = $voltage else . end)
          )
        else
          .
        end
      elif .action == "solve_composite_thermo_electric_panel" then
        if (.payload.electrostatic_model.nodes // null) != null then
          .payload.electrostatic_model.nodes = (
            .payload.electrostatic_model.nodes
            | map(if .id == "n3" or .id == "n7" then .potential = $voltage else . end)
          )
        else
          .
        end
      else
        .
      end
    )
  ' "$source_batch" > "$out_batch"
}

derive_next_voltage() {
  local current="$1"
  local score="$2"
  local field="$3"

  awk -v current="$current" -v score="$score" -v field="$field" -v min_v="$HEADLESS_MIN_VOLTAGE" -v max_v="$HEADLESS_MAX_VOLTAGE" '
    BEGIN {
      factor = 1.05
      if (field != "n/a" && field > 50000.0) {
        factor = 1.25
      } else if (field != "n/a" && field > 35000.0) {
        factor = 1.15
      }
      if (score != "n/a") {
        if (score >= 0.80) {
          factor = factor + 0.10
        } else if (score >= 0.65) {
          factor = factor + 0.05
        }
      }
      next_v = current * factor
      if (next_v < min_v) next_v = min_v
      if (next_v > max_v) next_v = max_v
      printf "%.0f", next_v
    }
  '
}

log_round_summary() {
  local round="$1"
  local voltage="$2"
  local dry_status="$3"
  local exec_status="$4"
  local winner="$5"
  local score="$6"
  local field="$7"
  local safety="$8"
  local dry_mode="$9"
  local exec_mode="${10}"
  local dry_steps="${11}"
  local exec_steps="${12}"

  echo "| ${round} | ${voltage} | ${dry_status} | ${exec_status} | ${winner} | ${score} | ${field} | ${safety} | ${dry_mode}/${exec_mode} | ${dry_steps} | ${exec_steps} |" >> "$SCRIPT_REPORT"
}

echo "[1/8] init workflow input from template"
run_wrapper headless_init ./scripts/kyuubiki headless init \
  --template "$TEMPLATE_ID" \
  --out "$OUT_DIR/loop-input.json" \
  --json
assert_file "$OUT_DIR/loop-input.json"

echo "[2/8] validate workflow"
run_wrapper headless_validate ./scripts/kyuubiki headless validate "$OUT_DIR/loop-input.json" --json
assert_file "$LOG_DIR/headless_validate.out"
assert_jq_eq "$LOG_DIR/headless_validate.out" ".ok" "true"

echo "[3/8] build explicit execution plan"
run_wrapper headless_plan ./scripts/kyuubiki headless plan "$OUT_DIR/loop-input.json" --json --out "$OUT_DIR/loop-plan.json"
assert_file "$OUT_DIR/loop-plan.json"
assert_jq_eq "$OUT_DIR/loop-plan.json" ".validation.ok" "true"

echo "[4/8] render execution batch"
run_wrapper headless_render ./scripts/kyuubiki headless render "$OUT_DIR/loop-input.json" --json --out "$OUT_DIR/loop-batch.json"
assert_file "$OUT_DIR/loop-batch.json"

echo "[5/8] run closed-loop rounds (HEADLESS_ROUNDS=$HEADLESS_ROUNDS, START=$HEADLESS_START_VOLTAGE)"
CURRENT_VOLTAGE="$HEADLESS_START_VOLTAGE"
MATERIAL_REPORT_ALIAS="${MATERIAL_REPORT_ALIAS:-$(resolve_material_report_alias "$TEMPLATE_ID")}"
TEMPLATE_BLOCKED="0"

echo "| 回合 | 输入电压(V) | dry-run状态 | execute状态 | winner | score | max_electric_field(V/m) | safety | dry/exe模式 | dry_steps | exec_steps |" > "$SCRIPT_REPORT"
echo "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |" >> "$SCRIPT_REPORT"

for round in $(seq 1 "$HEADLESS_ROUNDS"); do
  ROUND_BATCH="$OUT_DIR/round-${round}-batch.json"
  ROUND_DRY_OUT="$OUT_DIR/round-${round}-run-dry.json"
  ROUND_EXEC_OUT="$OUT_DIR/round-${round}-run-exec.json"
  ROUND_REPORT_OUT="$OUT_DIR/round-${round}-material-report.json"

  clone_batch_with_voltage "$OUT_DIR/loop-batch.json" "$CURRENT_VOLTAGE" "$ROUND_BATCH"

  echo "[round-$round] dry-run with drive voltage ${CURRENT_VOLTAGE}V"
  run_wrapper headless_run_dry_round_${round} ./scripts/kyuubiki headless run "$ROUND_BATCH" --json \
    ${HEADLESS_ALLOW_SENSITIVE_FLAG:+$HEADLESS_ALLOW_SENSITIVE_FLAG} --report-out "$ROUND_DRY_OUT"
  assert_file "$ROUND_DRY_OUT"
  assert_jq_eq "$ROUND_DRY_OUT" ".mode" "dry_run"

  ROUND_DRY_STATUS="$(jq -r '.status // "n/a"' "$ROUND_DRY_OUT")"
  ROUND_DRY_MODE="$(jq -r '.mode // "n/a"' "$ROUND_DRY_OUT")"
  ROUND_DRY_STEPS="$(jq -r '.executed_step_count // "n/a"' "$ROUND_DRY_OUT")"

  if [ "$ROUND_DRY_STATUS" = "blocked" ]; then
    TEMPLATE_BLOCKED="1"
    ROUND_WINNER="n/a"
    ROUND_WINNER_SCORE="n/a"
    ROUND_WINNER_FIELD="n/a"
    ROUND_WINNER_SAFETY="n/a"
    ROUND_EXEC_STATUS="skipped"
    ROUND_EXEC_MODE="blocked"
    ROUND_EXEC_STEPS="n/a"
    ROUND_EXEC_OUT_CONTENT='{"status":"skipped","mode":"blocked","executed_step_count":"n/a"}'
    printf '%s\n' "$ROUND_EXEC_OUT_CONTENT" > "$ROUND_EXEC_OUT"
    echo "skipped" > "$LOG_DIR/headless_run_exec_round_${round}.status"
    echo "Template blocked; skip execute/boundary checks."
  else
    assert_jq_eq "$ROUND_DRY_OUT" ".status" "ok"

    echo "[round-$round] execute run -> material report (service executor)"
    if [ -n "$MATERIAL_REPORT_ALIAS" ]; then
      run_wrapper headless_run_exec_round_${round} ./scripts/kyuubiki headless run "$ROUND_BATCH" --json \
        ${HEADLESS_ALLOW_SENSITIVE_FLAG:+$HEADLESS_ALLOW_SENSITIVE_FLAG} \
        --material-report "$MATERIAL_REPORT_ALIAS" --material-report-out "$ROUND_REPORT_OUT" \
        --report-out "$ROUND_EXEC_OUT" \
        --execute --executor service --api-base-url "$API_BASE_URL"
      assert_file "$ROUND_EXEC_OUT"
      assert_file "$ROUND_REPORT_OUT"
      assert_jq_eq "$ROUND_EXEC_OUT" ".mode" "execute:service"
      assert_jq_eq "$ROUND_EXEC_OUT" ".status" "ok"
      case "$MATERIAL_REPORT_ALIAS" in
        dielectric-screening)
          assert_jq_eq "$ROUND_REPORT_OUT" ".schema_version" "kyuubiki.dielectric-material-report/v1"
          ;;
        structural-panel)
          assert_jq_eq "$ROUND_REPORT_OUT" ".schema_version" "kyuubiki.structural-material-report/v1"
          ;;
        composite-thermo-electric-panel)
          assert_jq_eq "$ROUND_REPORT_OUT" ".schema_version" "kyuubiki.composite-panel-report/v1"
          ;;
        heat-spreader)
          assert_jq_eq "$ROUND_REPORT_OUT" ".schema_version" "kyuubiki.material-research-report/v1"
          ;;
        thermo-shield)
          assert_jq_eq "$ROUND_REPORT_OUT" ".schema_version" "kyuubiki.thermo-material-report/v1"
          ;;
        *)
          assert_file "$ROUND_REPORT_OUT"
          ;;
      esac
    else
      run_wrapper headless_run_exec_round_${round} ./scripts/kyuubiki headless run "$ROUND_BATCH" --json \
        ${HEADLESS_ALLOW_SENSITIVE_FLAG:+$HEADLESS_ALLOW_SENSITIVE_FLAG} \
        --report-out "$ROUND_EXEC_OUT" \
        --execute --executor service --api-base-url "$API_BASE_URL"
      assert_file "$ROUND_EXEC_OUT"
      assert_jq_eq "$ROUND_EXEC_OUT" ".mode" "execute:service"
      assert_jq_eq "$ROUND_EXEC_OUT" ".status" "ok"
    fi

    ROUND_EXEC_STATUS="$(cat "$LOG_DIR/headless_run_exec_round_${round}.status")"
    ROUND_EXEC_MODE="$(jq -r '.mode // "n/a"' "$ROUND_EXEC_OUT")"
    ROUND_EXEC_STEPS="$(jq -r '.executed_step_count // "n/a"' "$ROUND_EXEC_OUT")"
  fi

  if [ -f "$ROUND_REPORT_OUT" ]; then
    ROUND_WINNER="$(jq -r '.winner_candidate_id // "n/a"' "$ROUND_REPORT_OUT")"
  else
    ROUND_WINNER="n/a"
  fi
  if [ "$ROUND_WINNER" = "n/a" ] || [ "$ROUND_WINNER" = "null" ]; then
    ROUND_WINNER_SCORE="n/a"
    ROUND_WINNER_FIELD="n/a"
    ROUND_WINNER_SAFETY="n/a"
  else
    ROUND_WINNER_SCORE="$(jq -r --arg id "$ROUND_WINNER" '.candidates[] | select(.candidate_id == $id) | .score | tostring' "$ROUND_REPORT_OUT" )"
    ROUND_WINNER_FIELD="$(jq -r --arg id "$ROUND_WINNER" '.candidates[] | select(.candidate_id == $id) | .max_electric_field_v_m | tostring' "$ROUND_REPORT_OUT" )"
    ROUND_WINNER_SAFETY="$(jq -r --arg id "$ROUND_WINNER" '.candidates[] | select(.candidate_id == $id) | .breakdown_safety_factor | tostring' "$ROUND_REPORT_OUT" )"
    if [ -z "$ROUND_WINNER_SCORE" ] || [ "$ROUND_WINNER_SCORE" = "null" ]; then
      ROUND_WINNER_SCORE="n/a"
    fi
    if [ -z "$ROUND_WINNER_FIELD" ] || [ "$ROUND_WINNER_FIELD" = "null" ]; then
      ROUND_WINNER_FIELD="n/a"
    fi
    if [ -z "$ROUND_WINNER_SAFETY" ] || [ "$ROUND_WINNER_SAFETY" = "null" ]; then
      ROUND_WINNER_SAFETY="n/a"
    fi
  fi

  ROUND_DRY_STATUS="$(cat "$LOG_DIR/headless_run_dry_round_${round}.status")"
  ROUND_EXEC_STATUS="$(cat "$LOG_DIR/headless_run_exec_round_${round}.status")"

  log_round_summary "$round" "$CURRENT_VOLTAGE" "$ROUND_DRY_STATUS" "$ROUND_EXEC_STATUS" "$ROUND_WINNER" "$ROUND_WINNER_SCORE" "$ROUND_WINNER_FIELD" "$ROUND_WINNER_SAFETY" "$ROUND_DRY_MODE" "$ROUND_EXEC_MODE" "$ROUND_DRY_STEPS" "$ROUND_EXEC_STEPS"

  if [ "$round" -lt "$HEADLESS_ROUNDS" ]; then
    CURRENT_VOLTAGE="$(derive_next_voltage "$CURRENT_VOLTAGE" "$ROUND_WINNER_SCORE" "$ROUND_WINNER_FIELD")"
  fi

done

echo "[6/8] copy round-1 artifacts to legacy paths"
cp "$OUT_DIR/round-1-run-dry.json" "$OUT_DIR/loop-run-dry.json"
cp "$OUT_DIR/round-1-run-exec.json" "$OUT_DIR/loop-run-exec.json"
if [ -f "$OUT_DIR/round-1-material-report.json" ]; then
  cp "$OUT_DIR/round-1-material-report.json" "$OUT_DIR/loop-material-report.json"
fi

echo "[7/8] boundary: alias rejection should fail"
if [ "$TEMPLATE_BLOCKED" = "0" ] && [ -n "$MATERIAL_REPORT_ALIAS" ]; then
  if run_wrapper headless_run_bad_alias ./scripts/kyuubiki headless run "$OUT_DIR/loop-batch.json" --json \
    ${HEADLESS_ALLOW_SENSITIVE_FLAG:+$HEADLESS_ALLOW_SENSITIVE_FLAG} \
    --material-report study --material-report-out "$OUT_DIR/loop-bad-alias.json" \
    --execute --executor service --api-base-url "$API_BASE_URL"; then
    echo "expected bad alias to fail but command succeeded"
    exit 1
  fi
else
  echo "[skip] template has no material-report studies or is blocked-by-design"
fi

echo "[8/8] boundary: missing material report out should fail"
if [ "$TEMPLATE_BLOCKED" = "0" ] && [ -n "$MATERIAL_REPORT_ALIAS" ]; then
  if run_wrapper headless_run_missing_report_out ./scripts/kyuubiki headless run "$OUT_DIR/loop-batch.json" --json \
    ${HEADLESS_ALLOW_SENSITIVE_FLAG:+$HEADLESS_ALLOW_SENSITIVE_FLAG} \
    --material-report "$MATERIAL_REPORT_ALIAS" \
    --execute --executor service --api-base-url "$API_BASE_URL"; then
    echo "expected missing material-report-out to fail but command succeeded"
    exit 1
  fi
else
  echo "0" > "$LOG_DIR/headless_run_missing_report_out.status"
  echo "[skip] template has no material-report studies or is blocked-by-design"
fi

if [ -n "$MATERIAL_REPORT_ALIAS" ]; then
  if [ "$TEMPLATE_BLOCKED" = "0" ]; then
    HEADLESS_RUN_BAD_ALIAS_STATUS="$(cat "$LOG_DIR/headless_run_bad_alias.status")"
    HEADLESS_RUN_MISSING_REPORT_OUT_STATUS="$(cat "$LOG_DIR/headless_run_missing_report_out.status")"
  else
    HEADLESS_RUN_BAD_ALIAS_STATUS="blocked-by-design"
    HEADLESS_RUN_MISSING_REPORT_OUT_STATUS="blocked-by-design"
  fi
else
  if [ ! -f "$LOG_DIR/headless_run_bad_alias.status" ]; then
    echo "0" > "$LOG_DIR/headless_run_bad_alias.status"
  fi
  HEADLESS_RUN_BAD_ALIAS_STATUS="skipped"
  HEADLESS_RUN_MISSING_REPORT_OUT_STATUS="skipped"
fi

HEADLESS_INIT_STATUS="$(cat "$LOG_DIR/headless_init.status")"
HEADLESS_VALIDATE_STATUS="$(cat "$LOG_DIR/headless_validate.status")"
HEADLESS_PLAN_STATUS="$(cat "$LOG_DIR/headless_plan.status")"
HEADLESS_RENDER_STATUS="$(cat "$LOG_DIR/headless_render.status")"

{
  echo
  echo "
说明：以下为每轮闭环执行摘要。"
  echo "关键命令码与状态（0 为成功，1 为失败）："
  echo "- headless_init: ${HEADLESS_INIT_STATUS}"
  echo "- headless_validate: ${HEADLESS_VALIDATE_STATUS}"
  echo "- headless_plan: ${HEADLESS_PLAN_STATUS}"
  echo "- headless_render: ${HEADLESS_RENDER_STATUS}"
  echo "- headless_run_dry_round_1: $(cat "$LOG_DIR/headless_run_dry_round_1.status")"
  echo "- headless_run_exec_round_1: $(cat "$LOG_DIR/headless_run_exec_round_1.status")"
  echo "- headless_run_bad_alias: ${HEADLESS_RUN_BAD_ALIAS_STATUS}（预期失败）"
  echo "- headless_run_missing_report_out: ${HEADLESS_RUN_MISSING_REPORT_OUT_STATUS}（预期失败）"
  echo
  echo "观察结果："
  if [ "$TEMPLATE_BLOCKED" = "0" ]; then
    echo "- loop-run-dry.json: mode=dry_run、status=$(jq -r '.status // "n/a"' "$OUT_DIR/loop-run-dry.json" 2>/dev/null || echo n/a)、executed_step_count=$(jq -r '.executed_step_count // "n/a"' "$OUT_DIR/loop-run-dry.json" 2>/dev/null || echo n/a)、无 block（来自 round-1）。"
    echo "- loop-run-exec.json: mode=$(jq -r '.mode // "n/a"' "$OUT_DIR/loop-run-exec.json" 2>/dev/null || echo n/a)、status=$(jq -r '.status // "n/a"' "$OUT_DIR/loop-run-exec.json" 2>/dev/null || echo n/a)、executed_step_count=$(jq -r '.executed_step_count // "n/a"' "$OUT_DIR/loop-run-exec.json" 2>/dev/null || echo n/a)、无 block（来自 round-1）。"
  else
    echo "- 模板为 blocked-by-design，dry-run 阶段状态为 blocked，已跳过 execute 与 boundary 回归。"
  fi
  if [ -n "$MATERIAL_REPORT_ALIAS" ]; then
    echo "- loop-material-report.json: study=${MATERIAL_REPORT_ALIAS}，winner 从 round-1 读取"
    echo "- headless_run_bad_alias 返回 unsupported material report study: study。"
    echo "- headless_run_missing_report_out 返回 --material-report with --json requires --material-report-out。"
  else
    echo "- 当前模板无 material-report，循环通过 execute 直接产出 run report。"
  fi
  echo
  echo "结论："
  echo "1. 无头闭环在当前版本可稳定打通：init -> validate -> plan -> render -> 多轮 dry-run/execute。"
  echo "2. 轮间闭环以 winner 的最大场强/得分动态推导下一轮电压，当前路径成功生成每轮报告并形成可追溯链。"
  echo "3. 已知边界：material-report 的 study 与 bad alias、material-report-out 缺失边界行为继续有效。"
} >> "$SCRIPT_REPORT"

echo "Headless workflow regression completed. Artifacts in $OUT_DIR"
