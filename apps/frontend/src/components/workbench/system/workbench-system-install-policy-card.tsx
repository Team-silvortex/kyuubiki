"use client";

type WorkbenchSystemInstallPolicyCardProps = {
  title: string;
  hint: string;
  integrityLabel: string;
  integrityValue: string;
  integrityAction?: { label: string; onClick: () => void; target: string; tone?: "good" | "watch" };
  integritySecondaryAction?: { label: string; onClick: () => void };
  updateLabel: string;
  updateValue: string;
  updateAction?: { label: string; onClick: () => void; target: string; tone?: "good" | "watch" };
  updateSecondaryAction?: { label: string; onClick: () => void };
  cleanupLabel: string;
  cleanupValue: string;
  cleanupAction?: { label: string; onClick: () => void; target: string; tone?: "good" | "watch" };
  cleanupSecondaryAction?: { label: string; onClick: () => void };
  formatLabel: string;
  formatValue: string;
  formatAction?: { label: string; onClick: () => void; target: string; tone?: "good" | "watch" };
  formatSecondaryAction?: { label: string; onClick: () => void };
};

export function WorkbenchSystemInstallPolicyCard({
  title,
  hint,
  integrityLabel,
  integrityValue,
  integrityAction,
  integritySecondaryAction,
  updateLabel,
  updateValue,
  updateAction,
  updateSecondaryAction,
  cleanupLabel,
  cleanupValue,
  cleanupAction,
  cleanupSecondaryAction,
  formatLabel,
  formatValue,
  formatAction,
  formatSecondaryAction,
}: WorkbenchSystemInstallPolicyCardProps) {
  const rows = [
    { label: integrityLabel, value: integrityValue, action: integrityAction, secondaryAction: integritySecondaryAction },
    { label: updateLabel, value: updateValue, action: updateAction, secondaryAction: updateSecondaryAction },
    { label: cleanupLabel, value: cleanupValue, action: cleanupAction, secondaryAction: cleanupSecondaryAction },
    { label: formatLabel, value: formatValue, action: formatAction, secondaryAction: formatSecondaryAction },
  ] as const;

  return (
    <section className="sidebar-card sidebar-card--compact system-policy-card">
      <div className="card-head">
        <h2>{title}</h2>
        <span>policy</span>
      </div>
      <p className="card-copy">{hint}</p>
      <div className="sidebar-list system-policy-card__list">
        {rows.map((row) => (
          <section className="system-policy-entry" key={row.label}>
            <div className="sidebar-list__row">
              <span>{row.label}</span>
              {row.action ? (
                <span className={`status-pill status-pill--${row.action.tone ?? "watch"}`}>{row.action.target}</span>
              ) : null}
            </div>
            <p className="card-copy">{row.value}</p>
            {row.action ? (
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={row.action.onClick} type="button">{row.action.label}</button>
                {row.secondaryAction ? (
                  <button className="ghost-button ghost-button--compact" onClick={row.secondaryAction.onClick} type="button">
                    {row.secondaryAction.label}
                  </button>
                ) : null}
              </div>
            ) : null}
          </section>
        ))}
      </div>
    </section>
  );
}
