import Config

config :logger, :default_formatter,
  format: "$time $metadata[$level] $message\n",
  metadata: [:job_id, :stage]

storage_backend =
  case System.get_env("KYUUBIKI_STORAGE_BACKEND") do
    "postgres" -> :postgres
    "memory" -> :memory
    "json" -> :memory
    _ -> :sqlite
  end

deployment_mode =
  case System.get_env("KYUUBIKI_DEPLOYMENT_MODE") do
    "cloud" -> :cloud
    "distributed" -> :distributed
    _ -> :local
  end

agent_discovery =
  case System.get_env("KYUUBIKI_AGENT_DISCOVERY") do
    "manifest" -> :manifest
    _ -> :static
  end

agent_manifest_path =
  System.get_env(
    "KYUUBIKI_AGENT_MANIFEST_PATH",
    Path.expand("../../../deploy/agents.local.json", __DIR__)
  )

database_url =
  System.get_env(
    "DATABASE_URL",
    "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev"
  )

sqlite_database_path =
  System.get_env(
    "SQLITE_DATABASE_PATH",
    Path.expand("../../../tmp/data/kyuubiki_dev.sqlite3", __DIR__)
  )

agent_endpoints =
  System.get_env("KYUUBIKI_AGENT_ENDPOINTS", "127.0.0.1:5001")
  |> String.split(",", trim: true)
  |> Enum.map(&String.trim/1)
  |> Enum.reject(&(&1 == ""))
  |> Enum.map(fn endpoint ->
    case String.split(endpoint, "@", parts: 2) do
      [id, address] ->
        [host, port] = String.split(address, ":", parts: 2)
        %{id: id, host: host, port: String.to_integer(port)}

      [address] ->
        [host, port] = String.split(address, ":", parts: 2)
        %{id: "#{host}:#{port}", host: host, port: String.to_integer(port)}
    end
  end)

config :kyuubiki_web,
  storage_backend: storage_backend,
  ecto_repos: [KyuubikiWeb.PostgresRepo, KyuubikiWeb.SqliteRepo]

config :kyuubiki_web, KyuubikiWeb.Security,
  api_token: System.get_env("KYUUBIKI_API_TOKEN"),
  cluster_api_token: System.get_env("KYUUBIKI_CLUSTER_API_TOKEN"),
  cluster_allowed_agent_ids:
    System.get_env("KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS", "")
    |> String.split(",", trim: true)
    |> Enum.map(&String.trim/1)
    |> Enum.reject(&(&1 == ""))
    |> MapSet.new(),
  cluster_allowed_cluster_ids:
    System.get_env("KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS", "")
    |> String.split(",", trim: true)
    |> Enum.map(&String.trim/1)
    |> Enum.reject(&(&1 == ""))
    |> MapSet.new(),
  cluster_timestamp_window_ms:
    String.to_integer(System.get_env("KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS", "30000")),
  protect_reads?: System.get_env("KYUUBIKI_PROTECT_READS", "false") == "true"

config :kyuubiki_web, KyuubikiWeb.Playground.AgentPool,
  endpoints: agent_endpoints,
  deployment_mode: deployment_mode,
  discovery: agent_discovery,
  manifest_path: agent_manifest_path

config :kyuubiki_web, KyuubikiWeb.Playground.AgentClient,
  connect_timeout_ms: String.to_integer(System.get_env("KYUUBIKI_AGENT_CONNECT_TIMEOUT_MS", "1500")),
  recv_timeout_ms: String.to_integer(System.get_env("KYUUBIKI_AGENT_RECV_TIMEOUT_MS", "15000"))

config :kyuubiki_web, KyuubikiWeb.Playground.AgentRegistry,
  stale_after_ms: String.to_integer(System.get_env("KYUUBIKI_REMOTE_AGENT_STALE_AFTER_MS", "15000"))

config :kyuubiki_web, KyuubikiWeb.Jobs.Watchdog,
  scan_interval_ms: String.to_integer(System.get_env("KYUUBIKI_WATCHDOG_SCAN_INTERVAL_MS", "5000")),
  stale_job_ms: String.to_integer(System.get_env("KYUUBIKI_WATCHDOG_STALE_JOB_MS", "30000")),
  job_timeout_ms: String.to_integer(System.get_env("KYUUBIKI_WATCHDOG_JOB_TIMEOUT_MS", "120000"))

config :kyuubiki_web, KyuubikiWeb.PostgresRepo,
  url: database_url,
  pool_size: String.to_integer(System.get_env("POOL_SIZE", "5")),
  show_sensitive_data_on_connection_error: true

config :kyuubiki_web, KyuubikiWeb.SqliteRepo,
  database: sqlite_database_path,
  pool_size: 1,
  journal_mode: :wal,
  busy_timeout: 5_000
