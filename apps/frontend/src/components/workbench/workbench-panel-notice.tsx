"use client";

import { WorkbenchAlertStrip } from "@/components/workbench/workbench-alert-strip";
import {
  dismissWorkbenchNotice,
  type WorkbenchNoticeItem,
  type WorkbenchNoticeStateSetter,
  workbenchNoticeToAlertItem,
} from "@/components/workbench/workbench-notice-state";

type WorkbenchPanelNoticeProps = {
  notice: WorkbenchNoticeItem | null;
  setNotice: WorkbenchNoticeStateSetter;
  wrapperProps?: Record<string, string>;
};

export function WorkbenchPanelNotice({
  notice,
  setNotice,
  wrapperProps,
}: WorkbenchPanelNoticeProps) {
  if (!notice) return null;

  return (
    <div {...wrapperProps}>
      <WorkbenchAlertStrip
        alerts={[
          workbenchNoticeToAlertItem(
            notice,
            () => dismissWorkbenchNotice(setNotice),
          ),
        ]}
      />
    </div>
  );
}
