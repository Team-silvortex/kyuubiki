"use client";

import { WorkbenchLibrarySidebar } from "@/components/workbench/library/workbench-library-sidebar";
import type {
  JobRow,
  LibraryLabels,
  LibraryPanelTab,
  ModelRow,
  SampleRow,
  VersionRow,
} from "@/components/workbench/library/workbench-library-sidebar-types";
import type { ModelRecord, ModelVersionRecord, ProjectRecord, WorkflowCatalogEntry } from "@/lib/api";

type WorkbenchLibrarySectionMountProps = {
  labels: LibraryLabels;
  libraryTab: LibraryPanelTab;
  onLibraryTabChange: (tab: LibraryPanelTab) => void;
  sampleRows: SampleRow[];
  workflowCatalogEntries: WorkflowCatalogEntry[];
  workflowCatalogBusy: boolean;
  projects: ProjectRecord[];
  selectedProjectId: string | null;
  setSelectedProjectId: (projectId: string | null) => void;
  setSelectedModelId: (modelId: string | null) => void;
  projectNameDraft: string;
  setProjectNameDraft: (value: string) => void;
  projectDescriptionDraft: string;
  setProjectDescriptionDraft: (value: string) => void;
  createProjectRecord: () => void;
  updateProjectRecord: () => void;
  deleteProjectRecord: () => void;
  downloadProjectBundleJson: () => Promise<void>;
  downloadProjectBundleZip: () => Promise<void>;
  importProjectBundle: (file: File) => Promise<void>;
  selectedProjectModels: ModelRecord[];
  modelRows: ModelRow[];
  selectedModelId: string | null;
  loadedModelName: string;
  setLoadedModelName: (value: string) => void;
  saveModelVersion: (saveAs: boolean) => void;
  deleteSavedModelRecord: () => void;
  openSavedModel: (model: ModelRecord) => void;
  versionRows: VersionRow[];
  modelVersions: ModelVersionRecord[];
  selectedVersionId: string | null;
  renameSelectedVersion: () => void;
  deleteSelectedVersion: () => void;
  openSavedVersion: (version: ModelVersionRecord) => void;
  jobRows: JobRow[];
  jobCount: number;
  activeJobId: string | null;
  openHistoryJob: (jobId: string) => void;
  openSample: (href: string) => void;
  refreshWorkflowCatalog: () => Promise<void>;
  runWorkflowCatalogEntry: (workflowId: string) => void;
  refreshJobHistory: () => Promise<void>;
  refreshProjects: () => Promise<void>;
  importModel: (file: File) => Promise<void>;
};

export function WorkbenchLibrarySectionMount({
  labels,
  libraryTab,
  onLibraryTabChange,
  sampleRows,
  workflowCatalogEntries,
  workflowCatalogBusy,
  projects,
  selectedProjectId,
  setSelectedProjectId,
  setSelectedModelId,
  projectNameDraft,
  setProjectNameDraft,
  projectDescriptionDraft,
  setProjectDescriptionDraft,
  createProjectRecord,
  updateProjectRecord,
  deleteProjectRecord,
  downloadProjectBundleJson,
  downloadProjectBundleZip,
  importProjectBundle,
  selectedProjectModels,
  modelRows,
  selectedModelId,
  loadedModelName,
  setLoadedModelName,
  saveModelVersion,
  deleteSavedModelRecord,
  openSavedModel,
  versionRows,
  modelVersions,
  selectedVersionId,
  renameSelectedVersion,
  deleteSelectedVersion,
  openSavedVersion,
  jobRows,
  jobCount,
  activeJobId,
  openHistoryJob,
  openSample,
  refreshWorkflowCatalog,
  runWorkflowCatalogEntry,
  refreshJobHistory,
  refreshProjects,
  importModel,
}: WorkbenchLibrarySectionMountProps) {
  return (
    <WorkbenchLibrarySidebar
      libraryTab={libraryTab}
      onLibraryTabChange={onLibraryTabChange}
      labels={labels}
      sampleRows={sampleRows}
      workflowCatalogEntries={workflowCatalogEntries}
      workflowCatalogBusy={workflowCatalogBusy}
      projects={projects}
      selectedProjectId={selectedProjectId}
      onSelectedProjectChange={(projectId) => {
        setSelectedProjectId(projectId);
        setSelectedModelId(null);
      }}
      projectNameDraft={projectNameDraft}
      onProjectNameDraftChange={setProjectNameDraft}
      projectDescriptionDraft={projectDescriptionDraft}
      onProjectDescriptionDraftChange={setProjectDescriptionDraft}
      onCreateProject={createProjectRecord}
      onUpdateProject={updateProjectRecord}
      onDeleteProject={deleteProjectRecord}
      onExportProjectJson={() => void downloadProjectBundleJson()}
      onExportProjectZip={() => void downloadProjectBundleZip()}
      onImportProjectBundle={(file) => {
        if (!file) return;
        void importProjectBundle(file);
      }}
      selectedProjectModelCount={selectedProjectModels.length}
      modelRows={modelRows}
      selectedModelId={selectedModelId}
      loadedModelName={loadedModelName}
      onLoadedModelNameChange={setLoadedModelName}
      onSaveModel={saveModelVersion}
      onDeleteSavedModel={deleteSavedModelRecord}
      onOpenSavedModel={(modelId) => {
        const model = selectedProjectModels.find((entry) => entry.model_id === modelId);
        if (model) openSavedModel(model);
      }}
      versionRows={versionRows}
      modelVersionCount={modelVersions.length}
      selectedVersionId={selectedVersionId}
      onRenameSelectedVersion={renameSelectedVersion}
      onDeleteSelectedVersion={deleteSelectedVersion}
      onOpenSavedVersion={(versionId) => {
        const version = modelVersions.find((entry) => entry.version_id === versionId);
        if (version) openSavedVersion(version);
      }}
      jobRows={jobRows}
      jobCount={jobCount}
      activeJobId={activeJobId}
      onOpenHistoryJob={openHistoryJob}
      onOpenSample={openSample}
      onRefreshWorkflowCatalog={() => void refreshWorkflowCatalog()}
      onRunWorkflowCatalog={runWorkflowCatalogEntry}
      onRefresh={() => {
        void refreshJobHistory();
        void refreshProjects();
      }}
      onImportModel={(file) => {
        if (!file) return;
        void importModel(file);
      }}
    />
  );
}
