defmodule KyuubikiWeb.WorkflowQualityObjectiveRuntime do
  @moduledoc false

  defdelegate compose_quality_objective(payload, config),
    to: KyuubikiWeb.WorkflowQualityComposition,
    as: :run

  defdelegate rank_quality_candidates(payload, config),
    to: KyuubikiWeb.WorkflowQualityRanking,
    as: :run

  defdelegate prepare_quality_next_round_request(payload, config),
    to: KyuubikiWeb.WorkflowQualityNextRound,
    as: :run

  defdelegate build_quality_parameter_sweep_plan(payload, config),
    to: KyuubikiWeb.WorkflowQualitySweep,
    as: :plan

  defdelegate materialize_quality_sweep_expansion(payload, config),
    to: KyuubikiWeb.WorkflowQualitySweep,
    as: :materialize

  defdelegate compose_quality_execution_batch(payload, config),
    to: KyuubikiWeb.WorkflowQualityExecutionBatch,
    as: :run
end
