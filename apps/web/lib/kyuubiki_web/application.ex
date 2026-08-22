defmodule KyuubikiWeb.Application do
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children =
      storage_children() ++
        [
          {Registry, keys: :unique, name: KyuubikiWeb.Orchestra.WorkflowRunnerRegistry},
          KyuubikiWeb.Playground.AgentRegistry,
          KyuubikiWeb.Playground.AgentPool,
          KyuubikiWeb.Playground.AgentExecutionGate,
          {Task.Supervisor, name: KyuubikiWeb.TaskSupervisor},
          KyuubikiWeb.Orchestra.HeadlessHandoffRegistry,
          KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator,
          KyuubikiWeb.Jobs.Watchdog
        ] ++ maybe_http_server()

    Supervisor.start_link(children, strategy: :one_for_one, name: KyuubikiWeb.Supervisor)
  end

  defp storage_children do
    if KyuubikiWeb.Storage.sql?() do
      ensure_sqlite_directory!()

      [
        KyuubikiWeb.Storage.repo_module(),
        {KyuubikiWeb.Storage.SchemaSetup, []}
      ]
    else
      [
        {KyuubikiWeb.Jobs.MemoryBackend, []},
        {KyuubikiWeb.AnalysisResultMemoryBackend, []},
        {KyuubikiWeb.Orchestra.LeaseMemoryBackend, []},
        {KyuubikiWeb.Library.MemoryBackend, []},
        {KyuubikiWeb.SecurityEvents.MemoryBackend, []}
      ]
    end
  end

  defp maybe_http_server do
    if Application.get_env(:kyuubiki_web, :start_http_server, true) do
      [
        {Bandit,
         [
           scheme: :http,
           plug: KyuubikiWeb.Router,
           ip: bind_ip(),
           port: port()
         ] ++ KyuubikiWeb.HttpTransportSecurity.server_options()}
      ]
    else
      []
    end
  end

  defp port do
    System.get_env("PORT", "4000") |> String.to_integer()
  end

  defp bind_ip do
    :kyuubiki_web
    |> Application.get_env(:http_bind_ip, "127.0.0.1")
    |> to_charlist()
    |> :inet.parse_address()
    |> case do
      {:ok, ip} -> ip
      {:error, _reason} -> {127, 0, 0, 1}
    end
  end

  defp ensure_sqlite_directory! do
    if KyuubikiWeb.Storage.sqlite?() do
      database = Application.fetch_env!(:kyuubiki_web, KyuubikiWeb.SqliteRepo)[:database]
      database |> Path.dirname() |> File.mkdir_p!()
    end
  end
end
