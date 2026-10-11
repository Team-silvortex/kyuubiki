defmodule KyuubikiWeb.TestSupport.HeadlessLiveAgentPool do
  @moduledoc false
  alias KyuubikiWeb.Playground.AgentPool

  def configure do
    port = System.fetch_env!("KYUUBIKI_HEADLESS_LIVE_AGENT_PORT") |> String.to_integer()
    true = port in 1..65_535

    # Configure original targets before startup recovery can attempt receipt retrieval.
    endpoints = [
      %{id: "owned-live-agent", host: "127.0.0.1", port: port, tags: ["owned-live-owner"]}
    ]

    endpoints =
      case System.get_env("KYUUBIKI_HEADLESS_LIVE_AGENT_PEER_PORT") do
        nil ->
          endpoints

        peer ->
          peer_port = String.to_integer(peer)
          true = peer_port in 1..65_535 and peer_port != port

          endpoints ++
            [
              %{
                id: "owned-live-peer",
                host: "127.0.0.1",
                port: peer_port,
                tags: ["owned-live-peer"]
              }
            ]
      end

    endpoints =
      if System.get_env("KYUUBIKI_HEADLESS_LIVE_DESCRIBE_AGENTS") == "1",
        do: Enum.map(endpoints, &describe/1),
        else: endpoints

    # Static endpoints have unknown package readiness; do not invent a ready advertisement.
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: endpoints)
  end

  defp describe(endpoint) do
    id = "owned-live-descriptor"
    request = %{"rpc_version" => 1, "id" => id, "method" => "describe_agent", "params" => %{}}

    {:ok, descriptor} =
      KyuubikiWeb.Playground.AgentRpcTransport.request(
        endpoint,
        id,
        request,
        fn _ -> :ok end,
        request_timeout_ms: 5_000,
        max_rpc_frame_bytes: 1024 * 1024
      )

    endpoint
    |> Map.put(:capabilities, Map.fetch!(descriptor, "capabilities"))
    |> Map.put(:methods, get_in(descriptor, ["protocol", "methods"]))
  end
end
