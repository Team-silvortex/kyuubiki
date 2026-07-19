type HubProjectBundleElements = {
  projectBundlePath?: HTMLInputElement | null;
  projectBundleOutPath?: HTMLInputElement | null;
  projectBundleComparePath?: HTMLInputElement | null;
};

export function currentProjectBundlePayload(elements: HubProjectBundleElements): Record<string, unknown>;
export function currentProjectBundleOutputPayload(elements: HubProjectBundleElements): Record<string, unknown>;
export function currentProjectBundleComparePayload(elements: HubProjectBundleElements): Record<string, unknown>;
