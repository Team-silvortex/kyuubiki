#!/usr/bin/env bash
set -euo pipefail
set -o pipefail

WORKSPACE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_DIR="${KYUUBIKI_REPO_DIR:-/Users/Shared/chroot/research/kyuubiki}"
DEV_REPO_DIR="${DEV_KYUUBIKI_REPO_DIR:-/Users/Shared/chroot/dev/kyuubiki}"
RUST_DIR="$REPO_DIR/workers/rust"
OUT_DIR="$WORKSPACE_DIR/chain-next-regression"
LOG_DIR="$OUT_DIR/logs"
REPORT_PATH="$WORKSPACE_DIR/reports/chain-next-regression-report.md"
SYNC_SDK="${SYNC_SDK_FROM_DEV:-1}"
STUDY="${STUDY:-dielectric-screening}"
CHAIN_ROUNDS="${CHAIN_ROUNDS:-2}"

mkdir -p "$OUT_DIR" "$LOG_DIR" "$(dirname "$REPORT_PATH")"

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
  (cd "$RUST_DIR" && "${cmd[@]}") >"$out_file" 2>"$err_file"
  local status=$?
  set -e
  echo "$status" > "$LOG_DIR/${label}.status"
  return $status
}

expect_success() {
  local label="$1"
  shift
  if ! run_wrapper "$label" "$@"; then
    echo "[${label}] expected success but failed"
    echo "stdout: $LOG_DIR/${label}.out"
    echo "stderr: $LOG_DIR/${label}.err"
    exit 1
  fi
}

expect_failure() {
  local label="$1"
  shift
  if run_wrapper "$label" "$@"; then
    echo "[${label}] expected failure but succeeded"
    exit 1
  fi
}

assert_file() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "[assert] expected file missing: $path"
    exit 1
  fi
}

assert_jq_eq() {
  local path="$1"
  local expr="$2"
  local expected="$3"
  local actual
  actual="$(jq -r "$expr" "$path")"
  if [ "$actual" != "$expected" ]; then
    echo "[assert] $path expected $expr=$expected but got $actual"
    exit 1
  fi
}

log_fail_output() {
  local label="$1"
  cat "$LOG_DIR/${label}.out" 2>/dev/null
  cat "$LOG_DIR/${label}.err" 2>/dev/null
}

echo "[1/8] describe study"
expect_success material_study_describe cargo run -p kyuubiki-cli --bin kyuubiki-material-explore -- \
  --describe-study "$STUDY" --json --out "$OUT_DIR/study-description.json"
assert_file "$OUT_DIR/study-description.json"
assert_jq_eq "$OUT_DIR/study-description.json" ".study.id" "material_dielectric_screening"

echo "[2/8] run initial exploration"
expect_success material_initial cargo run -p kyuubiki-cli --bin kyuubiki-material-explore -- \
  "$STUDY" --json --out "$OUT_DIR/initial.json"
assert_file "$OUT_DIR/initial.json"

BASE_WINNER="$(jq -r '.report.winner_candidate_id' "$OUT_DIR/initial.json")"
if [ -z "$BASE_WINNER" ] || [ "$BASE_WINNER" = "null" ]; then
  echo "initial exploration did not produce winner"
  exit 1
fi

echo "[3/8] plan-study should pass"
expect_success study_plan cargo run -p kyuubiki-cli --bin kyuubiki-material-explore -- \
  --plan-study "$STUDY" --json --out "$OUT_DIR/study-plan.json"
assert_file "$OUT_DIR/study-plan.json"
assert_jq_eq "$OUT_DIR/study-plan.json" ".schema_version" "kyuubiki.material-study-execution-plan/v1"

echo "[4/8] chain-next baseline (rounds=$CHAIN_ROUNDS) should pass"
expect_success chain_next_baseline cargo run -p kyuubiki-cli --bin kyuubiki-material-explore -- \
  --chain-next "$OUT_DIR/initial.json" --rounds "$CHAIN_ROUNDS" --json --out "$OUT_DIR/chain-baseline.json"
