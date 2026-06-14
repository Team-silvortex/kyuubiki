defmodule KyuubikiWeb.Playground.AgentSessionState do
  @moduledoc false

  def state(agent) when is_map(agent) do
    case {Map.get(agent, :control_mode) || Map.get(agent, "control_mode"), Map.get(agent, :orch_session_id) || Map.get(agent, "orch_session_id")} do
      {"offline_mesh", _} -> "offline_mesh"
      {"orch_managed", session_id} when is_binary(session_id) and session_id != "" -> "orch_session_bound"
      {"orch_managed", _} -> "orch_bound_pending_session"
      _ -> "unknown"
    end
  end

  def transition(nil, next_agent, source), do: new_transition("unregistered", state(next_agent), source, "registered", next_agent)

  def transition(current, next_agent, source) do
    current_state = state(current)
    next_state = state(next_agent)

    cond do
      current_state != next_state ->
        new_transition(current_state, next_state, source, transition_reason(current_state, next_state), next_agent)

      changed_session_id?(current, next_agent) ->
        new_transition(current_state, next_state, source, "session_rebound", next_agent)

      true ->
        nil
    end
  end

  defp changed_session_id?(current, next_agent) do
    current_session_id = Map.get(current, :orch_session_id) || Map.get(current, "orch_session_id")
    next_session_id = Map.get(next_agent, :orch_session_id) || Map.get(next_agent, "orch_session_id")
    current_session_id != next_session_id
  end

  defp transition_reason("orch_bound_pending_session", "orch_session_bound"), do: "session_attached"
  defp transition_reason("orch_session_bound", "offline_mesh"), do: "mesh_detached"
  defp transition_reason("orch_bound_pending_session", "offline_mesh"), do: "mesh_detached"
  defp transition_reason("offline_mesh", "orch_bound_pending_session"), do: "orchestrator_attached"
  defp transition_reason("offline_mesh", "orch_session_bound"), do: "orchestrator_attached"
  defp transition_reason(_current_state, _next_state), do: "state_changed"

  defp new_transition(from_state, to_state, source, reason, agent) do
    %{
      "from" => from_state,
      "to" => to_state,
      "source" => source,
      "reason" => reason,
      "at" => DateTime.utc_now() |> DateTime.to_iso8601(),
      "agent_id" => Map.get(agent, :id) || Map.get(agent, "id"),
      "orch_id" => Map.get(agent, :orch_id) || Map.get(agent, "orch_id"),
      "orch_session_id" => Map.get(agent, :orch_session_id) || Map.get(agent, "orch_session_id")
    }
  end
end
