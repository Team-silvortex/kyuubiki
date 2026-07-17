"use client";

import { getWorkbenchScriptDslCopy } from "@/components/workbench/workbench-script-dsl-copy";

type WorkbenchScriptDslCardProps = {
  dslCode: string;
  dslError: string | null;
  language: string;
  onCompileDsl: () => void;
  onLoadDslTemplate: () => void;
  onRunDsl: () => void;
  onUseCurrentMacroDraft: () => void;
  setDslCode: (value: string) => void;
};

export function WorkbenchScriptDslCard({
  dslCode,
  dslError,
  language,
  onCompileDsl,
  onLoadDslTemplate,
  onRunDsl,
  onUseCurrentMacroDraft,
  setDslCode,
}: WorkbenchScriptDslCardProps) {
  const copy = getWorkbenchScriptDslCopy(language);

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{copy.title}</h2>
        <span>JSON</span>
      </div>
      <p className="card-copy">{copy.subtitle}</p>
      <p className="card-copy">{copy.hint}</p>
      {dslError ? <p className="card-copy">{dslError}</p> : null}
      <textarea
        className="script-panel__editor"
        rows={14}
        spellCheck={false}
        value={dslCode}
        onChange={(event) => setDslCode(event.target.value)}
      />
      <div className="button-row">
        <button className="ghost-button" onClick={onCompileDsl} type="button">{copy.compile}</button>
        <button className="ghost-button" onClick={onRunDsl} type="button">{copy.run}</button>
        <button className="ghost-button" onClick={onLoadDslTemplate} type="button">{copy.reset}</button>
        <button className="ghost-button" onClick={onUseCurrentMacroDraft} type="button">{copy.macro}</button>
      </div>
    </section>
  );
}
