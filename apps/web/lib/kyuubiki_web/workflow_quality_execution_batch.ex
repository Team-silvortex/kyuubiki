defmodule KyuubikiWeb.WorkflowQualityExecutionBatch do
  @moduledoc false
  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.WorkflowReportingChecks, as: Check
  alias KyuubikiWeb.WorkflowSweepChecks, as: Sweep
  alias KyuubikiWeb.WorkflowSweepExpansion
  alias KyuubikiWeb.WorkflowQualityBatchAdmission, as: Admission

  def run(payload, config) do
    Check.run("transform.compose_quality_execution_batch", fn ->
      payload = Check.object(payload, "payload")
      config = Check.config(config)
      operator = Sweep.text(config, "operator_id", nil, "config")
      prefix = Sweep.text(config, "task_id_prefix", "quality-task", "config")
      source = Sweep.text(config, "payload_source", "model", "config")
      merge = Sweep.flag(config, "merge_case_config", false, "config")

      mode =
        if Map.has_key?(config, "rpc_mode"),
          do: Check.choice(config, "rpc_mode", "execute", ~w(preflight execute))

      operator_config =
        Sweep.object(Map.get(config, "operator_config", %{}), "config.operator_config")

      context =
        Sweep.object(
          Map.get(config, "orchestration_context", %{}),
          "config.orchestration_context"
        )

      dataset = Sweep.object(Map.get(config, "dataset_contract", %{}), "config.dataset_contract")
      tags = strings(config, "placement_tags")
      capabilities = strings(config, "required_capabilities")

      payload =
        if Map.has_key?(payload, "quality_sweep_expansion_contract") do
          case WorkflowSweepExpansion.run(payload, config) do
            {:ok, expanded} -> expanded
            {:error, reason} -> Check.fail(reason)
          end
        else
          payload
        end

      ensure_enabled(payload)
      cases = Sweep.values(payload["cases"], "payload.cases")
      Sweep.require_count(payload, "case_count", length(cases))
      ids = Sweep.case_ids(cases)
      Admission.validate(payload, "cases")

      prepared =
        Enum.zip(cases, ids)
        |> Enum.with_index()
        |> Enum.map(fn {{entry, id}, index} ->
          path = "payload.cases[#{index}]"

          input =
            Sweep.object(
              if(source == "case", do: entry, else: entry[source]),
              "#{path}.#{source}"
            )

          parameters = Sweep.object(Map.get(entry, "parameters", %{}), "#{path}.parameters")
          local = Sweep.object(Map.get(entry, "config", %{}), "#{path}.config")

          {id, input, parameters,
           if(merge, do: Map.merge(operator_config, local), else: operator_config),
           Map.put_new(Admission.evidence(entry), "metadata", nil)}
        end)

      tasks =
        Enum.map(prepared, fn {id, input, parameters, task_config, evidence} ->
          case OperatorTaskIR.build(operator, input, task_config,
                 task_id: "#{prefix}:#{id}",
                 orchestration_context: context,
                 dataset_contract: dataset,
                 placement_tags: tags,
                 required_capabilities: capabilities
               ) do
            {:ok, task} ->
              %{
                "case_id" => id,
                "parameters" => parameters,
                "operator_id" => operator,
                "task_id" => task["task_id"],
                "task_digest" => task["integrity"]["task_digest"],
                "task_ir" => task,
                "agent_rpc" => %{
                  "method" => OperatorTaskIR.agent_rpc_method(),
                  "params" => OperatorTaskIR.agent_rpc_params(task, mode: mode),
                  "routing_opts" =>
                    Map.new(OperatorTaskIR.agent_routing_opts(task), fn {key, value} ->
                      {Atom.to_string(key), value}
                    end)
                }
              }
              |> Map.merge(evidence)

            {:error, reason} ->
              Check.fail("case #{id}: #{inspect(reason)}")
          end
        end)

      %{
        "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
        "operator_id" => operator,
        "task_count" => length(tasks),
        "agent_rpc_method" => OperatorTaskIR.agent_rpc_method(),
        "tasks" => tasks,
        "case_index" => Enum.map(tasks, &Map.take(&1, ~w(case_id task_id task_digest))),
        "batch_summary" => "Prepared #{length(tasks)} quality execution task(s) for #{operator}."
      }
      |> Map.merge(Admission.evidence(payload))
    end)
  end

  defp strings(config, key) do
    list = Check.list(Map.get(config, key, []), "config.#{key}")

    Enum.with_index(list)
    |> Enum.each(fn {value, index} ->
      Sweep.text(%{"value" => value}, "value", nil, "config.#{key}[#{index}]")
    end)

    list
  end

  defp ensure_enabled(payload) do
    {_budget, blocked} = Sweep.budget(payload)

    enabled =
      Sweep.flag(payload, "sweep_enabled") && Sweep.flag(payload, "expansion_enabled") &&
        Sweep.flag(payload, "expansion_budget_ready")

    if !enabled || blocked,
      do: Check.fail("payload is disabled or budget-blocked; no executable batch may be issued")
  end
end
