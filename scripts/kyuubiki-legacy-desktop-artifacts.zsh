desktop_target_apps() {
  echo "hub-gui installer-gui workbench-gui"
}

desktop_icon_dir_for_app() {
  local app="$1"
  echo "$ROOT_DIR/apps/${app}/src-tauri/icons"
}

desktop_has_icon_pattern() {
  local app="$1"
  local pattern="$2"
  local icon_dir candidate
  icon_dir="$(desktop_icon_dir_for_app "$app")"

  for candidate in "$icon_dir"/$~pattern; do
    if [[ -f "$candidate" ]]; then
      return 0
    fi
  done

  return 1
}

desktop_icon_status() {
  local platform="$1"
  local app="$2"

  case "$platform" in
    macos)
      if desktop_has_icon_pattern "$app" "*.png" && desktop_has_icon_pattern "$app" "*.icns"; then
        echo "ready (.png + .icns)"
      else
        echo "missing macOS icons"
      fi
      ;;
    linux)
      if desktop_has_icon_pattern "$app" "*.png"; then
        echo "ready (.png)"
      else
        echo "missing Linux icons"
      fi
      ;;
    windows)
      if desktop_has_icon_pattern "$app" "*.png" && desktop_has_icon_pattern "$app" "*.ico"; then
        echo "ready (.png + .ico)"
      else
        echo "missing Windows icons"
      fi
      ;;
    *)
      echo "unknown"
      ;;
  esac
}

desktop_manifest_status() {
  local platform="$1"
  local app="$2"
  local manifest="$ROOT_DIR/dist/${platform}/desktop/${app}/manifest.json"

  if [[ -f "$manifest" ]]; then
    echo "present"
  else
    echo "missing"
  fi
}

desktop_bundle_dir_for_app() {
  local app="$1"
  echo "$ROOT_DIR/apps/${app}/src-tauri/target/release/bundle"
}

json_escape() {
  local value="${1:-}"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/ }"
  echo "$value"
}

desktop_artifact_stage_dir_for_app() {
  local platform="$1"
  local app="$2"
  echo "$ROOT_DIR/dist/${platform}/desktop/${app}/artifacts"
}

desktop_artifact_manifest_path_for_app() {
  local platform="$1"
  local app="$2"
  echo "$ROOT_DIR/dist/${platform}/desktop/${app}/artifacts.json"
}

desktop_artifact_summary_path() {
  local platform="$1"
  echo "$ROOT_DIR/dist/${platform}/desktop/artifacts-summary.json"
}

desktop_build_summary_path() {
  local platform="$1"
  echo "$ROOT_DIR/dist/${platform}/desktop/build-summary.json"
}

desktop_expected_artifact_count() {
  local platform="$1"
  case "$platform" in
    macos) echo 2 ;;
    linux) echo 3 ;;
    windows) echo 2 ;;
    *) echo 0 ;;
  esac
}

desktop_artifact_count_for_app() {
  local platform="$1"
  local app="$2"
  local manifest_path artifact_count
  manifest_path="$(desktop_artifact_manifest_path_for_app "$platform" "$app")"
  artifact_count=0

  if [[ -f "$manifest_path" ]]; then
    artifact_count="$(sed -n 's/.*"artifact_count":[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$manifest_path" | head -n 1)"
    artifact_count="${artifact_count:-0}"
  fi

  echo "$artifact_count"
}

desktop_build_status_for_app() {
  local platform="$1"
  local app="$2"
  local artifact_count expected_count
  artifact_count="$(desktop_artifact_count_for_app "$platform" "$app")"
  expected_count="$(desktop_expected_artifact_count "$platform")"

  if [[ "$artifact_count" -le 0 ]]; then
    echo "failed"
  elif [[ "$artifact_count" -lt "$expected_count" ]]; then
    echo "partial"
  else
    echo "built"
  fi
}

