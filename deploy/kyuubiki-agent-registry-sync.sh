#!/usr/bin/env bash
set -euo pipefail

KYUUBIKI_HOME="${KYUUBIKI_HOME:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
ORCH_ENV_FILE="${KYUUBIKI_ORCHESTRATOR_ENV_FILE:-${KYUUBIKI_HOME}/deploy/kyuubiki-orchestrator.env}"
AGENT_ENV_FILE="${KYUUBIKI_AGENT_ENV_FILE:-${KYUUBIKI_HOME}/deploy/kyuubiki-agent.env}"
ORCH_BASE_URL="${KYUUBIKI_AGENT_REGISTRY_URL:-http://127.0.0.1:4000}"
HEARTBEAT_INTERVAL_MS="${KYUUBIKI_AGENT_REGISTRY_INTERVAL_MS:-5000}"
AGENT_HOST_FALLBACK="${KYUUBIKI_AGENT_ADVERTISE_HOST_FALLBACK:-kyuubiki-lab.local}"
AGENT_PORT_FALLBACK="${KYUUBIKI_AGENT_PORT_FALLBACK:-5001}"

read_env_value() {
  local file="$1"
  local key="$2"
  sed -n "s/^${key}=//p" "$file" | head -n 1
}

cluster_token="$(read_env_value "$ORCH_ENV_FILE" "KYUUBIKI_CLUSTER_API_TOKEN")"
agent_id="$(read_env_value "$AGENT_ENV_FILE" "KYUUBIKI_AGENT_ID")"
agent_host="$(read_env_value "$AGENT_ENV_FILE" "KYUUBIKI_AGENT_ADVERTISE_HOST")"
agent_port="$(read_env_value "$AGENT_ENV_FILE" "KYUUBIKI_AGENT_PORT")"
fingerprint="$(read_env_value "$AGENT_ENV_FILE" "KYUUBIKI_AGENT_FINGERPRINT")"
cluster_id="$(read_env_value "$AGENT_ENV_FILE" "KYUUBIKI_AGENT_CLUSTER_ID")"

agent_host="${agent_host:-$AGENT_HOST_FALLBACK}"
agent_port="${agent_port:-$AGENT_PORT_FALLBACK}"

if [[ -z "${cluster_token}" || -z "${agent_id}" || -z "${fingerprint}" ]]; then
  echo "missing registry sync configuration" >&2
  exit 1
fi

json_payload() {
  if [[ -n "${cluster_id}" ]]; then
    printf '{"id":"%s","host":"%s","port":%s,"role":"solver","control_mode":"orch_managed","orch_id":"%s","cluster_id":"%s","tags":["headless","standalone"],"methods":[],"capabilities":[],"health_score":100}' \
      "$agent_id" "$agent_host" "$agent_port" "$ORCH_BASE_URL" "$cluster_id"
  else
    printf '{"id":"%s","host":"%s","port":%s,"role":"solver","control_mode":"orch_managed","orch_id":"%s","tags":["headless","standalone"],"methods":[],"capabilities":[],"health_score":100}' \
      "$agent_id" "$agent_host" "$agent_port" "$ORCH_BASE_URL"
  fi
}

cluster_headers() {
  local ts nonce
  ts="$(date +%s%3N)"
  nonce="registry-${agent_id}-${ts}"
  printf '%s\n' \
    -H "content-type: application/json" \
    -H "x-kyuubiki-token: ${cluster_token}" \
    -H "x-kyuubiki-agent-id: ${agent_id}" \
    -H "x-kyuubiki-cluster-ts: ${ts}" \
    -H "x-kyuubiki-cluster-nonce: ${nonce}" \
    -H "x-kyuubiki-agent-fingerprint: ${fingerprint}"
}

post_with_headers() {
  local url="$1"
  local payload="$2"
  shift 2
  curl -fsS --max-time 8 -X POST "$url" "$@" --data-binary "$payload" >/dev/null
}

delete_with_headers() {
  local url="$1"
  shift
  curl -fsS --max-time 8 -X DELETE "$url" "$@" >/dev/null
}

register_once() {
  local payload
  payload="$(json_payload)"
  mapfile -t headers < <(cluster_headers)
  post_with_headers "${ORCH_BASE_URL}/api/v1/agents/register" "$payload" "${headers[@]}"
}

heartbeat_once() {
  local payload
  payload="$(json_payload)"
  mapfile -t headers < <(cluster_headers)
  post_with_headers "${ORCH_BASE_URL}/api/v1/agents/${agent_id}/heartbeat" "$payload" "${headers[@]}"
}

unregister_once() {
  mapfile -t headers < <(cluster_headers)
  delete_with_headers "${ORCH_BASE_URL}/api/v1/agents/${agent_id}" "${headers[@]}"
}

cleanup() {
  unregister_once || true
}

trap cleanup EXIT INT TERM

register_once

while true; do
  sleep "$(awk "BEGIN { print ${HEARTBEAT_INTERVAL_MS} / 1000 }")"
  heartbeat_once
done
