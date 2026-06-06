"use client";

import { useWorkbenchCoreComposition } from "@/components/workbench/workbench-core-composition";
import { WorkbenchMainShellMount } from "@/components/workbench/workbench-main-shell-mount";
import { useWorkbenchRootState } from "@/components/workbench/workbench-root-state";
import { WorkbenchSidebarMount } from "@/components/workbench/workbench-sidebar-mount";
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
    <div className="workbench-shell">
      <WorkbenchSidebarMount {...sidebarMountProps} />
      <WorkbenchMainShellMount {...mainShellMountProps} />
    </div>
  );
}
