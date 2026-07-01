defmodule KyuubikiWeb.Orchestra.OperatorTaskExecutor do
  @moduledoc """
  Local executor for the operator task IR contract.

  Rust agents can implement the same dispatch contract behind
  `run_operator_task_ir`; this module keeps Elixir-side tests executable.
  """

  alias KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary
  alias KyuubikiWeb.WorkflowOperatorRuntime

  @schema_version "kyuubiki.operator-task-ir/v1"

  @spec execute(map()) :: {:ok, map()} | {:error, term()}
  def execute(%{"schema_version" => @schema_version} = task_ir) do
    with :ok <- OperatorTaskExecutionSummary.validate_digest(task_ir),
         {:ok, summary} <- OperatorTaskExecutionSummary.build(task_ir),
         {:ok, input} <- input_artifact(task_ir),
         config <- config(task_ir),
         node <- execution_node(task_ir, summary["operator_id"]) do
      dispatch(summary["operator_kind"], summary["operator_id"], input, config, node)
    end
  end

  def execute(_task_ir), do: {:error, :invalid_operator_task_ir}

  defp dispatch("solver", operator_id, input, _config, node),
    do: WorkflowOperatorRuntime.run_solve_operator(operator_id, input, node)

  defp dispatch("transform", operator_id, input, config, _node),
    do: WorkflowOperatorRuntime.run_transform_operator(operator_id, input, config)

  defp dispatch("workflow_bridge", operator_id, input, config, _node),
    do: WorkflowOperatorRuntime.run_transform_operator(operator_id, input, config)

  defp dispatch("extract", operator_id, input, config, _node),
    do: WorkflowOperatorRuntime.run_extract_operator(operator_id, input, config)

  defp dispatch("export", operator_id, input, config, _node),
    do: WorkflowOperatorRuntime.run_export_operator(operator_id, input, config)

  defp dispatch(kind, _operator_id, _input, _config, _node),
    do: {:error, {:unsupported_operator_task_kind, kind}}

  defp input_artifact(%{"input_artifact" => input}) when is_map(input), do: {:ok, input}
  defp input_artifact(_task_ir), do: {:error, :missing_operator_task_input}

  defp config(%{"config" => config}) when is_map(config), do: config
  defp config(_task_ir), do: %{}

  defp execution_node(%{"node" => node, "orchestration_context" => context}, operator_id)
       when is_map(node) and is_map(context) do
    node
    |> Map.put_new("operator_id", operator_id)
    |> Map.put("orchestration_context", context)
  end

  defp execution_node(%{"node" => node}, operator_id) when is_map(node),
    do: Map.put_new(node, "operator_id", operator_id)

  defp execution_node(_task_ir, operator_id), do: %{"operator_id" => operator_id}
end
