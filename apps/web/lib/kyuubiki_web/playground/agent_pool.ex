defmodule KyuubikiWeb.Playground.AgentPool do
  @moduledoc """
  Tracks available Rust RPC agents and hands them out in round-robin order.
  """

  use GenServer

  alias KyuubikiWeb.Playground.AgentPoolRouter

  @default_host "127.0.0.1"
  @default_port 5001
  @default_discovery :static

  @type endpoint :: %{id: String.t(), host: String.t(), port: pos_integer()}

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  @spec checkout_endpoints(String.t() | nil, keyword()) :: [endpoint()]
  def checkout_endpoints(method \\ nil, opts \\ []) when is_list(opts) do
    GenServer.call(__MODULE__, {:checkout_endpoints, method, opts})
  end

  @spec report_failure(endpoint(), term()) :: :ok
  def report_failure(endpoint, reason) when is_map(endpoint) do
    GenServer.call(__MODULE__, {:report_failure, endpoint, inspect(reason)})
  end

  @spec report_success(endpoint()) :: :ok
  def report_success(endpoint) when is_map(endpoint) do
    GenServer.call(__MODULE__, {:report_success, endpoint})
  end

  @spec endpoints() :: [endpoint()]
  def endpoints do
    GenServer.call(__MODULE__, :endpoints)
  end

  @spec deployment_info() :: map()
  def deployment_info do
    GenServer.call(__MODULE__, :deployment_info)
  end

  @spec reload() :: :ok
  def reload do
    GenServer.call(__MODULE__, :reload)
  end

  @impl true
  def init(_opts) do
    endpoints = configured_endpoints()

    {:ok,
     %{
       endpoints: endpoints,
       cursor: 0,
       health: %{},
       deployment: deployment_info_for(endpoints, %{})
     }}
  end

  @impl true
  def handle_call(:endpoints, _from, state) do
    {:reply, enrich_endpoints(state.endpoints, state.health), state}
  end

  def handle_call(:deployment_info, _from, state) do
    {:reply, deployment_info_for(state.endpoints, state.health), state}
  end

  def handle_call(:reload, _from, state) do
    endpoints = configured_endpoints()
    health = prune_health(state.health, endpoints)

    {:reply, :ok,
     %{
       state
       | endpoints: endpoints,
         cursor: 0,
         health: health,
         deployment: deployment_info_for(endpoints, health)
     }}
  end

  def handle_call(
        {:checkout_endpoints, method, opts},
        _from,
        %{endpoints: endpoints, cursor: cursor, health: health} = state
      ) do
    endpoints = runtime_checkout_endpoints(endpoints)

    ordered =
      endpoints
      |> rotate(cursor)
      |> AgentPoolRouter.route(method, opts)
      |> deprioritize_cooling_endpoints(health)

    next_cursor = if endpoints == [], do: 0, else: rem(cursor + 1, length(endpoints))
    {:reply, ordered, %{state | endpoints: endpoints, cursor: next_cursor}}
  end

  def handle_call({:report_failure, endpoint, reason}, _from, state) do
    health = mark_failure(state.health, endpoint, reason)

    {:reply, :ok,
     %{state | health: health, deployment: deployment_info_for(state.endpoints, health)}}
  end

  def handle_call({:report_success, endpoint}, _from, state) do
    health = clear_health(state.health, endpoint)

    {:reply, :ok,
     %{state | health: health, deployment: deployment_info_for(state.endpoints, health)}}
  end

  defp rotate([], _cursor), do: []

  defp rotate(endpoints, cursor) do
    offset = rem(cursor, length(endpoints))
    Enum.drop(endpoints, offset) ++ Enum.take(endpoints, offset)
  end

  defp deprioritize_cooling_endpoints(endpoints, health) do
    {available, cooling} =
      Enum.split_with(endpoints, fn endpoint ->
        cooldown_remaining_ms(health, endpoint) <= 0
      end)

    case available do
      [] -> endpoints
      _ -> available ++ cooling
    end
  end

  defp configured_endpoints do
    config = Application.get_env(:kyuubiki_web, __MODULE__, [])

    case config[:endpoints] do
      endpoints when is_list(endpoints) and endpoints != [] ->
        Enum.map(endpoints, &normalize_endpoint/1)

      _ ->
        configured_endpoints_from_discovery(config)
    end
  end

  defp runtime_checkout_endpoints(current_endpoints) do
    config = Application.get_env(:kyuubiki_web, __MODULE__, [])

    case discovery_mode(config) do
      :registry -> registry_configured_endpoints()
      _ -> current_endpoints
    end
  end

  defp configured_endpoints_from_discovery(config) do
    case discovery_mode(config) do
      :registry -> registry_configured_endpoints()
      :manifest -> manifest_configured_endpoints(config)
      _ -> env_configured_endpoints()
    end
  end

  defp registry_configured_endpoints do
    case KyuubikiWeb.Playground.AgentRegistry.active_endpoints() do
      [] -> env_configured_endpoints()
      endpoints -> Enum.map(endpoints, &normalize_endpoint/1)
    end
  end

  defp env_configured_endpoints do
    endpoints_env = System.get_env("KYUUBIKI_AGENT_ENDPOINTS", "")

    endpoints =
      endpoints_env
      |> String.split(",", trim: true)
      |> Enum.map(&String.trim/1)
      |> Enum.reject(&(&1 == ""))
      |> Enum.map(&parse_env_endpoint/1)

    case endpoints do
      [] -> [default_endpoint()]
      _ -> endpoints
    end
  end

  defp manifest_configured_endpoints(config) do
    manifest_path = manifest_path(config)

    with path when is_binary(path) <- manifest_path,
         true <- File.exists?(path),
         {:ok, contents} <- File.read(path),
         {:ok, payload} <- Jason.decode(contents),
         agents when is_list(agents) <- Map.get(payload, "agents"),
         endpoints when endpoints != [] <-
           agents |> Enum.map(&normalize_manifest_endpoint/1) |> Enum.reject(&is_nil/1) do
      endpoints
    else
      _ -> env_configured_endpoints()
    end
  end

  defp parse_env_endpoint(endpoint) do
    case String.split(endpoint, "@", parts: 2) do
      [id, address] ->
        build_endpoint(id, address)

      [address] ->
        build_endpoint(nil, address)
    end
  end

  defp build_endpoint(id, address) do
    case String.split(address, ":", parts: 2) do
      [host, port_text] ->
        port = String.to_integer(port_text)
        %{id: id || "#{host}:#{port}", host: host, port: port}

      _ ->
        default_endpoint()
    end
  rescue
    ArgumentError -> default_endpoint()
  end

  defp normalize_endpoint(%{host: host, port: port} = endpoint)
       when is_binary(host) and is_integer(port) and port > 0 do
    endpoint
    |> Map.take([
      :active_lease,
      :control_mode,
      :orch_id,
      :orch_session_id,
      :session_state,
      :cluster_id,
      :fingerprint,
      :role,
      :region,
      :zone,
      :capacity,
      :tags,
      :operator_package_runtime
    ])
    |> Map.merge(%{
      id: Map.get(endpoint, :id, "#{host}:#{port}"),
      host: host,
      port: port,
      methods: normalize_methods(Map.get(endpoint, :methods)),
      capabilities: normalize_capabilities(Map.get(endpoint, :capabilities)),
      health_score: normalize_health_score(Map.get(endpoint, :health_score))
    })
  end

  defp normalize_endpoint(%{"host" => host, "port" => port} = endpoint)
       when is_binary(host) and is_integer(port) and port > 0 do
    %{
      id: Map.get(endpoint, "id", "#{host}:#{port}"),
      host: host,
      port: port,
      active_lease: Map.get(endpoint, "active_lease"),
      control_mode: Map.get(endpoint, "control_mode"),
      orch_id: Map.get(endpoint, "orch_id"),
      orch_session_id: Map.get(endpoint, "orch_session_id"),
      session_state: Map.get(endpoint, "session_state"),
      cluster_id: Map.get(endpoint, "cluster_id"),
      fingerprint: Map.get(endpoint, "fingerprint"),
      role: Map.get(endpoint, "role"),
      region: Map.get(endpoint, "region"),
      zone: Map.get(endpoint, "zone"),
      capacity: Map.get(endpoint, "capacity"),
      tags: Map.get(endpoint, "tags"),
      operator_package_runtime: Map.get(endpoint, "operator_package_runtime"),
      methods: normalize_methods(Map.get(endpoint, "methods")),
      capabilities: normalize_capabilities(Map.get(endpoint, "capabilities")),
      health_score: normalize_health_score(Map.get(endpoint, "health_score"))
    }
  end

  defp normalize_endpoint(_endpoint), do: default_endpoint()

  defp normalize_manifest_endpoint(%{"host" => host, "port" => port} = endpoint)
       when is_binary(host) and is_integer(port) and port > 0 do
    metadata =
      endpoint
      |> Map.take([
        "region",
        "zone",
        "role",
        "tags",
        "capacity",
        "methods",
        "capabilities",
        "health_score"
      ])
      |> Enum.into(%{}, fn {key, value} -> {String.to_atom(key), value} end)

    metadata
    |> Map.merge(%{
      id: Map.get(endpoint, "id", "#{host}:#{port}"),
      host: host,
      port: port
    })
  end

  defp normalize_manifest_endpoint(_endpoint), do: nil

  defp deployment_info_for(endpoints, health) do
    config = Application.get_env(:kyuubiki_web, __MODULE__, [])
    discovery = discovery_mode(config)
    cooling_down_count = Enum.count(endpoints, &(cooldown_remaining_ms(health, &1) > 0))

    %{
      mode: deployment_mode(config),
      discovery: discovery,
      manifest_path: manifest_path(config),
      endpoint_count: length(endpoints),
      cooling_down_count: cooling_down_count,
      ready_endpoint_count: max(length(endpoints) - cooling_down_count, 0)
    }
  end

  defp enrich_endpoints(endpoints, health) do
    Enum.map(endpoints, fn endpoint ->
      health_state = Map.get(health, endpoint.id, %{})
      remaining_ms = cooldown_remaining_ms(health, endpoint)

      endpoint
      |> Map.put(:cooldown_remaining_ms, max(remaining_ms, 0))
      |> Map.put(:consecutive_failures, Map.get(health_state, :consecutive_failures, 0))
      |> Map.put(:last_failure_reason, Map.get(health_state, :last_failure_reason))
      |> maybe_put_iso(:cooldown_until, Map.get(health_state, :cooldown_until))
      |> maybe_put_iso(:last_failure_at, Map.get(health_state, :last_failure_at))
    end)
  end

  defp maybe_put_iso(endpoint, _key, nil), do: endpoint

  defp maybe_put_iso(endpoint, key, %DateTime{} = value),
    do: Map.put(endpoint, key, DateTime.to_iso8601(value))

  defp prune_health(health, endpoints) do
    valid_ids = MapSet.new(Enum.map(endpoints, & &1.id))

    health
    |> Enum.filter(fn {id, _state} -> MapSet.member?(valid_ids, id) end)
    |> Enum.into(%{})
  end

  defp clear_health(health, %{id: id}) when is_binary(id), do: Map.delete(health, id)
  defp clear_health(health, _endpoint), do: health

  defp mark_failure(health, %{id: id}, reason) when is_binary(id) do
    current = Map.get(health, id, %{})
    failures = Map.get(current, :consecutive_failures, 0) + 1
    cooldown_ms = failure_cooldown_ms(failures)
    now = DateTime.utc_now()

    Map.put(health, id, %{
      consecutive_failures: failures,
      cooldown_until: DateTime.add(now, cooldown_ms, :millisecond),
      last_failure_at: now,
      last_failure_reason: reason
    })
  end

  defp mark_failure(health, _endpoint, _reason), do: health

  defp cooldown_remaining_ms(health, %{id: id}) when is_binary(id) do
    case Map.get(health, id) do
      %{cooldown_until: %DateTime{} = cooldown_until} ->
        DateTime.diff(cooldown_until, DateTime.utc_now(), :millisecond)

      _ ->
        0
    end
  end

  defp cooldown_remaining_ms(_health, _endpoint), do: 0

  defp failure_cooldown_ms(consecutive_failures) do
    base_ms =
      Application.get_env(:kyuubiki_web, __MODULE__, [])
      |> Keyword.get(:failure_cooldown_ms, 5_000)

    max_ms =
      Application.get_env(:kyuubiki_web, __MODULE__, [])
      |> Keyword.get(:failure_cooldown_max_ms, 30_000)

    multiplier = min(max(consecutive_failures - 1, 0), 3)
    min(base_ms * round(:math.pow(2, multiplier)), max_ms)
  end

  defp deployment_mode(config) do
    config
    |> Keyword.get(:deployment_mode, :local)
    |> normalize_mode()
  end

  defp discovery_mode(config) do
    config
    |> Keyword.get(:discovery, @default_discovery)
    |> normalize_discovery()
  end

  defp manifest_path(config) do
    case Keyword.get(config, :manifest_path) do
      path when is_binary(path) and path != "" -> path
      _ -> nil
    end
  end

  defp normalize_mode(mode) when mode in [:local, :cloud, :distributed], do: mode
  defp normalize_mode("local"), do: :local
  defp normalize_mode("cloud"), do: :cloud
  defp normalize_mode("distributed"), do: :distributed
  defp normalize_mode(_mode), do: :local

  defp normalize_discovery(discovery) when discovery in [:static, :manifest, :registry],
    do: discovery

  defp normalize_discovery("static"), do: :static
  defp normalize_discovery("manifest"), do: :manifest
  defp normalize_discovery("registry"), do: :registry
  defp normalize_discovery(_discovery), do: @default_discovery

  defp default_endpoint do
    %{id: "#{@default_host}:#{@default_port}", host: @default_host, port: @default_port}
  end

  defp normalize_methods(values) when is_list(values),
    do: values |> Enum.filter(&is_binary/1) |> Enum.uniq()

  defp normalize_methods(_values), do: []

  defp normalize_capabilities(values) when is_list(values),
    do: values |> Enum.map(&normalize_capability/1) |> Enum.reject(&is_nil/1)

  defp normalize_capabilities(_values), do: []

  defp normalize_capability(capability) when is_binary(capability) and capability != "",
    do: %{id: capability, role: capability, methods: [], tags: []}

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

  defp normalize_health_score(value) when is_integer(value) and value >= 0 and value <= 100,
    do: value

  defp normalize_health_score(_value), do: nil
end
