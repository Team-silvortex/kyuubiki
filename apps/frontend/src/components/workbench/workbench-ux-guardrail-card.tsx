import type { WorkbenchUxGuardrailSummary, WorkbenchUxGuardrailTone } from "@/components/workbench/workbench-ux-guardrails";

type WorkbenchUxGuardrailCardProps = {
  summary: WorkbenchUxGuardrailSummary;
  language: string;
  onOpenRuntime: () => void;
  onOpenWorkflow: () => void;
};

export function WorkbenchUxGuardrailCard({
  summary,
  language,
  onOpenRuntime,
  onOpenWorkflow,
}: WorkbenchUxGuardrailCardProps) {
  const copy = guardrailCopy(language);
  return (
    <section className="sidebar-card sidebar-card--compact" data-workbench-ux-guardrails="card">
      <div className="card-head">
        <div>
          <span>{copy.title}</span>
          <small>{copy.subtitle}</small>
        </div>
        <span className={`status-pill status-pill--${toneClass(summary.tone)}`}>{copy.tone[summary.tone]}</span>
      </div>
      <p className="card-copy">{summary.nextAction}</p>
      <div className="sidebar-list">
        <div className="sidebar-list__row"><span>{copy.blocked}</span><strong>{summary.blockedActionCount}</strong></div>
        <div className="sidebar-list__row"><span>{copy.warnings}</span><strong>{summary.warningCount}</strong></div>
      </div>
      <div className="history-list">
        {summary.items.slice(0, 4).map((item) => (
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

function guardrailCopy(language: string) {
  if (language === "zh") {
    return {
      title: "操作防呆",
      subtitle: "运行前检查和下一步",
      blocked: "阻断项",
      warnings: "提醒项",
      runtime: "打开运行时",
      workflow: "打开工作流",
      tone: { ok: "可继续", warn: "需注意", block: "先处理" },
    };
  }
  if (language === "ja") {
    return {
      title: "操作ガード",
      subtitle: "実行前チェックと次の一手",
      blocked: "ブロック",
      warnings: "注意",
      runtime: "ランタイム",
      workflow: "ワークフロー",
      tone: { ok: "続行可", warn: "注意", block: "先に対応" },
    };
  }
  return {
    title: "UX guardrails",
    subtitle: "Pre-run checks and next step",
    blocked: "Blocked",
    warnings: "Warnings",
    runtime: "Open runtime",
    workflow: "Open workflow",
    tone: { ok: "ready", warn: "review", block: "blocked" },
  };
}
