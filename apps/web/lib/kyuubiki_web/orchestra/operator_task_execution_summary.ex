defmodule KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary do
  @moduledoc """
  Builds the agent-facing execution summary for operator TaskIR payloads.
  """

  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  @solver_protocol "kyuubiki.solver-rpc/v1"
  @operator_protocol "kyuubiki.operator-execution/v1"
  @program_schema_version "kyuubiki.operator-execution-program/v1"
  @operator_kinds ["transform", "extract", "export", "workflow_bridge"]

  @spec validate_digest(map()) :: :ok | {:error, term()}
  def validate_digest(%{"integrity" => %{"task_digest" => task_digest}} = task)
      when is_binary(task_digest) and task_digest != "" do
    actual = OperatorTaskIR.compute_task_digest(task)

    if actual == task_digest do
      :ok
    else
      {:error, {:operator_task_digest_mismatch, %{expected: task_digest, actual: actual}}}
    end
  end

  def validate_digest(_task), do: {:error, :missing_operator_task_digest}

  @spec build(map()) :: {:ok, map()} | {:error, term()}
  def build(task) when is_map(task) do
    with :ok <- execution_program_present?(task),
         :ok <- validate_program_schema(task),
         {:ok, operator_id} <- required_string(task, ["operator", "id"]),
         {:ok, operator_kind} <- required_string(task, ["operator", "kind"]),
         {:ok, program_id} <- required_string(task, ["execution_program", "program_id"]),
         {:ok, program_kind} <- required_string(task, ["execution_program", "program_kind"]),
         {:ok, runtime_protocol} <-
           required_string(task, ["execution_program", "runtime_protocol"]),
         {:ok, abi_kind} <- required_string(task, ["execution_program", "abi", "kind"]),
         {:ok, entrypoint_kind} <-
           required_string(task, ["execution_program", "entrypoint", "kind"]),
         {:ok, entrypoint_name} <-
           required_string(task, ["execution_program", "entrypoint", "name"]),
         :ok <- validate_program_match(operator_id, operator_kind, program_id, program_kind),
         :ok <- validate_mirror_fields(task, operator_kind),
         :ok <- validate_execution_abi(program_kind, runtime_protocol, abi_kind, entrypoint_kind),
         :ok <- validate_entrypoint(program_kind, operator_id, entrypoint_name) do
      {:ok,
       %{
         "status" => "verified",
         "task_digest" => OperatorTaskIR.compute_task_digest(task),
         "task_id" => Map.get(task, "task_id", ""),
         "operator_id" => operator_id,
         "operator_kind" => operator_kind,
         "program_id" => program_id,
         "program_kind" => program_kind,
         "runtime_protocol" => runtime_protocol,
         "abi_kind" => abi_kind,
         "entrypoint_kind" => entrypoint_kind,
         "entrypoint_name" => entrypoint_name,
         "package_ref" => get_in(task, ["execution_program", "package_ref"]),
         "package_version" => get_in(task, ["execution_program", "package_version"]),
         "authority_mode" => get_in(task, ["runtime_hints", "authority_mode"]),
         "execution_mode" => get_in(task, ["runtime_hints", "execution_mode"]),
         "cache_scope" => get_in(task, ["runtime_hints", "cache_scope"]),
         "agent_fetchable" => get_in(task, ["runtime_hints", "agent_fetchable"])
       }}
    end
  end

  def build(_task), do: {:error, :invalid_operator_task_ir}

  defp execution_program_present?(%{"execution_program" => program}) when is_map(program), do: :ok
  defp execution_program_present?(_task), do: {:error, :missing_operator_execution_program}

  defp validate_program_schema(%{
         "execution_program" => %{"schema_version" => @program_schema_version}
       }),
       do: :ok

  defp validate_program_schema(_task), do: {:error, :invalid_operator_execution_program}

  defp validate_program_match(operator_id, operator_kind, operator_id, operator_kind), do: :ok

  defp validate_program_match(_operator_id, _operator_kind, _program_id, _program_kind),
    do: {:error, :operator_task_program_mismatch}

  defp validate_mirror_fields(task, operator_kind) do
    with :ok <-
           validate_mirror(
             task,
             ["operator", "kind"],
             operator_kind,
             ["execution_program", "entrypoint", "operator_kind"]
           ),
         :ok <-
           validate_mirror(
             task,
             ["operator", "kind"],
             operator_kind,
             ["runtime_hints", "operator_kind"]
           ),
         :ok <-
           validate_optional_mirror(
             task,
             ["execution_program", "package_ref"],
             ["operator", "execution", "package_ref"]
           ),
         :ok <-
           validate_optional_mirror(
             task,
             ["execution_program", "package_ref"],
             ["runtime_hints", "package_ref"]
           ),
         :ok <-
           validate_optional_mirror(
             task,
             ["execution_program", "package_version"],
             ["runtime_hints", "package_version"]
           ) do
      :ok
    end
  end

  defp validate_mirror(task, source_path, source_value, mirror_path) do
    case get_in(task, mirror_path) do
      nil -> :ok
      ^source_value -> :ok
      _ -> mirror_mismatch(source_path, mirror_path)
    end
  end

  defp validate_optional_mirror(task, source_path, mirror_path) do
    source_value = get_in(task, source_path)
    mirror_value = get_in(task, mirror_path)

    if is_nil(source_value) or is_nil(mirror_value) or source_value == mirror_value do
      :ok
    else
      mirror_mismatch(source_path, mirror_path)
    end
  end

  defp mirror_mismatch(source_path, mirror_path) do
    {:error,
     {:operator_task_mirror_mismatch,
      %{
        source: Enum.join(source_path, "."),
        mirror: Enum.join(mirror_path, ".")
      }}}
  end

  defp validate_execution_abi("solver", @solver_protocol, "solver_rpc", "solver_method"), do: :ok

  defp validate_execution_abi(kind, @operator_protocol, "operator_task", "operator_id")
       when kind in @operator_kinds,
       do: :ok

  defp validate_execution_abi(_kind, _protocol, _abi, _entrypoint),
    do: {:error, :operator_task_execution_abi_mismatch}

  defp validate_entrypoint("solver", _operator_id, _entrypoint_name), do: :ok
  defp validate_entrypoint(_kind, operator_id, operator_id), do: :ok

  defp validate_entrypoint(_kind, _operator_id, _entrypoint_name),
    do: {:error, :operator_task_entrypoint_mismatch}

  defp required_string(task, path) do
    case get_in(task, path) do
      value when is_binary(value) and value != "" -> {:ok, value}
      _ -> {:error, {:missing_operator_task_field, Enum.join(path, ".")}}
    end
  end
end
