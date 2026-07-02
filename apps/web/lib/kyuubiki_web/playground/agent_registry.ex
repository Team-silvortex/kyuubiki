defmodule KyuubikiWeb.Playground.AgentRegistry do
  @moduledoc """
  Keeps a lightweight registry of remotely managed solver agents for distributed deployments.
  """

  use GenServer

  alias KyuubikiWeb.Playground.AgentMeshTopology
  alias KyuubikiWeb.Playground.AgentRegistryPublic
  alias KyuubikiWeb.Playground.AgentSessionState

  @type execution_lease :: %{
          required(:lease_id) => String.t(),
          required(:agent_id) => String.t(),
          optional(:control_mode) => String.t(),
          optional(:orch_id) => String.t(),
          optional(:orch_session_id) => String.t(),
          optional(:cluster_id) => String.t(),
          optional(:job_id) => String.t(),
          optional(:method) => String.t(),
          required(:claimed_at) => DateTime.t()
        }

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
          optional(:watchdog) => map(),
          optional(:last_session_transition) => map(),
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

  @spec claim_execution(String.t(), map()) :: {:ok, execution_lease()} | {:error, term()}
  def claim_execution(agent_id, attrs) when is_binary(agent_id) and is_map(attrs) do
    GenServer.call(__MODULE__, {:claim_execution, agent_id, attrs})
  end

  @spec release_execution(String.t(), String.t()) :: :ok
  def release_execution(agent_id, lease_id) when is_binary(agent_id) and is_binary(lease_id) do
    GenServer.call(__MODULE__, {:release_execution, agent_id, lease_id})
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
    GenServer.call(__MODULE__, :public_agents)
  end

  @spec public_agent(agent() | map()) :: map()
  def public_agent(agent) when is_map(agent) do
    AgentRegistryPublic.public_agent(agent, agents())
  end

  @spec status_snapshot() :: map()
  def status_snapshot do
    GenServer.call(__MODULE__, :status_snapshot)
  end

  @impl true
  def init(_opts) do
    {:ok, %{agents: %{}, leases: %{}}}
  end

  @impl true
  def handle_call({:register, attrs}, _from, state) do
    current = Map.get(state.agents, agent_id_from_attrs(attrs))

    with {:ok, agent} <- build_agent(attrs),
         :ok <- validate_entity_uniqueness(state.agents, agent),
         :ok <- validate_control_transition(current, agent) do
      agent = attach_session_transition(current, agent, "register")
      agents = Map.put(state.agents, agent.id, agent)
      {:reply, {:ok, agent}, %{state | agents: agents}}
    else
      {:error, reason} -> {:reply, {:error, reason}, state}
    end
  end

  def handle_call({:heartbeat, agent_id, attrs}, _from, state) do
    current = Map.get(state.agents, agent_id, %{id: agent_id})

    with {:ok, agent} <- build_agent(merge_heartbeat_attrs(current, attrs, agent_id)),
         :ok <- validate_entity_uniqueness(state.agents, agent),
         :ok <- validate_control_transition(Map.get(state.agents, agent_id), agent) do
      agent = attach_session_transition(Map.get(state.agents, agent_id), agent, "heartbeat")
      agents = Map.put(state.agents, agent.id, agent)
      {:reply, {:ok, agent}, %{state | agents: agents}}
    else
      {:error, reason} -> {:reply, {:error, reason}, state}
    end
  end

  def handle_call({:unregister, agent_id}, _from, state) do
    {:reply, :ok,
     %{
       state
       | agents: Map.delete(state.agents, agent_id),
         leases: Map.delete(state.leases, agent_id)
     }}
  end

  def handle_call({:claim_execution, agent_id, attrs}, _from, state) do
    with %{id: ^agent_id} = agent <- Map.get(state.agents, agent_id),
         {:ok, lease} <- build_execution_lease(agent, attrs),
         :ok <- validate_execution_claim(Map.get(state.leases, agent_id), lease) do
      {:reply, {:ok, lease}, %{state | leases: Map.put(state.leases, agent_id, lease)}}
    else
      nil ->
        {:reply, {:error, {:agent_not_found, agent_id}}, state}

      {:error, reason} ->
        {:reply, {:error, reason}, state}
    end
  end

  def handle_call({:release_execution, agent_id, lease_id}, _from, state) do
    leases =
      case Map.get(state.leases, agent_id) do
        %{lease_id: ^lease_id} -> Map.delete(state.leases, agent_id)
        _ -> state.leases
      end

    {:reply, :ok, %{state | leases: leases}}
  end

  def handle_call(:agents, _from, state) do
    {:reply, state.agents |> Map.values() |> sort_agents(), state}
  end

  def handle_call(:public_agents, _from, state) do
    public_agents =
      state.agents
      |> Map.values()
      |> sort_agents()
      |> Enum.map(&attach_active_lease(&1, state.leases))
      |> then(fn agents ->
        Enum.map(agents, &AgentRegistryPublic.public_agent(&1, agents))
      end)

    {:reply, public_agents, state}
  end

  def handle_call(:active_endpoints, _from, state) do
    {:reply,
     active_agents(state.agents) |> Enum.map(&AgentRegistryPublic.endpoint(&1, state.leases)),
     state}
  end

  def handle_call(:status_snapshot, _from, state) do
    total = map_size(state.agents)
    agents = state.agents |> Map.values() |> sort_agents()
    active_agents = active_agents(state.agents)

    {:reply,
     %{
       total_agents: total,
       active_agents: length(active_agents),
       stale_agents: max(total - length(active_agents), 0),
       stale_after_ms: stale_after_ms(),
       active_execution_lease_count: map_size(state.leases),
       stale_execution_lease_count:
         AgentRegistryPublic.stale_execution_lease_count(state.leases, state.agents),
       control_modes: summarize_control_modes(agents),
       session_states: summarize_session_states(agents),
       active_execution_leases:
         AgentRegistryPublic.active_execution_leases(state.leases, state.agents),
       recent_session_transitions: summarize_recent_session_transitions(agents),
       authority_groups: summarize_authority_groups(agents),
       mesh_topology: AgentMeshTopology.summarize_mesh_topology(agents)
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
         watchdog: optional_map(attrs, "watchdog"),
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

  defp build_execution_lease(agent, attrs) do
    with {:ok, lease_id} <- fetch_string(attrs, "lease_id") do
      {:ok,
       %{
         lease_id: lease_id,
         agent_id: agent.id,
         control_mode: optional_string(attrs, "control_mode") || agent.control_mode,
         orch_id: optional_string(attrs, "orch_id") || agent.orch_id,
         orch_session_id: optional_string(attrs, "orch_session_id") || agent.orch_session_id,
         cluster_id: optional_string(attrs, "cluster_id") || agent.cluster_id,
         job_id: optional_string(attrs, "job_id"),
         method: optional_string(attrs, "method"),
         claimed_at: DateTime.utc_now()
       }}
    end
  end

  defp validate_execution_claim(nil, _lease), do: :ok
  defp validate_execution_claim(%{lease_id: lease_id}, %{lease_id: lease_id}), do: :ok

  defp validate_execution_claim(current, next_lease) do
    {:error,
     {:agent_execution_conflict,
      %{
        agent_id: next_lease.agent_id,
        current: AgentRegistryPublic.public_execution_lease(current, nil),
        attempted: AgentRegistryPublic.public_execution_lease(next_lease, nil)
      }}}
  end

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

  defp validate_entity_uniqueness(agents, next_agent) do
    case Enum.find(Map.values(agents), &entity_conflict?(&1, next_agent)) do
      nil ->
        :ok

      current ->
        {:error,
         {:agent_identity_conflict,
          %{
            agent_id: next_agent.id,
            current_agent_id: current.id,
            entity_key: entity_key(current),
            current: control_binding(current),
            attempted: control_binding(next_agent)
          }}}
    end
  end

  defp entity_conflict?(current, next_agent) do
    current.id != next_agent.id and entity_key(current) != nil and
      entity_key(current) == entity_key(next_agent)
  end

  defp entity_key(agent) do
    fingerprint = Map.get(agent, :fingerprint) || Map.get(agent, "fingerprint")
    host = Map.get(agent, :host) || Map.get(agent, "host")
    port = Map.get(agent, :port) || Map.get(agent, "port")

    cond do
      is_binary(fingerprint) and fingerprint != "" ->
        {:fingerprint, fingerprint}

      is_binary(host) and host != "" and is_integer(port) and port > 0 ->
        {:endpoint, host, port}

      true ->
        nil
    end
  end

  defp control_binding(agent) do
    %{
      control_mode:
        Map.get(agent, :control_mode) || Map.get(agent, "control_mode") || "orch_managed",
      session_state: AgentSessionState.state(agent),
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

  defp optional_map(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_map(value) -> value
      _ -> nil
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

  defp summarize_control_modes(agents) do
    Enum.reduce(agents, %{orch_managed: 0, offline_mesh: 0}, fn agent, acc ->
      case agent.control_mode do
        "offline_mesh" -> Map.update!(acc, :offline_mesh, &(&1 + 1))
        _ -> Map.update!(acc, :orch_managed, &(&1 + 1))
      end
    end)
  end

  defp summarize_session_states(agents),
    do: Enum.frequencies_by(agents, &AgentSessionState.state/1)

  defp summarize_recent_session_transitions(agents) do
    agents
    |> Enum.map(& &1[:last_session_transition])
    |> Enum.reject(&is_nil/1)
    |> Enum.sort_by(&Map.get(&1, "at"), :desc)
  end

  defp summarize_authority_groups(agents) do
    agents
    |> Enum.group_by(&authority_group_key/1)
    |> Enum.map(fn {{control_mode, orch_id, orch_session_id}, grouped_agents} ->
      %{
        control_mode: control_mode,
        orch_id: orch_id,
        orch_session_id: orch_session_id,
        agent_count: length(grouped_agents),
        agent_ids: grouped_agents |> Enum.map(& &1.id) |> Enum.sort()
      }
    end)
    |> Enum.sort_by(fn group ->
      {group.control_mode, group.orch_id || "", group.orch_session_id || ""}
    end)
  end

  defp authority_group_key(agent), do: {agent.control_mode, agent.orch_id, agent.orch_session_id}
  defp sort_agents(agents), do: Enum.sort_by(agents, & &1.id)

  defp attach_active_lease(agent, leases) do
    Map.put(agent, :active_lease, Map.get(leases, agent.id))
  end

  defp stale_after_ms do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:stale_after_ms, 15_000)
  end

  defp attach_session_transition(current, agent, source),
    do:
      Map.put(
        agent,
        :last_session_transition,
        AgentSessionState.transition(current, agent, source) ||
          (current && (current[:last_session_transition] || current["last_session_transition"]))
      )
end
