defmodule KyuubikiWeb.Orchestra.OperatorExecutionProgram do
  @moduledoc """
  Builds the language-neutral operator execution program consumed by agents.
  """

  @schema_version "kyuubiki.operator-execution-program/v1"

  @spec schema_version() :: String.t()
  def schema_version, do: @schema_version

  @spec build(map(), map(), map()) :: map()
  def build(operator, execution, node \\ %{})

  def build(operator, execution, node)
      when is_map(operator) and is_map(execution) and is_map(node) do
    operator_id = Map.get(operator, "id")
    kind = Map.get(operator, "kind")

    %{
      "schema_version" => @schema_version,
      "program_id" => operator_id,
      "program_family" => Map.get(operator, "family"),
      "program_kind" => kind,
      "operator_category_id" => Map.get(operator, "operator_category_id"),
      "package_ref" => Map.get(execution, "package_ref"),
      "package_version" => Map.get(execution, "package_version", "library-managed"),
      "package_integrity" => Map.get(execution, "integrity"),
      "runtime_protocol" => runtime_protocol(kind),
      "abi" => execution_abi(kind),
      "entrypoint" => entrypoint(operator_id, kind),
      "bindings" => bindings(),
      "node_binding" => node_binding(node)
    }
  end

  def build(_operator, _execution, _node), do: %{"schema_version" => @schema_version}

  defp runtime_protocol("solver"), do: "kyuubiki.solver-rpc/v1"
  defp runtime_protocol(_kind), do: "kyuubiki.operator-execution/v1"

  defp execution_abi("solver") do
    %{
      "kind" => "solver_rpc",
      "input_encoding" => "json",
      "output_encoding" => "json"
    }
  end

  defp execution_abi(_kind) do
    %{
      "kind" => "operator_task",
      "input_encoding" => "json",
      "output_encoding" => "json"
    }
  end

  defp entrypoint(operator_id, "solver") when is_binary(operator_id) do
    %{
      "kind" => "solver_method",
      "name" =>
        operator_id |> String.replace_prefix("solve.", "solve_") |> String.replace(".", "_")
    }
  end

  defp entrypoint(operator_id, kind) do
    %{
      "kind" => "operator_id",
      "name" => operator_id,
      "operator_kind" => kind
    }
  end

  defp bindings do
    %{
      "input_artifact" => "task.input_artifact",
      "config" => "task.config",
      "output_artifact" => "task.output_artifact"
    }
  end

  defp node_binding(%{"id" => node_id} = node) do
    %{
      "node_id" => node_id,
      "input_ports" => Map.get(node, "inputs", []),
      "output_ports" => Map.get(node, "outputs", [])
    }
  end

  defp node_binding(_node), do: %{"node_id" => nil, "input_ports" => [], "output_ports" => []}
end
