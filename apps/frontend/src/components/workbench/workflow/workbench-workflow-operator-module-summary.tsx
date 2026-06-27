import type { WorkflowOperatorModuleSummary } from "@/lib/api";

export function WorkbenchWorkflowOperatorModuleSummary(props: {
  modules: WorkflowOperatorModuleSummary[];
}) {
  if (props.modules.length === 0) return null;

  return (
    <div className="sidebar-stack">
      <div className="sidebar-list__row">
        <span>Operator library modules</span>
        <strong>{props.modules.length}</strong>
      </div>
      <div className="sidebar-list">
        {props.modules.slice(0, 8).map((module) => {
          const coverage =
            module.operator_count > 0
              ? Math.round((module.verified_count / module.operator_count) * 100)
              : 0;
          return (
            <div className="sidebar-list__row" key={module.id}>
              <span>
                {module.label}
                {module.partial_count > 0 ? " · roadmap" : ""}
              </span>
              <strong>
                {module.verified_count}/{module.operator_count} · {coverage}%
              </strong>
            </div>
          );
        })}
      </div>
    </div>
  );
}