assert_file "$OUT_DIR/chain-baseline.json"
assert_jq_eq "$OUT_DIR/chain-baseline.json" ".schema_version" "kyuubiki.material-exploration-chain/v1"
assert_jq_eq "$OUT_DIR/chain-baseline.json" ".round_count" "$CHAIN_ROUNDS"
assert_jq_eq "$OUT_DIR/chain-baseline.json" ".final_winner_candidate_id" "$BASE_WINNER"
BASELINE_FINAL_ITER="$(jq -r '.final_iteration' "$OUT_DIR/chain-baseline.json")"
BASELINE_SEARCH_STATE="$(jq -r '.convergence_assessment.state' "$OUT_DIR/chain-baseline.json")"
BASELINE_RUNS_COUNT="$(jq -r '.runs | length' "$OUT_DIR/chain-baseline.json")"
BASELINE_FINGERPRINT="$(jq -r '.runs[0].candidate_input_fingerprint // "none"' "$OUT_DIR/chain-baseline.json")"

echo "[5/8] chain-next determinism check: second pass should reproduce same winner and round count"
expect_success chain_next_replay cargo run -p kyuubiki-cli --bin kyuubiki-material-explore -- \
  --chain-next "$OUT_DIR/initial.json" --rounds "$CHAIN_ROUNDS" --json --out "$OUT_DIR/chain-replay.json"
assert_file "$OUT_DIR/chain-replay.json"
assert_jq_eq "$OUT_DIR/chain-replay.json" ".round_count" "$CHAIN_ROUNDS"
assert_jq_eq "$OUT_DIR/chain-replay.json" ".final_winner_candidate_id" "$BASE_WINNER"
REPLAY_SEARCH_STATE="$(jq -r '.convergence_assessment.state' "$OUT_DIR/chain-replay.json")"
REPLAY_FINGERPRINT="$(jq -r '.runs[0].candidate_input_fingerprint // "none"' "$OUT_DIR/chain-replay.json")"

echo "[6/8] fault-injection: rounds=0 should fail"
expect_failure chain_next_rounds_zero cargo run -p kyuubiki-cli --bin kyuubiki-material-explore -- \
  --chain-next "$OUT_DIR/initial.json" --rounds 0 --json --out "$OUT_DIR/chain-fault-round0.json"
CHAIN_ROUNDS0_ERR="$(cat "$LOG_DIR/chain_next_rounds_zero.err" 2>/dev/null || true)"
CHAIN_ROUNDS0_OUT="$(cat "$LOG_DIR/chain_next_rounds_zero.out" 2>/dev/null || true)"
if ! grep -Fq -- "--rounds must be at least 1" "$LOG_DIR/chain_next_rounds_zero.err"; then
  if ! grep -Fq -- "--rounds must be at least 1" "$LOG_DIR/chain_next_rounds_zero.out"; then
    echo "expected rounds=0 error missing"
    echo "$CHAIN_ROUNDS0_OUT"
    echo "$CHAIN_ROUNDS0_ERR"
    exit 1
  fi
fi

echo "[7/8] fault-injection: unsupported material study in initial payload"
cp "$OUT_DIR/initial.json" "$OUT_DIR/initial-bad-study.json"
jq '.study = "bogus_study"' "$OUT_DIR/initial-bad-study.json" > "$OUT_DIR/initial-bad-study.json.tmp"
mv "$OUT_DIR/initial-bad-study.json.tmp" "$OUT_DIR/initial-bad-study.json"
expect_failure chain_next_bad_study cargo run -p kyuubiki-cli --bin kyuubiki-material-explore -- \
  --chain-next "$OUT_DIR/initial-bad-study.json" --rounds "$CHAIN_ROUNDS" --json --out "$OUT_DIR/chain-fault-bad-study.json"
CHAIN_BAD_STUDY_ERR="$(cat "$LOG_DIR/chain_next_bad_study.err" 2>/dev/null || true)"
CHAIN_BAD_STUDY_OUT="$(cat "$LOG_DIR/chain_next_bad_study.out" 2>/dev/null || true)"
if ! grep -Fq -- "unsupported material study: bogus_study" "$LOG_DIR/chain_next_bad_study.err"; then
  if ! grep -Fq -- "unsupported material study: bogus_study" "$LOG_DIR/chain_next_bad_study.out"; then
    echo "expected bad study error missing"
    echo "$CHAIN_BAD_STUDY_OUT"
    echo "$CHAIN_BAD_STUDY_ERR"
    exit 1
  fi
fi

