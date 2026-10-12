defmodule KyuubikiWeb.WorkflowQualityBatchAdmission do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep
  alias KyuubikiWeb.WorkflowQualitySweepAdmission, as: Quality

  @quality_fields ~w(source_plan_contract coupled_readiness optimization_hint repair_strategy
    repair_focus sweep_enabled sweep_action sweep_blocking_reason source_ranking_complete
    source_rejected_candidates selected_candidate_ready sweep_budget converged stability_result
    expansion_enabled expansion_budget_ready)
  @metadata_fields ~w(metadata case_metadata)

  def evidence_fields, do: @quality_fields ++ @metadata_fields
  def evidence(value), do: Map.take(value, evidence_fields())

  def validate(payload, entries_key) do
    scope(payload, "payload")

    payload
    |> Map.fetch!(entries_key)
    |> Enum.with_index()
    |> Enum.each(fn
      {entry, index} when is_map(entry) -> scope(entry, "payload.#{entries_key}[#{index}]")
      _ -> :ok
    end)
  end

  def check_batch(batch) do
    case Check.run("quality execution batch", fn -> validate(batch, "tasks") end) do
      {:ok, _} -> :ok
      {:error, reason} -> {:error, {:operator_task_batch_quality_rejected, reason}}
    end
  end

  defp scope(value, path) do
    quality(value, path)

    for key <- @metadata_fields, Map.has_key?(value, key) do
      metadata = Sweep.json(value[key], "#{path}.#{key}")
      if is_map(metadata), do: quality(metadata, "#{path}.#{key}")
    end
  end

  defp quality(value, path) do
    if Enum.any?(@quality_fields, &Map.has_key?(value, &1)) do
      # Reuse plan intent rules; blocked seeds with explicit repair remain executable.
      enabled = Quality.admit_plan(value)
      expansion_enabled = Sweep.flag(value, "expansion_enabled")
      budget_ready = Sweep.flag(value, "expansion_budget_ready")
      budget = Check.config(value["sweep_budget"])
      blocked = Sweep.flag(budget, "case_budget_exceeded", false)

      if !enabled || !expansion_enabled || !budget_ready || blocked,
        do: Check.fail("disabled or budget-blocked evidence cannot authorize execution")
    end
  catch
    {:workflow_reporting_invalid, reason} -> Check.fail("#{path}: #{reason}")
  end
end
