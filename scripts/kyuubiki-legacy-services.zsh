start_orchestrator_background() {
  local port="${1:-4000}"
  local mode="${2:-default}"
  ensure_run_dir
  require_screen

  if port_in_use "$port"; then
    echo "orchestrator already running at http://127.0.0.1:${port}"
    return 0
  fi

  screen -S "$ORCHESTRATOR_SCREEN" -X quit >/dev/null 2>&1 || true
  screen -dmS "$ORCHESTRATOR_SCREEN" sh -lc "cd \"$WEB_DIR\" && $(storage_mode_exports "$mode") $(deployment_mode_exports "$mode") KYUUBIKI_AGENT_ENDPOINTS=\"$(agent_endpoints_value)\" PORT=\"$port\" mix run --no-halt >> \"$ORCHESTRATOR_LOG\" 2>&1"
  wait_for_port_state "$port" listening 15 || true

  echo "started orchestrator API at http://127.0.0.1:${port} ($(storage_mode_label "$mode"), $(deployment_mode_label "$mode"))"
  echo "log: $ORCHESTRATOR_LOG"
}

start_agent_background() {
  local port="${1:-5001}"
  local log_file="$RUN_DIR/agent-${port}.log"
  local screen_name="${AGENT_SCREEN_PREFIX}_${port}"
  ensure_run_dir
  require_screen

  if port_in_use "$port"; then
    echo "Rust FEM agent already running at tcp://127.0.0.1:${port}"
    return 0
  fi

  screen -S "$screen_name" -X quit >/dev/null 2>&1 || true
  screen -dmS "$screen_name" sh -lc "cd \"$RUST_DIR\" && cargo run -p kyuubiki-cli -- agent --port \"$port\" >> \"$log_file\" 2>&1"
  wait_for_port_state "$port" listening 20 || true

  echo "started Rust FEM agent at tcp://127.0.0.1:${port}"
  echo "log: $log_file"
}

start_agents_background() {
  local agent_ports=("${(@s:,:)$(agent_endpoints_value)}")
  local agent_port_spec agent_port

  for agent_port_spec in "${agent_ports[@]}"; do
    agent_port="${agent_port_spec##*:}"
    start_agent_background "$agent_port"
  done
}

start_frontend_background() {
  local port="${1:-3000}"
  ensure_run_dir
  require_screen

  if port_in_use "$port"; then
    echo "Next.js workbench already running at http://127.0.0.1:${port}"
    return 0
  fi

  screen -S "$FRONTEND_SCREEN" -X quit >/dev/null 2>&1 || true
  screen -dmS "$FRONTEND_SCREEN" sh -lc "cd \"$FRONTEND_DIR\" && npm run dev >> \"$FRONTEND_LOG\" 2>&1"
  wait_for_port_state "$port" listening 20 || true

  echo "started Next.js workbench at http://127.0.0.1:${port}"
  echo "log: $FRONTEND_LOG"
}

hot_screen_running() {
  screen -ls 2>/dev/null | grep -q "[[:space:]]${HOT_STACK_SCREEN}[[:space:]]"
}

hot_mode_label() {
  local mode="${1:-local}"
  case "$mode" in
    cloud) echo "cloud" ;;
    distributed) echo "distributed" ;;
    *) echo "local" ;;
  esac
}

start_hot_stack_background() {
  local mode="${1:-local}"
  ensure_run_dir
  ensure_hot_run_dir
  require_screen

  if hot_screen_running; then
    echo "managed hot-reload loop already running in screen session ${HOT_STACK_SCREEN}"
    return 0
  fi

  screen -S "$HOT_STACK_SCREEN" -X quit >/dev/null 2>&1 || true
  screen -dmS "$HOT_STACK_SCREEN" sh -lc "cd \"$ROOT_DIR\" && $(storage_mode_exports "$mode") $(deployment_mode_exports "$mode") KYUUBIKI_AGENT_ENDPOINTS=\"$(agent_endpoints_value)\" KYUUBIKI_HOT_LOG_DIR=\"$HOT_RUN_DIR\" node ./scripts/hot-dev.mjs stack --mode \"$mode\" --orchestrator-port 4000 --frontend-port 3000 --agent-endpoints \"$(agent_endpoints_value)\" >> \"$HOT_RUN_DIR/stack.console.log\" 2>&1"

  echo "started managed hot-reload loop (${mode}) in screen session ${HOT_STACK_SCREEN}"
  echo "logs: $HOT_RUN_DIR"
}

stop_hot_stack_background() {
  require_screen

  if hot_screen_running; then
    screen -S "$HOT_STACK_SCREEN" -X quit >/dev/null 2>&1 || true
    echo "stopped managed hot-reload loop"
  else
    echo "managed hot-reload loop is not running"
  fi
}

show_hot_status() {
  local agent_ports=("${(@s:,:)$(agent_endpoints_value)}")
  local agent_port_spec agent_port agent_pid

  if hot_screen_running; then
    echo "hot-loop: running (${HOT_STACK_SCREEN})"
  else
    echo "hot-loop: stopped"
  fi

  if port_in_use 4000; then
    echo "hot-web: listening on http://127.0.0.1:4000 (pid $(pid_on_port 4000))"
  else
    echo "hot-web: stopped"
  fi

  if port_in_use 3000; then
    echo "hot-frontend: listening on http://127.0.0.1:3000 (pid $(pid_on_port 3000))"
  else
    echo "hot-frontend: stopped"
  fi

  for agent_port_spec in "${agent_ports[@]}"; do
    agent_port="${agent_port_spec##*:}"
    agent_pid="$(pid_on_port "$agent_port")"
    if [[ -n "${agent_pid:-}" ]]; then
      echo "hot-agent[$agent_port]: listening on tcp://127.0.0.1:${agent_port} (pid $agent_pid)"
    else
      echo "hot-agent[$agent_port]: stopped"
    fi
  done

  if [[ -d "$HOT_RUN_DIR" ]]; then
    echo "hot-logs: $HOT_RUN_DIR"
  fi
}

stop_services() {
  local agent_ports=("${(@s:,:)$(agent_endpoints_value)}")
  local agent_port_spec

  for agent_port_spec in "${agent_ports[@]}"; do
    stop_port "${agent_port_spec##*:}"
  done
  stop_port 3000
  stop_port 4000
}

start_services() {
  local mode="${1:-default}"
  if [[ "$mode" != "distributed" ]]; then
    start_agents_background
  fi
  start_orchestrator_background 4000 "$mode"
  start_frontend_background 3000
}

restart_services() {
  local mode="${1:-default}"
  stop_services
  start_services "$mode"
  echo "restart complete"
}
