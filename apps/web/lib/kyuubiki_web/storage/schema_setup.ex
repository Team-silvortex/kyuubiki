defmodule KyuubikiWeb.Storage.SchemaSetup do
  @moduledoc false

  use GenServer

  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.CentralDatabase

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    ensure_tables!()
    {:ok, %{}}
  end

  defp ensure_tables! do
    repo = Storage.repo_module()

    SQL.query!(
      repo,
      create_projects_sql(),
      []
    )

    SQL.query!(
      repo,
      create_models_sql(),
      []
    )

    SQL.query!(
      repo,
      create_model_versions_sql(),
      []
    )

    SQL.query!(
      repo,
      create_jobs_sql(),
      []
    )

    if Storage.postgres?() do
      SQL.query!(
        repo,
        """
        ALTER TABLE kyuubiki_jobs
        ADD COLUMN IF NOT EXISTS model_version_id TEXT REFERENCES kyuubiki_model_versions(version_id) ON DELETE SET NULL
        """,
        []
      )
    end

    SQL.query!(
      repo,
      create_results_sql(),
      []
    )

    SQL.query!(
      repo,
      create_security_events_sql(),
      []
    )

    Enum.each(CentralDatabase.create_table_sqls(), fn sql ->
      SQL.query!(repo, sql, [])
    end)
  end

  defp create_projects_sql do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_projects (
        project_id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        updated_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp create_models_sql do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_models (
        model_id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES kyuubiki_projects(project_id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        kind TEXT NOT NULL,
        material TEXT,
        model_schema_version TEXT NOT NULL,
        payload #{json_type()} NOT NULL,
        latest_version_id TEXT,
        latest_version_number #{integer_type()},
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        updated_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp create_model_versions_sql do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_model_versions (
        version_id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES kyuubiki_projects(project_id) ON DELETE CASCADE,
        model_id TEXT NOT NULL REFERENCES kyuubiki_models(model_id) ON DELETE CASCADE,
        name TEXT,
        version_number #{integer_type()} NOT NULL,
        kind TEXT NOT NULL,
        material TEXT,
        model_schema_version TEXT NOT NULL,
        payload #{json_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        updated_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp create_jobs_sql do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_jobs (
        job_id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        model_version_id TEXT,
        simulation_case_id TEXT NOT NULL,
        worker_id TEXT,
        message TEXT,
        status TEXT NOT NULL,
        progress #{float_type()} NOT NULL,
        residual #{float_type()},
        iteration #{integer_type()},
        created_at #{timestamp_type()} NOT NULL,
        updated_at #{timestamp_type()} NOT NULL
      )
    """
  end

  defp create_results_sql do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_analysis_results (
        job_id TEXT PRIMARY KEY REFERENCES kyuubiki_jobs(job_id) ON DELETE CASCADE,
        payload #{json_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        updated_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp create_security_events_sql do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_security_events (
        event_id TEXT PRIMARY KEY,
        event_type TEXT NOT NULL,
        source TEXT NOT NULL,
        action TEXT NOT NULL,
        risk TEXT NOT NULL,
        status TEXT NOT NULL,
        note TEXT,
        context #{json_type()} NOT NULL,
        occurred_at #{timestamp_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        updated_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp timestamp_type, do: if(Storage.sqlite?(), do: "TEXT", else: "TIMESTAMPTZ")
  defp timestamp_default, do: if(Storage.sqlite?(), do: "CURRENT_TIMESTAMP", else: "NOW()")
  defp json_type, do: if(Storage.sqlite?(), do: "JSON", else: "JSONB")
  defp integer_type, do: if(Storage.sqlite?(), do: "INTEGER", else: "BIGINT")
  defp float_type, do: if(Storage.sqlite?(), do: "REAL", else: "DOUBLE PRECISION")
end
