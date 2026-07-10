"use client";

import { useWorkbenchCoreComposition } from "@/components/workbench/workbench-core-composition";
import { WorkbenchAppRail } from "@/components/workbench/workbench-app-rail";
import { WorkbenchMainShellMount } from "@/components/workbench/workbench-main-shell-mount";
import { useWorkbenchRootState } from "@/components/workbench/workbench-root-state";
import { WorkbenchShellFrame } from "@/components/workbench/workbench-shell-frame";
import { WorkbenchSidebarPanel } from "@/components/workbench/workbench-sidebar-panel";
import {
  cancelJob,
  resolvePlaneQuad2dJobInput,
  resolvePlaneTriangle2dJobInput,
  resolveTruss2dJobInput,
  resolveTruss3dJobInput,
} from "@/lib/api";
import { downloadTextFile } from "@/components/workbench/workbench-file-helpers";
import { workbenchProjectLibraryBackendService } from "@/lib/workbench/project-library-backend-service";

export function Workbench() {
  const rootState = useWorkbenchRootState();

  const { sidebarMountProps, mainShellMountProps } = useWorkbenchCoreComposition({
    rootState,
    createProject: workbenchProjectLibraryBackendService.createProject,
    createModel: workbenchProjectLibraryBackendService.createModel,
    createModelVersion: workbenchProjectLibraryBackendService.createModelVersion,
    updateModelVersion: workbenchProjectLibraryBackendService.updateModelVersion,
    fetchModel: workbenchProjectLibraryBackendService.fetchModel,
    fetchModelVersion: workbenchProjectLibraryBackendService.fetchModelVersion,
    updateProject: workbenchProjectLibraryBackendService.updateProject,
    deleteProject: workbenchProjectLibraryBackendService.deleteProject,
    updateModel: workbenchProjectLibraryBackendService.updateModel,
    deleteModel: workbenchProjectLibraryBackendService.deleteModel,
    deleteModelVersion: workbenchProjectLibraryBackendService.deleteModelVersion,
    fetchModelVersions: workbenchProjectLibraryBackendService.fetchModelVersions,
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
