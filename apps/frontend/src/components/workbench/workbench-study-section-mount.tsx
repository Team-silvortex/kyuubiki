"use client";

import type { ReactNode } from "react";

import { WorkbenchStudySidebar } from "@/components/workbench/study/workbench-study-sidebar";
import type { StudyDomainOption, StudyKind, StudyKindOptionGroup } from "@/lib/workbench/view-models";

type StudySidebarRow = {
  label: string;
  value: string | number;
};

type WorkbenchStudySectionMountProps = {
  studyTab: "summary" | "controls";
  onStudyTabChange: (tab: "summary" | "controls") => void;
  sectionTitle: string;
  summaryTabLabel: string;
  controlsTabLabel: string;
  loadedModelName: string;
  studyTypeLabel: string;
  studyKind: StudyKind;
  studyDomainLabel: string;
  studyDomainOptions: StudyDomainOption[];
  noDomainStudiesLabel: string;
  studyKindOptionGroups: StudyKindOptionGroup[];
  onStudyKindChange: (kind: StudyKind) => void;
  summaryRows: StudySidebarRow[];
  controlsRows: StudySidebarRow[];
  controlsContent?: ReactNode;
  controlsTitle: string;
  controlsSetupPageLabel: string;
  controlsReviewPageLabel: string;
  readyLabel: string;
  busyLabel: string;
  isPending: boolean;
  runLabel: string;
  runningLabel: string;
  onRun: () => void;
};

export function WorkbenchStudySectionMount({
  studyTab,
  onStudyTabChange,
  sectionTitle,
  summaryTabLabel,
  controlsTabLabel,
  loadedModelName,
  studyTypeLabel,
  studyKind,
  studyDomainLabel,
  studyDomainOptions,
  noDomainStudiesLabel,
  studyKindOptionGroups,
  onStudyKindChange,
  summaryRows,
  controlsRows,
  controlsContent,
  controlsTitle,
  controlsSetupPageLabel,
  controlsReviewPageLabel,
  readyLabel,
  busyLabel,
  isPending,
  runLabel,
  runningLabel,
  onRun,
}: WorkbenchStudySectionMountProps) {
  return (
    <WorkbenchStudySidebar
      studyTab={studyTab}
      onStudyTabChange={onStudyTabChange}
      sectionTitle={sectionTitle}
      summaryTabLabel={summaryTabLabel}
      controlsTabLabel={controlsTabLabel}
      loadedModelName={loadedModelName}
      studyTypeLabel={studyTypeLabel}
      studyKind={studyKind}
      studyDomainLabel={studyDomainLabel}
      studyDomainOptions={studyDomainOptions}
      noDomainStudiesLabel={noDomainStudiesLabel}
      studyKindOptionGroups={studyKindOptionGroups}
      onStudyKindChange={onStudyKindChange}
      summaryRows={summaryRows}
      controlsRows={controlsRows}
      controlsContent={controlsContent}
      controlsTitle={controlsTitle}
      controlsSetupPageLabel={controlsSetupPageLabel}
      controlsReviewPageLabel={controlsReviewPageLabel}
      readyLabel={readyLabel}
      busyLabel={busyLabel}
      isPending={isPending}
      runLabel={runLabel}
      runningLabel={runningLabel}
      onRun={onRun}
    />
  );
}
