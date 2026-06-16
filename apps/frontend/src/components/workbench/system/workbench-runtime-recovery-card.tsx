"use client";

import type { WorkbenchRuntimeRecoveryState } from "@/components/workbench/workbench-runtime-recovery";

type WorkbenchRuntimeRecoveryCardProps = {
  language: "en" | "zh" | "ja" | "es";
  recovery: WorkbenchRuntimeRecoveryState;
  onRetryAll: () => void;
  onRetryHealth: () => void;
  onRetryProjects: () => void;
  onRetrySecurityEvents: () => void;
  onRetryWorkflowCatalog: () => void;
};

export function WorkbenchRuntimeRecoveryCard({
  language,
  recovery,
  onRetryAll,
  onRetryHealth,
  onRetryProjects,
  onRetrySecurityEvents,
  onRetryWorkflowCatalog,
}: WorkbenchRuntimeRecoveryCardProps) {
  const copy =
    language === "zh"
      ? {
          title: "错误恢复",
          healthy: "正常",
          degraded: "降级",
          offline: "离线",
          empty: "当前没有待恢复的运行时故障。",
          retryAll: "全部重试",
          health: "重试健康检查",
          projects: "重试项目同步",
          security: "重试安全审计",
          workflow: "重试算子目录",
          hint: "恢复建议",
          lastFailure: "最近失败",
          code: "状态码",
        }
      : language === "ja"
        ? {
            title: "障害回復",
            healthy: "正常",
            degraded: "低下",
            offline: "オフライン",
            empty: "回復待ちのランタイム障害はありません。",
            retryAll: "すべて再試行",
            health: "ヘルス再試行",
            projects: "プロジェクト同期再試行",
            security: "監査再試行",
            workflow: "演算子カタログ再試行",
            hint: "回復ヒント",
            lastFailure: "直近の失敗",
            code: "ステータス",
          }
        : {
            title: "Recovery",
            healthy: "healthy",
            degraded: "degraded",
            offline: "offline",
            empty: "No runtime recovery issues are pending.",
            retryAll: "Retry all",
            health: "Retry health",
            projects: "Retry projects",
            security: "Retry audit",
            workflow: "Retry operators",
            hint: "Recovery hint",
            lastFailure: "Last failure",
            code: "Status",
          };
  const statusLabel =
    recovery.availability === "offline"
      ? copy.offline
      : recovery.availability === "degraded"
        ? copy.degraded
        : copy.healthy;

  return (
    <section className="sidebar-card sidebar-card--compact runtime-overview-card">
      <div className="card-head">
        <h2>{copy.title}</h2>
        <span>{statusLabel}</span>
      </div>
      {recovery.issues.length === 0 ? (
        <p className="card-copy">{copy.empty}</p>
      ) : (
        <div className="sidebar-list sidebar-list--metrics">
          {recovery.issues.slice(0, 3).map((issue) => (
            <div key={`${issue.channel}:${issue.lastFailureAt}`} style={{ display: "grid", gap: "0.35rem" }}>
              <div className="sidebar-list__row">
                <span>{issue.scopeLabel}</span>
                <strong>{issue.kind}</strong>
              </div>
              <p className="card-copy" style={{ margin: 0 }}>{issue.message}</p>
              <p className="card-copy" style={{ margin: 0 }}>{copy.hint}: {issue.recoveryHint}</p>
              <p className="card-copy" style={{ margin: 0 }}>
                {copy.lastFailure}: {new Date(issue.lastFailureAt).toLocaleString()}
                {typeof issue.statusCode === "number" ? ` · ${copy.code} ${issue.statusCode}` : ""}
              </p>
            </div>
          ))}
        </div>
      )}
      <div className="button-row" style={{ flexWrap: "wrap" }}>
        <button onClick={onRetryAll} type="button">{copy.retryAll}</button>
        <button onClick={onRetryHealth} type="button">{copy.health}</button>
        <button onClick={onRetryProjects} type="button">{copy.projects}</button>
        <button onClick={onRetrySecurityEvents} type="button">{copy.security}</button>
        <button onClick={onRetryWorkflowCatalog} type="button">{copy.workflow}</button>
      </div>
    </section>
  );
}
