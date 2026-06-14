defmodule KyuubikiWeb.Playground.AgentRegistry do
  @moduledoc """
  Keeps a lightweight registry of remotely managed solver agents for distributed deployments.
  """

  use GenServer

  @type agent :: %{
          required(:id) => String.t(),
          required(:host) => String.t(),
          required(:port) => pos_integer(),
          required(:control_mode) => String.t(),
          required(:orch_id) => String.t() | nil,
          required(:orch_session_id) => String.t() | nil,
          optional(:cluster_id) => String.t(),
          optional(:fingerprint) => String.t(),
          optional(:role) => String.t(),
          optional(:region) => String.t(),
          optional(:zone) => String.t(),
          optional(:capacity) => pos_integer(),
          optional(:tags) => [String.t()],
          optional(:methods) => [String.t()],
          optional(:capabilities) => [map()],
          optional(:health_score) => non_neg_integer(),
          required(:last_seen_at) => DateTime.t()
        }

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, %{}, name: __MODULE__)
  end

  @spec register(map()) :: {:ok, agent()} | {:error, term()}
  def register(attrs) when is_map(attrs) do
    GenServer.call(__MODULE__, {:register, attrs})
  end

  @spec heartbeat(String.t(), map()) :: {:ok, agent()} | {:error, term()}
  def heartbeat(agent_id, attrs \\ %{}) when is_binary(agent_id) and is_map(attrs) do
    GenServer.call(__MODULE__, {:heartbeat, agent_id, attrs})
  end

  @spec unregister(String.t()) :: :ok
  def unregister(agent_id) when is_binary(agent_id) do
    GenServer.call(__MODULE__, {:unregister, agent_id})
  end

  @spec agents() :: [agent()]
  def agents do
    GenServer.call(__MODULE__, :agents)
  end

  @spec active_endpoints() :: [map()]
  def active_endpoints do
    GenServer.call(__MODULE__, :active_endpoints)
  end

  @spec public_agents() :: [map()]
  def public_agents do
    agents()
    |> Enum.map(&public_agent/1)
  end

  @spec public_agent(agent() | map()) :: map()
  def public_agent(agent) when is_map(agent) do
    control_mode = Map.get(agent, :control_mode) || Map.get(agent, "control_mode") || "orch_managed"
    orch_id = Map.get(agent, :orch_id) || Map.get(agent, "orch_id")
    orch_session_id = Map.get(agent, :orch_session_id) || Map.get(agent, "orch_session_id")

    authority =
      case control_mode do
        "offline_mesh" ->
          %{
            "control_mode" => "offline_mesh",
            "authority_mode" => "offline_mesh",
            "orchestrator_id" => nil,
            "orchestrator_session_id" => nil,
            "accepts_multi_orchestrator_binding" => false,
            "agent_library_replication" => "central_fetch"
          }

        "orch_managed" ->
          %{
            "control_mode" => "orch_managed",
            "authority_mode" => "single_orchestrator",
            "orchestrator_id" => orch_id,
            "orchestrator_session_id" => orch_session_id,
            "accepts_multi_orchestrator_binding" => false,
            "agent_library_replication" => "central_fetch"
          }

        _ ->
          %{
            "control_mode" => "standalone",
            "authority_mode" => "self_directed",
            "orchestrator_id" => nil,
            "orchestrator_session_id" => nil,
            "accepts_multi_orchestrator_binding" => false,
            "agent_library_replication" => "central_fetch"
          }
      end

    %{
      "id" => Map.get(agent, :id) || Map.get(agent, "id"),
      "host" => Map.get(agent, :host) || Map.get(agent, "host"),
      "port" => Map.get(agent, :port) || Map.get(agent, "port"),
      "control_mode" => control_mode,
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
      "last_seen_at" => format_last_seen(Map.get(agent, :last_seen_at) || Map.get(agent, "last_seen_at")),
      "authority" => authority
    }
  end

  @spec status_snapshot() :: map()
  def status_snapshot do
    GenServer.call(__MODULE__, :status_snapshot)
  end

  @impl true
  def init(_opts) do
    {:ok, %{agents: %{}}}
  end

  @impl true
  def handle_call({:register, attrs}, _from, state) do
    current = Map.get(state.agents, agent_id_from_attrs(attrs))

    with {:ok, agent} <- build_agent(attrs),
         :ok <- validate_control_transition(current, agent) do
      agents = Map.put(state.agents, agent.id, agent)
      {:reply, {:ok, agent}, %{state | agents: agents}}
    else
      {:error, reason} -> {:reply, {:error, reason}, state}
    end
  end

  def handle_call({:heartbeat, agent_id, attrs}, _from, state) do
    current = Map.get(state.agents, agent_id, %{id: agent_id})

    with {:ok, agent} <- build_agent(merge_heartbeat_attrs(current, attrs, agent_id)),
         :ok <- validate_control_transition(Map.get(state.agents, agent_id), agent) do
      agents = Map.put(state.agents, agent.id, agent)
      {:reply, {:ok, agent}, %{state | agents: agents}}
    else
      {:error, reason} -> {:reply, {:error, reason}, state}
    end
  end

  def handle_call({:unregister, agent_id}, _from, state) do
    {:reply, :ok, %{state | agents: Map.delete(state.agents, agent_id)}}
  end

  def handle_call(:agents, _from, state) do
    {:reply, state.agents |> Map.values() |> sort_agents(), state}
  end

  def handle_call(:active_endpoints, _from, state) do
    {:reply, active_agents(state.agents) |> Enum.map(&to_endpoint/1), state}
  end

  def handle_call(:status_snapshot, _from, state) do
    total = map_size(state.agents)
    active = length(active_agents(state.agents))

    {:reply,
     %{
       total_agents: total,
       active_agents: active,
       stale_agents: max(total - active, 0),
       stale_after_ms: stale_after_ms()
     }, state}
  end

  defp build_agent(attrs) do
    with {:ok, id} <- fetch_string(attrs, "id"),
         {:ok, host} <- fetch_string(attrs, "host"),
         {:ok, port} <- fetch_port(attrs, "port"),
         {:ok, control_mode, orch_id, orch_session_id} <- build_control_binding(attrs) do
      now = DateTime.utc_now()

      {:ok,
       %{
         id: id,
         host: host,
         port: port,
         control_mode: control_mode,
         orch_id: orch_id,
         orch_session_id: orch_session_id,
         cluster_id: optional_string(attrs, "cluster_id"),
         fingerprint: optional_string(attrs, "fingerprint"),
         role: optional_string(attrs, "role"),
         region: optional_string(attrs, "region"),
         zone: optional_string(attrs, "zone"),
         capacity: optional_integer(attrs, "capacity"),
         tags: optional_tags(attrs, "tags"),
         methods: optional_methods(attrs, "methods"),
         capabilities: optional_capabilities(attrs, "capabilities"),
         health_score: optional_health_score(attrs, "health_score"),
         last_seen_at: now
       }}
    end
  end

  defp build_control_binding(attrs) do
    control_mode = optional_string(attrs, "control_mode") || "orch_managed"
    orch_id = optional_string(attrs, "orch_id") || default_orch_id(control_mode)
    orch_session_id = optional_string(attrs, "orch_session_id")

    case control_mode do
      "orch_managed" ->
        if is_binary(orch_id) and orch_id != "" do
          {:ok, control_mode, orch_id, orch_session_id}
        else
          {:error, {:invalid_agent_control, :orch_id_required}}
        end

      "offline_mesh" ->
        if orch_id == nil and orch_session_id == nil do
          {:ok, control_mode, nil, nil}
        else
          {:error, {:invalid_agent_control, :offline_mesh_cannot_bind_orchestra}}
        end

      _ ->
        {:error, {:invalid_agent_control, :control_mode}}
    end
  end

  defp default_orch_id("orch_managed"), do: "orchestra/default"
  defp default_orch_id(_control_mode), do: nil

  defp validate_control_transition(nil, _agent), do: :ok

  defp validate_control_transition(current, next_agent) do
    current_binding = control_binding(current)
    next_binding = control_binding(next_agent)

    cond do
      current_binding.control_mode == "offline_mesh" and
          next_binding.control_mode == "offline_mesh" ->
        :ok

      current_binding.control_mode == "orch_managed" and
        next_binding.control_mode == "orch_managed" and
        current_binding.orch_id == next_binding.orch_id and
          compatible_orch_session?(current_binding.orch_session_id, next_binding.orch_session_id) ->
        :ok

      true ->
        {:error,
         {:agent_control_conflict,
          %{
            agent_id: next_agent.id,
            current: current_binding,
            attempted: next_binding
          }}}
    end
  end

  defp compatible_orch_session?(current_session_id, next_session_id)

  defp compatible_orch_session?(_current_session_id, nil), do: true
  defp compatible_orch_session?(nil, _next_session_id), do: true
  defp compatible_orch_session?(session_id, session_id), do: true
  defp compatible_orch_session?(_current_session_id, _next_session_id), do: false

  defp control_binding(agent) do
    %{
      control_mode:
        Map.get(agent, :control_mode) || Map.get(agent, "control_mode") || "orch_managed",
      orch_id: Map.get(agent, :orch_id) || Map.get(agent, "orch_id") || "orchestra/default",
      orch_session_id: Map.get(agent, :orch_session_id) || Map.get(agent, "orch_session_id")
    }
  end

  defp agent_id_from_attrs(attrs) do
    Map.get(attrs, "id") || Map.get(attrs, :id)
  end

  defp merge_heartbeat_attrs(current, attrs, agent_id) do
    attrs =
      case Map.get(attrs, "control_mode") || Map.get(attrs, :control_mode) do
        "offline_mesh" ->
          attrs
          |> Map.put("orch_id", nil)
          |> Map.put("orch_session_id", nil)
          |> Map.put(:orch_id, nil)
          |> Map.put(:orch_session_id, nil)

        _ ->
          attrs
      end

    current
    |> Map.merge(attrs |> Map.put("id", agent_id))
  end

  defp fetch_string(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_binary(value) and value != "" -> {:ok, value}
      _ -> {:error, {:invalid_agent_field, key}}
    end
  end

  defp fetch_port(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_integer(value) and value > 0 ->
        {:ok, value}

      value when is_binary(value) ->
        case Integer.parse(value) do
          {port, ""} when port > 0 -> {:ok, port}
          _ -> {:error, {:invalid_agent_field, key}}
        end

      _ ->
        {:error, {:invalid_agent_field, key}}
    end
  end

  defp optional_string(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_binary(value) and value != "" -> value
      _ -> nil
    end
  end

  defp optional_integer(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_integer(value) and value > 0 ->
        value

      value when is_binary(value) ->
        case Integer.parse(value) do
          {integer, ""} when integer > 0 -> integer
          _ -> nil
        end

      _ ->
        nil
    end
  end

  defp optional_tags(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      values when is_list(values) -> Enum.filter(values, &is_binary/1)
      _ -> []
    end
  end

  defp optional_methods(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      values when is_list(values) -> values |> Enum.filter(&is_binary/1) |> Enum.uniq()
      _ -> []
    end
  end

  defp optional_capabilities(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      values when is_list(values) ->
        values
        |> Enum.map(&normalize_capability/1)
        |> Enum.reject(&is_nil/1)

      _ ->
        []
    end
  end

  defp normalize_capability(%{} = capability) do
    id = capability["id"] || capability[:id]
    role = capability["role"] || capability[:role]

    if is_binary(id) and id != "" and is_binary(role) and role != "" do
      %{
        id: id,
        role: role,
        methods:
          capability
          |> Map.get("methods", capability[:methods] || [])
          |> List.wrap()
          |> Enum.filter(&is_binary/1)
          |> Enum.uniq(),
        tags:
          capability
          |> Map.get("tags", capability[:tags] || [])
          |> List.wrap()
          |> Enum.filter(&is_binary/1)
          |> Enum.uniq()
      }
    end
  end

  defp normalize_capability(_capability), do: nil

  defp optional_health_score(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_integer(value) and value >= 0 and value <= 100 ->
        value

      value when is_binary(value) ->
        case Integer.parse(value) do
          {score, ""} when score >= 0 and score <= 100 -> score
          _ -> nil
        end

      _ ->
        nil
    end
  end

  defp active_agents(agents) do
    limit_ms = stale_after_ms()
    now = DateTime.utc_now()

    agents
    |> Map.values()
    |> Enum.filter(fn agent ->
      DateTime.diff(now, agent.last_seen_at, :millisecond) <= limit_ms
    end)
    |> sort_agents()
  end

  defp sort_agents(agents) do
    Enum.sort_by(agents, & &1.id)
  end

  defp to_endpoint(agent) do
    %{
      id: agent.id,
      host: agent.host,
      port: agent.port,
      control_mode: agent.control_mode,
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
      last_seen_at: DateTime.to_iso8601(agent.last_seen_at)
    }
  end

  defp stale_after_ms do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:stale_after_ms, 15_000)
  end

  defp format_last_seen(%DateTime{} = last_seen_at), do: DateTime.to_iso8601(last_seen_at)
  defp format_last_seen(last_seen_at), do: last_seen_at
end
