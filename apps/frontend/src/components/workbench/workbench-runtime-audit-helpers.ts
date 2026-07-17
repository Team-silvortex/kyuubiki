import type { SecurityEventRecord } from "@/lib/api";
import type { SecurityEventWindow } from "@/components/workbench/workbench-types";
import { getWorkbenchRuntimeAuditCopy } from "@/components/workbench/workbench-extended-language-copy";

export function buildRuntimeAuditSummaryRows(language: string, securityEventRecords: SecurityEventRecord[]) {
  const countByRisk = securityEventRecords.reduce<Record<string, number>>((accumulator, entry) => {
    accumulator[entry.risk] = (accumulator[entry.risk] ?? 0) + 1;
    return accumulator;
  }, {});
  const countByStatus = securityEventRecords.reduce<Record<string, number>>((accumulator, entry) => {
    accumulator[entry.status] = (accumulator[entry.status] ?? 0) + 1;
    return accumulator;
  }, {});

  const copy = getWorkbenchRuntimeAuditCopy(language);
  return [
    { label: copy.sensitive, value: String(countByRisk.sensitive ?? 0) },
    { label: copy.destructive, value: String(countByRisk.destructive ?? 0) },
    { label: copy.completed, value: String(countByStatus.completed ?? 0) },
    { label: copy.cancelled, value: String(countByStatus.cancelled ?? 0) },
    { label: copy.failed, value: String(countByStatus.failed ?? 0) },
    { label: copy.prompted, value: String(countByStatus.prompted ?? 0) },
  ];
}

export function buildRuntimeAuditTrendBars(
  language: string,
  securityEventRecords: SecurityEventRecord[],
  securityEventWindowFilter: SecurityEventWindow,
) {
  if (securityEventRecords.length === 0) return [];

  const bucketCount =
    securityEventWindowFilter === "1h" ? 6 : securityEventWindowFilter === "24h" ? 8 : securityEventWindowFilter === "7d" ? 7 : 6;
  const bucketWindowMs =
    securityEventWindowFilter === "1h"
      ? 10 * 60 * 1_000
      : securityEventWindowFilter === "24h"
        ? 3 * 60 * 60 * 1_000
        : securityEventWindowFilter === "7d"
          ? 24 * 60 * 60 * 1_000
          : securityEventWindowFilter === "30d"
            ? 5 * 24 * 60 * 60 * 1_000
            : 24 * 60 * 60 * 1_000;

  const bucketLabels = Array.from({ length: bucketCount }, (_, index) => {
    if (securityEventWindowFilter === "1h") {
      return language === "zh" ? `${(bucketCount - index) * 10} 分内` : language === "ja" ? `${(bucketCount - index) * 10}分以内` : `${(bucketCount - index) * 10}m`;
    }
    if (securityEventWindowFilter === "24h") {
      return language === "zh" ? `${(bucketCount - index) * 3} 小时内` : language === "ja" ? `${(bucketCount - index) * 3}時間以内` : `${(bucketCount - index) * 3}h`;
    }
    if (securityEventWindowFilter === "7d") {
      return language === "zh" ? `${bucketCount - index} 天内` : language === "ja" ? `${bucketCount - index}日以内` : `${bucketCount - index}d`;
    }
    if (securityEventWindowFilter === "30d") {
      return language === "zh" ? `${(bucketCount - index) * 5} 天内` : language === "ja" ? `${(bucketCount - index) * 5}日以内` : `${(bucketCount - index) * 5}d`;
    }
    return language === "zh" ? `${bucketCount - index} 天内` : language === "ja" ? `${bucketCount - index}日以内` : `${bucketCount - index}d`;
  });

  const now = Date.now();
  const counts = new Array(bucketCount).fill(0);
  securityEventRecords.forEach((entry) => {
    const occurredAt = Date.parse(entry.occurred_at);
    if (Number.isNaN(occurredAt)) return;
    const ageMs = Math.max(now - occurredAt, 0);
    const bucketIndex = Math.min(Math.floor(ageMs / bucketWindowMs), bucketCount - 1);
    counts[bucketCount - bucketIndex - 1] += 1;
  });

  const maxCount = Math.max(...counts, 1);
  return counts.map((count, index) => ({
    key: `${bucketLabels[index]}-${index}`,
    label: bucketLabels[index],
    value: String(count),
    ratio: count / maxCount,
  }));
}

export function buildRuntimeAuditSourceStatusFacets(language: string, securityEventRecords: SecurityEventRecord[]) {
  const counts = securityEventRecords.reduce<Map<string, number>>((accumulator, entry) => {
    const key = `${entry.source}:${entry.status}`;
    accumulator.set(key, (accumulator.get(key) ?? 0) + 1);
    return accumulator;
  }, new Map<string, number>());

  return [...counts.entries()]
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    .slice(0, 6)
    .map(([key, value]) => {
      const [source, status] = key.split(":");
      const copy = getWorkbenchRuntimeAuditCopy(language);
      const sourceLabel = source === "assistant" ? copy.assistant : copy.script;
      const statusLabel =
        status === "prompted"
          ? copy.prompted
          : status === "cancelled"
            ? copy.cancelled
            : status === "completed"
              ? copy.completed
              : copy.failed;
      return { key, label: `${sourceLabel} / ${statusLabel}`, value: String(value) };
    });
}

function buildShortIdFacet(records: SecurityEventRecord[], key: "project_id" | "model_version_id") {
  const counts = records.reduce<Map<string, number>>((accumulator, entry) => {
    const rawId = typeof entry.context[key] === "string" ? entry.context[key] : "";
    if (!rawId) return accumulator;
    const shortId = rawId.slice(0, 8);
    accumulator.set(shortId, (accumulator.get(shortId) ?? 0) + 1);
    return accumulator;
  }, new Map<string, number>());

  return [...counts.entries()]
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    .slice(0, 4)
    .map(([label, value]) => ({ key: label, label, value: String(value) }));
}

export function buildRuntimeAuditStudyFacets(securityEventRecords: SecurityEventRecord[]) {
  const counts = securityEventRecords.reduce<Map<string, number>>((accumulator, entry) => {
    const studyKind = typeof entry.context.study_kind === "string" ? entry.context.study_kind : "";
    if (!studyKind) return accumulator;
    accumulator.set(studyKind, (accumulator.get(studyKind) ?? 0) + 1);
    return accumulator;
  }, new Map<string, number>());

  return [...counts.entries()]
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    .slice(0, 4)
    .map(([label, value]) => ({ key: label, label, value: String(value) }));
}

export function buildRuntimeAuditProjectFacets(securityEventRecords: SecurityEventRecord[]) {
  return buildShortIdFacet(securityEventRecords, "project_id");
}

export function buildRuntimeAuditModelVersionFacets(securityEventRecords: SecurityEventRecord[]) {
  return buildShortIdFacet(securityEventRecords, "model_version_id");
}
