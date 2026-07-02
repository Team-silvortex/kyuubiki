ensure_desktop_cli_ready() {
  local app_dir="$1"
  local app_name="$2"

  if [[ ! -d "$app_dir/node_modules/@tauri-apps/cli" ]]; then
    echo "${app_name} desktop dependencies are missing under ${app_dir}/node_modules" >&2
    echo "run: cd \"$app_dir\" && npm install" >&2
    return 1
  fi
}

desktop_tauri_conf_for_app() {
  local app="$1"
  case "$app" in
    hub-gui) echo "$HUB_GUI_DIR/src-tauri/tauri.conf.json" ;;
    installer-gui) echo "$INSTALLER_GUI_DIR/src-tauri/tauri.conf.json" ;;
    workbench-gui) echo "$WORKBENCH_GUI_DIR/src-tauri/tauri.conf.json" ;;
    *)
      echo "unknown desktop app: $app" >&2
      return 1
      ;;
  esac
}

desktop_product_name_for_app() {
  local app="$1"
  local conf product_name
  conf="$(desktop_tauri_conf_for_app "$app")" || return 1
  product_name="$(sed -n 's/.*"productName":[[:space:]]*"\([^"]*\)".*/\1/p' "$conf" | head -n 1)"
  echo "${product_name:-$app}"
}

desktop_version_for_app() {
  local app="$1"
  local conf version
  conf="$(desktop_tauri_conf_for_app "$app")" || return 1
  version="$(sed -n 's/.*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "$conf" | head -n 1)"
  echo "${version:-0.0.0}"
}

desktop_macos_app_bundle_path_for_app() {
  local app="$1"
  local product_name bundle_dir
  product_name="$(desktop_product_name_for_app "$app")" || return 1
  bundle_dir="$(desktop_bundle_dir_for_app "$app")"
  echo "$bundle_dir/macos/${product_name}.app"
}

desktop_macos_dmg_path_for_app() {
  local app="$1"
  local product_name version arch bundle_dir
  product_name="$(desktop_product_name_for_app "$app")" || return 1
  version="$(desktop_version_for_app "$app")" || return 1
  arch="$(uname -m)"
  bundle_dir="$(desktop_bundle_dir_for_app "$app")"
  echo "$bundle_dir/dmg/${product_name}_${version}_${arch}.dmg"
}

ensure_fallback_macos_dmg_for_app() {
  local app="$1"
  local app_bundle dmg_path product_name

  if [[ "$(host_platform)" != "macos" ]]; then
    return 1
  fi

  app_bundle="$(desktop_macos_app_bundle_path_for_app "$app")" || return 1
  dmg_path="$(desktop_macos_dmg_path_for_app "$app")" || return 1
  product_name="$(desktop_product_name_for_app "$app")" || return 1

  if [[ ! -d "$app_bundle" ]]; then
    return 1
  fi

  if [[ -f "$dmg_path" ]]; then
    return 0
  fi

  mkdir -p "$(dirname "$dmg_path")"
  echo "creating fallback dmg for ${app} at ${dmg_path}"
  if hdiutil create -volname "$product_name" -srcfolder "$app_bundle" -ov -format UDZO "$dmg_path"; then
    return 0
  fi

  echo "fallback dmg creation failed for ${app}; this host session may not support hdiutil disk image creation" >&2
  return 1
}

run_hub_gui_dev() {
  require_dir "$HUB_GUI_DIR"
  ensure_desktop_cli_ready "$HUB_GUI_DIR" "hub-gui"
  (
    cd "$HUB_GUI_DIR"
    npm run tauri:dev
  )
}

run_hub_gui_build() {
  local platform="${1:-$(host_platform)}"
  local build_exit=0
  require_dir "$HUB_GUI_DIR"
  ensure_desktop_cli_ready "$HUB_GUI_DIR" "hub-gui"
  if (
    cd "$HUB_GUI_DIR"
    if [[ "$platform" == "$(host_platform)" ]]; then
      npm run tauri:build
    else
      echo "hub-gui cross-platform bundle build is not performed on this host; staging ${platform} desktop manifest instead"
    fi
  ); then
    build_exit=0
  else
    build_exit=$?
  fi
  if [[ "$platform" == "$(host_platform)" && "$platform" == "macos" && "$build_exit" -ne 0 ]]; then
    if ensure_fallback_macos_dmg_for_app "hub-gui"; then
      echo "hub-gui tauri dmg bundling failed; fallback dmg created from app bundle"
      build_exit=0
    fi
  fi
  run_installer stage-release "$platform"
  return "$build_exit"
}

run_installer_gui_dev() {
  require_dir "$INSTALLER_GUI_DIR"
  ensure_desktop_cli_ready "$INSTALLER_GUI_DIR" "installer-gui"
  (
    cd "$INSTALLER_GUI_DIR"
    npm run tauri:dev
  )
}

run_installer_gui_build() {
  local platform="${1:-$(host_platform)}"
  local build_exit=0
  require_dir "$INSTALLER_GUI_DIR"
  ensure_desktop_cli_ready "$INSTALLER_GUI_DIR" "installer-gui"
  if (
    cd "$INSTALLER_GUI_DIR"
    if [[ "$platform" == "$(host_platform)" ]]; then
      npm run tauri:build
    else
      echo "installer-gui cross-platform bundle build is not performed on this host; staging ${platform} desktop manifest instead"
    fi
  ); then
    build_exit=0
  else
    build_exit=$?
  fi
  if [[ "$platform" == "$(host_platform)" && "$platform" == "macos" && "$build_exit" -ne 0 ]]; then
    if ensure_fallback_macos_dmg_for_app "installer-gui"; then
      echo "installer-gui tauri dmg bundling failed; fallback dmg created from app bundle"
      build_exit=0
    fi
  fi
  run_installer stage-release "$platform"
  return "$build_exit"
}

run_workbench_gui_dev() {
  require_dir "$WORKBENCH_GUI_DIR"
  ensure_desktop_cli_ready "$WORKBENCH_GUI_DIR" "workbench-gui"
  (
    cd "$WORKBENCH_GUI_DIR"
    npm run tauri:dev
  )
}

run_workbench_gui_build() {
  local platform="${1:-$(host_platform)}"
  local build_exit=0
  require_dir "$WORKBENCH_GUI_DIR"
  ensure_desktop_cli_ready "$WORKBENCH_GUI_DIR" "workbench-gui"
  if (
    cd "$WORKBENCH_GUI_DIR"
    if [[ "$platform" == "$(host_platform)" ]]; then
      npm run tauri:build
    else
      echo "workbench-gui cross-platform bundle build is not performed on this host; staging ${platform} desktop manifest instead"
    fi
  ); then
    build_exit=0
  else
    build_exit=$?
  fi
  if [[ "$platform" == "$(host_platform)" && "$platform" == "macos" && "$build_exit" -ne 0 ]]; then
    if ensure_fallback_macos_dmg_for_app "workbench-gui"; then
      echo "workbench-gui tauri dmg bundling failed; fallback dmg created from app bundle"
      build_exit=0
    fi
  fi
  run_installer stage-release "$platform"
  return "$build_exit"
}

run_build_frontend() {
  require_dir "$FRONTEND_DIR"
  (
    cd "$FRONTEND_DIR"
    npm run build
  )
}

run_build_orchestrator() {
  require_dir "$WEB_DIR"
  (
    cd "$WEB_DIR"
    MIX_ENV=prod mix compile
  )
}

run_build_agent() {
  require_dir "$RUST_DIR"
  (
    cd "$RUST_DIR"
    cargo build -p kyuubiki-cli --release
  )
}

run_package_runtime() {
  run_installer stage-release "$@"
}

run_package_desktop() {
  local platform="${1:-$(host_platform)}"

  if [[ "$platform" == "all" ]]; then
    local target
    for target in macos linux windows; do
      run_installer stage-release "$target"
    done
    run_hub_gui_build "$(host_platform)"
    run_installer_gui_build "$(host_platform)"
    run_workbench_gui_build "$(host_platform)"
    return 0
  fi

  run_installer stage-release "$platform"
  run_hub_gui_build "$platform"
  run_installer_gui_build "$platform"
  run_workbench_gui_build "$platform"
}
