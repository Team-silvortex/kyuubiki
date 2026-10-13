defmodule KyuubikiWeb.WorkflowNativeOperatorRuntime do
  @moduledoc "TaskIR adapter for a bounded set of engine-owned workflow operators."

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskCompletion,
    OperatorTaskExecutionSummary,
    OperatorTaskIR,
    WorkflowNativeDispatchEvidence,
    WorkflowNativeResultRecovery
  }

  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.WorkflowNativeInvocation
  alias KyuubikiWeb.WorkflowTaskDigestPolicy

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

  def task_id(context, node, id), do: "#{context["job_id"] || "workflow"}:#{node["id"]}:#{id}"

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
    on_progress = Keyword.get(opts, :on_progress, fn _ -> :ok end)
    before_dispatch = Keyword.get(opts, :before_dispatch, fn -> :ok end)
    checkpoint = Keyword.get(opts, :native_result_checkpoint)

    cond do
      not is_function(on_progress, 1) or not is_function(before_dispatch, 0) or
        (not is_nil(checkpoint) and not is_function(checkpoint, 1)) or
          not is_boolean(Keyword.get(opts, :recover_native_results, false)) ->
        {:error, :invalid_native_workflow_operator_request}

      supports?(kind, id) ->
        execute(id, payload, if(is_nil(config), do: %{}, else: config), node, context, opts)

      true ->
        {:error, {:unsupported_native_workflow_operator, kind, id}}
    end
  end

  def run(_kind, _id, _payload, _config, _node, _context, _opts),
    do: {:error, :invalid_native_workflow_operator_request}

  defp execute(id, payload, config, node, context, opts) do
    context = WorkflowNativeInvocation.bind_context(context, payload, config)
    configured = Application.get_env(:kyuubiki_web, __MODULE__, [])
    client = Keyword.get(opts, :client, Keyword.get(configured, :client, AgentClient))
    on_progress = Keyword.get(opts, :on_progress, fn _ -> :ok end)

    task_opts = [
      node: node,
      orchestration_context: context,
      task_id: task_id(context, node, id),
      placement_tags: Map.get(node, "placement_tags", []),
      required_capabilities: Map.get(node, "required_capabilities", [])
    ]

    rpc_opts =
      if context["job_id"],
        do: [mode: :execute, job_id: context["job_id"]],
        else: [mode: :execute]

    rpc_opts = Keyword.merge(rpc_opts, Keyword.take(opts, [:before_dispatch]))

    with {:ok, task} <- OperatorTaskIR.build(id, payload, config, task_opts),
         {:ok, task} <-
           WorkflowTaskDigestPolicy.seal(
             task,
             Keyword.get(opts, :native_task_digest_encoding, WorkflowTaskDigestPolicy.legacy())
           ),
         {:ok, summary} <- OperatorTaskExecutionSummary.build(task) do
      rpc_opts =
        Keyword.merge(rpc_opts, WorkflowNativeDispatchEvidence.dispatch_options(summary, opts))

      case WorkflowNativeResultRecovery.resolve(task, summary, context, opts) do
        :execute -> dispatch(client, task, summary, rpc_opts, on_progress, context, opts)
        result -> result
      end
    end
  end

  defp dispatch(client, task, summary, rpc_opts, on_progress, context, opts) do
    with {:ok, result} <- client.run_operator_task_ir(task, rpc_opts, on_progress),
         {:ok, %{"status" => "executed", "result" => %{"result" => output}}} <-
           OperatorTaskCompletion.agent_receipt(summary, result),
         :ok <- WorkflowNativeResultRecovery.retain(summary, context, result, opts) do
      {:ok, output}
    else
      {:error, :operator_task_dispatch_evidence_unavailable} ->
        {:error, {:native_workflow_recovery_blocked, :dispatch_evidence_commit_failed}}

      {:error, :operator_task_dispatch_outcome_unknown} ->
        {:error, {:native_workflow_recovery_blocked, :dispatch_unconfirmed}}

      {:error, _} = error ->
        error

      {:ok, receipt} ->
        {:error, {:native_workflow_operator_not_executed, receipt["status"]}}
    end
  end
end
