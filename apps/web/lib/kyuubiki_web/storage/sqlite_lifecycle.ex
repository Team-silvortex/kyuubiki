defmodule KyuubikiWeb.Storage.SqliteLifecycle do
  @moduledoc "Database-aware snapshots and copy upgrades. Never activates or overwrites a running store."
  alias Exqlite.Sqlite3
  alias KyuubikiWeb.Storage.{DatabaseMigrations, MigrationQuery}
  alias KyuubikiWeb.Storage.SqliteLifecycleFiles, as: Files

  def plan!(path) do
    path = Path.expand(path)
    Files.standalone!(path)
    digest = Files.digest!(path)
    plan = with_database(path, :readonly, &DatabaseMigrations.plan!(&1, :sqlite))
    Files.approve!(path, digest)
    Map.put(plan, "source_sha256", digest)
  end

  def backup!(source, output) do
    source = Path.expand(source)
    Files.regular!(source)
    Files.safe_sidecars!(source)

    Files.publish!(output, fn candidate ->
      with_database(source, :readonly, fn connection ->
        [[pages]] = MigrationQuery.rows!(connection, "PRAGMA page_count")
        [[size]] = MigrationQuery.rows!(connection, "PRAGMA page_size")
        if pages * size > Files.max_bytes(), do: raise("database exceeds snapshot safety limit")
        # SQLite supplies one transactional snapshot including committed WAL pages.
        MigrationQuery.rows!(connection, "VACUUM main INTO $1", [candidate])
      end)

      plan = checked_plan!(candidate)
      %{"operation" => "backup", "snapshot_kind" => "sqlite_vacuum_into", "database" => plan}
    end)
  end

  def upgrade!(source, output, expected_sha256),
    do: copy_operation!(source, output, expected_sha256, :upgrade)

  def restore!(source, output, expected_sha256),
    do: copy_operation!(source, output, expected_sha256, :restore)

  def verify!(path, receipt) do
    validate_receipt!(receipt)
    Files.approve!(path, receipt["output_sha256"])

    if File.stat!(path).size != receipt["output_bytes"],
      do: raise("database receipt byte count differs")

    plan = checked_plan!(path)
    # An older trusted snapshot receipt remains verifiable after adding a new reader revision.
    receipt_plan =
      DatabaseMigrations.plan_for_target(plan, receipt["database"]["target_data_version"])

    if receipt_plan != receipt["database"],
      do: raise("database plan does not match the trusted receipt")

    Files.approve!(path, receipt["output_sha256"])

    %{
      "schema_version" => "kyuubiki.database-verification/v1",
      "status" => "verified",
      "output_sha256" => receipt["output_sha256"],
      "database" => plan
    }
  end

  defp validate_receipt!(receipt) do
    unless is_map(receipt) and
             receipt["schema_version"] == "kyuubiki.sqlite-lifecycle-receipt/v1" and
             receipt["operation"] in ["backup", "upgrade", "restore"] and
             receipt["source_retained"] == true and
             receipt["activation"] == "explicit_service_switch" and
             is_boolean(receipt["directory_synced"]) and is_map(receipt["database"]) and
             is_integer(receipt["output_bytes"]) and receipt["output_bytes"] > 0 and
             receipt["output_bytes"] <= Files.max_bytes() and
             is_binary(receipt["created_at"]) and
             match?({:ok, _, _}, DateTime.from_iso8601(receipt["created_at"])) do
      raise "unsupported or incomplete database lifecycle receipt"
    end

    allowed =
      ~w(schema_version operation output_sha256 output_bytes directory_synced source_retained activation created_at database snapshot_kind source_sha256 source_data_version rollback cleanup_pending)

    if Map.keys(receipt) -- allowed != [], do: raise("unknown database receipt fields")

    if Map.has_key?(receipt, "cleanup_pending") and
         (not is_binary(receipt["cleanup_pending"]) or
            not Regex.match?(~r/\A\.kyuubiki-data-[0-9a-f]{32}\z/, receipt["cleanup_pending"])),
       do: raise("invalid database cleanup receipt")

    if receipt["operation"] == "backup" do
      unless receipt["snapshot_kind"] == "sqlite_vacuum_into",
        do: raise("invalid snapshot receipt")
    else
      unless is_binary(receipt["source_sha256"]) and
               Regex.match?(~r/\A[0-9a-f]{64}\z/, receipt["source_sha256"]) and
               receipt["source_data_version"] in 0..DatabaseMigrations.version() and
               receipt["rollback"] == "reopen_retained_snapshot_with_compatible_reader",
             do: raise("invalid database copy receipt")
    end
  end

  defp copy_operation!(source, output, expected, operation) do
    source = Path.expand(source)
    Files.approve!(source, expected)

    Files.publish!(output, fn candidate ->
      Files.copy!(source, candidate)
      Files.approve!(candidate, expected)
      Files.approve!(source, expected)
      before = checked_plan!(candidate)

      if operation == :upgrade do
        with_database(candidate, :readwrite, &DatabaseMigrations.upgrade_candidate!(&1, :sqlite))
      end

      after_plan = checked_plan!(candidate)
      Files.approve!(source, expected)

      %{
        "operation" => Atom.to_string(operation),
        "source_sha256" => expected,
        "source_data_version" => before["data_version"],
        "database" => after_plan,
        "rollback" => "reopen_retained_snapshot_with_compatible_reader"
      }
    end)
  end

  defp checked_plan!(path) do
    Files.standalone!(path)

    with_database(path, :readonly, fn connection ->
      unless MigrationQuery.rows!(connection, "PRAGMA integrity_check") == [["ok"]],
        do: raise("database integrity check failed")

      unless MigrationQuery.rows!(connection, "SELECT 1 FROM pragma_foreign_key_check LIMIT 1") ==
               [],
             do: raise("database contains broken foreign-key references")

      DatabaseMigrations.plan!(connection, :sqlite)
    end)
  end

  defp with_database(path, mode, fun) do
    Files.regular!(path)
    Files.safe_sidecars!(path)
    {:ok, db} = Sqlite3.open(path, mode: mode)

    try do
      :ok = Sqlite3.set_busy_timeout(db, 5_000)

      :ok =
        Sqlite3.execute(
          db,
          "PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL"
        )

      if mode == :readwrite, do: :ok = Sqlite3.execute(db, "PRAGMA journal_mode=DELETE")
      fun.({:sqlite, db})
    after
      Sqlite3.close(db)
    end
  end
end
