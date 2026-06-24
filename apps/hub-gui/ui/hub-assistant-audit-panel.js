import {
  mirrorHubAssistantAuditToSecurityEvents as mirrorHubAssistantAuditToSecurityEventsModule,
  rememberHubAssistantAuditEntry,
  renderHubAssistantAuditEntries,
  updateHubAssistantAuditDeliveryEntry,
} from "./hub-assistant-audit.js";

export function createHubAssistantAuditPanel(context) {
  function assistantStatusStateClass(status) {
    switch (status) {
      case "failed":
      case "cancelled":
        return "desktop-shell-state desktop-shell-state--danger";
      case "prompted":
      case "confirmed":
        return "desktop-shell-state desktop-shell-state--warning";
      case "completed":
        return "desktop-shell-state desktop-shell-state--healthy";
      default:
        return "desktop-shell-state desktop-shell-state--idle";
    }
  }

  function assistantDeliveryStateClass(delivery) {
    switch (delivery) {
      case "synced":
        return "desktop-shell-state desktop-shell-state--healthy";
      case "sync_failed":
        return "desktop-shell-state desktop-shell-state--danger";
      default:
        return "desktop-shell-state desktop-shell-state--idle";
    }
  }

  function renderHubAssistantAudit(entries = context.loadHubAssistantAudit()) {
    renderHubAssistantAuditEntries({
      appendTextElement: context.appendTextElement,
      assistantAuditList: context.elements.assistantAuditList,
      assistantDeliveryStateClass,
      assistantRiskStateClass: context.assistantRiskStateClass,
      assistantStatusStateClass,
      entries,
      renderEmptyHistoryState: context.renderEmptyHistoryState,
    });
  }

  function rememberHubAssistantAudit(entry) {
    return rememberHubAssistantAuditEntry(entry, {
      auditLimit: context.auditLimit,
      loadHubAssistantAudit: context.loadHubAssistantAudit,
      mirrorHubAssistantAuditToSecurityEvents,
      persistHubAssistantAudit: context.persistHubAssistantAudit,
      renderHubAssistantAudit,
    });
  }

  function currentAssistantAuditContext() {
    return {
      section: context.state.activeSection,
      runtime: String(context.elements.currentRuntimeMode?.textContent || "").trim(),
      profile: String(context.elements.currentProfile?.textContent || "").trim(),
      bundle_path: String(context.elements.projectBundlePath?.value || "").trim(),
      compare_path: String(context.elements.projectBundleComparePath?.value || "").trim(),
      output_path: String(context.elements.projectBundleOutPath?.value || "").trim(),
    };
  }

  function updateHubAssistantAuditDelivery(auditId, delivery, noteSuffix = "") {
    updateHubAssistantAuditDeliveryEntry(auditId, delivery, noteSuffix, {
      loadHubAssistantAudit: context.loadHubAssistantAudit,
      persistHubAssistantAudit: context.persistHubAssistantAudit,
      renderHubAssistantAudit,
    });
  }

  async function mirrorHubAssistantAuditToSecurityEvents(entry) {
    await mirrorHubAssistantAuditToSecurityEventsModule(entry, {
      currentAssistantAuditContext,
      currentOrchestratorBaseUrl: context.currentOrchestratorBaseUrl,
      updateHubAssistantAuditDelivery,
    });
  }

  return {
    renderHubAssistantAudit,
    rememberHubAssistantAudit,
    updateHubAssistantAuditDelivery,
  };
}
