defmodule KyuubikiWeb.Orchestra.OperatorTaskIR do
  @moduledoc """
  Converts catalog operator descriptions into an engine-facing task IR.
  """

  alias KyuubikiWeb.WorkflowOperatorCatalog

  @schema_version "kyuubiki.operator-task-ir/v1"
  @agent_rpc_method "run_operator_task_ir"

  @spec build(String.t(), map(), map(), keyword()) :: {:ok, map()} | {:error, term()}
  def build(operator_id, input_artifact, config \\ %{}, opts \\ [])

  def build(operator_id, input_artifact, config, opts)
      when is_binary(operator_id) and is_map(input_artifact) and is_map(config) and is_list(opts) do
    with {:ok, %{"operator" => operator}} <- WorkflowOperatorCatalog.fetch(operator_id) do
      {:ok, envelope(operator, input_artifact, config, opts)}
    end
  end

  def build(_operator_id, _input_artifact, _config, _opts),
    do: {:error, :invalid_operator_task_ir_request}

  @spec from_node(map(), map(), keyword()) :: {:ok, map()} | {:error, term()}
  def from_node(node, input_artifact, opts \\ [])

  def from_node(%{"operator_id" => operator_id} = node, input_artifact, opts)
      when is_binary(operator_id) and is_map(input_artifact) and is_list(opts) do
    config =
      case Map.get(node, "config") do
        value when is_map(value) -> value
        _ -> %{}
      end

    build(operator_id, input_artifact, config, Keyword.put_new(opts, :node, node))
  end

  def from_node(_node, _input_artifact, _opts), do: {:error, :invalid_operator_task_node}

  @spec agent_rpc_method() :: String.t()
  def agent_rpc_method, do: @agent_rpc_method

  @spec agent_rpc_params(map()) :: map()
  def agent_rpc_params(%{"schema_version" => @schema_version} = task_ir),
    do: %{"task_ir" => task_ir}

  def agent_rpc_params(_task_ir), do: %{"task_ir" => nil}

  @spec agent_routing_opts(map()) :: keyword()
  def agent_routing_opts(%{"schema_version" => @schema_version} = task_ir) do
    runtime_hints = Map.get(task_ir, "runtime_hints", %{})

    []
    |> maybe_put_routing_opt(
      :required_capabilities,
      normalize_string_list(Map.get(runtime_hints, "required_capabilities"))
    )
    |> maybe_put_routing_opt(
      :placement_tags,
      normalize_string_list(Map.get(runtime_hints, "placement_tags"))
    )
    |> maybe_put_routing_opt(
      :orchestration,
      normalize_context(Map.get(task_ir, "orchestration_context"))
    )
    |> maybe_put_routing_opt(:job_id, Map.get(task_ir, "task_id"))
  end

  def agent_routing_opts(_task_ir), do: []

  defp envelope(operator, input_artifact, config, opts) do
    execution = Map.get(operator, "execution", %{})
    node = Keyword.get(opts, :node, %{})
    task_id = Keyword.get(opts, :task_id) || default_task_id(operator, node)
    orchestration_context = Keyword.get(opts, :orchestration_context, %{})
    dataset_contract = Keyword.get(opts, :dataset_contract, %{})

    %{
      "schema_version" => @schema_version,
      "task_id" => task_id,
      "operator" => operator_snapshot(operator),
      "node" => node_snapshot(node),
      "input_artifact" => input_artifact,
      "config" => config,
      "dataset_contract" => dataset_contract,
      "orchestration_context" => orchestration_context,
      "runtime_hints" => runtime_hints(operator, execution, opts),
      "integrity" => integrity_snapshot(operator, execution)
    }
  end

  defp operator_snapshot(operator) do
    Map.take(operator, [
      "id",
      "version",
      "domain",
      "family",
      "kind",
      "origin",
      "summary",
      "capability_tags",
      "input_schema",
      "output_schema",
      "inputs",
      "outputs",
      "validation",
      "module",
      "execution"
    ])
  end

  defp node_snapshot(%{"id" => _id} = node) do
    Map.take(node, ["id", "kind", "operator_id", "inputs", "outputs", "placement_tags"])
  end

  defp node_snapshot(_node), do: %{}

  defp runtime_hints(operator, execution, opts) do
    %{
      "authority_mode" => Map.get(execution, "authority_mode", "central_operator_library"),
      "execution_mode" => Map.get(execution, "execution_mode", "orchestra_fetch"),
      "source_ref" => Map.get(execution, "source_ref"),
      "package_ref" => Map.get(execution, "package_ref"),
      "package_version" => Map.get(execution, "package_version", "library-managed"),
      "placement_tags" =>
        Keyword.get(opts, :placement_tags, Map.get(execution, "placement_tags", [])),
      "required_capabilities" =>
        Keyword.get(opts, :required_capabilities, Map.get(execution, "required_capabilities", [])),
      "cache_scope" => Map.get(execution, "cache_scope", "job"),
      "agent_fetchable" => Map.get(execution, "agent_fetchable", true),
      "operator_kind" => Map.get(operator, "kind")
    }
  end

  defp maybe_put_routing_opt(opts, _key, []), do: opts
  defp maybe_put_routing_opt(opts, _key, %{} = value) when map_size(value) == 0, do: opts
  defp maybe_put_routing_opt(opts, _key, nil), do: opts
  defp maybe_put_routing_opt(opts, key, value), do: Keyword.put(opts, key, value)

  defp normalize_string_list(values) when is_list(values) do
    values
    |> Enum.filter(&is_binary/1)
    |> Enum.uniq()
  end

  defp normalize_string_list(_values), do: []

  defp normalize_context(%{} = context), do: context
  defp normalize_context(_context), do: %{}

  defp integrity_snapshot(operator, execution) do
    %{
      "operator_integrity" => Map.get(execution, "integrity"),
      "descriptor_digest" => descriptor_digest(operator),
      "digest_algorithm" => "sha256"
    }
  end

  defp descriptor_digest(operator) do
    json = Jason.encode!(operator_snapshot(operator))
    :crypto.hash(:sha256, json) |> Base.encode16(case: :lower)
  end

  defp default_task_id(operator, %{"id" => node_id}) when is_binary(node_id),
    do: "operator-task:#{node_id}:#{Map.get(operator, "id")}"

  defp default_task_id(operator, _node), do: "operator-task:#{Map.get(operator, "id")}"
end
