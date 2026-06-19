"use client";

import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";

export type WorkbenchAlertStateSetter = Dispatch<SetStateAction<WorkbenchAlertItem[]>>;

export function dismissWorkbenchAlert(setAlerts: WorkbenchAlertStateSetter, alertId: string) {
  setAlerts((current) => current.filter((alert) => alert.id !== alertId));
}

export function upsertWorkbenchAlert(
  setAlerts: WorkbenchAlertStateSetter,
  alert: WorkbenchAlertItem,
) {
  setAlerts((current) => [...current.filter((entry) => entry.id !== alert.id), alert]);
}
