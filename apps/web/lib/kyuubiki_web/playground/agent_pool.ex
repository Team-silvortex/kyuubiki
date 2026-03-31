defmodule KyuubikiWeb.Playground.AgentPool do
  @moduledoc """
  Tracks available Rust RPC agents and hands them out in round-robin order.
  """

  use GenServer

  @default_host "127.0.0.1"
  @default_port 5001

  @type endpoint :: %{id: String.t(), host: String.t(), port: pos_integer()}

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  @spec checkout_endpoints() :: [endpoint()]
  def checkout_endpoints do
    GenServer.call(__MODULE__, :checkout_endpoints)
  end

  @spec endpoints() :: [endpoint()]
  def endpoints do
    GenServer.call(__MODULE__, :endpoints)
  end

  @spec reload() :: :ok
  def reload do
    GenServer.call(__MODULE__, :reload)
  end

  @impl true
  def init(_opts) do
    {:ok, %{endpoints: configured_endpoints(), cursor: 0}}
  end

  @impl true
  def handle_call(:endpoints, _from, state) do
    {:reply, state.endpoints, state}
  end

  def handle_call(:reload, _from, state) do
    endpoints = configured_endpoints()
    {:reply, :ok, %{state | endpoints: endpoints, cursor: 0}}
  end

  def handle_call(:checkout_endpoints, _from, %{endpoints: endpoints, cursor: cursor} = state) do
    ordered = rotate(endpoints, cursor)
    next_cursor = if endpoints == [], do: 0, else: rem(cursor + 1, length(endpoints))
    {:reply, ordered, %{state | cursor: next_cursor}}
  end

  defp rotate([], _cursor), do: []

  defp rotate(endpoints, cursor) do
    offset = rem(cursor, length(endpoints))
    Enum.drop(endpoints, offset) ++ Enum.take(endpoints, offset)
  end

  defp configured_endpoints do
    case Application.get_env(:kyuubiki_web, __MODULE__, [])[:endpoints] do
      endpoints when is_list(endpoints) and endpoints != [] ->
        Enum.map(endpoints, &normalize_endpoint/1)

      _ ->
        env_configured_endpoints()
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
    %{
      id: Map.get(endpoint, :id, "#{host}:#{port}"),
      host: host,
      port: port
    }
  end

  defp normalize_endpoint(%{"host" => host, "port" => port} = endpoint)
       when is_binary(host) and is_integer(port) and port > 0 do
    %{
      id: Map.get(endpoint, "id", "#{host}:#{port}"),
      host: host,
      port: port
    }
  end

  defp normalize_endpoint(_endpoint), do: default_endpoint()

  defp default_endpoint do
    %{id: "#{@default_host}:#{@default_port}", host: @default_host, port: @default_port}
  end
end
