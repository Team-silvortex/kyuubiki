Code.require_file("workflow_api_fixtures.exs", __DIR__)
Code.require_file("workflow_api_test_support.exs", __DIR__)

alias KyuubikiWeb.TestSupport.WorkflowApi
alias KyuubikiWeb.Playground.AgentPool

scenario = System.get_env("KYUUBIKI_HEADLESS_LIVE_SCENARIO", "electrostatic_quad_summary")

# Only this owned live harness opts into a stable test journal. Ordinary test
# runs ignore inherited production data roots and use their isolated directory.
if root = System.get_env("KYUUBIKI_HEADLESS_LIVE_JOURNAL_ROOT") do
  Application.put_env(:kyuubiki_web, :operator_dispatch_journal_test_root, Path.dirname(root))
end

if scenario == "real_agent" do
  Code.require_file("headless_live_agent_pool.exs", __DIR__)
  KyuubikiWeb.TestSupport.HeadlessLiveAgentPool.configure()
end

if System.get_env("KYUUBIKI_HEADLESS_LIVE_NATIVE_GATE_ROOT") do
  Code.require_file("headless_native_recovery_gate.exs", __DIR__)

  Application.put_env(:kyuubiki_web, KyuubikiWeb.WorkflowNativeOperatorRuntime,
    client: KyuubikiWeb.TestSupport.HeadlessNativeRecoveryGate
  )
end

# The launcher uses --no-start so recovery never sees an empty temporary journal first.
{:ok, _} = Application.ensure_all_started(:kyuubiki_web)

case scenario do
  "electrostatic_quad_summary" ->
    {:ok, _pid} = WorkflowApi.start_electrostatic_quad_summary_session()

  "guarded_quad_blocked" ->
    {:ok, _pid} = WorkflowApi.start_guarded_quad_sessions(:blocked)

  "guarded_quad_continued" ->
    {:ok, _pid} = WorkflowApi.start_guarded_quad_sessions(:continued)

  "real_agent" ->
    AgentPool.reload()

  other ->
    raise "unsupported headless live scenario: #{other}"
end

if System.get_env("KYUUBIKI_HEADLESS_LIVE_RESULT_FIXTURES") == "1" do
  # Seed incomplete records only after startup recovery has scanned the empty fixture DB.
  %{"lease" => %{"status" => "owner"}} =
    KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator.snapshot()

  Code.require_file("headless_result_fixtures.exs", __DIR__)
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
