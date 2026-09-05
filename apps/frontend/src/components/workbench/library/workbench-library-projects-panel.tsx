"use client";

import type {
  ProjectPage,
  WorkbenchLibrarySidebarProps,
} from "@/components/workbench/library/workbench-library-sidebar-types";

type WorkbenchLibraryProjectsPanelProps = Pick<
  WorkbenchLibrarySidebarProps,
  | "labels"
  | "onCreateProject"
  | "onDeleteProject"
  | "onExportProjectJson"
  | "onExportProjectZip"
  | "onImportProjectBundle"
  | "onProjectDescriptionDraftChange"
  | "onProjectNameDraftChange"
  | "onSelectedProjectChange"
  | "onUpdateProject"
  | "projectDescriptionDraft"
  | "projectNameDraft"
  | "projects"
  | "selectedProjectId"
> & {
  projectPage: ProjectPage;
  setProjectPage: (page: ProjectPage) => void;
};

export function WorkbenchLibraryProjectsPanel({
  labels,
  onCreateProject,
  onDeleteProject,
  onExportProjectJson,
  onExportProjectZip,
  onImportProjectBundle,
  onProjectDescriptionDraftChange,
  onProjectNameDraftChange,
  onSelectedProjectChange,
  onUpdateProject,
  projectDescriptionDraft,
  projectNameDraft,
  projectPage,
  projects,
  selectedProjectId,
  setProjectPage,
}: WorkbenchLibraryProjectsPanelProps) {
  return (
    <section className="sidebar-card" data-workbench-library-projects="panel">
      <div className="card-head">
        <h2>{labels.projectLibrary}</h2>
        <span>{projects.length}</span>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button
          className={`panel-tab${projectPage === "manage" ? " panel-tab--active" : ""}`}
          data-workbench-library-project-page="manage"
          onClick={() => setProjectPage("manage")}
          type="button"
        >
          {labels.projectManagePage}
        </button>
        <button
          className={`panel-tab${projectPage === "exchange" ? " panel-tab--active" : ""}`}
          data-workbench-library-project-page="exchange"
          onClick={() => setProjectPage("exchange")}
          type="button"
        >
          {labels.projectExchangePage}
        </button>
      </div>
      {projectPage === "manage" ? (
        <>
          <div className="form-grid compact">
            <label>
              <span>{labels.projectNameField}</span>
              <input
                data-workbench-library-project-field="name"
                value={projectNameDraft}
                onChange={(event) => onProjectNameDraftChange(event.target.value)}
              />
            </label>
            <label>
              <span>{labels.projectDescriptionField}</span>
              <input
                data-workbench-library-project-field="description"
                value={projectDescriptionDraft}
                onChange={(event) => onProjectDescriptionDraftChange(event.target.value)}
              />
            </label>
            <label>
              <span>{labels.projectLibrary}</span>
              <select
                data-workbench-library-project-field="selection"
                value={selectedProjectId ?? ""}
                onChange={(event) => onSelectedProjectChange(event.target.value || null)}
              >
                <option value="">{labels.none}</option>
                {projects.map((project) => (
                  <option key={project.project_id} value={project.project_id}>
                    {project.name}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="button-row">
            <button className="ghost-button" data-workbench-library-project-action="create" onClick={onCreateProject} type="button">
              {labels.createProject}
            </button>
            <button className="ghost-button" data-workbench-library-project-action="update" disabled={!selectedProjectId} onClick={onUpdateProject} type="button">
              {labels.updateProject}
            </button>
            <button className="ghost-button" data-workbench-library-project-action="delete" disabled={!selectedProjectId} onClick={onDeleteProject} type="button">
              {labels.deleteProject}
            </button>
          </div>
          {projects.length === 0 ? <p className="card-copy">{labels.projectEmpty}</p> : null}
        </>
      ) : (
        <>
          <div className="button-row">
            <button className="ghost-button" data-workbench-library-project-action="export-json" disabled={!selectedProjectId} onClick={onExportProjectJson} type="button">
              {labels.exportProjectJson}
            </button>
            <button className="ghost-button" data-workbench-library-project-action="export-zip" disabled={!selectedProjectId} onClick={onExportProjectZip} type="button">
              {labels.exportProjectZip}
            </button>
          </div>
          <label className="import-box">
            <span>{labels.importProject}</span>
            <small>{labels.importProjectHint}</small>
            <input
              data-workbench-library-project-action="import"
              type="file"
              accept=".kyuubiki,.kyuubiki.json,application/json,application/zip"
              onChange={(event) => void onImportProjectBundle(event.target.files?.[0])}
            />
          </label>
        </>
      )}
    </section>
  );
}
