defmodule KyuubikiWeb.Orchestra.OperatorTaskExecutor do
  @moduledoc """
  Local executor for the operator task IR contract.

  Rust agents can implement the same dispatch contract behind
  `run_operator_task_ir`; this module keeps Elixir-side tests executable.
  """

  alias KyuubikiWeb.WorkflowOperatorRuntime

  @schema_version "kyuubiki.operator-task-ir/v1"

  @spec execute(map()) :: {:ok, map()} | {:error, term()}
  def execute(%{"schema_version" => @schema_version} = task_ir) do
    with {:ok, operator} <- operator(task_ir),
         {:ok, operator_id} <- operator_id(operator),
         {:ok, kind} <- operator_kind(operator),
         {:ok, input} <- input_artifact(task_ir),
         config <- config(task_ir),
         node <- execution_node(task_ir, operator_id) do
      dispatch(kind, operator_id, input, config, node)
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

  defp operator(%{"operator" => operator}) when is_map(operator), do: {:ok, operator}
  defp operator(_task_ir), do: {:error, :missing_operator_task_operator}

  defp operator_id(%{"id" => operator_id}) when is_binary(operator_id) and operator_id != "",
    do: {:ok, operator_id}

  defp operator_id(_operator), do: {:error, :missing_operator_task_operator_id}

  defp operator_kind(%{"kind" => kind}) when is_binary(kind) and kind != "", do: {:ok, kind}
  defp operator_kind(_operator), do: {:error, :missing_operator_task_kind}

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
