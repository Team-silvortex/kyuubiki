"use client";

type WorkbenchSystemInstallPolicyCardProps = {
  title: string;
  hint: string;
  integrityLabel: string;
  integrityValue: string;
  integrityAction?: { label: string; onClick: () => void; target: string; tone?: "good" | "watch" };
  updateLabel: string;
  updateValue: string;
  updateAction?: { label: string; onClick: () => void; target: string; tone?: "good" | "watch" };
  cleanupLabel: string;
  cleanupValue: string;
  cleanupAction?: { label: string; onClick: () => void; target: string; tone?: "good" | "watch" };
  formatLabel: string;
  formatValue: string;
  formatAction?: { label: string; onClick: () => void; target: string; tone?: "good" | "watch" };
};

export function WorkbenchSystemInstallPolicyCard({
  title,
  hint,
  integrityLabel,
  integrityValue,
  integrityAction,
  updateLabel,
  updateValue,
  updateAction,
  cleanupLabel,
  cleanupValue,
  cleanupAction,
  formatLabel,
  formatValue,
  formatAction,
}: WorkbenchSystemInstallPolicyCardProps) {
  const rows = [
    { label: integrityLabel, value: integrityValue, action: integrityAction },
    { label: updateLabel, value: updateValue, action: updateAction },
    { label: cleanupLabel, value: cleanupValue, action: cleanupAction },
    { label: formatLabel, value: formatValue, action: formatAction },
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
              </div>
            ) : null}
          </section>
        ))}
      </div>
    </section>
  );
}
