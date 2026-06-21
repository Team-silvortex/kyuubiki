import { applyDesktopState } from "./shared/tauri-bridge.js";

function regressionGateKind(status) {
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

export function renderRegressionGateReport(ui, report) {
  if (!report) {
    applyDesktopState(ui.regressionGateStatus, "unavailable", { kind: "activity" });
    if (ui.regressionGateWarningCount) ui.regressionGateWarningCount.textContent = "0";
    if (ui.regressionGateFailingCount) ui.regressionGateFailingCount.textContent = "0";
    if (ui.regressionGateCatalogPath) ui.regressionGateCatalogPath.textContent = "tmp/regression-lane-catalog.json";
    if (ui.regressionGateSummary) ui.regressionGateSummary.textContent = "Unified gate report unavailable on this machine.";
    if (ui.regressionGateReasons) ui.regressionGateReasons.textContent = "No lane reasons loaded.";
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

  applyDesktopState(ui.regressionGateStatus, report.overall_gate_status || "unknown", {
    kind: regressionGateKind(report.overall_gate_status),
  });
  if (ui.regressionGateWarningCount) ui.regressionGateWarningCount.textContent = String(warnings);
  if (ui.regressionGateFailingCount) ui.regressionGateFailingCount.textContent = String(failures);
  if (ui.regressionGateCatalogPath) ui.regressionGateCatalogPath.textContent = report.catalog_path || "tmp/regression-lane-catalog.json";
  if (ui.regressionGateSummary) ui.regressionGateSummary.textContent = `${lanes.length} lanes tracked. ${warnings} warning lanes, ${failures} failing lanes.`;
  if (ui.regressionGateReasons) ui.regressionGateReasons.textContent = reasons.length > 0 ? reasons.slice(0, 3).join(" | ") : "All tracked lanes are within gate.";
}
