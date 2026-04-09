defmodule KyuubikiWeb.Playground.AgentRegistry do
  @moduledoc """
  Keeps a lightweight registry of remotely managed solver agents for distributed deployments.
  """

  use GenServer

  @type agent :: %{
          required(:id) => String.t(),
          required(:host) => String.t(),
          required(:port) => pos_integer(),
          optional(:cluster_id) => String.t(),
          optional(:fingerprint) => String.t(),
          optional(:role) => String.t(),
          optional(:region) => String.t(),
          optional(:zone) => String.t(),
          optional(:capacity) => pos_integer(),
          optional(:tags) => [String.t()],
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
    with {:ok, agent} <- build_agent(attrs) do
      agents = Map.put(state.agents, agent.id, agent)
      {:reply, {:ok, agent}, %{state | agents: agents}}
    else
      {:error, reason} -> {:reply, {:error, reason}, state}
    end
  end

  def handle_call({:heartbeat, agent_id, attrs}, _from, state) do
    current = Map.get(state.agents, agent_id, %{id: agent_id})

    with {:ok, agent} <- build_agent(Map.merge(current, attrs |> Map.put("id", agent_id))) do
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
         {:ok, port} <- fetch_port(attrs, "port") do
      now = DateTime.utc_now()

      {:ok,
       %{
         id: id,
         host: host,
         port: port,
         cluster_id: optional_string(attrs, "cluster_id"),
         fingerprint: optional_string(attrs, "fingerprint"),
         role: optional_string(attrs, "role"),
         region: optional_string(attrs, "region"),
         zone: optional_string(attrs, "zone"),
         capacity: optional_integer(attrs, "capacity"),
         tags: optional_tags(attrs, "tags"),
         last_seen_at: now
       }}
    end
  end

  defp fetch_string(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_binary(value) and value != "" -> {:ok, value}
      _ -> {:error, {:invalid_agent_field, key}}
    end
  end

  defp fetch_port(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_integer(value) and value > 0 -> {:ok, value}
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
      value when is_integer(value) and value > 0 -> value
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

  defp active_agents(agents) do
    limit_ms = stale_after_ms()
    now = DateTime.utc_now()

    agents
    |> Map.values()
    |> Enum.filter(fn agent -> DateTime.diff(now, agent.last_seen_at, :millisecond) <= limit_ms end)
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
      cluster_id: agent.cluster_id,
      fingerprint: agent.fingerprint,
      role: agent.role,
      region: agent.region,
      zone: agent.zone,
      capacity: agent.capacity,
      tags: agent.tags,
      last_seen_at: DateTime.to_iso8601(agent.last_seen_at)
    }
  end

  defp stale_after_ms do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:stale_after_ms, 15_000)
  end
end
