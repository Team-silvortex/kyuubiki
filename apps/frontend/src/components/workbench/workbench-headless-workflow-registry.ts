"use client";

import type {
  DraftStep,
  HeadlessActionContract,
  HeadlessActionGuidanceNote,
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

function t(en: string, zh: string, ja: string, es: string) {
  return { en, zh, ja, es };
}

const SERVICE_GUIDANCE_OVERRIDES: Partial<Record<string, HeadlessActionGuidanceNote[]>> = {
  workflow_submit_catalog: [
    {
      label: t("Control authority", "控制权", "制御権限", "Autoridad de control"),
      value: t(
        "Keep this action on the orchestrated control-plane path. Do not mix direct-mesh tokens into the same submission contract.",
        "这个动作应当留在编排控制面路径上，不要把 direct-mesh 凭证混进同一次提交流程。",
        "このアクションはオーケストレーション制御プレーン経路に留め、同一投入契約へ direct-mesh トークンを混在させないでください。",
        "Mantenga esta accion en la ruta del plano de control orquestado. No mezcle tokens de direct mesh en el mismo contrato de envio.",
      ),
    },
    {
      label: t("Install visibility", "安装可见性", "インストール可視性", "Visibilidad de instalacion"),
      value: t(
        "Package source, residual cleanup policy, and repair steps should stay visible in installer diagnostics before catalog workflows become standard.",
        "在目录工作流成为标准路径前，包来源、残留清理策略和修复步骤都应该在 installer 诊断里保持可见。",
        "カタログ workflow を標準経路にする前に、パッケージ由来、残留クリーンアップ方針、修復手順を installer 診断で可視に保ってください。",
        "Antes de convertir estos workflows de catalogo en la ruta estandar, mantenga visibles en el instalador el origen del paquete, la politica de residuos y los pasos de reparacion.",
      ),
    },
  ],
  workflow_submit_graph: [
    {
      label: t("Control authority", "控制权", "制御権限", "Autoridad de control"),
      value: t(
        "Ad hoc graph submission still belongs to one orchestrated authority. Avoid combining multiple clusters or runtime modes inside one graph launch.",
        "临时图提交依然只能属于一个编排控制权，不要在一次图启动里混入多个集群或多种 runtime mode。",
        "アドホックグラフ投入も単一のオーケストレーション権限に属します。1 回のグラフ起動へ複数クラスターや複数 runtime mode を混在させないでください。",
        "El envio de grafos ad hoc sigue perteneciendo a una sola autoridad orquestada. Evite combinar varios clusters o modos de runtime en un mismo lanzamiento.",
      ),
    },
    {
      label: t("Repair path", "修复路径", "修復経路", "Ruta de reparacion"),
      value: t(
        "If graph dependencies drift, surface repair-only installer mode first instead of silently retrying with unknown package state.",
        "如果图依赖发生漂移，应先暴露 installer 的 repair-only 模式，而不是在未知包状态下静默重试。",
        "グラフ依存がドリフトした場合、未知のパッケージ状態で黙って再試行するより先に installer の repair-only モードを提示してください。",
        "Si las dependencias del grafo se desalinean, exponga primero el modo repair-only del instalador en lugar de reintentar silenciosamente con un estado de paquetes desconocido.",
      ),
    },
  ],
  direct_mesh_solve: [
    {
      label: t("Single authority", "单一控制权", "単一権限", "Autoridad unica"),
      value: t(
        "Direct-mesh solve should use one visible direct-mesh authority only. Do not combine orchestrator and mesh credentials in the same request path.",
        "direct-mesh 求解只能使用一个可见的直连控制权，不要在同一请求路径里同时混用 orchestrator 和 mesh 凭证。",
        "direct-mesh solve では可視な単一 direct-mesh 権限のみを使い、同一リクエスト経路へ orchestrator と mesh の認証を混在させないでください。",
        "La resolucion direct mesh debe usar una sola autoridad direct mesh visible. No combine credenciales del orquestador y de la malla en la misma ruta de solicitud.",
      ),
    },
    {
      label: t("Safe mode", "安全模式", "セーフモード", "Modo seguro"),
      value: t(
        "When cluster exposure or runtime drift appears, the frontend downgrades back to orchestrated GUI safe mode. Headless callers should follow the same expectation.",
        "一旦出现集群暴露或 runtime 漂移，前端会自动降级回 orchestrated GUI 安全模式。无头调用方也应遵守同样的预期。",
        "クラスター露出や runtime ドリフトが生じた場合、フロントエンドは orchestrated GUI のセーフモードへ自動降格します。ヘッドレス呼び出し側も同じ前提に従ってください。",
        "Cuando aparezcan exposicion de clusters o deriva de runtime, el frontend volvera automaticamente al modo seguro orchestrated GUI. Los consumidores headless deben seguir esa misma expectativa.",
      ),
    },
    {
      label: t("Endpoint discipline", "端点纪律", "エンドポイント規律", "Disciplina de endpoints"),
      value: t(
        "Keep endpoint lists explicit, reviewable, and tied to standard install rules so old solver residues do not become invisible routing state.",
        "端点列表必须显式、可审计，并且与标准安装规则绑定，避免旧 solver 残留变成不可见的路由状态。",
        "エンドポイント一覧は明示的かつ監査可能に保ち、標準インストール規則へ結び付けて旧 solver 残留が不可視な経路状態にならないようにしてください。",
        "Mantenga la lista de endpoints explicita y auditable, ligada a reglas de instalacion estandar para que residuos antiguos del solver no se conviertan en estado de ruteo invisible.",
      ),
    },
  ],
  solve_and_wait_from_model_version: [
    {
      label: t("Execution scope", "执行范围", "実行スコープ", "Alcance de ejecucion"),
      value: t(
        "This shortcut folds submit, wait, and fetch into one action, but it still inherits the same single-authority runtime contract as the underlying solve path.",
        "这个快捷动作虽然把提交、等待、抓取折叠到一步里，但仍然继承底层求解链路的单一控制权 runtime 契约。",
        "このショートカットは投入・待機・取得を 1 ステップに畳み込みますが、基盤 solve 経路と同じ単一権限 runtime 契約を継承します。",
        "Este atajo agrupa enviar, esperar y obtener en una sola accion, pero sigue heredando el mismo contrato de runtime de autoridad unica que la ruta de resolucion subyacente.",
      ),
    },
    {
      label: t("Visibility", "可见性", "可視性", "Visibilidad"),
      value: t(
        "Because follow-up is implicit here, diagnostics and audit output should remain enabled so job transitions do not disappear from operator review.",
        "因为这里把后续步骤隐含掉了，所以诊断和审计输出要保持开启，避免任务状态迁移从操作员视野里消失。",
        "ここでは後続手順が暗黙化されるため、ジョブ遷移が運用者レビューから消えないよう診断と監査出力を有効に保ってください。",
        "Como aqui el seguimiento queda implicito, mantenga activos el diagnostico y la auditoria para que las transiciones del trabajo no desaparezcan de la revision operativa.",
      ),
    },
  ],
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

const SERVICE_HEADLESS_ACTIONS: HeadlessActionContract[] = (headlessServiceActionContracts as AutomationContractRecord[])
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
    guidanceNotes: SERVICE_GUIDANCE_OVERRIDES[contract.id],
  }));

