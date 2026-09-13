defmodule KyuubikiWeb.Storage.DatabaseMigrations do
  @moduledoc "Versioned runtime storage gate. Application releases and data revisions are independent."
  alias KyuubikiWeb.Storage.{CheckpointSchema, DatabaseShape, MigrationQuery, RuntimeSchema}

  @ledger "kyuubiki_data_migrations"
  @migration_id "runtime-baseline-0001"
  @checkpoint_id "runtime-checkpoint-requests-0002"
  @version 2
  @checksums %{
    sqlite: "47a5d48a814f16803536b3edbaa4e15542f496b15e49ac656f62452258a6e586",
    postgres: "7655c5ee1acdb0e82367ba730d72acbd48b24a5ea420c0adc1c0ad2622d7e5a7"
  }
  @checkpoint_checksums %{
    sqlite: "96724e85166deffcedc3fc08d9491635097cab3e10f0ffd7a872c9f64a3452cb",
    postgres: "68a00b919521c676e60fb4b3508ca2e44e23471154da2c411efe911acfc4eb1e"
  }

  def version, do: @version

  def plan_for_target(plan, target) when target in 1..@version do
    if plan["data_version"] > target, do: raise("database is newer than the receipt's reader")

    backend =
      case plan["backend"] do
        "sqlite" -> :sqlite
        "postgres" -> :postgres
      end

    Map.merge(plan, %{
      "target_data_version" => target,
      "minimum_reader_version" => target,
      "migration_id" => if(target == 1, do: @migration_id, else: @checkpoint_id),
      "migration_sha256" => checksum(backend, target),
      "status" => if(plan["data_version"] == target, do: "current", else: plan["status"])
    })
  end

  def plan_for_target(_plan, _target), do: raise("unsupported receipt migration target")

  def checksum(backend, version \\ @version)

  def checksum(backend, 1) do
    # The legacy-column operation is part of the immutable baseline recipe too.
    recipe = ["#{@migration_id}:legacy-job-columns-v1" | RuntimeSchema.statements(backend)]
    actual = :crypto.hash(:sha256, Enum.join(recipe, "\n")) |> Base.encode16(case: :lower)

    if actual != Map.fetch!(@checksums, backend),
      do: raise("immutable migration recipe changed; add a new revision instead")

    actual
  end

  def checksum(backend, 2) do
    recipe = [@checkpoint_id | CheckpointSchema.statements(backend)]
    actual = :crypto.hash(:sha256, Enum.join(recipe, "\n")) |> Base.encode16(case: :lower)

    if actual != Map.fetch!(@checkpoint_checksums, backend),
      do: raise("immutable migration recipe changed; add a new revision instead")

    actual
  end

  def plan!(connection, backend) when backend in [:sqlite, :postgres] do
    tables = DatabaseShape.tables!(connection, backend)
    ledger? = @ledger in tables
    data_version = if ledger?, do: validate_ledger!(connection, backend), else: 0
    state = if ledger?, do: :current, else: :legacy
    DatabaseShape.validate!(connection, backend, state, max(data_version, 1))

    managed = Enum.filter(tables, &Map.has_key?(DatabaseShape.contracts(backend), &1))

    status =
      cond do
        data_version == @version -> "current"
        ledger? -> "upgrade_required"
        managed == [] and tables == [] -> "empty"
        managed == [] -> raise "database has unrelated tables but no Kyuubiki schema"
        true -> "legacy_unversioned"
      end

    %{
      "schema_version" => "kyuubiki.database-migration-plan/v1",
      "backend" => Atom.to_string(backend),
      "status" => status,
      "data_version" => data_version,
      "target_data_version" => @version,
      "minimum_reader_version" => @version,
      "migration_id" => @checkpoint_id,
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
          apply_checkpoint!(connection, backend)

        "current" ->
          :ok

        "upgrade_required" ->
          raise "database upgrade required: stop writers and explicitly copy-upgrade before switching services"

        "legacy_unversioned" ->
          raise "unversioned Kyuubiki database: stop writers, select a fresh empty database for disposable development data, " <>
                  "or explicitly copy-upgrade to retain valuable data; see docs/data-lifecycle.html#development-retention"
      end
    end)
  end

  # Maintenance callers own a private, verified candidate; never pass the active database.
  def upgrade_candidate!(connection, backend) do
    MigrationQuery.transaction!(connection, backend, fn ->
      case plan!(connection, backend)["status"] do
        "current" ->
          :ok

        "upgrade_required" ->
          apply_checkpoint!(connection, backend)

        _ ->
          apply_baseline!(connection, backend)
          apply_checkpoint!(connection, backend)
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

    DatabaseShape.validate!(connection, backend, :current, 1)

    MigrationQuery.rows!(connection, """
    CREATE TABLE #{@ledger} (
      version INTEGER PRIMARY KEY,
      migration_id TEXT NOT NULL UNIQUE,
      checksum TEXT NOT NULL,
      minimum_reader_version INTEGER NOT NULL,
      applied_at TEXT NOT NULL
    )
    """)

    record_migration!(connection, backend, 1, @migration_id)
  end

  defp apply_checkpoint!(connection, backend) do
    Enum.each(CheckpointSchema.statements(backend), &MigrationQuery.rows!(connection, &1))
    DatabaseShape.validate!(connection, backend, :current, 2)
    record_migration!(connection, backend, 2, @checkpoint_id)
  end

  defp record_migration!(connection, backend, version, id) do
    MigrationQuery.rows!(
      connection,
      "INSERT INTO #{@ledger} (version, migration_id, checksum, minimum_reader_version, applied_at) VALUES ($1, $2, $3, $4, $5)",
      [
        version,
        id,
        checksum(backend, version),
        version,
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

    # Only a complete known prefix is upgradeable; gaps and future revisions fail closed.
    rows =
      MigrationQuery.rows!(
        connection,
        "SELECT version, migration_id, checksum, minimum_reader_version, applied_at FROM #{@ledger} ORDER BY version LIMIT 3"
      )

    if rows == [] or length(rows) > @version,
      do: raise("unsupported or incomplete database migration history; refusing writes")

    Enum.each(Enum.with_index(rows, 1), fn {row, expected} ->
      case row do
        [^expected, id, digest, ^expected, applied_at] ->
          expected_id = if expected == 1, do: @migration_id, else: @checkpoint_id
          if id != expected_id, do: raise("unsupported database migration history")

          if digest != checksum(backend, expected),
            do: raise("database migration checksum mismatch")

          if not is_binary(applied_at) or
               not match?({:ok, _, _}, DateTime.from_iso8601(applied_at)),
             do: raise("invalid migration timestamp")

        _ ->
          raise "unsupported or incomplete database migration history; refusing writes"
      end
    end)

    length(rows)
  end
end