collect_desktop_artifacts_for_app() {
  local platform="$1"
  local app="$2"
  local bundle_dir dest_dir manifest_path
  bundle_dir="$(desktop_bundle_dir_for_app "$app")"
  dest_dir="$(desktop_artifact_stage_dir_for_app "$platform" "$app")"
  manifest_path="$(desktop_artifact_manifest_path_for_app "$platform" "$app")"

  mkdir -p "$(dirname "$manifest_path")"
  rm -rf "$dest_dir"
  mkdir -p "$dest_dir"

  local artifacts_json=""
  local artifact_count=0

  add_desktop_artifact() {
    local kind="$1"
    local candidate="${2:-}"

    if [[ -z "$candidate" || ! -e "$candidate" ]]; then
      return 0
    fi

    local name staged_path entry_type
    name="$(basename "$candidate")"
    staged_path="$dest_dir/$name"
    rm -rf "$staged_path"
    cp -R "$candidate" "$staged_path"

    if [[ -d "$candidate" ]]; then
      entry_type="directory"
    else
      entry_type="file"
    fi

    if [[ -n "$artifacts_json" ]]; then
      artifacts_json+=","
      artifacts_json+=$'\n'
    fi

    artifacts_json+="    {"
    artifacts_json+=$'\n'
    artifacts_json+="      \"kind\": \"$(json_escape "$kind")\","
    artifacts_json+=$'\n'
    artifacts_json+="      \"name\": \"$(json_escape "$name")\","
    artifacts_json+=$'\n'
    artifacts_json+="      \"type\": \"$(json_escape "$entry_type")\","
    artifacts_json+=$'\n'
    artifacts_json+="      \"source_path\": \"$(json_escape "$candidate")\","
    artifacts_json+=$'\n'
    artifacts_json+="      \"staged_path\": \"$(json_escape "$staged_path")\""
    artifacts_json+=$'\n'
    artifacts_json+="    }"

    artifact_count=$((artifact_count + 1))
  }

  case "$platform" in
    macos)
      add_desktop_artifact "app" "$bundle_dir/macos/"*.app(N)
      add_desktop_artifact "dmg" "$bundle_dir/dmg/"*.dmg(N)
      ;;
    linux)
      add_desktop_artifact "appimage" "$bundle_dir/appimage/"*.AppImage(N)
      add_desktop_artifact "deb" "$bundle_dir/deb/"*.deb(N)
      add_desktop_artifact "rpm" "$bundle_dir/rpm/"*.rpm(N)
      ;;
    windows)
      add_desktop_artifact "msi" "$bundle_dir/msi/"*.msi(N)
      add_desktop_artifact "nsis" "$bundle_dir/nsis/"*.exe(N)
      ;;
  esac

  cat > "$manifest_path" <<EOF
{
  "schema_version": "kyuubiki.desktop-artifacts/v1",
  "app": "$(json_escape "$app")",
  "platform": "$(json_escape "$platform")",
  "bundle_dir": "$(json_escape "$bundle_dir")",
  "artifact_count": $artifact_count,
  "artifacts": [
${artifacts_json}
  ]
}
EOF
}

collect_host_desktop_artifacts() {
  local platform="${1:-$(host_platform)}"
  local summary_path app manifest_path summary_entries="" artifact_count=0
  summary_path="$(desktop_artifact_summary_path "$platform")"
  mkdir -p "$(dirname "$summary_path")"

  for app in ${(s: :)$(desktop_target_apps)}; do
    collect_desktop_artifacts_for_app "$platform" "$app"
    manifest_path="$(desktop_artifact_manifest_path_for_app "$platform" "$app")"

    local app_count=0
    app_count="$(desktop_artifact_count_for_app "$platform" "$app")"

    if [[ -n "$summary_entries" ]]; then
      summary_entries+=","
      summary_entries+=$'\n'
    fi
    summary_entries+="    {"
    summary_entries+=$'\n'
    summary_entries+="      \"app\": \"$(json_escape "$app")\","
    summary_entries+=$'\n'
    summary_entries+="      \"artifact_manifest\": \"$(json_escape "$manifest_path")\","
    summary_entries+=$'\n'
    summary_entries+="      \"artifact_count\": ${app_count}"
    summary_entries+=$'\n'
    summary_entries+="    }"

    artifact_count=$((artifact_count + app_count))
  done

  cat > "$summary_path" <<EOF
{
  "schema_version": "kyuubiki.desktop-artifact-summary/v1",
  "platform": "$(json_escape "$platform")",
  "artifact_count": $artifact_count,
  "apps": [
${summary_entries}
  ]
}
EOF

  echo "collected host desktop artifacts under $ROOT_DIR/dist/${platform}/desktop"
}

