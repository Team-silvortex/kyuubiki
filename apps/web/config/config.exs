import Config

config :logger, :default_formatter,
  format: "$time $metadata[$level] $message\n",
  metadata: [:job_id, :stage]

storage_backend =
  case System.get_env("KYUUBIKI_STORAGE_BACKEND") do
    "postgres" -> :postgres
    _ -> :memory
  end

database_url =
  System.get_env(
    "DATABASE_URL",
    "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev"
  )

config :kyuubiki_web,
  storage_backend: storage_backend,
  ecto_repos: [KyuubikiWeb.Repo]

config :kyuubiki_web, KyuubikiWeb.Repo,
  url: database_url,
  pool_size: String.to_integer(System.get_env("POOL_SIZE", "5")),
  show_sensitive_data_on_connection_error: true
