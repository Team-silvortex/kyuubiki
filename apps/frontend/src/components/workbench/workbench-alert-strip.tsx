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
};

export function WorkbenchAlertStrip({ alerts }: WorkbenchAlertStripProps) {
  if (alerts.length === 0) return null;

  return (
    <div className="workbench-alert-strip" data-workbench-alert-strip="true">
      {alerts.map((alert) => (
        <div
          key={alert.id}
          className={`card-copy workbench-alert-strip__item workbench-alert-strip__item--${alert.tone ?? "info"}`}
        >
          <span>{alert.message}</span>
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
