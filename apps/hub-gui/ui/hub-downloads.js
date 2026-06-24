export function downloadHubBlob(filename, blob) {
  const url = window.URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  window.URL.revokeObjectURL(url);
}

export function downloadHubJson(filename, payload) {
  downloadHubBlob(
    filename,
    new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" }),
  );
}
