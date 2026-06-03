type HistoryWorkspaceCardProps = {
  title: string;
  hint: string;
  actionLabel: string;
  actionDisabled: boolean;
  onAction: () => void;
  metrics: Array<{ label: string; value: string | number }>;
};

export function HistoryWorkspaceCard({
  title,
  hint,
  actionLabel,
  actionDisabled,
  onAction,
  metrics,
}: HistoryWorkspaceCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
      </div>
      <p className="card-copy">{hint}</p>
      <div className="sidebar-list sidebar-list--metrics">
        {metrics.map((metric) => (
          <div key={metric.label} className="sidebar-list__row">
            <span>{metric.label}</span>
            <strong>{metric.value}</strong>
          </div>
        ))}
      </div>
      <div className="button-row">
        <button className="ghost-button" disabled={actionDisabled} onClick={onAction} type="button">
          {actionLabel}
        </button>
      </div>
    </section>
  );
}
