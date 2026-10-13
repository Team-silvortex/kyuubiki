export function durableDirectRecoveryCopy(language: string) {
  if (language === "zh" || language.startsWith("zh-")) return {
    localPending: "本机待确认任务", cleanup: "最终回执已核对，但本机凭据尚未清理；请重新核对，不要重复提交。",
    title: "直连执行恢复", hint: "读取 Agent 已落盘的记录，不是 Orchestra 历史。仅列出持久化提交的任务；普通同步求解不在此列表。",
    endpoint: "固定 Agent", refresh: "读取第一页", next: "下一页", empty: "当前页没有接收记录。", choose: "选择执行记录",
    id: "执行 ID", inspect: "核对回执", cancel: "请求取消", pending: "正在查询或请求…", unchecked: "尚未核对",
    safety: "列表按 ID 排序，不代表时间顺序。未知或未找到不表示任务从未执行，不能自动重算。取消请求不等于已停止。",
    cancelSent: "取消请求已登记，请再次核对回执。", noOwner: "未观察到可取消的异步执行实例，请重新查询。",
    noRun: "记录已不在运行态，请重新查询最终回执。", result: "结果回执已保存；此处不覆盖当前模型或渲染结果。",
    states: { running: "运行中", completed: "已完成", failed: "已失败", outcome_unknown: "结果未知，禁止自动重算", not_found: "未找到记录，不能据此重算" },
  };
  if (language === "ja") return {
    localPending: "この端末の未確認実行", cleanup: "最終記録を確認しましたが、端末の実行情報は未整理です。再確認し、重複送信しないでください。",
    title: "直接実行の復旧", hint: "Orchestra の履歴ではなく、Agent に永続化された受理記録です。通常の同期実行は含みません。",
    endpoint: "固定 Agent", refresh: "最初のページ", next: "次のページ", empty: "このページに記録はありません。", choose: "実行記録を選択",
    id: "実行 ID", inspect: "実行記録を確認", cancel: "取消を要求", pending: "確認・要求中…", unchecked: "未確認",
    safety: "ID 順であり時系列ではありません。不明・未検出は未実行の証拠ではなく、自動再実行は禁止です。取消要求は停止完了ではありません。",
    cancelSent: "取消要求を登録しました。最終状態を再確認してください。", noOwner: "取消可能な非同期実行が見つかりません。再確認してください。",
    noRun: "記録は実行中ではありません。最終状態を再確認してください。", result: "結果記録が保存されています。現在のモデルや表示結果は変更しません。",
    states: { running: "実行中", completed: "完了", failed: "失敗", outcome_unknown: "結果不明・自動再実行禁止", not_found: "記録未検出・再実行不可" },
  };
  return {
    localPending: "Local Pending Executions", cleanup: "Terminal receipt inspected, but local identity cleanup is pending. Inspect again; do not resubmit.",
    title: "Direct Execution Recovery", hint: "Read admissions persisted by the Agent, not Orchestra history. Only durable submissions appear here; ordinary synchronous solves do not.",
    endpoint: "Pinned Agent", refresh: "Read First Page", next: "Next Page", empty: "No admissions on this page.", choose: "Select Execution",
    id: "Execution ID", inspect: "Inspect Receipt", cancel: "Request Cancellation", pending: "Inspecting or requesting…", unchecked: "Not inspected",
    safety: "Sorted by ID, not time. Unknown or not found does not prove that work never ran. Never replay automatically. A cancellation request is not confirmation of termination.",
    cancelSent: "Cancellation registered. Inspect again for the terminal receipt.", noOwner: "No cancellable async owner observed. Inspect again.",
    noRun: "The record is no longer running. Inspect its final receipt.", result: "A result receipt is saved; this panel does not overwrite the current model or visualization.",
    states: { running: "Running", completed: "Completed", failed: "Failed", outcome_unknown: "Outcome unknown; no automatic replay", not_found: "Record not found; no replay authorized" },
  };
}
