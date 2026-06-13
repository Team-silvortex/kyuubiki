"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

export type WorkflowDomainGroup<T> = {
  key: string;
  label: string;
  entries: T[];
};

const DOMAIN_ORDER = [
  "electromagnetic",
  "thermal",
  "thermo_mechanical",
  "mechanical",
  "multi_domain",
  "other",
] as const;

const DOMAIN_LABELS: Record<(typeof DOMAIN_ORDER)[number], string> = {
  electromagnetic: "Electromagnetic",
  thermal: "Thermal",
  thermo_mechanical: "Thermo-Mechanical",
  mechanical: "Structural",
  multi_domain: "Multi-Domain",
  other: "Other",
};

function normalizeDomainCandidates(values: string[]) {
  return values.map((value) => value.trim().toLowerCase()).filter(Boolean);
}

function domainFromCandidates(candidates: string[]) {
  if (candidates.some((value) => value.includes("electro"))) return "electromagnetic";
  if (candidates.some((value) => value.includes("thermo"))) return "thermo_mechanical";
  if (candidates.some((value) => value.includes("thermal") || value.includes("heat"))) {
    return "thermal";
  }
  if (
    candidates.some((value) =>
      ["structural", "mechanical", "frame", "truss", "beam", "torsion", "spring", "plane"].some(
        (token) => value.includes(token),
      ),
    )
  ) {
    return "mechanical";
  }
  return "other";
}

function sortGroups<T>(groups: WorkflowDomainGroup<T>[]) {
  return [...groups].sort((left, right) => {
    const leftIndex = DOMAIN_ORDER.indexOf(left.key as (typeof DOMAIN_ORDER)[number]);
    const rightIndex = DOMAIN_ORDER.indexOf(right.key as (typeof DOMAIN_ORDER)[number]);
    return (leftIndex === -1 ? DOMAIN_ORDER.length : leftIndex) -
      (rightIndex === -1 ? DOMAIN_ORDER.length : rightIndex) || left.label.localeCompare(right.label);
  });
}

export function groupWorkflowCatalogEntriesByDomain(entries: WorkflowCatalogEntry[]) {
  const grouped = new Map<string, WorkflowCatalogEntry[]>();
  for (const entry of entries) {
    const key = domainFromCandidates(
      normalizeDomainCandidates([
        ...(entry.domains ?? []),
        ...(entry.capability_tags ?? []),
        ...(entry.local?.tags ?? []),
      ]),
    );
    const current = grouped.get(key) ?? [];
    current.push(entry);
    grouped.set(key, current);
  }
  return sortGroups(
    [...grouped.entries()].map(([key, groupedEntries]) => ({
      key,
      label: DOMAIN_LABELS[key as keyof typeof DOMAIN_LABELS] ?? "Other",
      entries: groupedEntries,
    })),
  );
}

export function groupTemplateChainsByDomain(chains: WorkflowTemplateChainDefinition[]) {
  const grouped = new Map<string, WorkflowTemplateChainDefinition[]>();
  for (const chain of chains) {
    const candidates = normalizeDomainCandidates([
      ...(chain.tags ?? []),
      ...chain.templates.flatMap((template) => [template.kind ?? "", template.operatorId ?? ""]),
      chain.summary ?? "",
      chain.label,
    ]);
    const key = domainFromCandidates(candidates);
    const current = grouped.get(key) ?? [];
    current.push(chain);
    grouped.set(key, current);
  }
  return sortGroups(
    [...grouped.entries()].map(([key, groupedEntries]) => ({
      key,
      label: DOMAIN_LABELS[key as keyof typeof DOMAIN_LABELS] ?? "Other",
      entries: groupedEntries,
    })),
  );
}
