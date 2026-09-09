defmodule KyuubikiWeb.Storage.DatabaseMigrations do
  @moduledoc "Versioned runtime storage gate. Application releases and data revisions are independent."
  alias KyuubikiWeb.Storage.{DatabaseShape, MigrationQuery, RuntimeSchema}

  @ledger "kyuubiki_data_migrations"
  @migration_id "runtime-baseline-0001"
  @version 1
  @checksums %{
    sqlite: "47a5d48a814f16803536b3edbaa4e15542f496b15e49ac656f62452258a6e586",
    postgres: "7655c5ee1acdb0e82367ba730d72acbd48b24a5ea420c0adc1c0ad2622d7e5a7"
  }

  def version, do: @version

  def checksum(backend) do
    # The legacy-column operation is part of the immutable baseline recipe too.
    recipe = ["#{@migration_id}:legacy-job-columns-v1" | RuntimeSchema.statements(backend)]
    actual = :crypto.hash(:sha256, Enum.join(recipe, "\n")) |> Base.encode16(case: :lower)

    if actual != Map.fetch!(@checksums, backend),
      do: raise("immutable migration recipe changed; add a new revision instead")

    actual
  end

  def plan!(connection, backend) when backend in [:sqlite, :postgres] do
    tables = DatabaseShape.tables!(connection, backend)
    ledger? = @ledger in tables
    state = if ledger?, do: :current, else: :legacy
    DatabaseShape.validate!(connection, backend, state)

    if ledger?, do: validate_ledger!(connection, backend)

    managed = Enum.filter(tables, &Map.has_key?(DatabaseShape.contracts(backend), &1))

    status =
      cond do
        ledger? -> "current"
        managed == [] and tables == [] -> "empty"
        managed == [] -> raise "database has unrelated tables but no Kyuubiki schema"
        true -> "legacy_unversioned"
      end

    %{
      "schema_version" => "kyuubiki.database-migration-plan/v1",
      "backend" => Atom.to_string(backend),
      "status" => status,
      "data_version" => if(ledger?, do: @version, else: 0),
      "target_data_version" => @version,
      "minimum_reader_version" => @version,
      "migration_id" => @migration_id,
      "migration_sha256" => checksum(backend),
      "managed_table_count" => length(managed),
      "pending_columns" =>
        if(ledger?, do: [], else: DatabaseShape.legacy_additions(connection, backend)),
      "destructive_changes_allowed" => false,
      "activation" => "explicit_service_switch"
    }
  end

  def boot!(connection, backend) do
    MigrationQuery.transaction!(connection, backend, fn ->
      case plan!(connection, backend)["status"] do
        "empty" ->
          apply_baseline!(connection, backend)

        "current" ->
          :ok

        "legacy_unversioned" ->
          raise "unversioned Kyuubiki database: stop writers, create a consistent backup, then explicitly upgrade a separate copy; see docs/data-lifecycle.html#runtime-database"
      end
    end)
  end

  # Maintenance callers own a private, verified candidate; never pass the active database.
  def upgrade_candidate!(connection, backend) do
    MigrationQuery.transaction!(connection, backend, fn ->
      case plan!(connection, backend)["status"] do
        "current" -> :ok
        _ -> apply_baseline!(connection, backend)
      end

      plan!(connection, backend)
    end)
  end

  defp apply_baseline!(connection, backend) do
    additions = DatabaseShape.legacy_additions(connection, backend)

    Enum.each(
      RuntimeSchema.statements(backend) ++ additions,
      &MigrationQuery.rows!(connection, &1)
    )

    DatabaseShape.validate!(connection, backend, :current)

    MigrationQuery.rows!(connection, """
    CREATE TABLE #{@ledger} (
      version INTEGER PRIMARY KEY,
      migration_id TEXT NOT NULL UNIQUE,
      checksum TEXT NOT NULL,
      minimum_reader_version INTEGER NOT NULL,
      applied_at TEXT NOT NULL
    )
    """)

    MigrationQuery.rows!(
      connection,
      "INSERT INTO #{@ledger} (version, migration_id, checksum, minimum_reader_version, applied_at) VALUES ($1, $2, $3, $4, $5)",
      [
        @version,
        @migration_id,
        checksum(backend),
        @version,
        DateTime.utc_now() |> DateTime.to_iso8601()
      ]
    )
  end

  defp validate_ledger!(connection, backend) do
    expected = %{
      "version" => %{type: "INTEGER", required: backend == :postgres, primary: true},
      "migration_id" => %{type: "TEXT", required: true, primary: false},
      "checksum" => %{type: "TEXT", required: true, primary: false},
      "minimum_reader_version" => %{type: "INTEGER", required: true, primary: false},
      "applied_at" => %{type: "TEXT", required: true, primary: false}
    }

    if DatabaseShape.columns!(connection, backend, @ledger) != expected,
      do: raise("unsupported database migration ledger shape")

    # Exact ordered history, not MAX(version): gaps, replacement recipes and future rows fail closed.
    case MigrationQuery.rows!(
           connection,
           "SELECT version, migration_id, checksum, minimum_reader_version, applied_at FROM #{@ledger} ORDER BY version LIMIT 2"
         ) do
      [[@version, @migration_id, digest, @version, applied_at]] ->
        if digest != checksum(backend), do: raise("database migration checksum mismatch")

        if not is_binary(applied_at) or not match?({:ok, _, _}, DateTime.from_iso8601(applied_at)),
          do: raise("invalid migration timestamp")

      _ ->
        raise "unsupported or incomplete database migration history; refusing writes"
    end
  end
end
