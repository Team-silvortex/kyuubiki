defmodule KyuubikiWeb.WorkflowSweepResultChecks do
  @moduledoc false
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep

  @reserved ~w(id case_id caseId model parameters metadata result_status result_error objective_score objective_feasible objective_breakdown)

  def metric_field(field, path) do
    if field in @reserved,
      do: Check.fail("#{path}: #{field} is reserved for identity or provenance")

    field
  end

  def success(value, status, error, path) do
    if Map.has_key?(value, status) do
      selected = Sweep.text(value, status, nil, path)

      if selected not in ~w(ok success succeeded completed complete done joined),
        do: Check.fail("#{path}.#{status}=#{selected} is not successful")
    end

    if Map.get(value, error) != nil, do: Check.fail("#{path} contains #{error}")
    Check.converged(value, path)
  end

  def complete(payload) do
    for flag <- ~w(join_complete summary_complete) do
      if !Sweep.flag(payload, flag),
        do: Check.fail("payload.#{flag} is false; evidence is incomplete")
    end

    for {counter, entries} <- [
          {"missing_summary_count", "missing_case_ids"},
          {"unmatched_result_count", "unmatched_result_ids"},
          {"rejected_result_count", "rejected_results"},
          {"missing_field_count", "missing_fields"}
        ] do
      list = Check.list(Map.get(payload, entries, []), "payload.#{entries}")
      count = Sweep.count(Map.get(payload, counter, length(list)), "payload.#{counter}")

      if Map.has_key?(payload, entries) && count != length(list),
        do: Check.fail("payload.#{counter} disagrees with #{entries}")

      if count > 0, do: Check.fail("payload.#{counter}=#{count}; evidence is incomplete")
    end
  end
end
