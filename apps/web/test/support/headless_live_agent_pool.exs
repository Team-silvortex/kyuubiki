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

    # Static endpoints have unknown package readiness; do not invent a ready advertisement.
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: endpoints)
  end
end
