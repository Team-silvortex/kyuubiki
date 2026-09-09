defmodule KyuubikiWeb.Storage.RuntimeSchema do
  @moduledoc false

  # Frozen baseline. Add a migration instead of editing an already shipped baseline.
  def statements(backend) when backend in [:sqlite, :postgres] do
    [
      create_projects_sql(backend),
      create_models_sql(backend),
      create_model_versions_sql(backend),
      create_jobs_sql(backend),
      create_results_sql(backend),
      create_orchestra_leases_sql(backend),
      create_security_events_sql(backend)
    ] ++ KyuubikiWeb.Storage.CentralDatabase.create_table_sqls(backend)
  end

  defp create_projects_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_projects (
        project_id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp create_models_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_models (
        model_id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES kyuubiki_projects(project_id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        kind TEXT NOT NULL,
        material TEXT,
        model_schema_version TEXT NOT NULL,
        payload #{json_type(backend)} NOT NULL,
        latest_version_id TEXT,
        latest_version_number #{integer_type(backend)},
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp create_model_versions_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_model_versions (
        version_id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES kyuubiki_projects(project_id) ON DELETE CASCADE,
        model_id TEXT NOT NULL REFERENCES kyuubiki_models(model_id) ON DELETE CASCADE,
        name TEXT,
        version_number #{integer_type(backend)} NOT NULL,
        kind TEXT NOT NULL,
        material TEXT,
        model_schema_version TEXT NOT NULL,
        payload #{json_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp create_jobs_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_jobs (
        job_id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        model_version_id TEXT,
        simulation_case_id TEXT NOT NULL,
        worker_id TEXT,
        message TEXT,
        status TEXT NOT NULL,
        progress #{float_type(backend)} NOT NULL,
        residual #{float_type(backend)},
        iteration #{integer_type(backend)},
        queue_timeout_ms #{integer_type(backend)},
        execution_timeout_ms #{integer_type(backend)},
        execution_started_at #{timestamp_type(backend)},
        created_at #{timestamp_type(backend)} NOT NULL,
        updated_at #{timestamp_type(backend)} NOT NULL
      )
    """
  end

  defp create_results_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_analysis_results (
        job_id TEXT PRIMARY KEY REFERENCES kyuubiki_jobs(job_id) ON DELETE CASCADE,
        payload #{json_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp create_orchestra_leases_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_orchestra_leases (
        lease_name TEXT PRIMARY KEY,
        owner_instance_id TEXT NOT NULL,
        fencing_token #{integer_type(backend)} NOT NULL,
        expires_at_ms #{integer_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp create_security_events_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS kyuubiki_security_events (
        event_id TEXT PRIMARY KEY,
        event_type TEXT NOT NULL,
        source TEXT NOT NULL,
        action TEXT NOT NULL,
        risk TEXT NOT NULL,
        status TEXT NOT NULL,
        note TEXT,
        context #{json_type(backend)} NOT NULL,
        occurred_at #{timestamp_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp timestamp_type(backend), do: if(backend == :sqlite, do: "TEXT", else: "TIMESTAMPTZ")

  defp timestamp_default(backend),
    do: if(backend == :sqlite, do: "CURRENT_TIMESTAMP", else: "NOW()")

  defp json_type(backend), do: if(backend == :sqlite, do: "JSON", else: "JSONB")
  defp integer_type(backend), do: if(backend == :sqlite, do: "INTEGER", else: "BIGINT")
  defp float_type(backend), do: if(backend == :sqlite, do: "REAL", else: "DOUBLE PRECISION")
end
