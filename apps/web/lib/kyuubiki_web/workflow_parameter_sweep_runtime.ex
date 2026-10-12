defmodule KyuubikiWeb.WorkflowParameterSweepRuntime do
  @moduledoc false

  defdelegate expand_parameter_sweep(payload, config),
    to: KyuubikiWeb.WorkflowSweepExpansion,
    as: :run

  defdelegate join_parameter_sweep_results(payload, config),
    to: KyuubikiWeb.WorkflowSweepJoin,
    as: :run

  defdelegate summarize_parameter_sweep(payload, config),
    to: KyuubikiWeb.WorkflowSweepSummary,
    as: :run

  defdelegate score_parameter_sweep(payload, config),
    to: KyuubikiWeb.WorkflowSweepScore,
    as: :run
end
