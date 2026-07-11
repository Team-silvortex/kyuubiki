defmodule KyuubikiWeb.Orchestra.OperatorTaskReadiness do
  @moduledoc """
  Normalizes agent-side operator TaskIR readiness payloads.

  Rust agents now report `execution_readiness` directly. This module keeps
  Elixir callers compatible with older agents that only returned legacy
  `blocked_stage`, `next_stage`, and `operator_package_runtime_ready` fields.
  """

  @blocked_runtime_not_attached "operator_package_runtime_not_yet_attached"
  @fetch_stage "fetch_package"

  @spec normalize_agent_result(map()) :: map()
  def normalize_agent_result(%{"execution_readiness" => readiness} = result)
      when is_map(readiness),
      do: result

  def normalize_agent_result(result) when is_map(result) do
    Map.put(result, "execution_readiness", readiness_from_legacy_result(result))
  end

  @spec local_executed() :: map()
  def local_executed do
    %{
      "status" => "executed",
      "requested_mode" => "execute",
      "ready_to_dispatch" => true,
      "current_stage" => "serialize_result",
      "blocking_stage" => nil,
      "blocking_reason" => nil,
      "blocking_owner" => nil,
      "required_action" => nil
    }
  end

  @spec local_failed(term()) :: map()
  def local_failed(reason) do
    %{
      "status" => "blocked",
      "requested_mode" => "execute",
      "ready_to_dispatch" => false,
      "current_stage" => "local_execute",
      "blocking_stage" => "local_execute",
      "blocking_reason" => inspect(reason),
      "blocking_owner" => "operator_task_executor",
      "required_action" => "fix_task_ir_or_operator_contract"
    }
  end

  @spec readiness_from_legacy_result(map()) :: map()
  def readiness_from_legacy_result(%{"operator_task_ir_status" => "executed"}) do
    local_executed()
  end

  def readiness_from_legacy_result(%{"operator_package_runtime_ready" => true} = result) do
    %{
      "status" => "ready_for_package_resolution",
      "requested_mode" => Map.get(result, "requested_mode", "preflight"),
      "ready_to_dispatch" => false,
      "current_stage" => Map.get(result, "next_stage", @fetch_stage),
      "blocking_stage" => nil,
      "blocking_reason" => nil,
      "blocking_owner" => nil,
      "required_action" => "resolve_fetch_verify_and_activate_package"
    }
  end

  def readiness_from_legacy_result(result) when is_map(result) do
    blocked_stage = Map.get(result, "blocked_stage", @fetch_stage)

    %{
      "status" => "blocked",
      "requested_mode" => Map.get(result, "requested_mode", "preflight"),
      "ready_to_dispatch" => false,
      "current_stage" => blocked_stage,
      "blocking_stage" => blocked_stage,
      "blocking_reason" =>
        Map.get(result, "execution_runtime_status", @blocked_runtime_not_attached),
      "blocking_owner" => "operator_package_runtime",
      "required_action" => "attach_operator_package_runtime"
    }
  end
end