export const HEADLESS_ACTIONS: HeadlessActionContract[] = SERVICE_HEADLESS_ACTIONS.concat([
  {
    id: "frontend_macro_bridge",
    risk: "normal",
    summary: {
      en: "Run a frontend automation subflow as an explicit bridge node inside this workflow draft.",
      zh: "把一段前端自动化子流程作为显式桥接节点挂进当前工作流草稿。",
      ja: "フロントエンド自動化サブフローを明示的なブリッジノードとして現在のワークフロー草稿へ組み込みます。",
      es: "Inserta un subflujo de automatizacion frontend como nodo puente explicito dentro de este borrador de workflow.",
    },
    payloadExample: {
      macro_id: "macro/frontend-subflow",
      replay_mode: "bridge",
      steps: [{ action: "runtime/refreshAll", payload: {} }],
    },
    inputSchema: [
      { key: "macro_id", label: "macro_id", required: true },
      { key: "replay_mode", label: "replay_mode" },
      { key: "steps", label: "steps", required: true },
    ],
    outputSchema: [
      { key: "macro_id", label: "macro_id" },
      { key: "step_count", label: "step_count" },
    ],
    guidanceNotes: [
      {
        label: t("Boundary", "边界", "境界", "Limite"),
        value: t(
          "This node is an explicit bridge into frontend automation. Keep the UI-owned flow stable and do not treat it like a fully headless service action.",
          "这个节点是通往前端自动化的显式桥接点。UI 侧流程属于固有界面，不应把它当成完全无头的 service action。",
          "このノードはフロントエンド自動化への明示的なブリッジです。UI 側フローは固有の画面に属し、完全なヘッドレス service action と同一視しないでください。",
          "Este nodo es un puente explicito hacia la automatizacion frontend. Mantenga estable el flujo gobernado por la UI y no lo trate como si fuera una accion de servicio completamente headless.",
        ),
      },
    ],
  },
]);

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
