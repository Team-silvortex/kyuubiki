"use client";

import { HistoryWorkspaceCard } from "@/components/workbench/library/workbench-history-workspace-card";
import type {
  ModelPage,
  WorkbenchLibrarySidebarProps,
} from "@/components/workbench/library/workbench-library-sidebar-types";
import { VirtualList } from "@/components/ui/virtual-list";

type WorkbenchLibraryModelsPanelProps = Pick<
  WorkbenchLibrarySidebarProps,
  | "labels"
  | "loadedModelName"
  | "modelRows"
  | "modelVersionCount"
  | "onDeleteSavedModel"
  | "onDeleteSelectedVersion"
  | "onLoadedModelNameChange"
  | "onOpenSavedModel"
  | "onOpenSavedVersion"
  | "onRenameSelectedVersion"
  | "onSaveModel"
  | "selectedModelId"
  | "selectedProjectModelCount"
  | "selectedVersionId"
  | "versionRows"
> & {
  modelPage: ModelPage;
  setModelPage: (page: ModelPage) => void;
};

export function WorkbenchLibraryModelsPanel({
  labels,
  loadedModelName,
  modelPage,
  modelRows,
  modelVersionCount,
  onDeleteSavedModel,
  onDeleteSelectedVersion,
  onLoadedModelNameChange,
  onOpenSavedModel,
  onOpenSavedVersion,
  onRenameSelectedVersion,
  onSaveModel,
  selectedModelId,
  selectedProjectModelCount,
  selectedVersionId,
  setModelPage,
  versionRows,
}: WorkbenchLibraryModelsPanelProps) {
  const latestModelRow = modelRows[0] ?? null;

  return (
    <>
      <HistoryWorkspaceCard
        title={labels.modelWorkspaceTitle}
        hint={labels.modelWorkspaceHint}
        actionLabel={labels.openLatestModel}
        actionDisabled={!latestModelRow}
        onAction={() => latestModelRow && onOpenSavedModel(latestModelRow.id)}
        metrics={[
          { label: labels.savedCount, value: modelRows.length },
          { label: labels.versionCount, value: modelVersionCount },
        ]}
      />
      <section className="sidebar-card">
        <div className="card-head">
          <h2>{labels.savedModels}</h2>
          <span>{selectedProjectModelCount}</span>
        </div>
        <div className="panel-tabs panel-tabs--wide">
          <button
            className={`panel-tab${modelPage === "saved" ? " panel-tab--active" : ""}`}
            data-workbench-library-model-page="saved"
            onClick={() => setModelPage("saved")}
            type="button"
          >
            {labels.modelSavedPage}
          </button>
          <button
            className={`panel-tab${modelPage === "versions" ? " panel-tab--active" : ""}`}
            data-workbench-library-model-page="versions"
            onClick={() => setModelPage("versions")}
            type="button"
          >
            {labels.modelVersionsPage}
          </button>
        </div>
        {modelPage === "saved" ? (
          <>
            <div className="form-grid compact">
              <label>
                <span>{labels.modelName}</span>
                <input value={loadedModelName} onChange={(event) => onLoadedModelNameChange(event.target.value)} />
              </label>
            </div>
            <div className="button-row">
              <button className="ghost-button" onClick={() => onSaveModel(false)} type="button">
                {labels.save}
              </button>
              <button className="ghost-button" onClick={() => onSaveModel(true)} type="button">
                {labels.saveAs}
              </button>
              <button className="ghost-button" disabled={!selectedModelId} onClick={onDeleteSavedModel} type="button">
                {labels.deleteSavedModel}
              </button>
            </div>
            <VirtualList
              className="history-list"
              items={modelRows}
              itemHeight={112}
              maxHeight={344}
              emptyState={<p className="card-copy">{labels.noSavedModels}</p>}
              itemKey={(model) => model.id}
              renderItem={(model) => (
                <button
                  className={`history-item${selectedModelId === model.id ? " history-item--active" : ""}`}
                  onClick={() => onOpenSavedModel(model.id)}
                  type="button"
                >
                  <strong>{model.name}</strong>
                  <span>{model.kindLabel}</span>
                  <small>
                    {labels.updatedAt}: {model.updatedAt}
                  </small>
                  <small>{model.versionLabel}</small>
                </button>
              )}
            />
          </>
        ) : (
          <>
            <div className="card-head">
              <h2>{labels.versions}</h2>
              <span>{modelVersionCount}</span>
            </div>
            <div className="button-row">
              <button className="ghost-button" disabled={!selectedVersionId} onClick={onRenameSelectedVersion} type="button">
                {labels.renameVersion}
              </button>
              <button className="ghost-button" disabled={!selectedVersionId} onClick={onDeleteSelectedVersion} type="button">
                {labels.deleteVersion}
              </button>
            </div>
            <VirtualList
              className="history-list"
              items={versionRows}
              itemHeight={100}
              maxHeight={320}
              emptyState={<p className="card-copy">{labels.noVersions}</p>}
              itemKey={(version) => version.id}
              renderItem={(version) => (
                <button
                  className={`history-item${selectedVersionId === version.id ? " history-item--active" : ""}`}
                  onClick={() => onOpenSavedVersion(version.id)}
                  type="button"
                >
                  <strong>{version.name}</strong>
                  <span>{version.versionLabel}</span>
                  <small>
                    {labels.updatedAt}: {version.updatedAt}
                  </small>
                </button>
              )}
            />
          </>
        )}
      </section>
    </>
  );
}
