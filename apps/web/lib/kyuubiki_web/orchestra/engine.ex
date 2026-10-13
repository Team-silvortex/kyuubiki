defmodule KyuubikiWeb.Orchestra.Engine do
  @moduledoc """
  Compact orchestration engine facade.

  The engine owns workflow catalog lookup and graph execution wiring. Web-facing
  services can stay focused on jobs, persistence, and transport concerns.
  """

  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.Playground.AgentRpcTransport
  alias KyuubikiWeb.Orchestra.WorkflowSolveCheckpoint
  alias KyuubikiWeb.Orchestra.WorkflowBranchCheckpoint
  alias KyuubikiWeb.WorkflowCondition
  alias KyuubikiWeb.WorkflowGraphPreflight
  alias KyuubikiWeb.WorkflowGraphProvenance
  alias KyuubikiWeb.WorkflowGraphResponse
  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowNativeOperatorRuntime
  alias KyuubikiWeb.WorkflowTaskDigestPolicy
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime
  alias KyuubikiWeb.WorkflowTemplateCatalog

  @type progress_callback :: (map() -> term())

  @spec list_workflow_catalog(map()) :: map()
  def list_workflow_catalog(filters \\ %{}) when is_map(filters),
    do: %{"workflows" => WorkflowTemplateCatalog.list(filters)}

  @spec list_operator_catalog(map()) :: map()
  def list_operator_catalog(filters \\ %{}) when is_map(filters),
    do: WorkflowOperatorCatalog.catalog(filters)

  @spec fetch_workflow_catalog_entry(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_workflow_catalog_entry(workflow_id) when is_binary(workflow_id),
    do: WorkflowTemplateCatalog.fetch(workflow_id)

  @spec fetch_operator_catalog_entry(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_operator_catalog_entry(operator_id) when is_binary(operator_id),
    do: WorkflowOperatorCatalog.fetch(operator_id)

  @spec workflow_graph_by_id(String.t()) :: {:ok, map()} | {:error, term()}
  def workflow_graph_by_id(workflow_id) when is_binary(workflow_id),
    do: WorkflowTemplateCatalog.graph_by_id(workflow_id)

  @spec run_workflow_graph(map()) :: {:ok, map()} | {:error, term()}
  def run_workflow_graph(params) when is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    with %{} = graph <- Map.get(normalized, "graph"),
         %{} = input_artifacts <- Map.get(normalized, "input_artifacts"),
         :ok <- WorkflowGraphPreflight.validate(graph, input_artifacts),
         response_options <-
           WorkflowGraphResponse.resolve_options(graph, Map.get(normalized, "response_options")),
         {:ok, result} <-
           execute_workflow_graph(graph, input_artifacts, %{}, nil, response_options) do
      {:ok, WorkflowGraphResponse.shape(graph, result, response_options)}
    else
      nil -> {:error, :invalid_workflow_graph_request}
      [] -> {:error, :invalid_workflow_graph_request}
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph_request}
    end
  end

  @spec execute_workflow_graph(map(), map(), map(), progress_callback() | nil, map(), keyword()) ::
          {:ok, map()} | {:error, term()}
  def execute_workflow_graph(
        graph,
        input_artifacts,
        orchestration_context \\ %{},
        progress_callback \\ nil,
        response_options \\ %{},
        execution_options \\ []
      )

  def execute_workflow_graph(
        %{} = graph,
        input_artifacts,
        orchestration_context,
        progress_callback,
        response_options,
        execution_options
      )
      when is_map(input_artifacts) and is_map(orchestration_context) and
             (is_nil(progress_callback) or is_function(progress_callback, 1)) and
             is_map(response_options) and is_list(execution_options) do
    with {:ok, encoding} <- WorkflowTaskDigestPolicy.resolve(graph) do
      run_graph(
        graph,
        input_artifacts,
        orchestration_context,
        progress_callback,
        response_options,
        Keyword.put(execution_options, :native_task_digest_encoding, encoding)
      )
    end
  end

  def execute_workflow_graph(
        _graph,
        _input_artifacts,
        _orchestration_context,
        _progress_callback,
        _response_options,
        _execution_options
      ),
      do: {:error, :invalid_workflow_graph}

  defp run_graph(
         graph,
         input_artifacts,
         orchestration_context,
         progress_callback,
         response_options,
         execution_options
       ) do
    WorkflowGraphRunner.run(
      graph,
      input_artifacts,
      dataset_contract: Map.get(graph, "dataset_contract"),
      result_options: response_options,
      progress_callback: progress_callback,
      execute_condition: fn payload, node, binding ->
        WorkflowBranchCheckpoint.run(
          node,
          payload,
          WorkflowGraphProvenance.bind_context(orchestration_context, binding),
          execution_options,
          fn ->
            WorkflowCondition.decide(payload, node)
          end
        )
      end,
      execute_solve: fn operator_id, payload, node, binding ->
        context = WorkflowGraphProvenance.bind_context(orchestration_context, binding)

        WorkflowSolveCheckpoint.run(
          node,
          payload,
          context,
          execution_options,
          fn ->
            WorkflowOperatorRuntime.run_solve_operator(
              operator_id,
              payload,
              Map.put(node, "orchestration_context", context),
              operator_activity_callback(node, progress_callback),
              before_dispatch: operator_dispatch_callback(node, progress_callback)
            )
          end
        )
      end,
      execute_transform: fn id, payload, config, node, binding ->
        if WorkflowNativeOperatorRuntime.supports?("transform", id),
          do:
            WorkflowNativeOperatorRuntime.run(
              "transform",
              id,
              payload,
              config,
              node,
              WorkflowGraphProvenance.bind_context(orchestration_context, binding),
              on_progress: operator_activity_callback(node, progress_callback),
              before_dispatch: operator_dispatch_callback(node, progress_callback),
              native_dispatch_evidence: Keyword.get(execution_options, :native_dispatch_evidence),
              native_result_checkpoint: Keyword.get(execution_options, :native_result_checkpoint),
              native_task_digest_encoding:
                Keyword.fetch!(execution_options, :native_task_digest_encoding),
              recover_native_results:
                Keyword.get(execution_options, :recover_native_results, false)
            ),
          else: WorkflowOperatorRuntime.run_transform_operator(id, payload, config)
      end,
      execute_extract: fn id, payload, config, node, binding ->
        if WorkflowNativeOperatorRuntime.supports?("extract", id),
          do:
            WorkflowNativeOperatorRuntime.run(
              "extract",
              id,
              payload,
              config,
              node,
              WorkflowGraphProvenance.bind_context(orchestration_context, binding),
              on_progress: operator_activity_callback(node, progress_callback),
              before_dispatch: operator_dispatch_callback(node, progress_callback),
              native_dispatch_evidence: Keyword.get(execution_options, :native_dispatch_evidence),
              native_result_checkpoint: Keyword.get(execution_options, :native_result_checkpoint),
              native_task_digest_encoding:
                Keyword.fetch!(execution_options, :native_task_digest_encoding),
              recover_native_results:
                Keyword.get(execution_options, :recover_native_results, false)
            ),
          else: WorkflowOperatorRuntime.run_extract_operator(id, payload, config)
      end,
      execute_export: &WorkflowOperatorRuntime.run_export_operator/3
    )
  end

  defp operator_dispatch_callback(node, progress_callback) do
    fn ->
      if is_function(progress_callback, 1),
        do:
          AgentRpcTransport.emit_progress(progress_callback, %{
            "event" => "operator_dispatch",
            "node_id" => node["id"]
          }),
        else: :ok
    end
  end

  defp operator_activity_callback(node, progress_callback) do
    fn activity ->
      if is_function(progress_callback, 1) do
        progress_callback.(%{
          "event" => "operator_activity",
          "node_id" => node["id"],
          "activity" => activity
        })
      end
    end
  end
end