write_desktop_build_summary() {
  local platform="$1"
  shift

  local summary_path app build_status summary_entries=""
  summary_path="$(desktop_build_summary_path "$platform")"
  mkdir -p "$(dirname "$summary_path")"

  while [[ $# -gt 0 ]]; do
    app="$1"
    build_status="$2"
    shift 2

    if [[ -n "$summary_entries" ]]; then
      summary_entries+=","
      summary_entries+=$'\n'
    fi

    summary_entries+="    {"
    summary_entries+=$'\n'
    summary_entries+="      \"app\": \"$(json_escape "$app")\","
    summary_entries+=$'\n'
    summary_entries+="      \"status\": \"$(json_escape "$build_status")\","
    summary_entries+=$'\n'
    summary_entries+="      \"artifact_manifest\": \"$(json_escape "$(desktop_artifact_manifest_path_for_app "$platform" "$app")")\""
    summary_entries+=$'\n'
    summary_entries+="    }"
  done

  cat > "$summary_path" <<EOF
{
  "schema_version": "kyuubiki.desktop-build-summary/v1",
  "platform": "$(json_escape "$platform")",
  "apps": [
${summary_entries}
  ]
}
EOF
}

desktop_host_bundle_status() {
  local app="$1"
  local bundle_dir
  bundle_dir="$(desktop_bundle_dir_for_app "$app")"

  if [[ -d "$bundle_dir" ]]; then
    echo "present"
  else
    echo "missing"
  fi
}

desktop_host_artifact_status() {
  local platform="$1"
  local app="$2"
  local manifest_path artifact_count
  manifest_path="$(desktop_artifact_manifest_path_for_app "$platform" "$app")"

  if [[ ! -f "$manifest_path" ]]; then
    echo "missing"
    return 0
  fi

  artifact_count="$(sed -n 's/.*"artifact_count":[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$manifest_path" | head -n 1)"
  artifact_count="${artifact_count:-0}"

  if [[ "$artifact_count" -gt 0 ]]; then
    echo "present (${artifact_count})"
  else
    echo "empty"
  fi
}

desktop_runtime_stage_status() {
  local platform="$1"
  local root="$ROOT_DIR/dist/${platform}"

  if [[ -d "$root/bin" && -d "$root/config" && -d "$root/desktop" ]]; then
    echo "present"
  else
    echo "missing"
  fi
}

print_desktop_status_for_platform() {
  local platform="$1"
  local runtime_status
  runtime_status="$(desktop_runtime_stage_status "$platform")"

  echo
  echo "platform: ${platform}"
  echo "  runtime scaffold: ${runtime_status}"

  local app manifest_status icon_status host_bundle_status
  local host_artifact_status
  for app in ${(s: :)$(desktop_target_apps)}; do
    manifest_status="$(desktop_manifest_status "$platform" "$app")"
    icon_status="$(desktop_icon_status "$platform" "$app")"
    printf '  %-16s manifest=%-8s icons=%s\n' "${app}:" "$manifest_status" "$icon_status"

    if [[ "$platform" == "$(host_platform)" ]]; then
      host_bundle_status="$(desktop_host_bundle_status "$app")"
      host_artifact_status="$(desktop_host_artifact_status "$platform" "$app")"
      printf '  %-16s host-bundle=%-8s staged-artifacts=%s\n' "${app}:" "$host_bundle_status" "$host_artifact_status"
    fi
  done

  if verify_desktop_platform "$platform" >/dev/null 2>&1; then
    echo "  verification: ready"
  else
    echo "  verification: needs attention"
  fi
}

print_desktop_next_steps() {
  local platform="$1"
  local host
  host="$(host_platform)"

  echo
  echo "next steps:"

  if [[ "$platform" == "all" ]]; then
    echo "  - Stage every platform scaffold: ./scripts/kyuubiki desktop-stage all"
    echo "  - Build this host's desktop bundles: ./scripts/kyuubiki desktop-build-host"
    echo "  - Verify manifests and icon inputs: ./scripts/kyuubiki desktop-verify all"
    echo "  - Review staged bundle manifests under: dist/<host>/desktop/*/artifacts.json"
    return 0
  fi

  if [[ "$(desktop_runtime_stage_status "$platform")" == "missing" ]]; then
    echo "  - Stage runtime + desktop manifests: ./scripts/kyuubiki desktop-stage ${platform}"
  fi

  if [[ "$platform" == "$host" ]]; then
    echo "  - Build host-native Tauri bundles: ./scripts/kyuubiki desktop-build-host"
    echo "  - Run the full host release pass: ./scripts/kyuubiki desktop-release ${platform}"
    echo "  - Review staged bundle manifests under: dist/${platform}/desktop/*/artifacts.json"
  else
    echo "  - This host only stages ${platform} manifests; build native bundles on a ${platform} machine"
    echo "  - Verify staged rollout descriptors: ./scripts/kyuubiki desktop-verify ${platform}"
  fi
}

run_desktop_status() {
  local platform="${1:-$(host_platform)}"
  local host
  host="$(host_platform)"

  echo "desktop packaging status"
  echo "  host platform: ${host}"
  echo "  dist root: $ROOT_DIR/dist"

  if [[ "$platform" == "all" ]]; then
    local target
    for target in macos linux windows; do
      print_desktop_status_for_platform "$target"
    done
    print_desktop_next_steps all
    return 0
  fi

  print_desktop_status_for_platform "$platform"
  print_desktop_next_steps "$platform"
}

run_desktop_stage() {
  local platform="${1:-$(host_platform)}"

  if [[ "$platform" == "all" ]]; then
    local target
    for target in macos linux windows; do
      run_installer stage-release "$target"
    done
    return 0
  fi

  run_installer stage-release "$platform"
}

run_desktop_build_host() {
  local platform
  platform="$(host_platform)"
  local hub_status="failed"
  local installer_status="failed"
  local workbench_status="failed"
  local build_failed=0

  if run_hub_gui_build "$platform"; then
    :
  else
    :
  fi

  if run_installer_gui_build "$platform"; then
    :
  else
    :
  fi

  if run_workbench_gui_build "$platform"; then
    :
  else
    :
  fi

  collect_host_desktop_artifacts "$platform"
  hub_status="$(desktop_build_status_for_app "$platform" "hub-gui")"
  installer_status="$(desktop_build_status_for_app "$platform" "installer-gui")"
  workbench_status="$(desktop_build_status_for_app "$platform" "workbench-gui")"

  if [[ "$hub_status" != "built" || "$installer_status" != "built" || "$workbench_status" != "built" ]]; then
    build_failed=1
  fi

  write_desktop_build_summary "$platform" \
    "hub-gui" "$hub_status" \
    "installer-gui" "$installer_status" \
    "workbench-gui" "$workbench_status"

  if [[ "$build_failed" -ne 0 ]]; then
    echo "desktop host build finished with failures; see $(desktop_build_summary_path "$platform")" >&2
    return 1
  fi
}
