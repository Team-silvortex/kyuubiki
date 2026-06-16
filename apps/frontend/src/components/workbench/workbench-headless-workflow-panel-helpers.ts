"use client";

import type { FrontendRuntimeMode } from "@/lib/api";
import type { WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";
import { readInMemoryWorkbenchSecrets } from "@/lib/workbench/helpers";

export function readStoredWorkbenchAuth(): {
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshEndpointsText: string;
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
} {
  if (typeof window === "undefined") {
    return {
      frontendRuntimeMode: "orchestrated_gui" as const,
      directMeshEndpointsText: "",
      controlPlaneApiToken: "",
      clusterApiToken: "",
      directMeshApiToken: "",
    };
  }

  try {
    const settings = JSON.parse(window.localStorage.getItem("kyuubiki-workbench-settings") ?? "{}") as {
      frontendRuntimeMode?: "orchestrated_gui" | "direct_mesh_gui";
      directMeshEndpointsText?: string;
    };
    const secrets = readInMemoryWorkbenchSecrets();

    return {
      frontendRuntimeMode:
        settings.frontendRuntimeMode === "direct_mesh_gui" ? "direct_mesh_gui" : "orchestrated_gui",
      directMeshEndpointsText: settings.directMeshEndpointsText ?? "",
      controlPlaneApiToken: secrets.controlPlaneApiToken ?? "",
      clusterApiToken: secrets.clusterApiToken ?? "",
      directMeshApiToken: secrets.directMeshApiToken ?? "",
    };
  } catch {
    return {
      frontendRuntimeMode: "orchestrated_gui" as FrontendRuntimeMode,
      directMeshEndpointsText: "",
      controlPlaneApiToken: "",
      clusterApiToken: "",
      directMeshApiToken: "",
    };
  }
}

export function buildHeadlessWorkflowPanelCopy(language: WorkbenchScriptLanguage) {
  return {
    title: language === "zh" ? "无头 SDK 工作流" : language === "ja" ? "ヘッドレス SDK ワークフロー" : "Headless SDK Workflow",
    subtitle:
      language === "zh"
        ? "这里专门编排脱离前端界面的 service / solver 无头链路，和 WASM Python 的前端自动化是分开的。"
        : language === "ja"
          ? "ここではフロントエンド UI を介さない service / solver 向けヘッドレスチェーンを組み立てます。WASM Python の前段自動化とは別系統です。"
          : "Compose service and solver headless chains here. This is separate from the WASM Python frontend-automation surface above.",
    draftId: language === "zh" ? "草稿 ID" : language === "ja" ? "ドラフト ID" : "Draft ID",
    loadTemplate: language === "zh" ? "载入模板" : language === "ja" ? "テンプレート読込" : "Load template",
    importJson: language === "zh" ? "导入 SDK 契约 JSON" : language === "ja" ? "SDK 契約 JSON 読込" : "Import SDK contract JSON",
    exportJson: language === "zh" ? "导出 SDK 契约 JSON" : language === "ja" ? "SDK 契約 JSON 書き出し" : "Export SDK contract JSON",
    exportBatch: language === "zh" ? "导出执行批次 JSON" : language === "ja" ? "実行バッチ JSON 書き出し" : "Export execution batch JSON",
    exportDispatch: language === "zh" ? "导出 agent 派发 JSON" : language === "ja" ? "agent 配送 JSON 書き出し" : "Export agent dispatch JSON",
    exportHandoff: language === "zh" ? "导出 orchestra handoff JSON" : language === "ja" ? "orchestra handoff JSON 書き出し" : "Export orchestra handoff JSON",
    submitHandoff: language === "zh" ? "提交 orchestra handoff" : language === "ja" ? "orchestra handoff を送信" : "Submit orchestra handoff",
    refreshHandoff: language === "zh" ? "查询 handoff 状态" : language === "ja" ? "handoff 状態を照会" : "Query handoff status",
    refreshHistory: language === "zh" ? "刷新 handoff 历史" : language === "ja" ? "handoff 履歴を更新" : "Refresh handoff history",
    exportHtml: language === "zh" ? "导出 SDK 契约 HTML" : language === "ja" ? "SDK 契約 HTML 書き出し" : "Export SDK contract HTML",
    runBatch: language === "zh" ? "运行执行批次" : language === "ja" ? "実行バッチを実行" : "Run execution batch",
    executionLog: language === "zh" ? "执行日志" : language === "ja" ? "実行ログ" : "Execution log",
    insert: language === "zh" ? "插入脚本编辑器" : language === "ja" ? "スクリプトへ挿入" : "Insert into editor",
    remove: language === "zh" ? "删除步骤" : language === "ja" ? "ステップ削除" : "Remove step",
    moveUp: language === "zh" ? "上移" : language === "ja" ? "上へ" : "Move up",
    moveDown: language === "zh" ? "下移" : language === "ja" ? "下へ" : "Move down",
    payloadJson: language === "zh" ? "完整参数 JSON" : language === "ja" ? "完全なペイロード JSON" : "Full payload JSON",
    endpoints: language === "zh" ? "求解端点" : language === "ja" ? "解析エンドポイント" : "Solve endpoints",
    guidanceTitle: language === "zh" ? "治理契约" : language === "ja" ? "ガバナンス契約" : "Governance contract",
    referenceTitle: language === "zh" ? "可用前序引用" : language === "ja" ? "利用可能な前段参照" : "Available prior-step refs",
    referenceApply: language === "zh" ? "绑定引用" : language === "ja" ? "参照を接続" : "Bind reference",
    referenceClear: language === "zh" ? "清除绑定" : language === "ja" ? "接続解除" : "Clear binding",
    referenceCurrent: language === "zh" ? "当前绑定" : language === "ja" ? "現在の接続" : "Current binding",
    noReferences: language === "zh" ? "当前字段前面还没有可匹配输出。" : language === "ja" ? "この入力に合う前段出力はまだありません。" : "No matching prior outputs for this input yet.",
    endpointsHint: language === "zh" ? "每行一个 endpoint" : language === "ja" ? "1 行につき 1 endpoint" : "One endpoint per line",
    normal: language === "zh" ? "普通" : language === "ja" ? "通常" : "Normal",
    sensitive: language === "zh" ? "敏感" : language === "ja" ? "注意" : "Sensitive",
    destructive: language === "zh" ? "高风险" : language === "ja" ? "高リスク" : "Destructive",
    invalidJson: language === "zh" ? "当前有无效 JSON，先修正参数。" : language === "ja" ? "現在の JSON が無効です。先に修正してください。" : "One or more payload JSON blocks are invalid.",
    executionDone: language === "zh" ? "执行批次已完成。" : language === "ja" ? "実行バッチが完了しました。" : "Execution batch completed.",
    imported: language === "zh" ? "已把 headless workflow 契约重新载入搭建面板。" : language === "ja" ? "headless workflow 契約をビルダーパネルへ再読み込みしました。" : "Reloaded the headless workflow contract back into the builder.",
    handoffSubmitted: language === "zh" ? "orchestra handoff 已提交，接收回执已写入执行日志。" : language === "ja" ? "orchestra handoff を送信し、受領レシートを実行ログへ記録しました。" : "Submitted the orchestra handoff and recorded the receipt in the execution log.",
    handoffRefreshed: language === "zh" ? "已刷新最近一次 handoff 的状态，并写入执行日志。" : language === "ja" ? "直近 handoff の状態を更新し、実行ログへ記録しました。" : "Refreshed the latest handoff status and recorded it in the execution log.",
    historyRefreshed: language === "zh" ? "已刷新当前运行时里的 handoff 历史列表。" : language === "ja" ? "現在のランタイム内 handoff 履歴を更新しました。" : "Refreshed the handoff history registered in the current runtime.",
    noHandoffYet: language === "zh" ? "还没有可查询的 handoff，请先提交一次。" : language === "ja" ? "照会できる handoff がまだありません。先に送信してください。" : "No handoff is available yet. Submit one first.",
    handoffHistory: language === "zh" ? "Handoff 历史" : language === "ja" ? "Handoff 履歴" : "Handoff history",
    handoffHistoryEmpty: language === "zh" ? "当前运行时里还没有已登记的 handoff。" : language === "ja" ? "このランタイムにはまだ登録済み handoff がありません。" : "No handoffs are currently registered in this runtime.",
    handoffSnapshot: language === "zh" ? "Handoff 快照" : language === "ja" ? "Handoff スナップショット" : "Handoff snapshot",
    handoffSnapshotEmpty: language === "zh" ? "选择一条 handoff 历史后，这里会显示完整快照。" : language === "ja" ? "handoff 履歴を選ぶと、ここに完全スナップショットを表示します。" : "Select a handoff entry to inspect the full snapshot here.",
    handoffInspect: language === "zh" ? "查看快照" : language === "ja" ? "スナップショット表示" : "Inspect snapshot",
    handoffFilter: language === "zh" ? "阶段筛选" : language === "ja" ? "段階フィルタ" : "Stage filter",
    handoffWorkflowFilter: language === "zh" ? "工作流 / Handoff 搜索" : language === "ja" ? "ワークフロー / Handoff 検索" : "Workflow / handoff search",
    handoffAllStages: language === "zh" ? "全部阶段" : language === "ja" ? "すべての段階" : "All stages",
    handoffStage: language === "zh" ? "阶段" : language === "ja" ? "段階" : "Stage",
    handoffStatusMessage: language === "zh" ? "状态说明" : language === "ja" ? "状態説明" : "Status message",
    handoffDispatchPlan: language === "zh" ? "Dispatch 规划" : language === "ja" ? "Dispatch 計画" : "Dispatch plan",
    handoffGovernance: language === "zh" ? "治理快照" : language === "ja" ? "ガバナンススナップショット" : "Governance snapshot",
    handoffRuntimeManifest: language === "zh" ? "运行时清单" : language === "ja" ? "ランタイムマニフェスト" : "Runtime manifest",
    handoffRawSnapshot: language === "zh" ? "原始快照 JSON" : language === "ja" ? "生スナップショット JSON" : "Raw snapshot JSON",
    handoffSelected: language === "zh" ? "当前选中的 handoff" : language === "ja" ? "現在選択中の handoff" : "Selected handoff",
    handoffLane: language === "zh" ? "派发通道" : language === "ja" ? "配送レーン" : "Dispatch lane",
    handoffLaneOrchestrator: language === "zh" ? "编排服务通道" : language === "ja" ? "オーケストレーター通路" : "Orchestrator service lane",
    handoffLaneDirectMesh: language === "zh" ? "直连求解通道" : language === "ja" ? "ダイレクトメッシュ通路" : "Direct-mesh solver lane",
    handoffLaneFrontendBridge: language === "zh" ? "前端桥接通道" : language === "ja" ? "フロントエンドブリッジ通路" : "Frontend bridge lane",
    handoffChosenAgent: language === "zh" ? "已选 agent" : language === "ja" ? "選択済み agent" : "Chosen agent",
    handoffCandidates: language === "zh" ? "候选数量" : language === "ja" ? "候補数" : "Candidates",
    handoffInspectCandidates: language === "zh" ? "展开候选 agent" : language === "ja" ? "候補 agent を展開" : "Expand candidate agents",
    handoffHideCandidates: language === "zh" ? "收起候选 agent" : language === "ja" ? "候補 agent を折りたたむ" : "Collapse candidate agents",
    handoffCapabilities: language === "zh" ? "必需能力" : language === "ja" ? "必要能力" : "Required capabilities",
    handoffNote: language === "zh" ? "调度说明" : language === "ja" ? "配送メモ" : "Dispatch note",
    handoffOverridePromote: language === "zh" ? "提升为本地首选" : language === "ja" ? "ローカル優先へ昇格" : "Promote to local preferred",
    handoffOverrideClear: language === "zh" ? "清除本地 override" : language === "ja" ? "ローカル override を解除" : "Clear local override",
    handoffOverrideLocal: language === "zh" ? "本地 override" : language === "ja" ? "ローカル override" : "Local override",
    handoffOverrideExport: language === "zh" ? "导出 override 草案" : language === "ja" ? "override 草案を書き出し" : "Export override draft",
    handoffOverrideExportHandoff: language === "zh" ? "导出带 override 的 handoff 草案" : language === "ja" ? "override 付き handoff 草案を書き出し" : "Export handoff draft with override",
    handoffOverrideImport: language === "zh" ? "导入 override 草案" : language === "ja" ? "override 草案を読み込み" : "Import override draft",
    handoffOverrideImported: language === "zh" ? "已把 override 草案应用到当前快照视图。" : language === "ja" ? "override 草案を現在のスナップショット表示へ適用しました。" : "Applied the override draft to the current snapshot view.",
    handoffOverrideHandoffExported: language === "zh" ? "已导出带 override 的 handoff 草案。" : language === "ja" ? "override 付き handoff 草案を書き出しました。" : "Exported the handoff draft with override.",
    handoffWinnerReason: language === "zh" ? "胜出原因" : language === "ja" ? "選定理由" : "Winner reason",
    handoffCandidateEndpoint: language === "zh" ? "候选端点" : language === "ja" ? "候補エンドポイント" : "Candidate endpoint",
    handoffCandidateCluster: language === "zh" ? "候选集群" : language === "ja" ? "候補クラスタ" : "Candidate cluster",
    handoffCandidateRuntime: language === "zh" ? "候选运行时" : language === "ja" ? "候補ランタイム" : "Candidate runtime",
    handoffCandidateScore: language === "zh" ? "候选分数" : language === "ja" ? "候補スコア" : "Candidate score",
    handoffCandidateHealth: language === "zh" ? "健康分" : language === "ja" ? "ヘルス点" : "Health score",
    handoffCandidateHeadlessBonus: language === "zh" ? "无头加成" : language === "ja" ? "ヘッドレス加点" : "Headless bonus",
    handoffCandidateRuntimeBonus: language === "zh" ? "运行时匹配加成" : language === "ja" ? "ランタイム一致加点" : "Runtime-match bonus",
    handoffAuthority: language === "zh" ? "Authority 诊断" : language === "ja" ? "Authority 診断" : "Authority diagnostics",
    handoffAuthorityLabel: language === "zh" ? "Authority 标签" : language === "ja" ? "Authority ラベル" : "Authority label",
    handoffControlMode: language === "zh" ? "控制模式" : language === "ja" ? "制御モード" : "Control mode",
    handoffTopology: language === "zh" ? "拓扑" : language === "ja" ? "トポロジ" : "Topology",
    handoffViolation: language === "zh" ? "有无违规" : language === "ja" ? "違反有無" : "Violation",
    handoffClusterScope: language === "zh" ? "集群范围" : language === "ja" ? "クラスタ範囲" : "Cluster scope",
    handoffRuntimeModes: language === "zh" ? "运行时模式" : language === "ja" ? "ランタイムモード" : "Runtime modes",
    handoffExposureLabel: language === "zh" ? "暴露范围" : language === "ja" ? "露出範囲" : "Exposure label",
    handoffDrift: language === "zh" ? "Drift 诊断" : language === "ja" ? "Drift 診断" : "Drift diagnostics",
    handoffDriftLabel: language === "zh" ? "漂移标签" : language === "ja" ? "ドリフトラベル" : "Drift label",
    handoffWarning: language === "zh" ? "告警数" : language === "ja" ? "警告数" : "Warnings",
    handoffReceivedCount: language === "zh" ? "已接收" : language === "ja" ? "受領済み" : "Received",
    handoffQueuedCount: language === "zh" ? "已排队" : language === "ja" ? "キュー済み" : "Queued",
    handoffDispatchPlannedCount: language === "zh" ? "已派发规划" : language === "ja" ? "配送計画済み" : "Dispatch planned",
    handoffReadyCount: language === "zh" ? "可交给 orchestra" : language === "ja" ? "orchestra 受け渡し可能" : "Ready for orchestra",
    handoffOverrideBadge: language === "zh" ? "含 override" : language === "ja" ? "override あり" : "Has override",
    handoffOverrideCount: language === "zh" ? "Override 数量" : language === "ja" ? "Override 数" : "Override count",
    handoffOverrideAcknowledged: language === "zh" ? "Override 已确认" : language === "ja" ? "Override 確認済み" : "Override acknowledged",
    handoffOverrideNote: language === "zh" ? "Override 注记" : language === "ja" ? "Override 注記" : "Override note",
    invalidInsert: language === "zh" ? "当前有无效 JSON，无法插入到脚本编辑器。" : language === "ja" ? "現在の JSON が無効なため、スクリプトエディタへ挿入できません。" : "Cannot insert while the payload JSON is invalid.",
    frontendAssets: language === "zh" ? "前端自动化子流程资产" : language === "ja" ? "フロントエンド自動化サブフロー資産" : "Frontend automation subflow assets",
    frontendAssetsHint: language === "zh" ? "这里接收上方面板筛出来的前端局部宏，再决定是否插成一个桥接节点。" : language === "ja" ? "上の面で絞り込んだフロントエンド部分マクロをここで受け取り、ブリッジノードとして挿入するか決められます。" : "Filtered frontend local macros arrive here first, then you can insert them as explicit bridge nodes.",
    frontendAssetEmpty: language === "zh" ? "还没有从前端时间线发送过来的局部宏。" : language === "ja" ? "まだフロントエンドのタイムラインから送られた部分マクロはありません。" : "No frontend local macros have been sent here from the timeline yet.",
    assetSource: language === "zh" ? "来源" : language === "ja" ? "ソース" : "Source",
    assetUpdatedAt: language === "zh" ? "更新时间" : language === "ja" ? "更新時刻" : "Updated",
    assetSourceTimeline: language === "zh" ? "时间线筛选" : language === "ja" ? "タイムライン選別" : "Timeline selection",
    assetSourceBridge: language === "zh" ? "桥接恢复" : language === "ja" ? "ブリッジ復元" : "Bridge restore",
    assetSourceDerived: language === "zh" ? "历史派生" : language === "ja" ? "履歴派生" : "History derived",
    assetSnapshotId: language === "zh" ? "快照 ID" : language === "ja" ? "スナップショット ID" : "Snapshot ID",
    bridgeInsert: language === "zh" ? "插成桥接节点" : language === "ja" ? "ブリッジノードとして挿入" : "Insert as bridge node",
    bridgeDerive: language === "zh" ? "派生新前端草稿" : language === "ja" ? "新しい前段ドラフトへ派生" : "Derive new frontend draft",
    bridgeSteps: language === "zh" ? "前端步数" : language === "ja" ? "フロントエンド手順数" : "Frontend steps",
    bridgeMacroId: language === "zh" ? "前端宏 ID" : language === "ja" ? "フロントエンドマクロ ID" : "Frontend macro ID",
    bridgeReplayMode: language === "zh" ? "桥接模式" : language === "ja" ? "ブリッジモード" : "Bridge mode",
    bridgeRestore: language === "zh" ? "恢复到前端" : language === "ja" ? "フロントエンドへ戻す" : "Restore to frontend",
    bridgeReplayModeHint: language === "zh" ? "这里描述这段前端子流程如何被当前工作流草稿引用。" : language === "ja" ? "この値は、現在のワークフロー草稿がこの前段サブフローをどう参照するかを表します。" : "This value describes how the current workflow draft references the frontend subflow.",
    bridgeActionList: language === "zh" ? "前端动作列表" : language === "ja" ? "フロントエンドアクション一覧" : "Frontend action list",
    bridgePreviewShow: language === "zh" ? "展开前端预览" : language === "ja" ? "前段プレビューを展開" : "Expand frontend preview",
    bridgePreviewHide: language === "zh" ? "收起前端预览" : language === "ja" ? "前段プレビューを折りたたむ" : "Collapse frontend preview",
    bridgePreviewPayload: language === "zh" ? "前端参数" : language === "ja" ? "フロントエンド payload" : "Frontend payload",
    rawPayloadJson: language === "zh" ? "原始参数 JSON" : language === "ja" ? "生ペイロード JSON" : "Raw payload JSON",
  };
}
