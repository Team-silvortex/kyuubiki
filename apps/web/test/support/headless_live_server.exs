Code.require_file("workflow_api_fixtures.exs", __DIR__)
Code.require_file("workflow_api_test_support.exs", __DIR__)

alias KyuubikiWeb.TestSupport.WorkflowApi

scenario = System.get_env("KYUUBIKI_HEADLESS_LIVE_SCENARIO", "electrostatic_quad_summary")

{:ok, _} = Application.ensure_all_started(:kyuubiki_web)

case scenario do
  "electrostatic_quad_summary" ->
    {:ok, _pid} = WorkflowApi.start_electrostatic_quad_summary_session()

  "guarded_quad_blocked" ->
    {:ok, _pid} = WorkflowApi.start_guarded_quad_sessions(:blocked)

  "guarded_quad_continued" ->
    {:ok, _pid} = WorkflowApi.start_guarded_quad_sessions(:continued)

  other ->
    raise "unsupported headless live scenario: #{other}"
end

fake_agent_port = WorkflowApi.await_fake_agent_port()
WorkflowApi.configure_fake_agent_pool(fake_agent_port)

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
