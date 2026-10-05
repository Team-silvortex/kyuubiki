Code.require_file("workflow_api_fixtures.exs", __DIR__)
Code.require_file("workflow_api_test_support.exs", __DIR__)

alias KyuubikiWeb.TestSupport.WorkflowApi
alias KyuubikiWeb.Playground.AgentPool

scenario = System.get_env("KYUUBIKI_HEADLESS_LIVE_SCENARIO", "electrostatic_quad_summary")

{:ok, _} = Application.ensure_all_started(:kyuubiki_web)

case scenario do
  "electrostatic_quad_summary" ->
    {:ok, _pid} = WorkflowApi.start_electrostatic_quad_summary_session()

  "guarded_quad_blocked" ->
    {:ok, _pid} = WorkflowApi.start_guarded_quad_sessions(:blocked)

  "guarded_quad_continued" ->
    {:ok, _pid} = WorkflowApi.start_guarded_quad_sessions(:continued)

  "real_agent" ->
    port = System.fetch_env!("KYUUBIKI_HEADLESS_LIVE_AGENT_PORT") |> String.to_integer()
    true = port in 1..65_535

    # Static endpoints have unknown package readiness; do not invent a ready advertisement.
    endpoints = [%{id: "owned-live-agent", host: "127.0.0.1", port: port}]

    endpoints =
      case System.get_env("KYUUBIKI_HEADLESS_LIVE_AGENT_PEER_PORT") do
        nil ->
          endpoints

        peer ->
          peer_port = String.to_integer(peer)
          true = peer_port in 1..65_535 and peer_port != port
          endpoints ++ [%{id: "owned-live-peer", host: "127.0.0.1", port: peer_port}]
      end

    Application.put_env(:kyuubiki_web, AgentPool, endpoints: endpoints)

    AgentPool.reload()

  other ->
    raise "unsupported headless live scenario: #{other}"
end

if scenario != "real_agent" do
  fake_agent_port = WorkflowApi.await_fake_agent_port()
  WorkflowApi.configure_fake_agent_pool(fake_agent_port)
end

{:ok, server_pid} =
  Bandit.start_link(
    plug: KyuubikiWeb.Router,
    scheme: :http,
    ip: {127, 0, 0, 1},
    port: 0,
    startup_log: false
  )

{:ok, {_address, http_port}} = ThousandIsland.listener_info(server_pid)
IO.puts("HEADLESS_LIVE_SERVER_READY #{http_port}")

receive do
after
  :infinity -> :ok
end
