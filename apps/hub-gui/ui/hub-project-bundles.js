export function currentProjectBundlePayload(elements) {
  return { path: elements.projectBundlePath?.value || "" };
}

export function currentProjectBundleOutputPayload(elements) {
  return {
    path: elements.projectBundlePath?.value || "",
    out: elements.projectBundleOutPath?.value || "",
  };
}

export function currentProjectBundleComparePayload(elements) {
  return {
    leftPath: elements.projectBundlePath?.value || "",
    rightPath: elements.projectBundleComparePath?.value || "",
  };
}

export async function runProjectBundleAction({
  action,
  command,
  payload,
  invokeTauri,
  saveProjectBundleRecents,
  outputTarget,
  elements,
  setBusy,
  projectActionLabels,
}) {
  const executedAt = new Date().toISOString();
  const tauriPayload =
    command === "guarded_mutation_action"
      ? {
          action: projectActionLabels[action] ? `project_bundle_${action.split(" ").at(-1)}` : "",
          ...payload,
        }
      : payload;

  try {
    const result = await invokeTauri(command, { payload: tauriPayload });
    saveProjectBundleRecents({
      action,
      bundlePath: elements.projectBundlePath?.value,
      comparePath: elements.projectBundleComparePath?.value,
      outputPath: elements.projectBundleOutPath?.value,
      status: "ok",
      note: result,
      executedAt,
    });
    outputTarget(result);
    setBusy(false, "ready");
  } catch (error) {
    const message = String(error);
    saveProjectBundleRecents({
      action,
      bundlePath: elements.projectBundlePath?.value,
      comparePath: elements.projectBundleComparePath?.value,
      outputPath: elements.projectBundleOutPath?.value,
      status: "failed",
      note: message,
      executedAt,
    });
    outputTarget(message);
    setBusy(false, "failed");
  }
}
