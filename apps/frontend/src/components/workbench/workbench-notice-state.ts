"use client";

import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchAlertItem, WorkbenchAlertTone } from "@/components/workbench/workbench-alert-strip";

export type WorkbenchNoticeItem = {
  id: string;
  message: string;
  tone?: WorkbenchAlertTone;
};

export type WorkbenchNoticeStateSetter = Dispatch<SetStateAction<WorkbenchNoticeItem | null>>;

export function dismissWorkbenchNotice(setNotice: WorkbenchNoticeStateSetter) {
  setNotice(null);
}

export function showWorkbenchNotice(
  setNotice: WorkbenchNoticeStateSetter,
  notice: WorkbenchNoticeItem,
) {
  setNotice(notice);
}

export function workbenchNoticeToAlertItem(
  notice: WorkbenchNoticeItem,
  onDismiss?: () => void,
): WorkbenchAlertItem {
  return {
    id: notice.id,
    message: notice.message,
    tone: notice.tone ?? "warning",
    onDismiss,
  };
}
