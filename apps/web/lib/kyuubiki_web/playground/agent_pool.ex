defmodule KyuubikiWeb.Playground.AgentPool do
  @moduledoc """
  Tracks available Rust RPC agents and hands them out in round-robin order.
  """

  use GenServer

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
    ordered =
      endpoints
      |> rotate(cursor)
      |> route_endpoints(method)
      |> filter_endpoints(method, opts)
      |> deprioritize_cooling_endpoints(health)

    next_cursor = if endpoints == [], do: 0, else: rem(cursor + 1, length(endpoints))
    {:reply, ordered, %{state | cursor: next_cursor}}
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

  defp route_endpoints(endpoints, nil), do: endpoints

  defp route_endpoints(endpoints, method) when is_binary(method) do
    tags = preferred_tags(method)

    {capable, remaining} =
      Enum.split_with(endpoints, fn endpoint ->
        supports_method?(endpoint, method)
      end)

    case capable do
      [] ->
        {preferred_by_tags, fallback} =
          Enum.split_with(remaining, fn endpoint ->
            endpoint_tags(endpoint)
            |> Enum.any?(&(&1 in tags))
          end)

        sort_by_health_capacity(preferred_by_tags) ++ fallback

      _ ->
        sort_by_health_capacity(capable) ++ remaining
    end
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

  defp filter_endpoints(endpoints, method, opts) do
    required_capabilities =
      opts
      |> Keyword.get(:required_capabilities, [])
      |> normalize_constraint_values()

    placement_tags =
      opts
      |> Keyword.get(:placement_tags, [])
      |> normalize_constraint_values()

    if required_capabilities == [] and placement_tags == [] do
      endpoints
    else
      {typed_matches, legacy_fallbacks} =
        Enum.reduce(endpoints, {[], []}, fn endpoint, {typed, legacy} ->
          case classify_endpoint(endpoint, method, required_capabilities, placement_tags) do
            {:typed_match, score} -> {[{endpoint, score} | typed], legacy}
            :legacy_fallback -> {typed, [endpoint | legacy]}
            :explicit_mismatch -> {typed, legacy}
          end
        end)

      typed_matches =
        typed_matches
        |> Enum.sort_by(fn {_endpoint, score} -> -score end)
        |> Enum.map(&elem(&1, 0))

      typed_matches ++ Enum.reverse(legacy_fallbacks)
    end
  end

  defp classify_endpoint(endpoint, method, required_capabilities, placement_tags) do
    capability_status = capability_match_status(endpoint, method, required_capabilities)
    tag_status = tag_match_status(endpoint, placement_tags)

    cond do
      capability_status == :mismatch or tag_status == :mismatch ->
        :explicit_mismatch

      capability_status == :unknown or tag_status == :unknown ->
        :legacy_fallback

      true ->
        {:typed_match, endpoint_match_score(endpoint, required_capabilities, placement_tags)}
    end
  end

  defp capability_match_status(_endpoint, _method, []), do: :match

  defp capability_match_status(endpoint, method, required_capabilities) do
    capability_ids =
      endpoint
      |> Map.get(:capabilities, [])
      |> Enum.map(& &1.id)

    has_capability_metadata? =
      capability_ids != [] or
        Map.get(endpoint, :methods, []) != [] or
        is_binary(Map.get(endpoint, :role)) or
        Enum.any?(Map.get(endpoint, :capabilities, []), &is_binary(&1.role))

    cond do
      not has_capability_metadata? ->
        :unknown

      Enum.all?(
        required_capabilities,
        &endpoint_supports_capability?(endpoint, method, &1, capability_ids)
      ) ->
        :match

      true ->
        :mismatch
    end
  end

  defp endpoint_supports_capability?(endpoint, method, "solver_rpc", capability_ids) do
    "solver_rpc" in capability_ids or
      Map.get(endpoint, :role) == "solver" or
      supports_method?(endpoint, method || "") or
      Enum.any?(Map.get(endpoint, :capabilities, []), &(&1.role == "solver"))
  end

  defp endpoint_supports_capability?(endpoint, _method, capability_id, capability_ids) do
    capability_id in capability_ids or Map.get(endpoint, :role) == capability_id
  end

  defp tag_match_status(_endpoint, []), do: :match

  defp tag_match_status(endpoint, placement_tags) do
    tags = endpoint_tags(endpoint)

    cond do
      tags == [] -> :unknown
      Enum.any?(placement_tags, &(&1 in tags)) -> :match
      true -> :mismatch
    end
  end

  defp endpoint_match_score(endpoint, required_capabilities, placement_tags) do
    capability_ids =
      endpoint
      |> Map.get(:capabilities, [])
      |> Enum.map(& &1.id)

    tags = endpoint_tags(endpoint)

    capability_score =
      Enum.count(required_capabilities, fn capability_id ->
        capability_id in capability_ids or Map.get(endpoint, :role) == capability_id
      end)

    tag_score = Enum.count(placement_tags, &(&1 in tags))
    capability_score * 10 + tag_score
  end

  defp normalize_constraint_values(values) when is_list(values) do
    values
    |> Enum.filter(&is_binary/1)
    |> Enum.reject(&(&1 == ""))
    |> Enum.uniq()
  end

  defp normalize_constraint_values(_values), do: []

  defp configured_endpoints do
    config = Application.get_env(:kyuubiki_web, __MODULE__, [])

    case config[:endpoints] do
      endpoints when is_list(endpoints) and endpoints != [] ->
        Enum.map(endpoints, &normalize_endpoint/1)

      _ ->
        configured_endpoints_from_discovery(config)
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
    |> Map.take([:role, :region, :zone, :capacity, :tags])
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
      role: Map.get(endpoint, "role"),
      region: Map.get(endpoint, "region"),
      zone: Map.get(endpoint, "zone"),
      capacity: Map.get(endpoint, "capacity"),
      tags: Map.get(endpoint, "tags"),
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

  defp endpoint_tags(endpoint) do
    direct =
      endpoint
      |> Map.get(:tags, [])
      |> List.wrap()
      |> Enum.filter(&is_binary/1)

    capability_tags =
      endpoint
      |> Map.get(:capabilities, [])
      |> List.wrap()
      |> Enum.flat_map(fn capability ->
        capability
        |> Map.get(:tags, [])
        |> List.wrap()
        |> Enum.filter(&is_binary/1)
      end)

    Enum.uniq(direct ++ capability_tags)
  end

  defp sort_by_health_capacity(endpoints) do
    endpoints
    |> Enum.with_index()
    |> Enum.sort_by(fn {endpoint, index} ->
      {
        -(Map.get(endpoint, :health_score) || 100),
        -Map.get(endpoint, :capacity, 1),
        index
      }
    end)
    |> Enum.map(&elem(&1, 0))
  end

  defp supports_method?(endpoint, method) when is_binary(method) do
    direct_methods =
      endpoint
      |> Map.get(:methods, [])
      |> List.wrap()
      |> Enum.filter(&is_binary/1)

    capability_methods =
      endpoint
      |> Map.get(:capabilities, [])
      |> List.wrap()
      |> Enum.flat_map(fn capability ->
        capability
        |> Map.get(:methods, [])
        |> List.wrap()
        |> Enum.filter(&is_binary/1)
      end)

    method in Enum.uniq(direct_methods ++ capability_methods)
  end

  defp normalize_methods(values) when is_list(values),
    do: values |> Enum.filter(&is_binary/1) |> Enum.uniq()

  defp normalize_methods(_values), do: []

  defp normalize_capabilities(values) when is_list(values) do
    values
    |> Enum.map(&normalize_capability/1)
    |> Enum.reject(&is_nil/1)
  end

  defp normalize_capabilities(_values), do: []

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

  defp preferred_tags("solve_bar_1d"), do: ["bar"]
  defp preferred_tags("solve_thermal_bar_1d"), do: ["bar", "thermal", "line"]
  defp preferred_tags("solve_heat_bar_1d"), do: ["heat", "bar", "line"]

  defp preferred_tags("solve_electrostatic_bar_1d"),
    do: ["electromagnetic", "electrostatic", "bar", "line"]

  defp preferred_tags("solve_electrostatic_plane_triangle_2d"),
    do: ["electromagnetic", "electrostatic", "plane", "triangle", "mesh"]

  defp preferred_tags("solve_electrostatic_plane_quad_2d"),
    do: ["electromagnetic", "electrostatic", "plane", "quad", "mesh"]

  defp preferred_tags("solve_heat_plane_triangle_2d"), do: ["heat", "plane", "mesh"]
  defp preferred_tags("solve_heat_plane_quad_2d"), do: ["heat", "plane", "mesh", "quad"]
  defp preferred_tags("solve_thermal_truss_2d"), do: ["truss", "thermal", "plane"]
  defp preferred_tags("solve_thermal_truss_3d"), do: ["truss", "thermal", "space"]
  defp preferred_tags("solve_spring_1d"), do: ["spring", "line", "support"]
  defp preferred_tags("solve_spring_2d"), do: ["spring", "plane", "support"]
  defp preferred_tags("solve_spring_3d"), do: ["spring", "space", "support"]
  defp preferred_tags("solve_beam_1d"), do: ["beam", "bending", "line"]
  defp preferred_tags("solve_thermal_beam_1d"), do: ["beam", "thermal", "bending", "line"]
  defp preferred_tags("solve_thermal_frame_2d"), do: ["frame", "thermal", "beam", "bending"]

  defp preferred_tags("solve_thermal_frame_3d"),
    do: ["frame", "thermal", "space", "beam", "bending"]

  defp preferred_tags("solve_torsion_1d"), do: ["torsion", "shaft", "line"]
  defp preferred_tags("solve_truss_2d"), do: ["truss"]
  defp preferred_tags("solve_truss_3d"), do: ["truss", "space"]
  defp preferred_tags("solve_plane_triangle_2d"), do: ["plane", "mesh"]
  defp preferred_tags("solve_thermal_plane_triangle_2d"), do: ["plane", "thermal", "mesh"]
  defp preferred_tags("solve_plane_quad_2d"), do: ["plane", "mesh", "quad"]
  defp preferred_tags("solve_thermal_plane_quad_2d"), do: ["plane", "thermal", "mesh", "quad"]
  defp preferred_tags("solve_frame_2d"), do: ["frame", "beam", "bending"]
  defp preferred_tags("solve_frame_3d"), do: ["frame", "space", "beam", "bending"]
  defp preferred_tags(_method), do: ["general", "cpu"]
end
