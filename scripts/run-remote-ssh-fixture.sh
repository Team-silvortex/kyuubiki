#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_DIR="${ROOT_DIR}/tests/integration/remote-ssh-fixture"
RUNTIME_DIR="${FIXTURE_DIR}/runtime"
KEY_PATH="${RUNTIME_DIR}/client_key"
KNOWN_HOSTS_PATH="${RUNTIME_DIR}/known_hosts"
COMPOSE_FILE="${FIXTURE_DIR}/compose.yaml"

cleanup() {
  docker compose -f "${COMPOSE_FILE}" down >/dev/null 2>&1 || true
}

mkdir -p "${RUNTIME_DIR}/workspace"

if [ ! -f "${KEY_PATH}" ]; then
  ssh-keygen -t ed25519 -N "" -f "${KEY_PATH}" -C kyuubiki-remote-ssh-fixture >/dev/null
fi

docker compose -f "${COMPOSE_FILE}" up -d --build
trap cleanup EXIT

for _ in $(seq 1 30); do
  if docker compose -f "${COMPOSE_FILE}" ps --status running | grep -q "ssh-fixture"; then
    if ssh \
      -i "${KEY_PATH}" \
      -o UserKnownHostsFile="${KNOWN_HOSTS_PATH}" \
      -o StrictHostKeyChecking=accept-new \
      -o ConnectTimeout=2 \
      -p 2222 \
      kyuubiki-fixture@127.0.0.1 \
      "cd /tmp/kyuubiki-fixture && printf '%s' 'kyuubiki-remote-ok'" \
      | grep -q "kyuubiki-remote-ok"; then
      echo "remote ssh fixture probe ok"
      exit 0
    fi
  fi
  sleep 1
done

echo "remote ssh fixture did not become reachable" >&2
exit 1
