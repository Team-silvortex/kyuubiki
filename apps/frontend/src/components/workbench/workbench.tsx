"use client";

import { useWorkbenchCoreComposition } from "@/components/workbench/workbench-core-composition";
import { WorkbenchAppRail } from "@/components/workbench/workbench-app-rail";
import { WorkbenchMainShellMount } from "@/components/workbench/workbench-main-shell-mount";
import { useWorkbenchRootState } from "@/components/workbench/workbench-root-state";
import { WorkbenchShellFrame } from "@/components/workbench/workbench-shell-frame";
import { WorkbenchSidebarPanel } from "@/components/workbench/workbench-sidebar-panel";
import {
  cancelJob,
  createModel,
  createModelVersion,
  createProject,
  deleteJobRecord,
  deleteModel,
  deleteModelVersion,
  deleteProject,
  deleteResultRecord,
  fetchJobStatus,
  fetchModel,
  fetchModelVersion,
  fetchModelVersions,
  fetchResults,
  resolvePlaneQuad2dJobInput,
  resolvePlaneTriangle2dJobInput,
  resolveTruss2dJobInput,
  resolveTruss3dJobInput,
  updateJobRecord,
  updateModel,
  updateModelVersion,
  updateProject,
  updateResultRecord,
} from "@/lib/api";
import { downloadTextFile } from "@/components/workbench/workbench-file-helpers";

export function Workbench() {
  const rootState = useWorkbenchRootState();

  const { sidebarMountProps, mainShellMountProps } = useWorkbenchCoreComposition({
    rootState,
    createProject,
    createModel,
    createModelVersion,
    updateModelVersion,
    fetchModel,
    fetchModelVersion,
    updateProject,
    deleteProject,
    updateModel,
    deleteModel,
    deleteModelVersion,
    fetchModelVersions,
    fetchJobStatus,
    fetchResults,
    updateJobRecord,
    deleteJobRecord,
    updateResultRecord,
    deleteResultRecord,
    cancelJob,
    resolveTruss2dJobInput,
    resolveTruss3dJobInput,
    resolvePlaneQuad2dJobInput,
    resolvePlaneTriangle2dJobInput,
    downloadTextFile,
  });

  return (
    <WorkbenchShellFrame
      sidebarSection={sidebarMountProps.sidebarSection}
      rail={
        <WorkbenchAppRail
          shortTitle={sidebarMountProps.shortTitle}
          railItems={sidebarMountProps.railItems}
          sidebarSection={sidebarMountProps.sidebarSection}
          onSidebarSectionChange={sidebarMountProps.onSidebarSectionChange}
          assistantLabel={sidebarMountProps.assistantLabel}
          assistantOpen={sidebarMountProps.assistantOpen}
          onAssistantToggle={sidebarMountProps.onAssistantToggle}
        />
      }
      sidebar={
        <WorkbenchSidebarPanel
          shortTitle={sidebarMountProps.shortTitle}
          roleLabel={sidebarMountProps.roleLabel}
          title={sidebarMountProps.title}
          subtitle={sidebarMountProps.subtitle}
          sidebarSection={sidebarMountProps.sidebarSection}
          studySection={sidebarMountProps.studySection}
          modelSection={sidebarMountProps.modelSection}
          workflowSection={sidebarMountProps.workflowSection}
          librarySection={sidebarMountProps.librarySection}
          systemSection={sidebarMountProps.systemSection}
        />
      }
      workspace={<WorkbenchMainShellMount {...mainShellMountProps} />}
    />
  );
}
