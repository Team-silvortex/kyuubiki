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
  ecto_repos: [KyuubikiWeb.Repo]

config :kyuubiki_web, KyuubikiWeb.Playground.AgentPool, endpoints: agent_endpoints

config :kyuubiki_web, KyuubikiWeb.Repo,
  url: database_url,
  pool_size: String.to_integer(System.get_env("POOL_SIZE", "5")),
  show_sensitive_data_on_connection_error: true
