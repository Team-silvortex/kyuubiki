defmodule KyuubikiWeb.Storage.DatabaseMigrationsTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Storage.{DatabaseMigrations, DatabaseShape, MigrationQuery, RuntimeSchema}
  alias KyuubikiWeb.TestSupport.DatabaseFixture, as: Fixture

  setup do
    path = Path.join(Fixture.directory(), "database.sqlite3")
    connection = Fixture.open!(path)
    on_exit(fn -> Fixture.close!(connection) end)
    %{connection: connection, path: path}
  end

  test "empty startup initializes the complete atomic history and repeated startup is idempotent",
       %{
         connection: db
       } do
    assert DatabaseMigrations.plan!(db, :sqlite)["status"] == "empty"
    DatabaseMigrations.boot!(db, :sqlite)
    history = MigrationQuery.rows!(db, "SELECT * FROM kyuubiki_data_migrations")
    DatabaseMigrations.boot!(db, :sqlite)
    assert DatabaseMigrations.plan!(db, :sqlite)["status"] == "current"
    assert MigrationQuery.rows!(db, "SELECT * FROM kyuubiki_data_migrations") == history
    assert length(history) == 2
  end

  test "legacy startup refuses before any schema or data mutation", %{connection: db} do
    Fixture.legacy!(db, :sqlite, old_jobs: true)
    before = MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name")

    error =
      assert_raise RuntimeError, ~r/unversioned/, fn -> DatabaseMigrations.boot!(db, :sqlite) end

    assert error.message =~ "fresh empty database for disposable development data"
    assert error.message =~ "retain valuable data"
    assert MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name") == before
    assert [["Thermal study"]] == MigrationQuery.rows!(db, "SELECT name FROM kyuubiki_projects")
  end

  test "legacy migration records a baseline and preserves scientific payloads", %{connection: db} do
    Fixture.legacy!(db, :sqlite, old_jobs: true)
    payload = MigrationQuery.rows!(db, "SELECT payload FROM kyuubiki_analysis_results")
    assert length(DatabaseMigrations.plan!(db, :sqlite)["pending_columns"]) == 4
    DatabaseMigrations.upgrade_candidate!(db, :sqlite)
    assert MigrationQuery.rows!(db, "SELECT payload FROM kyuubiki_analysis_results") == payload

    assert [[nil, nil, nil, nil]] ==
             MigrationQuery.rows!(
               db,
               "SELECT model_version_id, queue_timeout_ms, execution_timeout_ms, execution_started_at FROM kyuubiki_jobs"
             )
  end

  test "failure after ALTER rolls back all schema changes and leaves no partial ledger", %{
    connection: db
  } do
    Fixture.legacy!(db, :sqlite, old_jobs: true)
    # The table namespace check passes; a colliding index forces late CREATE TABLE failure.
    MigrationQuery.rows!(db, "CREATE INDEX kyuubiki_data_migrations ON kyuubiki_jobs(status)")
    before = MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name")
    assert_raise MatchError, fn -> DatabaseMigrations.upgrade_candidate!(db, :sqlite) end
    assert MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name") == before
    refute "kyuubiki_data_migrations" in DatabaseShape.tables!(db, :sqlite)
  end

  test "future and incomplete histories fail without repair", %{connection: db} do
    DatabaseMigrations.boot!(db, :sqlite)
    MigrationQuery.rows!(db, "UPDATE kyuubiki_data_migrations SET version = 99 WHERE version = 2")
    assert_raise RuntimeError, ~r/history/, fn -> DatabaseMigrations.boot!(db, :sqlite) end
    MigrationQuery.rows!(db, "DELETE FROM kyuubiki_data_migrations")
    assert_raise RuntimeError, ~r/history/, fn -> DatabaseMigrations.boot!(db, :sqlite) end
  end

  test "checksum and minimum reader tampering are rejected", %{connection: db} do
    DatabaseMigrations.boot!(db, :sqlite)

    MigrationQuery.rows!(
      db,
      "UPDATE kyuubiki_data_migrations SET checksum = 'changed' WHERE version = 2"
    )

    assert_raise RuntimeError, ~r/checksum/, fn -> DatabaseMigrations.boot!(db, :sqlite) end

    MigrationQuery.rows!(
      db,
      "UPDATE kyuubiki_data_migrations SET checksum = $1, minimum_reader_version = 10 WHERE version = 2",
      [DatabaseMigrations.checksum(:sqlite)]
    )

    assert_raise RuntimeError, ~r/history/, fn -> DatabaseMigrations.boot!(db, :sqlite) end
  end

  test "current missing table is not silently recreated", %{connection: db} do
    DatabaseMigrations.boot!(db, :sqlite)
    MigrationQuery.rows!(db, "DROP TABLE kyuubiki_analysis_results")

    assert_raise RuntimeError, ~r/missing managed table/, fn ->
      DatabaseMigrations.boot!(db, :sqlite)
    end

    refute "kyuubiki_analysis_results" in DatabaseShape.tables!(db, :sqlite)
  end

  test "malformed managed shape and unknown managed tables fail closed", %{connection: db} do
    MigrationQuery.rows!(
      db,
      "CREATE TABLE kyuubiki_projects (project_id TEXT PRIMARY KEY, name INTEGER)"
    )

    assert_raise RuntimeError, fn -> DatabaseMigrations.boot!(db, :sqlite) end
    MigrationQuery.rows!(db, "CREATE TABLE kyuubiki_future_state (id TEXT)")

    assert_raise RuntimeError, ~r/unknown managed table/, fn ->
      DatabaseMigrations.boot!(db, :sqlite)
    end
  end

  test "unrelated database is not silently claimed", %{connection: db} do
    MigrationQuery.rows!(db, "CREATE TABLE unrelated(id INTEGER)")
    assert_raise RuntimeError, ~r/unrelated/, fn -> DatabaseMigrations.boot!(db, :sqlite) end
  end

  test "a missing foreign-key declaration is not certified as the same baseline", %{
    connection: db
  } do
    Fixture.legacy!(db)
    MigrationQuery.rows!(db, "DROP TABLE kyuubiki_analysis_results")

    MigrationQuery.rows!(
      db,
      "CREATE TABLE kyuubiki_analysis_results(job_id TEXT PRIMARY KEY, payload JSON NOT NULL, inserted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)"
    )

    assert_raise RuntimeError, ~r/foreign-key contract/, fn ->
      DatabaseMigrations.boot!(db, :sqlite)
    end
  end

  test "unexpected ledger columns and invalid timestamps are refused", %{connection: db} do
    DatabaseMigrations.boot!(db, :sqlite)
    MigrationQuery.rows!(db, "UPDATE kyuubiki_data_migrations SET applied_at='not-time'")
    assert_raise RuntimeError, ~r/timestamp/, fn -> DatabaseMigrations.boot!(db, :sqlite) end
    MigrationQuery.rows!(db, "ALTER TABLE kyuubiki_data_migrations ADD COLUMN future_format TEXT")
    assert_raise RuntimeError, ~r/ledger shape/, fn -> DatabaseMigrations.boot!(db, :sqlite) end
  end

  test "baseline covers both backend types without consulting global storage config" do
    assert length(RuntimeSchema.statements(:sqlite)) == 13
    assert length(RuntimeSchema.statements(:postgres)) == 13
    assert DatabaseShape.contracts(:sqlite)["kyuubiki_jobs"]["progress"].type == "REAL"

    assert DatabaseShape.contracts(:postgres)["kyuubiki_jobs"]["progress"].type ==
             "DOUBLE PRECISION"

    refute DatabaseMigrations.checksum(:sqlite) == DatabaseMigrations.checksum(:postgres)
  end

  test "Ecto startup takes the same gate and atomically serializes competing bootstraps", %{
    path: path,
    connection: raw
  } do
    Fixture.close!(raw)
    repo = KyuubikiWeb.SqliteRepo

    {:ok, pid} =
      repo.start_link(
        name: nil,
        database: path,
        pool_size: 2,
        busy_timeout: 5_000,
        journal_mode: :delete
      )

    try do
      tasks =
        for _ <- 1..2 do
          Task.async(fn ->
            repo.put_dynamic_repo(pid)
            DatabaseMigrations.boot!(repo, :sqlite)
          end)
        end

      Enum.each(tasks, &Task.await(&1, 15_000))
      previous = repo.put_dynamic_repo(pid)

      try do
        assert [[2]] ==
                 MigrationQuery.rows!(repo, "SELECT count(*) FROM kyuubiki_data_migrations")
      after
        repo.put_dynamic_repo(previous)
      end
    after
      Supervisor.stop(pid)
    end
  end

  test "revision one requires an explicit additive upgrade and retains the immutable baseline", %{
    connection: db
  } do
    DatabaseMigrations.boot!(db, :sqlite)
    MigrationQuery.rows!(db, "DROP TABLE kyuubiki_checkpoint_requests")
    MigrationQuery.rows!(db, "DELETE FROM kyuubiki_data_migrations WHERE version = 2")
    baseline = MigrationQuery.rows!(db, "SELECT * FROM kyuubiki_data_migrations")
    before = MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name")
    assert DatabaseMigrations.plan!(db, :sqlite)["status"] == "upgrade_required"

    assert_raise RuntimeError, ~r/upgrade required/, fn ->
      DatabaseMigrations.boot!(db, :sqlite)
    end

    assert MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name") == before
    DatabaseMigrations.upgrade_candidate!(db, :sqlite)

    assert MigrationQuery.rows!(db, "SELECT * FROM kyuubiki_data_migrations WHERE version = 1") ==
             baseline

    assert DatabaseMigrations.plan!(db, :sqlite)["data_version"] == 2
    assert "kyuubiki_checkpoint_requests" in DatabaseShape.tables!(db, :sqlite)
  end

  test "failure adding receipt storage rolls back an explicit revision-one upgrade", %{
    connection: db
  } do
    DatabaseMigrations.boot!(db, :sqlite)
    MigrationQuery.rows!(db, "DROP TABLE kyuubiki_checkpoint_requests")
    MigrationQuery.rows!(db, "DELETE FROM kyuubiki_data_migrations WHERE version = 2")
    MigrationQuery.rows!(db, "CREATE INDEX kyuubiki_checkpoint_requests ON kyuubiki_jobs(status)")
    before = MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name")
    assert_raise MatchError, fn -> DatabaseMigrations.upgrade_candidate!(db, :sqlite) end
    assert MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name") == before
    assert DatabaseMigrations.plan!(db, :sqlite)["data_version"] == 1
  end
end
