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

  @spec checkout_endpoints(String.t() | nil) :: [endpoint()]
  def checkout_endpoints(method \\ nil) do
    GenServer.call(__MODULE__, {:checkout_endpoints, method})
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
    {:ok, %{endpoints: endpoints, cursor: 0, deployment: deployment_info_for(endpoints)}}
  end

  @impl true
  def handle_call(:endpoints, _from, state) do
    {:reply, state.endpoints, state}
  end

  def handle_call(:deployment_info, _from, state) do
    {:reply, state.deployment, state}
  end

  def handle_call(:reload, _from, state) do
    endpoints = configured_endpoints()
    {:reply, :ok, %{state | endpoints: endpoints, cursor: 0, deployment: deployment_info_for(endpoints)}}
  end

  def handle_call({:checkout_endpoints, method}, _from, %{endpoints: endpoints, cursor: cursor} = state) do
    ordered =
      endpoints
      |> rotate(cursor)
      |> route_endpoints(method)

    next_cursor = if endpoints == [], do: 0, else: rem(cursor + 1, length(endpoints))
    {:reply, ordered, %{state | cursor: next_cursor}}
  end

  defp rotate([], _cursor), do: []

  defp rotate(endpoints, cursor) do
    offset = rem(cursor, length(endpoints))
    Enum.drop(endpoints, offset) ++ Enum.take(endpoints, offset)
  end

  defp route_endpoints(endpoints, nil), do: endpoints
  defp route_endpoints(endpoints, method) when is_binary(method) do
    tags = preferred_tags(method)

    {preferred, fallback} =
      Enum.split_with(endpoints, fn endpoint ->
        endpoint_tags(endpoint)
        |> Enum.any?(&(&1 in tags))
      end)

    sort_by_capacity(preferred) ++ fallback
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
         endpoints when endpoints != [] <- agents |> Enum.map(&normalize_manifest_endpoint/1) |> Enum.reject(&is_nil/1) do
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
      port: port
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
      tags: Map.get(endpoint, "tags")
    }
  end

  defp normalize_endpoint(_endpoint), do: default_endpoint()

  defp normalize_manifest_endpoint(
         %{"host" => host, "port" => port} = endpoint
       )
       when is_binary(host) and is_integer(port) and port > 0 do
    metadata =
      endpoint
      |> Map.take(["region", "zone", "role", "tags", "capacity"])
      |> Enum.into(%{}, fn {key, value} -> {String.to_atom(key), value} end)

    metadata
    |> Map.merge(%{
      id: Map.get(endpoint, "id", "#{host}:#{port}"),
      host: host,
      port: port
    })
  end

  defp normalize_manifest_endpoint(_endpoint), do: nil

  defp deployment_info_for(endpoints) do
    config = Application.get_env(:kyuubiki_web, __MODULE__, [])
    discovery = discovery_mode(config)

    %{
      mode: deployment_mode(config),
      discovery: discovery,
      manifest_path: manifest_path(config),
      endpoint_count: length(endpoints)
    }
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

  defp normalize_discovery(discovery) when discovery in [:static, :manifest, :registry], do: discovery
  defp normalize_discovery("static"), do: :static
  defp normalize_discovery("manifest"), do: :manifest
  defp normalize_discovery("registry"), do: :registry
  defp normalize_discovery(_discovery), do: @default_discovery

  defp default_endpoint do
    %{id: "#{@default_host}:#{@default_port}", host: @default_host, port: @default_port}
  end

  defp endpoint_tags(endpoint) do
    endpoint
    |> Map.get(:tags, [])
    |> List.wrap()
    |> Enum.filter(&is_binary/1)
  end

  defp sort_by_capacity(endpoints) do
    endpoints
    |> Enum.with_index()
    |> Enum.sort_by(fn {endpoint, index} ->
      {-Map.get(endpoint, :capacity, 1), index}
    end)
    |> Enum.map(&elem(&1, 0))
  end

  defp preferred_tags("solve_bar_1d"), do: ["bar"]
  defp preferred_tags("solve_truss_2d"), do: ["truss"]
  defp preferred_tags("solve_truss_3d"), do: ["truss", "space"]
  defp preferred_tags("solve_plane_triangle_2d"), do: ["plane", "mesh"]
  defp preferred_tags(_method), do: ["general", "cpu"]
end
