defmodule KyuubikiWeb.Orchestra.Engine do
  @moduledoc """
  Compact orchestration engine facade.

  The engine owns workflow catalog lookup and graph execution wiring. Web-facing
  services can stay focused on jobs, persistence, and transport concerns.
  """

  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.WorkflowGraphResponse
  alias KyuubikiWeb.WorkflowGraphRunner
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
         response_options <-
           WorkflowGraphResponse.resolve_options(graph, Map.get(normalized, "response_options")),
         %{} = input_artifacts <- Map.get(normalized, "input_artifacts"),
         {:ok, result} <- execute_workflow_graph(graph, input_artifacts, %{}, nil, response_options) do
      {:ok, WorkflowGraphResponse.shape(graph, result, response_options)}
    else
      nil -> {:error, :invalid_workflow_graph_request}
      [] -> {:error, :invalid_workflow_graph_request}
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph_request}
    end
  end

  @spec execute_workflow_graph(map(), map(), map(), progress_callback() | nil, map()) ::
          {:ok, map()} | {:error, term()}
  def execute_workflow_graph(
        graph,
        input_artifacts,
        orchestration_context \\ %{},
        progress_callback \\ nil,
        response_options \\ %{}
      )

  def execute_workflow_graph(
        %{} = graph,
        input_artifacts,
        orchestration_context,
        progress_callback,
        response_options
      )
      when is_map(input_artifacts) and is_map(orchestration_context) and
             (is_nil(progress_callback) or is_function(progress_callback, 1)) and
             is_map(response_options) do
    WorkflowGraphRunner.run(
      graph,
      input_artifacts,
      dataset_contract: Map.get(graph, "dataset_contract"),
      result_options: response_options,
      progress_callback: progress_callback,
      execute_solve: fn operator_id, payload, node ->
        WorkflowOperatorRuntime.run_solve_operator(
          operator_id,
          payload,
          Map.put(node, "orchestration_context", orchestration_context)
        )
      end,
      execute_transform: &WorkflowOperatorRuntime.run_transform_operator/3,
      execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
      execute_export: &WorkflowOperatorRuntime.run_export_operator/3
    )
  end

  def execute_workflow_graph(
        _graph,
        _input_artifacts,
        _orchestration_context,
        _progress_callback,
        _response_options
      ),
      do: {:error, :invalid_workflow_graph}
end
