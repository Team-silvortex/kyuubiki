import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import { getWorkbenchGuardrailCopy } from "@/components/workbench/workbench-extended-language-copy";
import {
  localizeWorkbenchUxGuardrailSummary,
  type WorkbenchUxGuardrailSummary,
  type WorkbenchUxGuardrailTone,
} from "@/components/workbench/workbench-ux-guardrails";

type WorkbenchUxGuardrailCardProps = {
  summary: WorkbenchUxGuardrailSummary;
  workbenchCopy: WorkbenchCopy;
  language: string;
  onOpenRuntime: () => void;
  onOpenWorkflow: () => void;
};

export function WorkbenchUxGuardrailCard({
  summary,
  workbenchCopy,
  language,
  onOpenRuntime,
  onOpenWorkflow,
}: WorkbenchUxGuardrailCardProps) {
  const copy = getWorkbenchGuardrailCopy(language);
  const localizedSummary = localizeWorkbenchUxGuardrailSummary(summary, workbenchCopy);
  return (
    <section className="sidebar-card sidebar-card--compact" data-workbench-ux-guardrails="card">
      <div className="card-head">
        <div>
          <span>{copy.title}</span>
          <small>{copy.subtitle}</small>
        </div>
        <span className={`status-pill status-pill--${toneClass(localizedSummary.tone)}`}>{copy.tone[localizedSummary.tone]}</span>
      </div>
      <p className="card-copy">{localizedSummary.nextAction}</p>
      <div className="sidebar-list">
        <div className="sidebar-list__row"><span>{copy.blocked}</span><strong>{localizedSummary.blockedActionCount}</strong></div>
        <div className="sidebar-list__row"><span>{copy.warnings}</span><strong>{localizedSummary.warningCount}</strong></div>
      </div>
      <div className="history-list">
        {localizedSummary.items.slice(0, 4).map((item) => (
          <article className="history-item" key={item.id}>
            <div>
              <strong>{item.title}</strong>
              <span>{item.detail}</span>
              <small>{item.nextAction}</small>
            </div>
            <span className={`status-pill status-pill--${toneClass(item.tone)}`}>{copy.tone[item.tone]}</span>
          </article>
        ))}
      </div>
      <div className="button-row">
        <button onClick={onOpenRuntime} type="button">{copy.runtime}</button>
        <button onClick={onOpenWorkflow} type="button">{copy.workflow}</button>
      </div>
    </section>
  );
}

function toneClass(tone: WorkbenchUxGuardrailTone) {
  if (tone === "block") return "risk";
  if (tone === "warn") return "watch";
  return "good";
}
