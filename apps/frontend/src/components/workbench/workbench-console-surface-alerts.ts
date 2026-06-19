"use client";

import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import { dismissWorkbenchAlert } from "@/components/workbench/workbench-alert-state";
import {
  dismissWorkbenchNotice,
  type WorkbenchNoticeItem,
  type WorkbenchNoticeStateSetter,
  workbenchNoticeToAlertItem,
} from "@/components/workbench/workbench-notice-state";
import type { TrussDiagnostics } from "@/components/workbench/workbench-defaults";
import {
  buildWorkbenchRuntimeRecoveryAlerts,
  type WorkbenchRuntimeRecoveryState,
} from "@/components/workbench/workbench-runtime-recovery";

type BuildWorkbenchConsoleSurfaceAlertsParams = {
  importNotice?: WorkbenchNoticeItem | null;
  setImportNotice?: WorkbenchNoticeStateSetter | undefined;
  runtimeRecovery?: WorkbenchRuntimeRecoveryState;
  systemAlerts?: WorkbenchAlertItem[];
  setSystemAlerts?: Dispatch<SetStateAction<WorkbenchAlertItem[]>> | undefined;
  trussDiagnostics?: TrussDiagnostics | null;
  includeIntegrityAlerts?: boolean;
};

export function buildWorkbenchConsoleSurfaceAlerts({
  importNotice,
  setImportNotice,
  runtimeRecovery,
  systemAlerts = [],
  setSystemAlerts,
  trussDiagnostics,
  includeIntegrityAlerts = false,
}: BuildWorkbenchConsoleSurfaceAlertsParams): WorkbenchAlertItem[] {
  return [
    ...(importNotice
      ? [
          workbenchNoticeToAlertItem(
            importNotice,
            setImportNotice ? () => dismissWorkbenchNotice(setImportNotice) : undefined,
          ),
        ]
      : []),
    ...buildWorkbenchRuntimeRecoveryAlerts(
      runtimeRecovery ?? {
        availability: "healthy",
        issues: [],
        lastFailureAt: null,
      },
    ),
    ...systemAlerts.map((alert) => ({
      ...alert,
      onDismiss: setSystemAlerts
        ? () => dismissWorkbenchAlert(setSystemAlerts, alert.id)
        : undefined,
    })),
    ...(includeIntegrityAlerts
      ? (trussDiagnostics?.blockingMessages ?? []).slice(0, 3).map((message, index) => ({
          id: `integrity-blocking-${index}`,
          message,
          tone: "warning" as const,
        }))
      : []),
  ];
}
