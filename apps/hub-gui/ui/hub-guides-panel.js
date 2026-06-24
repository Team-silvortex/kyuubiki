import { renderHubLocalizationPanel } from "./hub-localization-panel.js";

function formatRegressionNumber(value, digits = 3) {
  return Number(value).toLocaleString(undefined, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
}

function formatRegressionInteger(value) {
  return Math.round(Number(value)).toLocaleString();
}

function regressionStatusText(copy, status) {
  const guides = copy.guides;
  switch (status) {
    case "within_baseline":
      return guides.regressionStatusWithinBaseline;
    case "regressed":
      return guides.regressionStatusRegressed;
    case "baseline_only":
    default:
      return guides.regressionStatusBaselineOnly;
  }
}

function regressionStatusNote(copy, status) {
  const guides = copy.guides;
  switch (status) {
    case "within_baseline":
      return guides.regressionNoteWithinBaseline;
    case "regressed":
      return guides.regressionNoteRegressed;
    case "baseline_only":
    default:
      return guides.regressionNoteBaselineOnly;
  }
}

function regressionStateKind(status) {
  switch (status) {
    case "within_baseline":
      return "health";
    case "regressed":
      return "danger";
    case "baseline_only":
    default:
      return "activity";
  }
}

function unifiedGateStateKind(status) {
  switch (status) {
    case "pass":
      return "health";
    case "fail":
      return "danger";
    case "warn":
    default:
      return "activity";
  }
}

export function renderGuidesPanelCopy(params) {
  const { elements, copy, activeLanguage, setText } = params;
  setText(elements.guidesPrimaryLabel, copy.guides.primaryLabel);
  setText(elements.guidesPrimaryTitle, copy.guides.primaryTitle);
  setText(elements.guidesPrimaryCopy, copy.guides.primaryCopy);
  setText(elements.guidesDocsTitle, copy.guides.docsTitle);
  setText(elements.guidesDocsCopy, copy.guides.docsCopy);
  setText(elements.guidesCurrentTitle, copy.guides.currentTitle);
  setText(elements.guidesCurrentCopy, copy.guides.currentCopy);
  setText(elements.guidesOverviewDocsLabel, copy.guides.overviewDocsLabel);
  setText(elements.guidesOverviewDocsTitle, copy.guides.overviewDocsTitle);
  setText(elements.guidesOverviewDocsCopy, copy.guides.overviewDocsCopy);
  setText(elements.guidesOverviewCurrentLabel, copy.guides.overviewCurrentLabel);
  setText(elements.guidesOverviewCurrentTitle, copy.guides.overviewCurrentTitle);
  setText(elements.guidesOverviewCurrentCopy, copy.guides.overviewCurrentCopy);
  setText(elements.guidesOverviewTroubleshootingLabel, copy.guides.overviewTroubleshootingLabel);
  setText(elements.guidesOverviewTroubleshootingTitle, copy.guides.overviewTroubleshootingTitle);
  setText(elements.guidesOverviewTroubleshootingCopy, copy.guides.overviewTroubleshootingCopy);
  setText(elements.guidesOperationsTitle, copy.guides.operationsTitle);
  setText(elements.guidesOperationsCopy, copy.guides.operationsCopy);
  setText(elements.guidesTroubleshootingTitle, copy.guides.troubleshootingTitle);
  setText(elements.guidesTroubleshootingCopy, copy.guides.troubleshootingCopy);
  setText(elements.guidesAccuracyLabel, copy.guides.accuracyLabel);
  setText(elements.guidesAccuracyTitle, copy.guides.accuracyTitle);
  setText(elements.guidesAccuracyCopy, copy.guides.accuracyCopy);
  setText(elements.guidesAccuracyPlanTitle, copy.guides.accuracyPlanTitle);
  setText(elements.guidesAccuracyPlanCopy, copy.guides.accuracyPlanCopy);
  setText(elements.guidesAccuracyBaselinesTitle, copy.guides.accuracyBaselinesTitle);
  setText(elements.guidesAccuracyBaselinesCopy, copy.guides.accuracyBaselinesCopy);
  setText(elements.guidesDirectMeshTitle, copy.guides.directMeshTitle);
  setText(elements.guidesDirectMeshCopy, copy.guides.directMeshCopy);
  setText(elements.guidesRegressionLabel, copy.guides.regressionLabel);
  setText(elements.guidesRegressionTitle, copy.guides.regressionTitle);
  setText(elements.guidesRegressionCopy, copy.guides.regressionCopy);
  setText(elements.guidesRegressionElapsedLabel, copy.guides.regressionElapsedLabel);
  setText(elements.guidesRegressionRssLabel, copy.guides.regressionRssLabel);
  setText(elements.guidesRegressionRepeatLabel, copy.guides.regressionRepeatLabel);
  setText(elements.guidesRegressionNetworkLabel, copy.guides.regressionNetworkLabel);
  setText(elements.guidesRegressionLatestLabel, copy.guides.regressionLatestLabel);
  setText(elements.guidesRegressionStatusLabel, copy.guides.regressionStatusLabel);
  setText(elements.guidesRegressionBaselinePathLabel, copy.guides.regressionBaselinePathLabel);
  setText(elements.guidesRegressionOutputPathLabel, copy.guides.regressionOutputPathLabel);
  setText(elements.guidesRegressionBaselineTitle, copy.guides.regressionBaselineTitle);
  setText(elements.guidesRegressionBaselineCopy, copy.guides.regressionBaselineCopy);
  setText(elements.guidesRegressionOutputTitle, copy.guides.regressionOutputTitle);
  setText(elements.guidesRegressionOutputCopy, copy.guides.regressionOutputCopy);
  setText(elements.guidesRegressionLaneTitle, copy.guides.regressionLaneTitle);
  setText(elements.guidesRegressionLaneCopy, copy.guides.regressionLaneCopy);
  renderHubLocalizationPanel({
    elements,
    copy,
    activeLanguage,
    setText,
  });
}

export function renderDirectMeshRegressionSnapshot(params) {
  const { elements, snapshot, copy, regressionGateReport, applyDesktopState } = params;
  if (!snapshot) {
    return;
  }

  if (elements.guidesRegressionElapsedValue) {
    elements.guidesRegressionElapsedValue.textContent = `${formatRegressionNumber(snapshot.baseline_mean_elapsed_s)} s`;
  }
  if (elements.guidesRegressionRssValue) {
    elements.guidesRegressionRssValue.textContent = `${formatRegressionInteger(snapshot.baseline_mean_rss_kib)} KiB`;
  }
  if (elements.guidesRegressionRepeatValue) {
    elements.guidesRegressionRepeatValue.textContent = `${snapshot.repeat} runs`;
  }
  if (elements.guidesRegressionNetworkValue) {
    elements.guidesRegressionNetworkValue.textContent = snapshot.docker_run_network;
  }
  if (elements.guidesRegressionBaselinePath) {
    elements.guidesRegressionBaselinePath.textContent = snapshot.baseline_path;
  }
  if (elements.guidesRegressionOutputPath) {
    elements.guidesRegressionOutputPath.textContent = snapshot.output_root;
  }
  if (elements.guidesRegressionLatestValue) {
    const latestText = snapshot.latest_exists && snapshot.latest_mean_elapsed_s != null
      ? `${formatRegressionNumber(snapshot.latest_mean_elapsed_s)} s`
      : "--";
    elements.guidesRegressionLatestValue.textContent = latestText;
  }
  if (elements.guidesRegressionStatusValue) {
    applyDesktopState(
      elements.guidesRegressionStatusValue,
      regressionStatusText(copy, snapshot.status),
      { kind: regressionStateKind(snapshot.status) },
    );
  }
  if (elements.guidesRegressionNote) {
    const generatedAt = snapshot.latest_exists && snapshot.latest_generated_at
      ? ` Latest summary: ${snapshot.latest_generated_at}.`
      : "";
    const elapsedDelta = snapshot.latest_exists && snapshot.elapsed_delta_pct != null
      ? ` Elapsed delta: ${formatRegressionNumber(snapshot.elapsed_delta_pct, 2)}%.`
      : "";
    const rssDelta = snapshot.latest_exists && snapshot.rss_delta_pct != null
      ? ` RSS delta: ${formatRegressionNumber(snapshot.rss_delta_pct, 2)}%.`
      : "";
    const regressionGate =
      regressionGateReport && regressionGateReport.overall_gate_status
        ? ` Unified gate: ${regressionGateReport.overall_gate_status}.`
        : "";
    elements.guidesRegressionNote.textContent =
      `${regressionStatusNote(copy, snapshot.status)}${generatedAt}${elapsedDelta}${rssDelta}${regressionGate}`;
  }
}

export function renderRegressionGateReport(params) {
  const { elements, report, applyDesktopState } = params;
  if (!report) {
    if (elements.guidesGateStatusValue) {
      applyDesktopState(elements.guidesGateStatusValue, "unavailable", { kind: "activity" });
    }
    if (elements.guidesGateWarningCount) elements.guidesGateWarningCount.textContent = "0";
    if (elements.guidesGateFailingCount) elements.guidesGateFailingCount.textContent = "0";
    if (elements.guidesGateLaneCount) elements.guidesGateLaneCount.textContent = "0";
    if (elements.guidesGateCatalogPath) elements.guidesGateCatalogPath.textContent = "tmp/regression-lane-catalog.json";
    if (elements.guidesGateNote) elements.guidesGateNote.textContent = "Unified regression gate report unavailable on this machine.";
    if (elements.guidesGateReasons) elements.guidesGateReasons.textContent = "No gate reasons loaded yet.";
    return;
  }

  const lanes = Array.isArray(report.lanes) ? report.lanes : [];
  const warnings =
    Number.isFinite(report.warning_lane_count) ? report.warning_lane_count : lanes.filter((lane) => lane.gate_status === "warn").length;
  const failures =
    Number.isFinite(report.failing_lane_count) ? report.failing_lane_count : lanes.filter((lane) => lane.gate_status === "fail").length;
  const reasons = lanes.flatMap((lane) =>
    Array.isArray(lane.gate_reasons)
      ? lane.gate_reasons.map((reason) => `${lane.title || lane.id || "lane"}: ${reason}`)
      : Array.isArray(lane.reasons)
      ? lane.reasons.map((reason) => `${lane.title || lane.id || "lane"}: ${reason}`)
      : [],
  );

  if (elements.guidesGateStatusValue) {
    applyDesktopState(elements.guidesGateStatusValue, report.overall_gate_status || "unknown", {
      kind: unifiedGateStateKind(report.overall_gate_status),
    });
  }
  if (elements.guidesGateWarningCount) elements.guidesGateWarningCount.textContent = String(warnings);
  if (elements.guidesGateFailingCount) elements.guidesGateFailingCount.textContent = String(failures);
  if (elements.guidesGateLaneCount) elements.guidesGateLaneCount.textContent = String(lanes.length);
  if (elements.guidesGateCatalogPath) {
    elements.guidesGateCatalogPath.textContent = report.catalog_path || "tmp/regression-lane-catalog.json";
  }
  if (elements.guidesGateNote) {
    elements.guidesGateNote.textContent =
      `${lanes.length} lanes tracked. ${warnings} warning lanes, ${failures} failing lanes.`;
  }
  if (elements.guidesGateReasons) {
    elements.guidesGateReasons.textContent = reasons.length > 0 ? reasons.slice(0, 4).join(" | ") : "All tracked lanes are within gate.";
  }
}

export function renderDirectMeshRegressionLoadError(params) {
  const { elements, copy, error, applyDesktopState, formatHubOperatorError } = params;
  if (elements.guidesRegressionStatusValue) {
    applyDesktopState(
      elements.guidesRegressionStatusValue,
      regressionStatusText(copy, "baseline_only"),
      { kind: regressionStateKind("baseline_only") },
    );
  }
  if (elements.guidesRegressionNote) {
    elements.guidesRegressionNote.textContent = formatHubOperatorError(error, {
      actionLabel: "Direct-mesh regression snapshot",
    });
  }
}

export async function loadRegressionGateReportPanel(params) {
  const {
    applyDesktopState,
    elements,
    hubCopy,
    invokeTauri,
    renderDirectMeshRegressionSnapshot,
    renderRegressionGateReport,
    state,
  } = params;
  try {
    state.regressionGateReport = await invokeTauri("hub_regression_gate_report");
    renderRegressionGateReport({ elements, report: state.regressionGateReport, applyDesktopState });
    renderDirectMeshRegressionSnapshot({
      elements,
      snapshot: state.directMeshRegressionSnapshot,
      copy: hubCopy(),
      regressionGateReport: state.regressionGateReport,
      applyDesktopState,
    });
  } catch {
    state.regressionGateReport = null;
    renderRegressionGateReport({ elements, report: null, applyDesktopState });
  }
}
