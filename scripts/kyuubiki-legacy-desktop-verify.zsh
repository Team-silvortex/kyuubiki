verify_manifest_bundle_kind() {
  local manifest_path="$1"
  local expected_fragment="$2"

  if ! grep -q "$expected_fragment" "$manifest_path"; then
    echo "missing bundle kind ${expected_fragment} in ${manifest_path}" >&2
    return 1
  fi
}

verify_desktop_icons() {
  local platform="$1"
  local app_dir="$2"
  local label="$3"

  local icon_dir="$ROOT_DIR/apps/${app_dir}/src-tauri/icons"
  local required=()

  case "$platform" in
    macos) required=("*.png" "*.icns") ;;
    linux) required=("*.png") ;;
    windows) required=("*.png" "*.ico") ;;
    *)
      echo "unsupported platform for icon verification: $platform" >&2
      return 1
      ;;
  esac

  local pattern match_found
  for pattern in "${required[@]}"; do
    match_found=0
    for candidate in "$icon_dir"/$~pattern; do
      if [[ -f "$candidate" ]]; then
        match_found=1
        break
      fi
    done

    if [[ "$match_found" -ne 1 ]]; then
      echo "missing ${pattern} icon input for ${label} under ${icon_dir}" >&2
      return 1
    fi
  done

  echo "ok: ${label} icon inputs for ${platform}"
}

verify_desktop_platform() {
  local platform="$1"
  local desktop_root="$ROOT_DIR/dist/${platform}/desktop"

  if [[ ! -d "$desktop_root" ]]; then
    echo "missing staged desktop directory: $desktop_root" >&2
    return 1
  fi

  local apps=("hub-gui" "installer-gui" "workbench-gui")
  local app manifest
  for app in "${apps[@]}"; do
    manifest="$desktop_root/$app/manifest.json"
    if [[ ! -f "$manifest" ]]; then
      echo "missing desktop manifest: $manifest" >&2
      return 1
    fi

    case "$platform" in
      macos)
        verify_manifest_bundle_kind "$manifest" "app"
        verify_manifest_bundle_kind "$manifest" "dmg"
        ;;
      linux)
        verify_manifest_bundle_kind "$manifest" "appimage"
        verify_manifest_bundle_kind "$manifest" "deb"
        verify_manifest_bundle_kind "$manifest" "rpm"
        ;;
      windows)
        verify_manifest_bundle_kind "$manifest" "msi"
        verify_manifest_bundle_kind "$manifest" "nsis"
        ;;
    esac
  done

  verify_desktop_icons "$platform" "hub-gui" "hub-gui"
  verify_desktop_icons "$platform" "installer-gui" "installer-gui"
  verify_desktop_icons "$platform" "workbench-gui" "workbench-gui"

  echo "desktop release verification passed for ${platform}"
}

run_desktop_verify() {
  local platform="${1:-$(host_platform)}"

  if [[ "$platform" == "all" ]]; then
    local target
    for target in macos linux windows; do
      verify_desktop_platform "$target"
    done
    return 0
  fi

  verify_desktop_platform "$platform"
}

run_desktop_release() {
  local platform="${1:-$(host_platform)}"
  run_desktop_stage "$platform"
  run_desktop_build_host
  run_desktop_verify "$platform"
  echo "desktop release artifacts staged under $ROOT_DIR/dist/$(host_platform)/desktop"
}
