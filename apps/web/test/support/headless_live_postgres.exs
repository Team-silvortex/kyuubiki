defmodule KyuubikiWeb.TestSupport.HeadlessLivePostgres do
  @moduledoc false
  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.PostgresRepo

  def configure do
    if schema = System.get_env("KYUUBIKI_HEADLESS_LIVE_POSTGRES_SCHEMA") do
      schema = checked_schema!(schema)
      with_repo(fn pid -> SQL.query!(pid, "CREATE SCHEMA IF NOT EXISTS #{schema}") end)
      config = Application.fetch_env!(:kyuubiki_web, PostgresRepo)
      parameters = Keyword.put(Keyword.get(config, :parameters, []), :search_path, schema)

      # Remote tunnel startup gets a bounded warm-up window, not relaxed runtime lease checks.
      config =
        config
        |> Keyword.put(:parameters, parameters)
        |> Keyword.put(:queue_target, 1_000)
        |> Keyword.put(:queue_interval, 10_000)

      Application.put_env(:kyuubiki_web, PostgresRepo, config)
    end
  end

  def cleanup do
    schema = checked_schema!(System.fetch_env!("KYUUBIKI_HEADLESS_LIVE_POSTGRES_SCHEMA"))
    with_repo(fn pid -> SQL.query!(pid, "DROP SCHEMA IF EXISTS #{schema} CASCADE") end)
  end

  defp checked_schema!(schema) do
    if KyuubikiWeb.Storage.postgres?() and Regex.match?(~r/^owned_live_[0-9a-f]{48}$/, schema),
      do: schema,
      else: raise("invalid owned live PostgreSQL test scope")
  end

  defp with_repo(callback) do
    {:ok, _} = Application.ensure_all_started(:ecto_sql)
    {:ok, _} = Application.ensure_all_started(:postgrex)
    {:ok, pid} = PostgresRepo.start_link(name: nil, pool_size: 1)

    try do
      callback.(pid)
    after
      Supervisor.stop(pid)
    end
  end
end
