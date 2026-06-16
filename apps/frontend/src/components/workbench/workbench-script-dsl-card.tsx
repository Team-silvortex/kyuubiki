"use client";

type WorkbenchScriptDslCardProps = {
  dslCode: string;
  dslError: string | null;
  language: "en" | "zh" | "ja" | "es";
  onCompileDsl: () => void;
  onLoadDslTemplate: () => void;
  onRunDsl: () => void;
  onUseCurrentMacroDraft: () => void;
  setDslCode: (value: string) => void;
};

function buildCopy(language: WorkbenchScriptDslCardProps["language"]) {
  if (language === "zh") {
    return {
      title: "前端 DSL",
      subtitle: "用结构化步骤描述 wasm Python 前端自动化，再编译到 Pyodide 执行层。",
      hint: "DSL 当前采用稳定 JSON 文档格式，适合作为录制、宏、snippet 和 UI 合约之间的统一桥。",
      compile: "编译到脚本",
      run: "直接运行 DSL",
      reset: "载入模板",
      macro: "用当前宏草稿填充",
    };
  }
  if (language === "ja") {
    return {
      title: "Frontend DSL",
      subtitle: "構造化ステップで wasm Python のフロントエンド自動化を記述し、Pyodide 実行層へコンパイルします。",
      hint: "DSL は安定した JSON 文書形式を使い、録画・マクロ・スニペット・UI 契約の共通ブリッジになります。",
      compile: "スクリプトへコンパイル",
      run: "DSL を実行",
      reset: "テンプレート読込",
      macro: "現在のマクロ草稿を使う",
    };
  }
  return {
    title: "Frontend DSL",
    subtitle: "Describe wasm Python frontend automation as structured steps, then compile into the Pyodide execution layer.",
    hint: "This DSL uses a stable JSON document format so recording, macros, snippets, and UI contracts can share one bridge.",
    compile: "Compile to script",
    run: "Run DSL",
    reset: "Load template",
    macro: "Use current macro draft",
  };
}

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
  const copy = buildCopy(language);

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
