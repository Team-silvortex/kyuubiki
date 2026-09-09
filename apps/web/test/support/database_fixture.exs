defmodule KyuubikiWeb.TestSupport.DatabaseFixture do
  alias Exqlite.Sqlite3
  alias KyuubikiWeb.Storage.{MigrationQuery, RuntimeSchema}

  def directory do
    path =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-db-test-" <> Base.encode16(:crypto.strong_rand_bytes(12))
      )

    File.mkdir!(path)
    ExUnit.Callbacks.on_exit(fn -> File.rm_rf!(path) end)
    path
  end

  def open!(path) do
    {:ok, db} = Sqlite3.open(path)
    {:sqlite, db}
  end

  def close!({:sqlite, db}), do: Sqlite3.close(db)

  def legacy!(connection, backend \\ :sqlite, options \\ []) do
    statements = RuntimeSchema.statements(backend)

    statements =
      if options[:old_jobs] do
        Enum.map(
          statements,
          &Regex.replace(
            ~r/^\s+(model_version_id|queue_timeout_ms|execution_timeout_ms|execution_started_at) [^\n]+\n/m,
            &1,
            ""
          )
        )
      else
        statements
      end

    Enum.each(statements, &MigrationQuery.rows!(connection, &1))

    MigrationQuery.rows!(
      connection,
      "INSERT INTO kyuubiki_projects (project_id, name, description) VALUES ($1, $2, $3)",
      ["research", "Thermal study", "retained original"]
    )

    MigrationQuery.rows!(
      connection,
      "INSERT INTO kyuubiki_jobs (job_id, project_id, simulation_case_id, status, progress, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
      ["job-1", "research", "thermal", "completed", 1.0, timestamp(backend), timestamp(backend)]
    )

    payload = ~s({"temperature":[273.15,320.5],"winner":"copper"})
    payload = if backend == :postgres, do: Jason.decode!(payload), else: payload

    MigrationQuery.rows!(
      connection,
      "INSERT INTO kyuubiki_analysis_results (job_id, payload) VALUES ($1, $2)",
      ["job-1", payload]
    )
  end

  defp timestamp(:sqlite), do: "2026-09-09T00:00:00Z"
  defp timestamp(:postgres), do: ~U[2026-09-09 00:00:00Z]
end
