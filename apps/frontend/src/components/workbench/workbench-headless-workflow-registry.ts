"use client";

import type {
  DraftStep,
  HeadlessActionContract,
  HeadlessReferenceToken,
  HeadlessWorkflowTemplate,
  PayloadObject,
} from "@/components/workbench/workbench-headless-workflow-contract";
import headlessServiceActionContracts from "@/lib/scripting/headless-service-action-contracts.json";
import type { WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";

type AutomationContractRecord = {
  id: string;
  engine: string;
  risk: "normal" | "sensitive" | "destructive";
  summary: string;
  examples: PayloadObject[];
};

const SERVICE_SUMMARY_OVERRIDES: Record<string, Record<string, string>> = {
  service_health: { zh: "直接检查控制面的健康状态。", ja: "コントロールプレーンの健全性を直接確認します。", es: "Comprueba directamente la salud del plano de control." },
  project_create: { zh: "不经过前端界面直接创建项目。", ja: "フロントエンド UI を開かずにプロジェクトを作成します。", es: "Crea un proyecto sin abrir la UI del frontend." },
  project_update: { zh: "直接通过服务 API 更新项目记录。", ja: "サービス API 経由でプロジェクト記録を直接更新します。", es: "Actualiza un proyecto directamente mediante la API de servicio." },
  project_delete: { zh: "直接通过服务 API 删除项目。", ja: "サービス API 経由でプロジェクトを直接削除します。", es: "Elimina un proyecto directamente mediante la API de servicio." },
  model_create: { zh: "在项目下创建模型记录。", ja: "プロジェクト配下にモデルレコードを作成します。", es: "Crea un registro de modelo dentro de un proyecto." },
  model_version_create: { zh: "持久化一个已保存模型版本。", ja: "保存済みモデルバージョンを永続化します。", es: "Persiste una version guardada del modelo." },
  workflow_submit_catalog: { zh: "基于已注册工作流目录直接提交任务。", ja: "登録済みワークフローカタログから直接ジョブ投入します。", es: "Envia un workflow desde una definicion registrada en catalogo." },
  workflow_submit_graph: { zh: "直接向工作流服务 API 提交临时工作流图。", ja: "アドホックなワークフローグラフをワークフロー API へ直接投入します。", es: "Envia un grafo de workflow ad hoc directamente a la API de workflow." },
  direct_mesh_solve: { zh: "直接向无头求解代理提交 direct mesh 求解请求。", ja: "direct mesh 解析リクエストをヘッドレス solver agent へ直接投入します。", es: "Envia una solicitud de resolucion direct mesh directamente a los agentes solver." },
  solve_from_model_version: { zh: "直接从已保存模型版本发起求解。", ja: "保存済みモデルバージョンから直接解析を開始します。", es: "Lanza la resolucion directamente desde una version guardada del modelo." },
  solve_and_wait_from_model_version: { zh: "一步完成求解、等待和结果抓取。", ja: "解析、待機、結果取得を 1 ステップで実行します。", es: "Resuelve, espera y obtiene el resultado en un solo paso." },
  job_wait: { zh: "轮询已提交任务直到结束。", ja: "投入済みジョブを完了までポーリングします。", es: "Consulta un trabajo enviado hasta que termine." },
  job_fetch: { zh: "直接抓取任务状态。", ja: "ジョブ状態を直接取得します。", es: "Obtiene directamente el estado del trabajo." },
  result_fetch: { zh: "直接抓取结果载荷。", ja: "結果ペイロードを直接取得します。", es: "Obtiene directamente la carga util del resultado." },
};

const SERVICE_SCHEMA_OVERRIDES: Record<string, Pick<HeadlessActionContract, "inputSchema" | "outputSchema">> = {
  service_health: { inputSchema: [], outputSchema: [{ key: "status", label: "status" }] },
  project_create: { inputSchema: [{ key: "name", label: "name", required: true }, { key: "description", label: "description" }], outputSchema: [{ key: "project_id", label: "project_id" }] },
  project_update: { inputSchema: [{ key: "project_id", label: "project_id", required: true, bindable: true }, { key: "name", label: "name" }, { key: "description", label: "description" }], outputSchema: [{ key: "project_id", label: "project_id" }] },
  project_delete: { inputSchema: [{ key: "project_id", label: "project_id", required: true, bindable: true }], outputSchema: [{ key: "project_id", label: "project_id" }] },
  model_create: { inputSchema: [{ key: "project_id", label: "project_id", required: true, bindable: true }, { key: "name", label: "name", required: true }, { key: "kind", label: "kind", required: true }, { key: "payload", label: "payload", required: true }], outputSchema: [{ key: "model_id", label: "model_id" }] },
  model_version_create: { inputSchema: [{ key: "model_id", label: "model_id", required: true, bindable: true }, { key: "name", label: "name" }, { key: "payload", label: "payload", required: true }], outputSchema: [{ key: "model_version_id", label: "model_version_id" }] },
  workflow_submit_catalog: { inputSchema: [{ key: "workflow_id", label: "workflow_id", required: true }, { key: "input_artifacts", label: "input_artifacts" }], outputSchema: [{ key: "job_id", label: "job_id" }] },
  workflow_submit_graph: { inputSchema: [{ key: "graph", label: "graph", required: true }, { key: "input_artifacts", label: "input_artifacts" }], outputSchema: [{ key: "job_id", label: "job_id" }] },
  direct_mesh_solve: { inputSchema: [{ key: "study_kind", label: "study_kind", required: true }, { key: "input", label: "input" }, { key: "model_payload", label: "model_payload" }, { key: "model_id", label: "model_id", bindable: true }, { key: "model_version_id", label: "model_version_id", bindable: true }, { key: "materials", label: "materials" }, { key: "selection_mode", label: "selection_mode" }, { key: "project_id", label: "project_id", bindable: true }, { key: "endpoints", label: "endpoints", required: true }], outputSchema: [{ key: "job_id", label: "job_id" }] },
  solve_from_model_version: { inputSchema: [{ key: "model_version_id", label: "model_version_id", required: true, bindable: true }, { key: "endpoints", label: "endpoints", required: true }], outputSchema: [{ key: "job_id", label: "job_id" }] },
  solve_and_wait_from_model_version: { inputSchema: [{ key: "model_version_id", label: "model_version_id", required: true, bindable: true }, { key: "endpoints", label: "endpoints", required: true }, { key: "timeout_ms", label: "timeout_ms" }], outputSchema: [{ key: "job_id", label: "job_id" }] },
  job_wait: { inputSchema: [{ key: "job_id", label: "job_id", required: true, bindable: true }, { key: "interval_ms", label: "interval_ms" }, { key: "timeout_ms", label: "timeout_ms" }], outputSchema: [{ key: "job_id", label: "job_id" }] },
  job_fetch: { inputSchema: [{ key: "job_id", label: "job_id", required: true, bindable: true }], outputSchema: [{ key: "job_id", label: "job_id" }, { key: "status", label: "status" }] },
  result_fetch: { inputSchema: [{ key: "job_id", label: "job_id", required: true, bindable: true }], outputSchema: [{ key: "result", label: "result" }] },
};

export const HEADLESS_ACTIONS: HeadlessActionContract[] = (headlessServiceActionContracts as AutomationContractRecord[])
  .filter((contract) => SERVICE_SCHEMA_OVERRIDES[contract.id])
  .map((contract) => ({
    id: contract.id,
    risk: contract.risk,
    summary: {
      en: contract.summary,
      ...(SERVICE_SUMMARY_OVERRIDES[contract.id] ?? {}),
    },
    payloadExample: (contract.examples[0] as PayloadObject | undefined) ?? {},
    inputSchema: SERVICE_SCHEMA_OVERRIDES[contract.id].inputSchema,
    outputSchema: SERVICE_SCHEMA_OVERRIDES[contract.id].outputSchema,
  }));

export const HEADLESS_WORKFLOW_TEMPLATES: HeadlessWorkflowTemplate[] = [
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
  {
    id: "graph_submit_monitor",
    title: { en: "Graph Submit", zh: "图工作流提交链", ja: "グラフ投入チェーン", es: "Cadena de grafo" },
    description: {
      en: "Submit an ad hoc graph and keep follow-up polling and result fetch explicit.",
      zh: "提交临时图工作流，并显式保留轮询和结果抓取步骤。",
      ja: "アドホックグラフを投入し、その後の待機と結果取得を明示します。",
      es: "Envia un grafo ad hoc y deja explicitos el sondeo y la captura del resultado.",
    },
    steps: [
      { action: "workflow_submit_graph", payload: { graph: { nodes: [], edges: [] }, input_artifacts: {} } },
      { action: "job_wait", payload: { job_id: "{{payload.job_id}}", interval_ms: 1000, timeout_ms: 60000 } },
      { action: "result_fetch", payload: { job_id: "{{payload.job_id}}" } },
    ],
  },
  {
    id: "direct_mesh_pipeline",
    title: { en: "Direct Mesh Solve", zh: "直连求解链", ja: "ダイレクト解析チェーン", es: "Cadena direct mesh" },
    description: {
      en: "Resolve directly from raw mesh payload and keep the job follow-up explicit.",
      zh: "直接从 mesh 载荷发起求解，并显式保留任务跟踪步骤。",
      ja: "生の mesh ペイロードから直接解析し、その後のジョブ追跡も明示します。",
      es: "Parte del payload mesh y mantiene explicito el seguimiento del trabajo.",
    },
    steps: [
      { action: "direct_mesh_solve", payload: { study_kind: "truss_3d", input: { nodes: [], elements: [] }, endpoints: ["http://127.0.0.1:7001"] } },
      { action: "job_wait", payload: { job_id: "{{payload.job_id}}", interval_ms: 1000, timeout_ms: 60000 } },
      { action: "result_fetch", payload: { job_id: "{{payload.job_id}}" } },
    ],
  },
];

export function localizeWorkflowText(language: WorkbenchScriptLanguage, value: Record<string, string>) {
  return value[language] ?? value.en ?? "";
}

export function buildReferenceTokens(steps: DraftStep[], currentIndex: number, actionMap: Map<string, HeadlessActionContract>): HeadlessReferenceToken[] {
  return steps.slice(0, currentIndex).flatMap((entry, previousIndex) => {
    const contract = actionMap.get(entry.action);
    if (!contract) return [];
    const stepNumber = previousIndex + 1;
    return contract.outputSchema.map((output) => ({
      label: `${stepNumber}.${output.label}`,
      outputKey: output.key,
      template: `{{steps.${stepNumber}.result.${output.key}}}`,
    }));
  });
}

export function buildStepFromTemplate(action: string, payload: PayloadObject) {
  return {
    id: `${action}-${Math.random().toString(36).slice(2, 10)}`,
    action,
    payloadText: JSON.stringify(payload, null, 2),
  };
}
