"use client";

export type WorkbenchAlertTone = "info" | "warning" | "error";

export type WorkbenchAlertItem = {
  id: string;
  message: string;
  tone?: WorkbenchAlertTone;
  onDismiss?: () => void;
};

type WorkbenchAlertStripProps = {
  alerts: WorkbenchAlertItem[];
  compact?: boolean;
};

export function WorkbenchAlertStrip({ alerts, compact = false }: WorkbenchAlertStripProps) {
  if (alerts.length === 0) return null;

  return (
    <div className={`workbench-alert-strip${compact ? " workbench-alert-strip--compact" : ""}`}
      data-workbench-alert-strip="true" aria-live="polite">
      {alerts.map((alert) => (
        <div
          key={alert.id}
          data-workbench-alert-id={alert.id}
          className={`card-copy workbench-alert-strip__item workbench-alert-strip__item--${alert.tone ?? "info"}`}
        >
          <span title={compact ? alert.message : undefined}>{alert.message}</span>
          {alert.onDismiss ? (
            <button
              aria-label="Dismiss alert"
              className="workbench-alert-strip__dismiss"
              onClick={alert.onDismiss}
              type="button"
            >
              x
            </button>
          ) : null}
        </div>
      ))}
    </div>
  );
}
