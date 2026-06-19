defmodule KyuubikiWeb.Playground.AgentMeshTopology do
  @moduledoc false

  def public_mesh(agent, agents) when is_map(agent) and is_list(agents) do
    case control_mode(agent) do
      "offline_mesh" ->
        cluster_id = cluster_id(agent)
        peers = mesh_peers(agent, agents)

        %{
          "cluster_id" => cluster_id,
          "peer_group_id" => peer_group_id(agent),
          "peer_count" => length(peers),
          "topology_role" => topology_role(agent),
          "relay_candidate" => relay_candidate?(agent),
          "peers" => peers
        }

      _ ->
        nil
    end
  end

  def summarize_mesh_topology(agents) when is_list(agents) do
    managed_agents = Enum.filter(agents, &(control_mode(&1) == "orch_managed"))
    offline_mesh_agents = Enum.filter(agents, &(control_mode(&1) == "offline_mesh"))

    %{
      managed_orchestrators: summarize_managed_orchestrators(managed_agents),
      offline_mesh: %{
        agent_count: length(offline_mesh_agents),
        agent_ids: offline_mesh_agents |> Enum.map(&agent_id/1) |> Enum.sort(),
        clustered_meshes: summarize_offline_mesh_clusters(offline_mesh_agents),
        unclustered_agent_ids: summarize_unclustered_offline_mesh_agents(offline_mesh_agents)
      }
    }
  end

  defp summarize_managed_orchestrators(agents) do
    agents
    |> Enum.group_by(&orch_id/1)
    |> Enum.map(fn {group_orch_id, grouped_agents} ->
      %{
        orch_id: group_orch_id,
        agent_count: length(grouped_agents),
        agent_ids: grouped_agents |> Enum.map(&agent_id/1) |> Enum.sort(),
        session_ids:
          grouped_agents
          |> Enum.map(&orch_session_id/1)
          |> Enum.reject(&is_nil/1)
          |> Enum.uniq()
          |> Enum.sort()
      }
    end)
    |> Enum.sort_by(&(&1.orch_id || ""))
  end

  defp summarize_offline_mesh_clusters(agents) do
    agents
    |> Enum.reject(&(is_nil(cluster_id(&1)) or cluster_id(&1) == ""))
    |> Enum.group_by(&cluster_id/1)
    |> Enum.map(fn {group_cluster_id, grouped_agents} ->
      %{
        cluster_id: group_cluster_id,
        agent_count: length(grouped_agents),
        agent_ids: grouped_agents |> Enum.map(&agent_id/1) |> Enum.sort(),
        relay_candidate_ids:
          grouped_agents
          |> Enum.filter(&relay_candidate?/1)
          |> Enum.map(&agent_id/1)
          |> Enum.sort()
      }
    end)
    |> Enum.sort_by(& &1.cluster_id)
  end

  defp summarize_unclustered_offline_mesh_agents(agents) do
    agents
    |> Enum.filter(&(is_nil(cluster_id(&1)) or cluster_id(&1) == ""))
    |> Enum.map(&agent_id/1)
    |> Enum.sort()
  end

  defp mesh_peers(agent, agents) do
    agent
    |> mesh_peer_candidates(agents)
    |> Enum.map(fn peer ->
      %{
        "id" => agent_id(peer),
        "address" => "#{host(peer)}:#{port(peer)}",
        "cluster_id" => cluster_id(peer),
        "health_score" => health_score(peer),
        "status" => peer_status(peer),
        "topology_role" => topology_role(peer)
      }
    end)
    |> Enum.sort_by(fn peer -> {peer["status"], peer["id"]} end)
  end

  defp mesh_peer_candidates(agent, agents) do
    source_cluster_id = cluster_id(agent)
    source_id = agent_id(agent)

    Enum.filter(agents, fn peer ->
      peer_id = agent_id(peer)

      control_mode(peer) == "offline_mesh" and peer_id != source_id and
        mesh_peer_match?(source_cluster_id, cluster_id(peer))
    end)
  end

  defp mesh_peer_match?(source_cluster_id, peer_cluster_id)
       when is_binary(source_cluster_id) and source_cluster_id != "" do
    peer_cluster_id == source_cluster_id
  end

  defp mesh_peer_match?(_source_cluster_id, _peer_cluster_id), do: true

  defp peer_status(agent) do
    score = health_score(agent)

    cond do
      score >= 85 -> "healthy"
      score >= 50 -> "degraded"
      true -> "unknown"
    end
  end

  defp relay_candidate?(agent) do
    tag_set =
      agent
      |> tags()
      |> MapSet.new()

    MapSet.member?(tag_set, "relay") or
      (capacity(agent) || 0) > 1 or
      role(agent) == "relay"
  end

  defp topology_role(agent) do
    if relay_candidate?(agent), do: "relay_candidate", else: "peer"
  end

  defp peer_group_id(agent) do
    case cluster_id(agent) do
      cluster when is_binary(cluster) and cluster != "" -> "offline_mesh:#{cluster}"
      _ -> "offline_mesh:unclustered:#{agent_id(agent)}"
    end
  end

  defp agent_id(agent), do: Map.get(agent, :id) || Map.get(agent, "id")
  defp control_mode(agent), do: Map.get(agent, :control_mode) || Map.get(agent, "control_mode")
  defp cluster_id(agent), do: Map.get(agent, :cluster_id) || Map.get(agent, "cluster_id")
  defp orch_id(agent), do: Map.get(agent, :orch_id) || Map.get(agent, "orch_id")

  defp orch_session_id(agent),
    do: Map.get(agent, :orch_session_id) || Map.get(agent, "orch_session_id")

  defp host(agent), do: Map.get(agent, :host) || Map.get(agent, "host")
  defp port(agent), do: Map.get(agent, :port) || Map.get(agent, "port")
  defp capacity(agent), do: Map.get(agent, :capacity) || Map.get(agent, "capacity")
  defp role(agent), do: Map.get(agent, :role) || Map.get(agent, "role")

  defp health_score(agent),
    do: Map.get(agent, :health_score) || Map.get(agent, "health_score") || 0

  defp tags(agent), do: Map.get(agent, :tags) || Map.get(agent, "tags") || []
end
