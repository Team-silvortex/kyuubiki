"use client";

import { useMemo, useState } from "react";
import type { DraftStep, HeadlessReferenceToken, PayloadObject } from "@/components/workbench/workbench-headless-workflow-contract";
import { formatPayload, parsePayloadText, updatePayloadField } from "@/components/workbench/workbench-headless-workflow-contract";
import { WorkbenchHeadlessWorkflowStepEditor } from "@/components/workbench/workbench-headless-workflow-step-editor";
import type { WorkbenchRecordedMacroDraft, WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";

export type HeadlessActionDescriptor = {
  id: string;
  risk: "normal" | "sensitive" | "destructive";
  summary: Record<WorkbenchScriptLanguage, string>;
  payloadExample: PayloadObject;
};

type HeadlessWorkflowTemplate = {
  id: string;
  title: Record<WorkbenchScriptLanguage, string>;
  description: Record<WorkbenchScriptLanguage, string>;
  steps: Array<{ action: string; payload: PayloadObject }>;
};

type WorkbenchHeadlessWorkflowPanelProps = {
  language: WorkbenchScriptLanguage;
  onInsertMacroDraft: (draft: WorkbenchRecordedMacroDraft) => void;
};

const HEADLESS_ACTIONS: HeadlessActionDescriptor[] = [
  {
    id: "service_health",
    risk: "normal",
    summary: {
      en: "Check control-plane health directly.",
      zh: "直接检查控制面的健康状态。",
      ja: "コントロールプレーンの健全性を直接確認します。",
      es: "Comprueba directamente la salud del plano de control.",
    },
    payloadExample: {},
  },
  {
    id: "project_create",
    risk: "normal",
    summary: {
      en: "Create a project without opening the frontend UI.",
      zh: "不经过前端界面直接创建项目。",
      ja: "フロントエンド UI を開かずにプロジェクトを作成します。",
      es: "Crea un proyecto sin abrir la UI del frontend.",
    },
    payloadExample: { name: "Headless Project", description: "created by service workflow" },
  },
  {
    id: "model_create",
    risk: "normal",
    summary: {
      en: "Create a model record under a project.",
      zh: "在项目下创建模型记录。",
      ja: "プロジェクト配下にモデルレコードを作成します。",
      es: "Crea un registro de modelo dentro de un proyecto.",
    },
    payloadExample: { project_id: "proj_123", name: "truss-demo", kind: "truss_3d", payload: { nodes: [], elements: [] } },
  },
  {
    id: "model_version_create",
    risk: "normal",
    summary: {
      en: "Persist a saved model version.",
      zh: "持久化一个已保存模型版本。",
      ja: "保存済みモデルバージョンを永続化します。",
      es: "Persiste una version guardada del modelo.",
    },
    payloadExample: { model_id: "model_123", name: "v2", payload: { nodes: [], elements: [] } },
  },
  {
    id: "workflow_submit_catalog",
    risk: "normal",
    summary: {
      en: "Submit a workflow from a registered catalog definition.",
      zh: "基于已注册工作流目录直接提交任务。",
      ja: "登録済みワークフローカタログから直接ジョブ投入します。",
      es: "Envia un workflow desde una definicion registrada en catalogo.",
    },
    payloadExample: { workflow_id: "wf_demo", input_artifacts: {} },
  },
  {
    id: "solve_from_model_version",
    risk: "normal",
    summary: {
      en: "Solve directly from a saved model version.",
      zh: "直接从已保存模型版本发起求解。",
      ja: "保存済みモデルバージョンから直接解析を開始します。",
      es: "Lanza la resolucion directamente desde una version guardada del modelo.",
    },
    payloadExample: { model_version_id: "ver_123", endpoints: ["http://127.0.0.1:7001"] },
  },
  {
    id: "solve_and_wait_from_model_version",
    risk: "normal",
    summary: {
      en: "Solve, wait, and fetch result in one step.",
      zh: "一步完成求解、等待和结果抓取。",
      ja: "解析、待機、結果取得を 1 ステップで実行します。",
      es: "Resuelve, espera y obtiene el resultado en un solo paso.",
    },
    payloadExample: { model_version_id: "ver_123", endpoints: ["http://127.0.0.1:7001"], timeout_ms: 60000 },
  },
  {
    id: "job_wait",
    risk: "normal",
    summary: {
      en: "Poll a submitted job until it finishes.",
      zh: "轮询已提交任务直到结束。",
      ja: "投入済みジョブを完了までポーリングします。",
      es: "Consulta un trabajo enviado hasta que termine.",
    },
    payloadExample: { job_id: "{{payload.job_id}}", interval_ms: 1000, timeout_ms: 60000 },
  },
  {
    id: "job_fetch",
    risk: "normal",
    summary: {
      en: "Fetch job status directly.",
      zh: "直接抓取任务状态。",
      ja: "ジョブ状態を直接取得します。",
      es: "Obtiene directamente el estado del trabajo.",
    },
    payloadExample: { job_id: "job_123" },
  },
  {
    id: "result_fetch",
    risk: "normal",
    summary: {
      en: "Fetch result payload directly.",
      zh: "直接抓取结果载荷。",
      ja: "結果ペイロードを直接取得します。",
      es: "Obtiene directamente la carga util del resultado.",
    },
    payloadExample: { job_id: "job_123" },
  },
];

const HEADLESS_WORKFLOW_TEMPLATES: HeadlessWorkflowTemplate[] = [
  {
    id: "solve_wait_result",
    title: { en: "Solve From Version", zh: "版本直解链", ja: "バージョン直解析", es: "Resolver desde version" },
    description: {
      en: "Start from a saved model version and go straight to final result.",
      zh: "从已保存模型版本直接跑到最终结果。",
      ja: "保存済みモデルバージョンから最終結果まで一直線に進めます。",
      es: "Parte de una version guardada del modelo y llega al resultado final.",
    },
    steps: [{ action: "solve_and_wait_from_model_version", payload: { model_version_id: "ver_123", endpoints: ["http://127.0.0.1:7001"], timeout_ms: 60000 } }],
  },
  {
    id: "project_model_solve",
    title: { en: "Project To Solve", zh: "项目到求解", ja: "プロジェクトから解析へ", es: "Proyecto a resolucion" },
    description: {
      en: "Create project/model records, then solve from the saved version.",
      zh: "先创建项目/模型记录，再从保存版本发起求解。",
      ja: "プロジェクト/モデル記録を作成し、その保存バージョンから解析します。",
      es: "Crea proyecto/modelo y despues resuelve desde la version guardada.",
    },
    steps: [
      { action: "project_create", payload: { name: "Headless Project", description: "service workflow draft" } },
      { action: "model_create", payload: { project_id: "proj_123", name: "truss-demo", kind: "truss_3d", payload: { nodes: [], elements: [] } } },
      { action: "model_version_create", payload: { model_id: "model_123", name: "baseline", payload: { nodes: [], elements: [] } } },
      { action: "solve_and_wait_from_model_version", payload: { model_version_id: "ver_123", endpoints: ["http://127.0.0.1:7001"], timeout_ms: 60000 } },
    ],
  },
  {
    id: "workflow_submit_monitor",
    title: { en: "Workflow Submit", zh: "工作流提交链", ja: "ワークフロー投入チェーン", es: "Cadena de workflow" },
    description: {
      en: "Submit a workflow job and keep the follow-up fetch steps explicit.",
      zh: "提交工作流任务，并把后续轮询与结果抓取步骤显式化。",
      ja: "ワークフロージョブを投入し、その後の追跡取得も明示します。",
      es: "Envia un workflow y deja explicitos los pasos de seguimiento y resultado.",
    },
    steps: [
      { action: "workflow_submit_catalog", payload: { workflow_id: "wf_demo", input_artifacts: {} } },
      { action: "job_wait", payload: { job_id: "{{payload.job_id}}", interval_ms: 1000, timeout_ms: 60000 } },
      { action: "result_fetch", payload: { job_id: "{{payload.job_id}}" } },
    ],
  },
];

function localize(language: WorkbenchScriptLanguage, value: Record<WorkbenchScriptLanguage, string>) {
  return value[language] ?? value.en;
}

function buildStep(action: string, payload: PayloadObject): DraftStep {
  return {
    id: `${action}-${Math.random().toString(36).slice(2, 10)}`,
    action,
    payloadText: formatPayload(payload),
  };
}

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
  const [steps, setSteps] = useState<DraftStep[]>(() => HEADLESS_WORKFLOW_TEMPLATES[0].steps.map((step) => buildStep(step.action, step.payload)));
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
    title: language === "zh" ? "无头工作流面板" : language === "ja" ? "ヘッドレスワークフロー面" : "Headless Workflow Panel",
    subtitle:
      language === "zh"
        ? "这里专门编排绕过前端界面的 service headless 动作，优先面向项目、模型、求解和结果。"
        : language === "ja"
          ? "ここではフロントエンド UI を介さない service headless アクションを組み立てます。"
          : "Compose service headless workflows here so automation can bypass the frontend UI and talk to project, solve, and result services directly.",
    draftId: language === "zh" ? "草稿 ID" : language === "ja" ? "ドラフト ID" : "Draft ID",
    loadTemplate: language === "zh" ? "载入模板" : language === "ja" ? "テンプレート読込" : "Load template",
    exportJson: language === "zh" ? "导出 Headless JSON" : language === "ja" ? "Headless JSON 書き出し" : "Export headless JSON",
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
              <strong>{localize(language, template.title)}</strong>
              <span>{template.steps.length}</span>
            </div>
            <p className="card-copy">{localize(language, template.description)}</p>
            <div className="button-row">
              <button
                className="ghost-button ghost-button--compact"
                onClick={() => {
                  setSteps(template.steps.map((step) => buildStep(step.action, step.payload)));
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
            onClick={() => setSteps((current) => [...current, buildStep(action.id, action.payloadExample)])}
            type="button"
          >
            {action.id}
          </button>
        ))}
      </div>

      <div className="script-panel__catalog">
        {steps.map((step, index) => {
          const descriptor = actionMap.get(step.action);
          const references: HeadlessReferenceToken[] = steps.slice(0, index).flatMap((entry, previousIndex) => {
            const stepNumber = previousIndex + 1;
            if (entry.action === "project_create") {
              return [{ label: `${stepNumber}.project_id`, outputKey: "project_id", template: `{{steps.${stepNumber}.result.project_id}}` }];
            }
            if (entry.action === "model_create") {
              return [{ label: `${stepNumber}.model_id`, outputKey: "model_id", template: `{{steps.${stepNumber}.result.model_id}}` }];
            }
            if (entry.action === "model_version_create") {
              return [{ label: `${stepNumber}.model_version_id`, outputKey: "model_version_id", template: `{{steps.${stepNumber}.result.model_version_id}}` }];
            }
            if (entry.action === "workflow_submit_catalog" || entry.action === "solve_from_model_version" || entry.action === "solve_and_wait_from_model_version") {
              return [{ label: `${stepNumber}.job_id`, outputKey: "job_id", template: `{{steps.${stepNumber}.result.job_id}}` }];
            }
            return [];
          });
          const riskLabel =
            descriptor?.risk === "destructive" ? ui.destructive : descriptor?.risk === "sensitive" ? ui.sensitive : ui.normal;

          return (
            <article className="script-panel__action" key={step.id}>
              <div className="script-panel__action-head">
                <strong>{`${index + 1}. ${step.action}`}</strong>
                <span>{riskLabel}</span>
              </div>
              {descriptor ? <p className="card-copy">{localize(language, descriptor.summary)}</p> : null}
              <WorkbenchHeadlessWorkflowStepEditor
                endpointsHint={ui.endpointsHint}
                endpointsLabel={ui.endpoints}
                language={language}
                noReferencesLabel={ui.noReferences}
                parsePayloadText={parsePayloadText}
                payloadJsonLabel={ui.payloadJson}
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
