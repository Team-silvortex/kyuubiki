export function createHubWorkloadAdapter({
  buildWorkloadIdentity,
  libraryLimit,
  mergeStoredHubWorkloadLibrary,
  normalizeStoredHubWorkloadEntry,
}) {
  function workloadIdentity(entry) {
    return buildWorkloadIdentity(entry);
  }

  function normalizeHubWorkloadEntry(entry) {
    return normalizeStoredHubWorkloadEntry(entry);
  }

  function mergeHubWorkloadLibrary(existingEntries, incomingEntries) {
    return mergeStoredHubWorkloadLibrary(
      existingEntries,
      incomingEntries,
      libraryLimit,
    );
  }

  return {
    mergeHubWorkloadLibrary,
    normalizeHubWorkloadEntry,
    workloadIdentity,
  };
}
