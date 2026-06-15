import type { WorkflowOperatorDescriptor } from "@/lib/api";
import {
  normalizeBridgeConfigForOperator,
  resolveBridgeContractForOperator,
  type WorkflowBridgeContract,
  type WorkflowBridgeContractSupport,
} from "@/lib/workbench/workflow-bridge-contract";

export type WorkflowBridgeContractNormalizationAdjustment = {
  field:
    | "source.field"
    | "source.distribution"
    | "source.node_index_fields"
    | "transform.reduction"
    | "target.field";
  previous: string;
  next: string;
};

export function resolveBridgeContractFieldOptions(
  contract: WorkflowBridgeContract,
  support?: WorkflowBridgeContractSupport | null,
) {
  const distributionOptions = Object.keys(support?.source.distributions ?? {});
  const sourceFieldOptions =
    support?.source.distributions?.[contract.source.distribution] ??
    support?.source.fields ??
    [];
  const reductionOptions = support?.transform.reductions ?? [];
  const targetFieldOptions = support?.target.fields ?? [];
  const nodeIndexFieldOptions =
    contract.source.distribution === "element_to_nodes" ? support?.source.node_index_fields ?? [] : [];

  return {
    distributionOptions,
    sourceFieldOptions,
    reductionOptions,
    targetFieldOptions,
    nodeIndexFieldOptions,
  };
}

export function applyBridgeDistributionDefaults(
  contract: WorkflowBridgeContract,
  nextDistribution: string,
  support?: WorkflowBridgeContractSupport | null,
) {
  const next = structuredClone(contract);
  next.source.distribution = nextDistribution;

  const sourceFieldOptions =
    support?.source.distributions?.[nextDistribution] ?? support?.source.fields ?? [];
  if (sourceFieldOptions.length > 0 && !sourceFieldOptions.includes(next.source.field)) {
    next.source.field = sourceFieldOptions[0];
  }

  next.source.node_index_fields =
    nextDistribution === "element_to_nodes" ? [...(support?.source.node_index_fields ?? next.source.node_index_fields)] : [];

  const defaultReduction = support?.transform.default_reduction_by_distribution?.[nextDistribution];
  if (defaultReduction) {
    next.transform.reduction = defaultReduction;
  } else if (
    support?.transform.reductions?.length &&
    !support.transform.reductions.includes(next.transform.reduction)
  ) {
    next.transform.reduction = support.transform.reductions[0];
  }

  if (
    support?.target.fields?.length &&
    !support.target.fields.includes(next.target.field)
  ) {
    next.target.field = support.target.default_field ?? support.target.fields[0];
  }

  return next;
}

export function normalizeBridgeContractWithSupport(
  contract: WorkflowBridgeContract,
  support?: WorkflowBridgeContractSupport | null,
) {
  if (!support) return structuredClone(contract);

  const distributionOptions = Object.keys(support.source.distributions ?? {});
  const nextDistribution =
    distributionOptions.length > 0 && !distributionOptions.includes(contract.source.distribution)
      ? distributionOptions[0]
      : contract.source.distribution;
  const next = applyBridgeDistributionDefaults(contract, nextDistribution, support);

  const sourceFieldOptions =
    support.source.distributions?.[next.source.distribution] ?? support.source.fields ?? [];
  if (sourceFieldOptions.length > 0 && !sourceFieldOptions.includes(next.source.field)) {
    next.source.field = sourceFieldOptions[0];
  }

  if (
    support.transform.reductions?.length &&
    !support.transform.reductions.includes(next.transform.reduction)
  ) {
    next.transform.reduction =
      support.transform.default_reduction_by_distribution?.[next.source.distribution] ??
      support.transform.reductions[0];
  }

  if (support.target.fields?.length && !support.target.fields.includes(next.target.field)) {
    next.target.field = support.target.default_field ?? support.target.fields[0];
  }

  return next;
}

export function listBridgeContractNormalizationAdjustments(
  contract: WorkflowBridgeContract,
  support?: WorkflowBridgeContractSupport | null,
): WorkflowBridgeContractNormalizationAdjustment[] {
  if (!support) return [];
  const next = normalizeBridgeContractWithSupport(contract, support);
  const adjustments: WorkflowBridgeContractNormalizationAdjustment[] = [];

  if (contract.source.field !== next.source.field) {
    adjustments.push({ field: "source.field", previous: contract.source.field, next: next.source.field });
  }
  if (contract.source.distribution !== next.source.distribution) {
    adjustments.push({
      field: "source.distribution",
      previous: contract.source.distribution,
      next: next.source.distribution,
    });
  }

  const previousNodeIndexFields = contract.source.node_index_fields.join(", ");
  const nextNodeIndexFields = next.source.node_index_fields.join(", ");
  if (previousNodeIndexFields !== nextNodeIndexFields) {
    adjustments.push({
      field: "source.node_index_fields",
      previous: previousNodeIndexFields || "--",
      next: nextNodeIndexFields || "--",
    });
  }
  if (contract.transform.reduction !== next.transform.reduction) {
    adjustments.push({
      field: "transform.reduction",
      previous: contract.transform.reduction,
      next: next.transform.reduction,
    });
  }
  if (contract.target.field !== next.target.field) {
    adjustments.push({ field: "target.field", previous: contract.target.field, next: next.target.field });
  }

  return adjustments;
}

export function normalizeBridgeConfigWithSupport(
  operatorId?: string | null,
  config?: Record<string, unknown> | null,
  descriptor?: WorkflowOperatorDescriptor | null,
) {
  const normalizedConfig = normalizeBridgeConfigForOperator(operatorId, config);
  if (!descriptor?.contract_support) return normalizedConfig;
  const contract = resolveBridgeContractForOperator(operatorId, normalizedConfig);
  if (!contract) return normalizedConfig;
  const adjustments = listBridgeContractNormalizationAdjustments(contract, descriptor.contract_support);
  return {
    ...(normalizedConfig ?? {}),
    contract: normalizeBridgeContractWithSupport(contract, descriptor.contract_support),
    ...(adjustments.length > 0 ? { contract_normalization: adjustments } : { contract_normalization: undefined }),
  };
}
