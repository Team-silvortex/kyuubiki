defmodule KyuubikiWeb.Storage.CentralDatabase do
  @moduledoc """
  Database contract for the future central server surface.

  The central server remains read-only for publishing, but its table boundary is
  explicit so remote deployments can validate storage readiness early.
  """

  alias KyuubikiWeb.Storage

  @schema_version "kyuubiki.central-database-contract/v1"

  @tables [
    %{
      "name" => "central_store_sources",
      "domain" => "catalog_entries",
      "purpose" => "catalog source registry and source-level sync metadata"
    },
    %{
      "name" => "central_store_entries",
      "domain" => "catalog_entries",
      "purpose" => "operator, workflow template, frontend DSL, and language-pack catalog entries"
    },
    %{
      "name" => "central_publishers",
      "domain" => "publisher_accounts",
      "purpose" => "publisher identity records before account login is enabled"
    },
    %{
      "name" => "central_publisher_tokens",
      "domain" => "publisher_accounts",
      "purpose" => "hashed publisher token metadata without storing raw credentials"
    },
    %{
      "name" => "central_artifacts",
      "domain" => "release_artifacts",
      "purpose" => "downloadable artifacts, package versions, and retained checksums"
    },
    %{
      "name" => "central_artifact_signatures",
      "domain" => "release_artifacts",
      "purpose" => "artifact signature attestations and signing key references"
    }
  ]

  def schema_version, do: @schema_version

  def table_specs, do: @tables

  def status_report do
    managed_tables = Enum.map(@tables, & &1["name"])
    domains = persistence_domains()

    %{
      "schema_version" => "kyuubiki.central-database-status/v1",
      "contract_schema_version" => @schema_version,
      "status" => if(Storage.sql?(), do: "schema_ready_preview", else: "memory_preview"),
      "backend" => Atom.to_string(Storage.backend()),
      "sql_enabled" => Storage.sql?(),
      "repo_module" => repo_module_name(),
      "managed_table_count" => length(managed_tables),
      "managed_tables" => managed_tables,
      "domain_count" => length(domains),
      "domains" => domains,
      "coverage" => %{
        "catalog_entries" => coverage_for("catalog_entries"),
        "publisher_accounts" => coverage_for("publisher_accounts"),
        "release_artifacts" => coverage_for("release_artifacts")
      }
    }
  end

  def migration_plan do
    %{
      "schema_version" => @schema_version,
      "mode" => "versioned_migrations",
      "future_mode" => "versioned_migrations",
      "startup_schema_check" => true,
      "data_version" => KyuubikiWeb.Storage.DatabaseMigrations.version(),
      "legacy_startup" => "refuse_until_explicit_copy_upgrade",
      "ledger_table" => "kyuubiki_data_migrations",
      "destructive_changes_allowed" => false,
      "managed_tables" => Enum.map(@tables, & &1["name"])
    }
  end

  def persistence_domains do
    [
      %{
        "id" => "catalog_entries",
        "status" => "schema_ready_preview",
        "tables" => tables_for("catalog_entries"),
        "owned_kinds" => [
          "operator",
          "workflow_template",
          "frontend_dsl_template",
          "language_pack"
        ]
      },
      %{
        "id" => "publisher_accounts",
        "status" => "schema_ready_preview",
        "tables" => tables_for("publisher_accounts"),
        "owned_kinds" => ["publisher", "token"]
      },
      %{
        "id" => "release_artifacts",
        "status" => "schema_ready_preview",
        "tables" => tables_for("release_artifacts"),
        "owned_kinds" => ["artifact", "signature"]
      },
      %{
        "id" => "existing_runtime_records",
        "status" => "ready",
        "tables" => [
          "kyuubiki_projects",
          "kyuubiki_models",
          "kyuubiki_model_versions",
          "kyuubiki_jobs",
          "kyuubiki_analysis_results",
          "kyuubiki_security_events"
        ],
        "owned_kinds" => ["project", "model", "job", "result", "security_event"]
      }
    ]
  end

  def create_table_sqls(backend \\ Storage.backend()) do
    [
      create_sources_sql(backend),
      create_entries_sql(backend),
      create_publishers_sql(backend),
      create_publisher_tokens_sql(backend),
      create_artifacts_sql(backend),
      create_artifact_signatures_sql(backend)
    ]
  end

  defp tables_for(domain) do
    @tables
    |> Enum.filter(&(&1["domain"] == domain))
    |> Enum.map(& &1["name"])
  end

  defp coverage_for(domain) do
    tables = tables_for(domain)

    %{
      "domain" => domain,
      "table_count" => length(tables),
      "tables" => tables,
      "status" => if(tables == [], do: "missing", else: "schema_ready_preview")
    }
  end

  defp repo_module_name do
    case Storage.repo_module() do
      nil -> nil
      module -> Atom.to_string(module)
    end
  end

  defp create_sources_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS central_store_sources (
        source_id TEXT PRIMARY KEY,
        source_type TEXT NOT NULL,
        label TEXT NOT NULL,
        enabled #{boolean_type(backend)} NOT NULL DEFAULT #{boolean_default(backend)},
        status TEXT NOT NULL,
        metadata #{json_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp create_entries_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS central_store_entries (
        kind TEXT NOT NULL,
        entry_id TEXT NOT NULL,
        source_id TEXT NOT NULL REFERENCES central_store_sources(source_id) ON DELETE CASCADE,
        title TEXT NOT NULL,
        version TEXT NOT NULL,
        package_ref TEXT,
        payload #{json_type(backend)} NOT NULL,
        metadata #{json_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        PRIMARY KEY (kind, entry_id)
      )
    """
  end

  defp create_publishers_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS central_publishers (
        publisher_id TEXT PRIMARY KEY,
        display_name TEXT NOT NULL,
        status TEXT NOT NULL,
        metadata #{json_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        updated_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp create_publisher_tokens_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS central_publisher_tokens (
        token_id TEXT PRIMARY KEY,
        publisher_id TEXT NOT NULL REFERENCES central_publishers(publisher_id) ON DELETE CASCADE,
        token_fingerprint TEXT NOT NULL,
        status TEXT NOT NULL,
        scopes #{json_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)},
        expires_at #{timestamp_type(backend)}
      )
    """
  end

  defp create_artifacts_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS central_artifacts (
        artifact_id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,
        entry_id TEXT NOT NULL,
        version TEXT NOT NULL,
        storage_uri TEXT NOT NULL,
        sha256 TEXT NOT NULL,
        metadata #{json_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp create_artifact_signatures_sql(backend) do
    """
      CREATE TABLE IF NOT EXISTS central_artifact_signatures (
        signature_id TEXT PRIMARY KEY,
        artifact_id TEXT NOT NULL REFERENCES central_artifacts(artifact_id) ON DELETE CASCADE,
        key_id TEXT NOT NULL,
        signature TEXT NOT NULL,
        metadata #{json_type(backend)} NOT NULL,
        inserted_at #{timestamp_type(backend)} NOT NULL DEFAULT #{timestamp_default(backend)}
      )
    """
  end

  defp timestamp_type(backend), do: if(backend == :sqlite, do: "TEXT", else: "TIMESTAMPTZ")

  defp timestamp_default(backend),
    do: if(backend == :sqlite, do: "CURRENT_TIMESTAMP", else: "NOW()")

  defp json_type(backend), do: if(backend == :sqlite, do: "JSON", else: "JSONB")
  defp boolean_type(backend), do: if(backend == :sqlite, do: "INTEGER", else: "BOOLEAN")
  defp boolean_default(backend), do: if(backend == :sqlite, do: "1", else: "TRUE")
end
