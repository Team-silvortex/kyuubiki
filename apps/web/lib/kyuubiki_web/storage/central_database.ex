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

  def migration_plan do
    %{
      "schema_version" => @schema_version,
      "mode" => "schema_setup_preview",
      "future_mode" => "versioned_migrations",
      "startup_schema_check" => true,
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
        "owned_kinds" => ["operator", "workflow_template", "frontend_dsl_template", "language_pack"]
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

  def create_table_sqls do
    [
      create_sources_sql(),
      create_entries_sql(),
      create_publishers_sql(),
      create_publisher_tokens_sql(),
      create_artifacts_sql(),
      create_artifact_signatures_sql()
    ]
  end

  defp tables_for(domain) do
    @tables
    |> Enum.filter(&(&1["domain"] == domain))
    |> Enum.map(& &1["name"])
  end

  defp create_sources_sql do
    """
      CREATE TABLE IF NOT EXISTS central_store_sources (
        source_id TEXT PRIMARY KEY,
        source_type TEXT NOT NULL,
        label TEXT NOT NULL,
        enabled #{boolean_type()} NOT NULL DEFAULT #{boolean_default(true)},
        status TEXT NOT NULL,
        metadata #{json_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        updated_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp create_entries_sql do
    """
      CREATE TABLE IF NOT EXISTS central_store_entries (
        kind TEXT NOT NULL,
        entry_id TEXT NOT NULL,
        source_id TEXT NOT NULL REFERENCES central_store_sources(source_id) ON DELETE CASCADE,
        title TEXT NOT NULL,
        version TEXT NOT NULL,
        package_ref TEXT,
        payload #{json_type()} NOT NULL,
        metadata #{json_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        updated_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        PRIMARY KEY (kind, entry_id)
      )
    """
  end

  defp create_publishers_sql do
    """
      CREATE TABLE IF NOT EXISTS central_publishers (
        publisher_id TEXT PRIMARY KEY,
        display_name TEXT NOT NULL,
        status TEXT NOT NULL,
        metadata #{json_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        updated_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp create_publisher_tokens_sql do
    """
      CREATE TABLE IF NOT EXISTS central_publisher_tokens (
        token_id TEXT PRIMARY KEY,
        publisher_id TEXT NOT NULL REFERENCES central_publishers(publisher_id) ON DELETE CASCADE,
        token_fingerprint TEXT NOT NULL,
        status TEXT NOT NULL,
        scopes #{json_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()},
        expires_at #{timestamp_type()}
      )
    """
  end

  defp create_artifacts_sql do
    """
      CREATE TABLE IF NOT EXISTS central_artifacts (
        artifact_id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,
        entry_id TEXT NOT NULL,
        version TEXT NOT NULL,
        storage_uri TEXT NOT NULL,
        sha256 TEXT NOT NULL,
        metadata #{json_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp create_artifact_signatures_sql do
    """
      CREATE TABLE IF NOT EXISTS central_artifact_signatures (
        signature_id TEXT PRIMARY KEY,
        artifact_id TEXT NOT NULL REFERENCES central_artifacts(artifact_id) ON DELETE CASCADE,
        key_id TEXT NOT NULL,
        signature TEXT NOT NULL,
        metadata #{json_type()} NOT NULL,
        inserted_at #{timestamp_type()} NOT NULL DEFAULT #{timestamp_default()}
      )
    """
  end

  defp timestamp_type, do: if(Storage.sqlite?(), do: "TEXT", else: "TIMESTAMPTZ")
  defp timestamp_default, do: if(Storage.sqlite?(), do: "CURRENT_TIMESTAMP", else: "NOW()")
  defp json_type, do: if(Storage.sqlite?(), do: "JSON", else: "JSONB")
  defp boolean_type, do: if(Storage.sqlite?(), do: "INTEGER", else: "BOOLEAN")
  defp boolean_default(true), do: if(Storage.sqlite?(), do: "1", else: "TRUE")
end
