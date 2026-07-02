defmodule KyuubikiWeb.Playground.AgentRegistryPublic do
  @moduledoc false

  alias KyuubikiWeb.Playground.AgentMeshTopology
  alias KyuubikiWeb.Playground.AgentRegistry
  alias KyuubikiWeb.Playground.AgentSessionState

  def public_agent(agent, all_agents) when is_map(agent) and is_list(all_agents) do
    control_mode =
      Map.get(agent, :control_mode) || Map.get(agent, "control_mode") || "orch_managed"

    orch_id = Map.get(agent, :orch_id) || Map.get(agent, "orch_id")
    orch_session_id = Map.get(agent, :orch_session_id) || Map.get(agent, "orch_session_id")
    active_lease = Map.get(agent, :active_lease) || Map.get(agent, "active_lease")
    public_lease = public_execution_lease(active_lease, agent)

    %{
      "id" => Map.get(agent, :id) || Map.get(agent, "id"),
      "host" => Map.get(agent, :host) || Map.get(agent, "host"),
      "port" => Map.get(agent, :port) || Map.get(agent, "port"),
      "control_mode" => control_mode,
      "session_state" => AgentSessionState.state(agent),
      "orch_id" => orch_id,
      "orch_session_id" => orch_session_id,
      "cluster_id" => Map.get(agent, :cluster_id) || Map.get(agent, "cluster_id"),
      "fingerprint" => Map.get(agent, :fingerprint) || Map.get(agent, "fingerprint"),
      "role" => Map.get(agent, :role) || Map.get(agent, "role"),
      "region" => Map.get(agent, :region) || Map.get(agent, "region"),
      "zone" => Map.get(agent, :zone) || Map.get(agent, "zone"),
      "capacity" => Map.get(agent, :capacity) || Map.get(agent, "capacity"),
      "tags" => Map.get(agent, :tags) || Map.get(agent, "tags") || [],
      "methods" => Map.get(agent, :methods) || Map.get(agent, "methods") || [],
      "capabilities" => Map.get(agent, :capabilities) || Map.get(agent, "capabilities") || [],
      "health_score" => Map.get(agent, :health_score) || Map.get(agent, "health_score"),
      "watchdog" => Map.get(agent, :watchdog) || Map.get(agent, "watchdog"),
      "execution_state" => execution_state_for(public_lease),
      "active_lease" => public_lease,
      "last_session_transition" =>
        Map.get(agent, :last_session_transition) || Map.get(agent, "last_session_transition"),
      "mesh" => AgentMeshTopology.public_mesh(agent, all_agents),
      "last_seen_at" =>
        format_last_seen(Map.get(agent, :last_seen_at) || Map.get(agent, "last_seen_at")),
      "authority" => authority(control_mode, orch_id, orch_session_id, agent)
    }
  end

  def endpoint(agent, leases) do
    %{
      id: agent.id,
      host: agent.host,
      port: agent.port,
      control_mode: agent.control_mode,
      session_state: AgentSessionState.state(agent),
      orch_id: agent.orch_id,
      orch_session_id: agent.orch_session_id,
      cluster_id: agent.cluster_id,
      fingerprint: agent.fingerprint,
      role: agent.role,
      region: agent.region,
      zone: agent.zone,
      capacity: agent.capacity,
      tags: agent.tags,
      methods: agent.methods,
      capabilities: agent.capabilities,
      health_score: agent.health_score,
      active_lease: public_execution_lease(Map.get(leases, agent.id), agent),
      last_session_transition: agent[:last_session_transition],
      last_seen_at: DateTime.to_iso8601(agent.last_seen_at)
    }
  end

  def active_execution_leases(leases, agents) do
    leases
    |> Enum.map(fn {agent_id, lease} ->
      public_execution_lease(lease, Map.get(agents, agent_id))
    end)
    |> Enum.sort_by(&Map.get(&1, "agent_id"))
  end

  def stale_execution_lease_count(leases, agents) do
    leases
    |> Enum.count(fn {agent_id, lease} ->
      public_execution_lease(lease, Map.get(agents, agent_id))
      |> Map.get("is_stale", false)
    end)
  end

  def public_execution_lease(nil, _agent), do: nil

  def public_execution_lease(lease, agent) when is_map(lease) do
    claimed_at = Map.get(lease, :claimed_at) || Map.get(lease, "claimed_at")
    age_ms = execution_lease_age_ms(claimed_at)
    stale? = execution_lease_stale?(age_ms, agent)

    %{
      "lease_id" => Map.get(lease, :lease_id) || Map.get(lease, "lease_id"),
      "agent_id" => Map.get(lease, :agent_id) || Map.get(lease, "agent_id"),
      "control_mode" => Map.get(lease, :control_mode) || Map.get(lease, "control_mode"),
      "orch_id" => Map.get(lease, :orch_id) || Map.get(lease, "orch_id"),
      "orch_session_id" => Map.get(lease, :orch_session_id) || Map.get(lease, "orch_session_id"),
      "cluster_id" => Map.get(lease, :cluster_id) || Map.get(lease, "cluster_id"),
      "job_id" => Map.get(lease, :job_id) || Map.get(lease, "job_id"),
      "method" => Map.get(lease, :method) || Map.get(lease, "method"),
      "claimed_at" => format_last_seen(claimed_at),
      "age_ms" => age_ms,
      "is_stale" => stale?
    }
  end

  def format_last_seen(%DateTime{} = last_seen_at), do: DateTime.to_iso8601(last_seen_at)
  def format_last_seen(last_seen_at), do: last_seen_at

  defp authority("offline_mesh", _orch_id, _orch_session_id, _agent) do
    %{
      "control_mode" => "offline_mesh",
      "authority_mode" => "offline_mesh",
      "session_state" => "offline_mesh",
      "orchestrator_id" => nil,
      "orchestrator_session_id" => nil,
      "accepts_multi_orchestrator_binding" => false,
      "agent_library_replication" => "central_fetch"
    }
  end

  defp authority("orch_managed", orch_id, orch_session_id, agent) do
    %{
      "control_mode" => "orch_managed",
      "authority_mode" => "single_orchestrator",
      "session_state" => AgentSessionState.state(agent),
      "orchestrator_id" => orch_id,
      "orchestrator_session_id" => orch_session_id,
      "accepts_multi_orchestrator_binding" => false,
      "agent_library_replication" => "central_fetch"
    }
  end

  defp authority(_control_mode, _orch_id, _orch_session_id, _agent) do
    %{
      "control_mode" => "standalone",
      "authority_mode" => "self_directed",
      "orchestrator_id" => nil,
      "orchestrator_session_id" => nil,
      "accepts_multi_orchestrator_binding" => false,
      "agent_library_replication" => "central_fetch"
    }
  end

  defp execution_state_for(nil), do: "idle"
  defp execution_state_for(%{"is_stale" => true}), do: "lease_stale"
  defp execution_state_for(_lease), do: "leased"

  defp execution_lease_age_ms(%DateTime{} = claimed_at),
    do: max(DateTime.diff(DateTime.utc_now(), claimed_at, :millisecond), 0)

  defp execution_lease_age_ms(_claimed_at), do: nil

  defp execution_lease_stale?(age_ms, agent) do
    stale_by_age? =
      is_integer(age_ms) and age_ms > execution_lease_stale_after_ms()

    stale_by_agent? =
      case agent do
        %{last_seen_at: %DateTime{} = last_seen_at} ->
          DateTime.diff(DateTime.utc_now(), last_seen_at, :millisecond) > stale_after_ms()

        _ ->
          false
      end

    stale_by_age? or stale_by_agent?
  end

  defp stale_after_ms do
    Application.get_env(:kyuubiki_web, AgentRegistry, [])
    |> Keyword.get(:stale_after_ms, 15_000)
  end

  defp execution_lease_stale_after_ms do
    Application.get_env(:kyuubiki_web, AgentRegistry, [])
    |> Keyword.get(:execution_lease_stale_after_ms, 60_000)
  end
end
