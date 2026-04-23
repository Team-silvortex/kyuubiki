"use client";

type WorkbenchModelToolsCardProps = {
  title: string;
  status: string;
  hint: string;
  selectionHint: string;
  addNodeLabel: string;
  addBranchNodeLabel: string;
  deleteNodeLabel: string;
  toggleMemberLabel: string;
  deleteMemberLabel: string;
  linkModeLabel: string;
  linkModeActiveLabel: string;
  linkModeHint: string;
  undoLabel: string;
  redoLabel: string;
  downloadLabel: string;
  saveForSolverLabel: string;
  canAddBranchNode: boolean;
  canDeleteNode: boolean;
  canDeleteMember: boolean;
  canUndo: boolean;
  canRedo: boolean;
  isTruss: boolean;
  isTruss3d: boolean;
  truss3dLinkMode: boolean;
  onAddNode: () => void;
  onAddBranchNode: () => void;
  onDeleteNode: () => void;
  onToggleMember: () => void;
  onDeleteMember: () => void;
  onToggleLinkMode: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onDownload: () => void;
  onSaveForSolver: () => void;
};

export function WorkbenchModelToolsCard({
  title,
  status,
  hint,
  selectionHint,
  addNodeLabel,
  addBranchNodeLabel,
  deleteNodeLabel,
  toggleMemberLabel,
  deleteMemberLabel,
  linkModeLabel,
  linkModeActiveLabel,
  linkModeHint,
  undoLabel,
  redoLabel,
  downloadLabel,
  saveForSolverLabel,
  canAddBranchNode,
  canDeleteNode,
  canDeleteMember,
  canUndo,
  canRedo,
  isTruss,
  isTruss3d,
  truss3dLinkMode,
  onAddNode,
  onAddBranchNode,
  onDeleteNode,
  onToggleMember,
  onDeleteMember,
  onToggleLinkMode,
  onUndo,
  onRedo,
  onDownload,
  onSaveForSolver,
}: WorkbenchModelToolsCardProps) {
  return (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{status}</span>
      </div>
      <p className="card-copy">{hint}</p>
      {isTruss ? (
        <>
          <div className="button-row">
            <button className="ghost-button" onClick={onAddNode} type="button">
              {addNodeLabel}
            </button>
            <button className="ghost-button" disabled={!canAddBranchNode} onClick={onAddBranchNode} type="button">
              {addBranchNodeLabel}
            </button>
            <button className="ghost-button" disabled={!canDeleteNode} onClick={onDeleteNode} type="button">
              {deleteNodeLabel}
            </button>
          </div>
          <div className="button-row">
            <button className="ghost-button" onClick={onToggleMember} type="button">
              {toggleMemberLabel}
            </button>
            <button className="ghost-button" disabled={!canDeleteMember} onClick={onDeleteMember} type="button">
              {deleteMemberLabel}
            </button>
          </div>
        </>
      ) : null}
      {isTruss3d ? (
        <>
          <div className="button-row">
            <button className="ghost-button" onClick={onAddNode} type="button">
              {addNodeLabel}
            </button>
            <button className="ghost-button" disabled={!canAddBranchNode} onClick={onAddBranchNode} type="button">
              {addBranchNodeLabel}
            </button>
            <button className="ghost-button" disabled={!canDeleteNode} onClick={onDeleteNode} type="button">
              {deleteNodeLabel}
            </button>
          </div>
          <div className="button-row">
            <button className={`ghost-button${truss3dLinkMode ? " ghost-button--active" : ""}`} onClick={onToggleLinkMode} type="button">
              {truss3dLinkMode ? linkModeActiveLabel : linkModeLabel}
            </button>
            <button className="ghost-button" onClick={onToggleMember} type="button">
              {toggleMemberLabel}
            </button>
            <button className="ghost-button" disabled={!canDeleteMember} onClick={onDeleteMember} type="button">
              {deleteMemberLabel}
            </button>
          </div>
          <p className="card-copy">{linkModeHint}</p>
        </>
      ) : null}
      <div className="button-row">
        <button className="ghost-button" disabled={!canUndo} onClick={onUndo} type="button">
          {undoLabel}
        </button>
        <button className="ghost-button" disabled={!canRedo} onClick={onRedo} type="button">
          {redoLabel}
        </button>
      </div>
      <div className="button-row">
        <button className="ghost-button" onClick={onDownload} type="button">
          {downloadLabel}
        </button>
        <button className="ghost-button" onClick={onSaveForSolver} type="button">
          {saveForSolverLabel}
        </button>
      </div>
      <p className="card-copy">{selectionHint}</p>
    </section>
  );
}
