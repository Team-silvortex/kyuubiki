"use client";

import { lazy, memo, Suspense, useState } from "react";

import type {
  LibraryPanelTab,
  ModelPage,
  ProjectPage,
  SamplePage,
  WorkbenchLibrarySidebarProps,
} from "@/components/workbench/library/workbench-library-sidebar-types";
import type { StudyDomainKey } from "@/lib/workbench/view-models";

const WorkbenchLibraryHistoryPanel = lazy(() =>
  import("@/components/workbench/library/workbench-library-history-panel").then((module) => ({
    default: module.WorkbenchLibraryHistoryPanel,
  })),
);
const WorkbenchLibraryModelsPanel = lazy(() =>
  import("@/components/workbench/library/workbench-library-models-panel").then((module) => ({
    default: module.WorkbenchLibraryModelsPanel,
  })),
);
const WorkbenchLibraryProjectsPanel = lazy(() =>
  import("@/components/workbench/library/workbench-library-projects-panel").then((module) => ({
    default: module.WorkbenchLibraryProjectsPanel,
  })),
);
const WorkbenchLibrarySamplesPanel = lazy(() =>
  import("@/components/workbench/library/workbench-library-samples-panel").then((module) => ({
    default: module.WorkbenchLibrarySamplesPanel,
  })),
);

const TAB_GLYPHS: Record<LibraryPanelTab, string> = {
  jobs: "J",
  results: "R",
  models: "M",
  projects: "P",
  samples: "S",
};

function LoadingPanel({ label, tab }: { label: string; tab: LibraryPanelTab }) {
  return (
    <section
      aria-busy="true"
      aria-live="polite"
      className="sidebar-card"
      data-workbench-library-loading={tab}
    >
      <div className="card-head">
        <h2>{label}</h2>
        <span>...</span>
      </div>
    </section>
  );
}

export const WorkbenchLibrarySidebar = memo(function WorkbenchLibrarySidebar(
  props: WorkbenchLibrarySidebarProps,
) {
  const { labels, libraryTab, onLibraryTabChange } = props;
  const [selectedSampleDomain, setSelectedSampleDomain] = useState<StudyDomainKey>("mechanical");
  const [samplePage, setSamplePage] = useState<SamplePage>("catalog");
  const [projectPage, setProjectPage] = useState<ProjectPage>("manage");
  const [modelPage, setModelPage] = useState<ModelPage>("saved");
  const tabs = Object.keys(TAB_GLYPHS) as LibraryPanelTab[];

  return (
    <div className="sidebar-stack panel-scroll-window" data-workbench-library="panel">
      <div className="panel-tabs panel-tabs--wide panel-tabs--library">
        {tabs.map((tab) => (
          <button
            key={tab}
            aria-label={`workbench-library-tab:${tab}`}
            className={`panel-tab panel-tab--icon${libraryTab === tab ? " panel-tab--active" : ""}`}
            data-workbench-library-tab={tab}
            onClick={() => onLibraryTabChange(tab)}
            type="button"
          >
            <span className="panel-tab__glyph">{TAB_GLYPHS[tab]}</span>
            <span>{labels.tabs[tab]}</span>
          </button>
        ))}
      </div>

      <Suspense fallback={<LoadingPanel label={labels.tabs[libraryTab]} tab={libraryTab} />}>
        {libraryTab === "jobs" || libraryTab === "results" ? (
          <WorkbenchLibraryHistoryPanel
            activeJobId={props.activeJobId}
            jobCount={props.jobCount}
            jobRows={props.jobRows}
            labels={labels}
            mode={libraryTab}
            onOpenHistoryJob={props.onOpenHistoryJob}
          />
        ) : null}
        {libraryTab === "samples" ? (
          <WorkbenchLibrarySamplesPanel
            labels={labels}
            onImportModel={props.onImportModel}
            onOpenSample={props.onOpenSample}
            onRefreshWorkflowCatalog={props.onRefreshWorkflowCatalog}
            onRunWorkflowCatalog={props.onRunWorkflowCatalog}
            samplePage={samplePage}
            sampleRows={props.sampleRows}
            selectedSampleDomain={selectedSampleDomain}
            setSamplePage={setSamplePage}
            setSelectedSampleDomain={setSelectedSampleDomain}
            workflowCatalogBusy={props.workflowCatalogBusy}
            workflowCatalogEntries={props.workflowCatalogEntries}
          />
        ) : null}
        {libraryTab === "projects" ? (
          <WorkbenchLibraryProjectsPanel
            labels={labels}
            onCreateProject={props.onCreateProject}
            onDeleteProject={props.onDeleteProject}
            onExportProjectJson={props.onExportProjectJson}
            onExportProjectZip={props.onExportProjectZip}
            onImportProjectBundle={props.onImportProjectBundle}
            onProjectDescriptionDraftChange={props.onProjectDescriptionDraftChange}
            onProjectNameDraftChange={props.onProjectNameDraftChange}
            onSelectedProjectChange={props.onSelectedProjectChange}
            onUpdateProject={props.onUpdateProject}
            projectDescriptionDraft={props.projectDescriptionDraft}
            projectNameDraft={props.projectNameDraft}
            projectPage={projectPage}
            projects={props.projects}
            selectedProjectId={props.selectedProjectId}
            setProjectPage={setProjectPage}
          />
        ) : null}
        {libraryTab === "models" ? (
          <WorkbenchLibraryModelsPanel
            labels={labels}
            loadedModelName={props.loadedModelName}
            modelPage={modelPage}
            modelRows={props.modelRows}
            modelVersionCount={props.modelVersionCount}
            onDeleteSavedModel={props.onDeleteSavedModel}
            onDeleteSelectedVersion={props.onDeleteSelectedVersion}
            onLoadedModelNameChange={props.onLoadedModelNameChange}
            onOpenSavedModel={props.onOpenSavedModel}
            onOpenSavedVersion={props.onOpenSavedVersion}
            onRenameSelectedVersion={props.onRenameSelectedVersion}
            onSaveModel={props.onSaveModel}
            selectedModelId={props.selectedModelId}
            selectedProjectModelCount={props.selectedProjectModelCount}
            selectedVersionId={props.selectedVersionId}
            setModelPage={setModelPage}
            versionRows={props.versionRows}
          />
        ) : null}
      </Suspense>
    </div>
  );
});
