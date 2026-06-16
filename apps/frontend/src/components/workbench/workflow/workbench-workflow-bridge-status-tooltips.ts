"use client";

export function getBridgeStatusSummaryTooltipProps() {
  return {
    "aria-label": "Bridge runtime status summary",
    title: "Bridge runtime status summary",
  };
}

export function getBridgeRunStatusTooltipProps() {
  return {
    "aria-label": "Bridge runtime status for this run",
    title: "Bridge runtime status for this run",
  };
}

export function getBridgeOverviewNavTooltipProps(
  target: "aligned" | "drift" | "missing-runtime",
) {
  if (target === "aligned") {
    return {
      "aria-label": "Open catalog filtered to bridge aligned workflows",
      title: "Open catalog: Bridge Aligned",
    };
  }
  if (target === "drift") {
    return {
      "aria-label": "Open catalog filtered to bridge drift workflows",
      title: "Open catalog: Bridge Drift",
    };
  }
  return {
    "aria-label": "Open runs filtered to bridge missing runtime workflows",
    title: "Open runs: Bridge Missing Runtime",
  };
}
