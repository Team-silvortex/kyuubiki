"use client";

import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import { WorkbenchSystemInstallPolicyCard } from "@/components/workbench/system/workbench-system-install-policy-card";
import {
  buildSystemWorkflowPolicyAction,
  buildSystemWorkflowPolicyCommandAction,
} from "@/components/workbench/system/workbench-system-workflow-bridge";
import type { WorkflowSurfaceTab } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchSystemInstallPolicyMountProps = {
  t: WorkbenchCopy;
  setSidebarSection: (section: "study" | "model" | "workflow" | "library" | "system") => void;
  handleWorkflowPanelTabChange: (tab: WorkflowSurfaceTab) => void;
};

export function WorkbenchSystemInstallPolicyMount({
  t,
  setSidebarSection,
  handleWorkflowPanelTabChange,
}: WorkbenchSystemInstallPolicyMountProps) {
  return (
    <WorkbenchSystemInstallPolicyCard
      title={t.settingsInstallPolicyTitle}
      hint={t.settingsInstallPolicyHint}
      integrityLabel={t.workflowValidationTitle}
      integrityValue={t.settingsInstallPolicyIntegrityValue}
      integrityAction={buildSystemWorkflowPolicyAction(t, setSidebarSection, handleWorkflowPanelTabChange, "validation", "watch")}
      integritySecondaryAction={buildSystemWorkflowPolicyCommandAction(t, setSidebarSection, handleWorkflowPanelTabChange, "validation", "preview-validation-fixes", t.workflowValidationPreviewLabel)}
      updateLabel={t.workflowPackageInstallRulesPortabilityLabel}
      updateValue={t.settingsInstallPolicyUpdateValue}
      updateAction={buildSystemWorkflowPolicyAction(t, setSidebarSection, handleWorkflowPanelTabChange, "package-policy", "good")}
      updateSecondaryAction={buildSystemWorkflowPolicyCommandAction(t, setSidebarSection, handleWorkflowPanelTabChange, "package-policy", "scan-package-residuals", t.workflowPackageInstallRulesScanLabel)}
      cleanupLabel={t.workflowPackageInstallRulesCleanupLabel}
      cleanupValue={t.workflowLocalWorkflowDeletedLabel}
      cleanupAction={buildSystemWorkflowPolicyAction(t, setSidebarSection, handleWorkflowPanelTabChange, "snapshots", "watch")}
      cleanupSecondaryAction={buildSystemWorkflowPolicyCommandAction(t, setSidebarSection, handleWorkflowPanelTabChange, "snapshots", "preview-package-repairs", t.workflowPackageInstallRulesRepairLabel)}
      formatLabel={t.workflowPackageInstallRulesFormatLabel}
      formatValue="kyuubiki.workflow-package v1 + workflow dataset contract JSON"
      formatAction={buildSystemWorkflowPolicyAction(t, setSidebarSection, handleWorkflowPanelTabChange, "package-policy", "good")}
      formatSecondaryAction={buildSystemWorkflowPolicyCommandAction(t, setSidebarSection, handleWorkflowPanelTabChange, "package-policy", "scan-package-residuals", t.workflowPackageInstallRulesScanLabel)}
    />
  );
}
