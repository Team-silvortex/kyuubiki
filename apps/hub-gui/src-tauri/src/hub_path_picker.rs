#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundlePathPickerPayload {
    kind: BundlePathPickerKind,
    initial_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum BundlePathPickerKind {
    Directory,
    Bundle,
}

#[tauri::command]
async fn project_bundle_pick_path(
    window: tauri::WebviewWindow,
    payload: BundlePathPickerPayload,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    // Only the fixed Hub window may show a chooser. No filesystem scope is granted.
    let mut dialog = window.dialog().file().set_parent(&window);
    if let Some(initial) = payload.initial_path.filter(|path| path.len() <= 4096) {
        let path = PathBuf::from(initial);
        if path.is_absolute() {
            let directory = if path.is_dir() {
                Some(path.as_path())
            } else {
                path.parent()
            };
            if let Some(directory) = directory.filter(|path| path.is_dir()) {
                dialog = dialog.set_directory(directory);
            }
        }
    }
    tauri::async_runtime::spawn_blocking(move || {
        let picked = match payload.kind {
            BundlePathPickerKind::Directory => dialog.blocking_pick_folder(),
            BundlePathPickerKind::Bundle => dialog
                .add_filter("Kyuubiki bundle", &["kyuubiki"])
                .blocking_pick_file(),
        };
        picked
            .map(|file| {
                let path = file.into_path().map_err(|error| error.to_string())?;
                path.into_os_string()
                    .into_string()
                    .map_err(|_| "selected path must be valid UTF-8".to_string())
            })
            .transpose()
    })
    .await
    .map_err(|error| format!("native path chooser failed: {error}"))?
}