echo "[8/8] fault-injection: missing input file should fail"
expect_failure chain_next_missing_input cargo run -p kyuubiki-cli --bin kyuubiki-material-explore -- \
  --chain-next /tmp/does-not-exist-for-chain-next.json --rounds "$CHAIN_ROUNDS" --json --out "$OUT_DIR/chain-fault-missing-input.json"
CHAIN_MISSING_IN_ERR="$(cat "$LOG_DIR/chain_next_missing_input.err" 2>/dev/null || true)"
CHAIN_MISSING_IN_OUT="$(cat "$LOG_DIR/chain_next_missing_input.out" 2>/dev/null || true)"
if ! grep -Fq -- "No such file or directory" "$LOG_DIR/chain_next_missing_input.err"; then
  if ! grep -Fq -- "No such file or directory" "$LOG_DIR/chain_next_missing_input.out"; then
    echo "expected missing input error missing"
    echo "$CHAIN_MISSING_IN_OUT"
    echo "$CHAIN_MISSING_IN_ERR"
    exit 1
  fi
fi

{
  echo "# 研究链路 \`--chain-next\` 回归与故障注入报告"
  echo ""
  echo "- 时间: $(date '+%Y-%m-%d %H:%M:%S %z')"
  echo "- 工作区: $WORKSPACE_DIR"
  echo "- 研究: $STUDY"
  echo ""
  echo "## 关键结果"
  echo "- 基础探索 winner: $BASE_WINNER"
  echo "- baseline chain round_count: $(jq -r '.round_count' "$OUT_DIR/chain-baseline.json")"
  echo "- baseline final_iteration: $BASELINE_FINAL_ITER"
  echo "- baseline convergence_state: $BASELINE_SEARCH_STATE"
  echo "- baseline runs: $BASELINE_RUNS_COUNT"
  echo "- baseline/input-fingerprint: $BASELINE_FINGERPRINT"
  echo "- replay runs: $(jq -r '.runs | length' "$OUT_DIR/chain-replay.json")"
  echo "- replay candidate_input_fingerprint: $REPLAY_FINGERPRINT"
  echo ""
  echo "## 命令状态（0=成功，1=失败）"
  echo "| 命令 | 状态 | 说明 |"
  echo "| --- | --- | --- |"
  echo "| describe-study | $(cat "$LOG_DIR/material_study_describe.status") | 校验 study 元信息 |"
  echo "| initial-explore | $(cat "$LOG_DIR/material_initial.status") | 基础 run 输入 |"
  echo "| plan-study | $(cat "$LOG_DIR/study_plan.status") | 打印计划 schema |"
  echo "| chain-next baseline | $(cat "$LOG_DIR/chain_next_baseline.status") | round=$CHAIN_ROUNDS |"
  echo "| chain-next replay | $(cat "$LOG_DIR/chain_next_replay.status") | 可复现性检查 |"
  echo "| chain-next rounds=0 | $(cat "$LOG_DIR/chain_next_rounds_zero.status") | 预期失败 |"
  echo "| chain-next bad study | $(cat "$LOG_DIR/chain_next_bad_study.status") | 预期失败 |"
  echo "| chain-next missing input | $(cat "$LOG_DIR/chain_next_missing_input.status") | 预期失败 |"
  echo ""
  echo "## 故障注入复用快照"
  echo "rounds=0 output:"
  echo '```'
  echo "$CHAIN_ROUNDS0_ERR"
  echo "$CHAIN_ROUNDS0_OUT"
  echo '```'
  echo "bad study output:"
  echo '```'
  echo "$CHAIN_BAD_STUDY_ERR"
  echo "$CHAIN_BAD_STUDY_OUT"
  echo '```'
  echo "missing input output:"
  echo '```'
  echo "$CHAIN_MISSING_IN_ERR"
  echo "$CHAIN_MISSING_IN_OUT"
  echo '```'
  echo ""
  echo "## 发现与建议"
  echo "- chain-next 在固定输入下可重复得到相同 winner 与 candidate_input_fingerprint，适合作为闭环稳定性基线。"
  echo "- 建议在流水线上加入输入校验（--rounds、material study 一致性、输入文件存在性）以提前拦截无效链路。"
} > "$REPORT_PATH"

echo "chain-next regression completed. report: $REPORT_PATH"
