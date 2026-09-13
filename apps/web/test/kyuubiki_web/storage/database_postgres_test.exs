defmodule KyuubikiWeb.Storage.DatabasePostgresTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.PostgresRepo, as: Repo
  alias KyuubikiWeb.Storage.{DatabaseMigrations, DatabaseShape, MigrationQuery}
  alias KyuubikiWeb.TestSupport.DatabaseFixture, as: Fixture

  @moduletag skip: System.get_env("KYUUBIKI_TEST_DATABASE_URL") in [nil, ""]

  setup do
    schema = "lifecycle_" <> Base.encode16(:crypto.strong_rand_bytes(12), case: :lower)

    {:ok, pid} =
      Repo.start_link(
        name: nil,
        url: System.fetch_env!("KYUUBIKI_TEST_DATABASE_URL"),
        pool_size: 2,
        parameters: [search_path: schema]
      )

    previous = Repo.put_dynamic_repo(pid)
    MigrationQuery.rows!(Repo, "CREATE SCHEMA #{schema}")

    on_exit(fn ->
      Repo.put_dynamic_repo(pid)
      MigrationQuery.rows!(Repo, "DROP SCHEMA #{schema} CASCADE")
      Supervisor.stop(pid)
      Repo.put_dynamic_repo(previous)
    end)

    # Let on_exit own shutdown, rather than the test process exit stopping the Repo first.
    Process.unlink(pid)
    %{pid: pid, schema: schema}
  end

  test "fresh PostgreSQL startup records the compatible revision history" do
    DatabaseMigrations.boot!(Repo, :postgres)
    assert DatabaseMigrations.plan!(Repo, :postgres)["status"] == "current"
    before = MigrationQuery.rows!(Repo, "SELECT * FROM kyuubiki_data_migrations")
    DatabaseMigrations.boot!(Repo, :postgres)
    assert MigrationQuery.rows!(Repo, "SELECT * FROM kyuubiki_data_migrations") == before
  end

  test "legacy startup refuses and explicit candidate adoption preserves JSONB and jobs" do
    Fixture.legacy!(Repo, :postgres, old_jobs: true)

    assert_raise RuntimeError, ~r/unversioned/, fn ->
      DatabaseMigrations.boot!(Repo, :postgres)
    end

    refute "kyuubiki_data_migrations" in DatabaseShape.tables!(Repo, :postgres)
    before = MigrationQuery.rows!(Repo, "SELECT payload FROM kyuubiki_analysis_results")
    DatabaseMigrations.upgrade_candidate!(Repo, :postgres)
    assert MigrationQuery.rows!(Repo, "SELECT payload FROM kyuubiki_analysis_results") == before

    assert [[nil, nil, nil, nil]] ==
             MigrationQuery.rows!(
               Repo,
               "SELECT model_version_id, queue_timeout_ms, execution_timeout_ms, execution_started_at FROM kyuubiki_jobs"
             )
  end

  test "DDL failure after column additions rolls back the full migration" do
    Fixture.legacy!(Repo, :postgres, old_jobs: true)
    MigrationQuery.rows!(Repo, "CREATE INDEX kyuubiki_data_migrations ON kyuubiki_jobs(status)")
    before = DatabaseShape.columns!(Repo, :postgres, "kyuubiki_jobs")
    assert_raise Postgrex.Error, fn -> DatabaseMigrations.upgrade_candidate!(Repo, :postgres) end
    assert DatabaseShape.columns!(Repo, :postgres, "kyuubiki_jobs") == before
    refute "kyuubiki_data_migrations" in DatabaseShape.tables!(Repo, :postgres)
  end

  test "future history, changed checksum and missing current tables fail closed" do
    DatabaseMigrations.boot!(Repo, :postgres)
    MigrationQuery.rows!(Repo, "UPDATE kyuubiki_data_migrations SET version=99 WHERE version=2")
    assert_raise RuntimeError, ~r/history/, fn -> DatabaseMigrations.boot!(Repo, :postgres) end

    MigrationQuery.rows!(
      Repo,
      "UPDATE kyuubiki_data_migrations SET version=2, checksum='bad' WHERE version=99"
    )

    assert_raise RuntimeError, ~r/checksum/, fn -> DatabaseMigrations.boot!(Repo, :postgres) end

    MigrationQuery.rows!(
      Repo,
      "UPDATE kyuubiki_data_migrations SET checksum=$1 WHERE version=2",
      [
        DatabaseMigrations.checksum(:postgres)
      ]
    )

    MigrationQuery.rows!(Repo, "DROP TABLE kyuubiki_analysis_results")

    assert_raise RuntimeError, ~r/missing managed table/, fn ->
      DatabaseMigrations.boot!(Repo, :postgres)
    end
  end

  test "database-scoped advisory lock excludes a competing migrator and releases on exit", %{
    pid: pid
  } do
    parent = self()

    holder =
      Task.async(fn ->
        Repo.put_dynamic_repo(pid)

        MigrationQuery.transaction!(Repo, :postgres, fn ->
          send(parent, :migration_locked)

          receive do
            :release -> :ok
          after
            10_000 -> raise "lock test timeout"
          end
        end)
      end)

    assert_receive :migration_locked, 5_000

    try do
      assert_raise RuntimeError, ~r/transaction lock/, fn ->
        DatabaseMigrations.boot!(Repo, :postgres)
      end
    after
      send(holder.pid, :release)
      Task.await(holder)
    end

    DatabaseMigrations.boot!(Repo, :postgres)
    assert DatabaseMigrations.plan!(Repo, :postgres)["status"] == "current"
  end
end
