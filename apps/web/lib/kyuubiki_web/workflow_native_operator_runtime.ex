defmodule KyuubikiWeb.WorkflowNativeOperatorRuntime do
  @moduledoc "TaskIR adapter for a bounded set of engine-owned workflow operators."

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskCompletion,
    OperatorTaskExecutionSummary,
    OperatorTaskIR
  }

  alias KyuubikiWeb.Playground.AgentClient

  @operators %{
    "extract.transport_result_diagnostics" => "extract",
    "extract.thermal_result_diagnostics" => "extract",
    "extract.electrostatic_result_diagnostics" => "extract",
    "extract.magnetostatic_result_diagnostics" => "extract",
    "extract.thermo_result_diagnostics" => "extract",
    "extract.stokes_flow_result_diagnostics" => "extract",
    "transform.score_transport_quality" => "transform",
    "transform.score_thermal_quality" => "transform",
    "transform.score_electrostatic_quality" => "transform",
    "transform.score_magnetostatic_quality" => "transform",
    "transform.score_cfd_quality" => "transform"
  }

  def supports?(kind, id), do: Map.get(@operators, id) == kind and is_binary(kind)

  def enrich_descriptor(%{"id" => id} = descriptor) when is_map_key(@operators, id) do
    Map.put(descriptor, "execution", %{
      "authority_mode" => "agent_local",
      "execution_mode" => "agent_native",
      "source_ref" => "builtin://engine/#{id}",
      "package_ref" => nil,
      "package_version" => "built-in",
      "integrity" => nil,
      "placement_tags" => [],
      "required_capabilities" => [],
      "cache_scope" => "none",
      "agent_fetchable" => false
    })
  end

  def enrich_descriptor(descriptor), do: descriptor

  def run(kind, id, payload, config, node, context, opts \\ [])

  def run(kind, id, payload, config, node, context, opts)
      when is_map(payload) and (is_nil(config) or is_map(config)) and is_map(node) and
             is_map(context) and is_list(opts) do
    if supports?(kind, id) do
      execute(id, payload, if(is_nil(config), do: %{}, else: config), node, context, opts)
    else
      {:error, {:unsupported_native_workflow_operator, kind, id}}
    end
  end

  def run(_kind, _id, _payload, _config, _node, _context, _opts),
    do: {:error, :invalid_native_workflow_operator_request}

  defp execute(id, payload, config, node, context, opts) do
    client = Keyword.get(opts, :client, AgentClient)

    task_opts = [
      node: node,
      orchestration_context: context,
      task_id: "#{context["job_id"] || "workflow"}:#{node["id"]}:#{id}",
      placement_tags: Map.get(node, "placement_tags", []),
      required_capabilities: Map.get(node, "required_capabilities", [])
    ]

    rpc_opts =
      if context["job_id"],
        do: [mode: :execute, job_id: context["job_id"]],
        else: [mode: :execute]

    with {:ok, task} <- OperatorTaskIR.build(id, payload, config, task_opts),
         {:ok, summary} <- OperatorTaskExecutionSummary.build(task),
         {:ok, result} <- client.run_operator_task_ir(task, rpc_opts),
         {:ok, %{"status" => "executed", "result" => %{"result" => output}}} <-
           OperatorTaskCompletion.agent_receipt(summary, result) do
      {:ok, output}
    else
      {:error, _} = error -> error
      {:ok, receipt} -> {:error, {:native_workflow_operator_not_executed, receipt["status"]}}
    end
  end
end
