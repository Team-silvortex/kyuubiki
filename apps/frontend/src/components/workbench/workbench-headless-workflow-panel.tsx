"use client";

import { useMemo, useState } from "react";
import type { DraftStep, PayloadObject } from "@/components/workbench/workbench-headless-workflow-contract";
import { formatPayload, parsePayloadText, updatePayloadField } from "@/components/workbench/workbench-headless-workflow-contract";
import { buildReferenceTokens, buildStepFromTemplate, HEADLESS_ACTIONS, HEADLESS_WORKFLOW_TEMPLATES, localizeWorkflowText } from "@/components/workbench/workbench-headless-workflow-registry";
import { WorkbenchHeadlessWorkflowStepEditor } from "@/components/workbench/workbench-headless-workflow-step-editor";
import type { WorkbenchRecordedMacroDraft, WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";

type WorkbenchHeadlessWorkflowPanelProps = {
  language: WorkbenchScriptLanguage;
  onInsertMacroDraft: (draft: WorkbenchRecordedMacroDraft) => void;
};


function moveItem<T>(items: T[], fromIndex: number, toIndex: number) {
  if (toIndex < 0 || toIndex >= items.length || fromIndex === toIndex) return items;
  const next = [...items];
  const [item] = next.splice(fromIndex, 1);
  next.splice(toIndex, 0, item);
  return next;
}

function downloadJson(filename: string, contents: string) {
  const blob = new Blob([contents], { type: "application/json;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function WorkbenchHeadlessWorkflowPanel({ language, onInsertMacroDraft }: WorkbenchHeadlessWorkflowPanelProps) {
  const [draftId, setDraftId] = useState("macro/headless-service-workflow");
  const [steps, setSteps] = useState<DraftStep[]>(() => HEADLESS_WORKFLOW_TEMPLATES[0].steps.map((step) => buildStepFromTemplate(step.action, step.payload)));
  const [error, setError] = useState<string | null>(null);
  const actionMap = useMemo(() => new Map(HEADLESS_ACTIONS.map((action) => [action.id, action])), []);

  const draft = useMemo<WorkbenchRecordedMacroDraft | null>(() => {
    try {
      return {
        id: draftId.trim() || "macro/headless-service-workflow",
        steps: steps.map((step) => {
          const payload = parsePayloadText(step.payloadText);
          if (!payload) throw new Error("invalid payload");
          return { action: step.action, payload };
        }),
      };
    } catch {
      return null;
    }
  }, [draftId, steps]);

  const patchStepPayload = (stepId: string, updater: (payload: PayloadObject | null) => PayloadObject | null) => {
    setSteps((current) =>
      current.map((step) => {
        if (step.id !== stepId) return step;
        const nextPayload = updater(parsePayloadText(step.payloadText));
        return nextPayload ? { ...step, payloadText: formatPayload(nextPayload) } : step;
      }),
    );
  };

  const ui = {
    title: language === "zh" ? "无头 SDK 工作流" : language === "ja" ? "ヘッドレス SDK ワークフロー" : "Headless SDK Workflow",
    subtitle:
      language === "zh"
        ? "这里专门编排脱离前端界面的 service / solver 无头链路，和 WASM Python 的前端自动化是分开的。"
        : language === "ja"
          ? "ここではフロントエンド UI を介さない service / solver 向けヘッドレスチェーンを組み立てます。WASM Python の前段自動化とは別系統です。"
          : "Compose service and solver headless chains here. This is separate from the WASM Python frontend-automation surface above.",
    draftId: language === "zh" ? "草稿 ID" : language === "ja" ? "ドラフト ID" : "Draft ID",
    loadTemplate: language === "zh" ? "载入模板" : language === "ja" ? "テンプレート読込" : "Load template",
    exportJson: language === "zh" ? "导出 SDK 工作流 JSON" : language === "ja" ? "SDK ワークフロー JSON 書き出し" : "Export SDK workflow JSON",
    insert: language === "zh" ? "插入脚本编辑器" : language === "ja" ? "スクリプトへ挿入" : "Insert into editor",
    remove: language === "zh" ? "删除步骤" : language === "ja" ? "ステップ削除" : "Remove step",
    moveUp: language === "zh" ? "上移" : language === "ja" ? "上へ" : "Move up",
    moveDown: language === "zh" ? "下移" : language === "ja" ? "下へ" : "Move down",
    payloadJson: language === "zh" ? "完整参数 JSON" : language === "ja" ? "完全なペイロード JSON" : "Full payload JSON",
    endpoints: language === "zh" ? "求解端点" : language === "ja" ? "解析エンドポイント" : "Solve endpoints",
    referenceTitle: language === "zh" ? "可用前序引用" : language === "ja" ? "利用可能な前段参照" : "Available prior-step refs",
    referenceApply: language === "zh" ? "绑定引用" : language === "ja" ? "参照を接続" : "Bind reference",
    referenceClear: language === "zh" ? "清除绑定" : language === "ja" ? "接続解除" : "Clear binding",
    referenceCurrent: language === "zh" ? "当前绑定" : language === "ja" ? "現在の接続" : "Current binding",
    noReferences: language === "zh" ? "当前字段前面还没有可匹配输出。" : language === "ja" ? "この入力に合う前段出力はまだありません。" : "No matching prior outputs for this input yet.",
    endpointsHint:
      language === "zh"
        ? "每行一个 endpoint"
        : language === "ja"
          ? "1 行につき 1 endpoint"
          : "One endpoint per line",
    normal: language === "zh" ? "普通" : language === "ja" ? "通常" : "Normal",
    sensitive: language === "zh" ? "敏感" : language === "ja" ? "注意" : "Sensitive",
    destructive: language === "zh" ? "高风险" : language === "ja" ? "高リスク" : "Destructive",
    invalidJson:
      language === "zh"
        ? "当前有无效 JSON，先修正参数。"
        : language === "ja"
          ? "現在の JSON が無効です。先に修正してください。"
          : "One or more payload JSON blocks are invalid.",
    invalidInsert:
      language === "zh"
        ? "当前有无效 JSON，无法插入到脚本编辑器。"
        : language === "ja"
          ? "現在の JSON が無効なため、スクリプトエディタへ挿入できません。"
          : "Cannot insert while the payload JSON is invalid.",
  };

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{ui.title}</h2>
        <span>{steps.length}</span>
      </div>
      <p className="card-copy">{ui.subtitle}</p>
      <label className="field-label">
        <span>{ui.draftId}</span>
        <input className="text-input" onChange={(event) => setDraftId(event.target.value)} type="text" value={draftId} />
      </label>

      <div className="script-panel__catalog">
        {HEADLESS_WORKFLOW_TEMPLATES.map((template) => (
          <article className="script-panel__action" key={template.id}>
            <div className="script-panel__action-head">
              <strong>{localizeWorkflowText(language, template.title)}</strong>
              <span>{template.steps.length}</span>
            </div>
            <p className="card-copy">{localizeWorkflowText(language, template.description)}</p>
            <div className="button-row">
              <button
                className="ghost-button ghost-button--compact"
                onClick={() => {
                  setSteps(template.steps.map((step) => buildStepFromTemplate(step.action, step.payload)));
                  setDraftId(`macro/${template.id}`);
                  setError(null);
                }}
                type="button"
              >
                {ui.loadTemplate}
              </button>
            </div>
          </article>
        ))}
      </div>

      <div className="button-row">
        {HEADLESS_ACTIONS.map((action) => (
          <button
            className="ghost-button ghost-button--compact"
            key={action.id}
            onClick={() => setSteps((current) => [...current, buildStepFromTemplate(action.id, action.payloadExample)])}
            type="button"
          >
            {action.id}
          </button>
        ))}
      </div>

      <div className="script-panel__catalog">
        {steps.map((step, index) => {
          const descriptor = actionMap.get(step.action);
          const references = buildReferenceTokens(steps, index, actionMap);
          const riskLabel =
            descriptor?.risk === "destructive" ? ui.destructive : descriptor?.risk === "sensitive" ? ui.sensitive : ui.normal;

          return (
            <article className="script-panel__action" key={step.id}>
              <div className="script-panel__action-head">
                <strong>{`${index + 1}. ${step.action}`}</strong>
                <span>{riskLabel}</span>
              </div>
              {descriptor ? <p className="card-copy">{localizeWorkflowText(language, descriptor.summary)}</p> : null}
              <WorkbenchHeadlessWorkflowStepEditor
                endpointsHint={ui.endpointsHint}
                endpointsLabel={ui.endpoints}
                contract={descriptor}
                noReferencesLabel={ui.noReferences}
                parsePayloadText={parsePayloadText}
                patchStepPayload={patchStepPayload}
                referenceApplyLabel={ui.referenceApply}
                referenceClearLabel={ui.referenceClear}
                referenceCurrentLabel={ui.referenceCurrent}
                references={references}
                referenceTitle={ui.referenceTitle}
                step={step}
              />

              <label className="field-label">
                <span>{ui.payloadJson}</span>
                <textarea
                  className="script-panel__editor"
                  onChange={(event) =>
                    setSteps((current) => current.map((entry) => (entry.id === step.id ? { ...entry, payloadText: event.target.value } : entry)))
                  }
                  rows={6}
                  spellCheck={false}
                  value={step.payloadText}
                />
              </label>

              <div className="button-row">
                <button
                  className="ghost-button ghost-button--compact"
                  onClick={() => setSteps((current) => moveItem(current, index, index - 1))}
                  type="button"
                >
                  {ui.moveUp}
                </button>
                <button
                  className="ghost-button ghost-button--compact"
                  onClick={() => setSteps((current) => moveItem(current, index, index + 1))}
                  type="button"
                >
                  {ui.moveDown}
                </button>
                <button className="ghost-button ghost-button--compact" onClick={() => setSteps((current) => current.filter((entry) => entry.id !== step.id))} type="button">
                  {ui.remove}
                </button>
              </div>
            </article>
          );
        })}
      </div>

      {error ? <p className="card-copy">{error}</p> : null}
      <div className="button-row">
        <button
          className="ghost-button"
          onClick={() => {
            if (!draft) return setError(ui.invalidJson);
            downloadJson(`${draft.id.replace(/\//g, "-")}.json`, JSON.stringify(draft, null, 2));
            setError(null);
          }}
          type="button"
        >
          {ui.exportJson}
        </button>
        <button
          className="ghost-button"
          onClick={() => {
            if (!draft) return setError(ui.invalidInsert);
            onInsertMacroDraft(draft);
            setError(null);
          }}
          type="button"
        >
          {ui.insert}
        </button>
      </div>
      <pre className="script-panel__snapshot">{draft ? JSON.stringify(draft, null, 2) : "{\n  \"error\": \"invalid payload json\"\n}"}</pre>
    </section>
  );
}
