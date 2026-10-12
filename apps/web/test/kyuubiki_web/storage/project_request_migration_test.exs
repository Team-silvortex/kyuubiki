defmodule KyuubikiWeb.Storage.ProjectRequestMigrationTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Storage.{DatabaseMigrations, DatabaseShape, MigrationQuery}
  alias KyuubikiWeb.TestSupport.DatabaseFixture, as: Fixture

  setup do
    db = Fixture.open!(Path.join(Fixture.directory(), "revision-two.sqlite3"))
    on_exit(fn -> Fixture.close!(db) end)
    Fixture.legacy!(db)
    DatabaseMigrations.upgrade_candidate!(db, :sqlite)
    MigrationQuery.rows!(db, "DROP TABLE kyuubiki_project_requests")
    MigrationQuery.rows!(db, "DELETE FROM kyuubiki_data_migrations WHERE version = 3")
    %{db: db}
  end

  test "revision two upgrades only the candidate and retains both immutable ledger entries", %{
    db: db
  } do
    before = MigrationQuery.rows!(db, "SELECT * FROM kyuubiki_data_migrations ORDER BY version")
    payload = MigrationQuery.rows!(db, "SELECT payload FROM kyuubiki_analysis_results")
    plan = DatabaseMigrations.plan!(db, :sqlite)
    assert plan["status"] == "upgrade_required"
    assert plan["data_version"] == 2
    assert plan["target_data_version"] == 3
    old_plan = DatabaseMigrations.plan_for_target(plan, 2)
    assert old_plan["status"] == "current"
    assert old_plan["migration_id"] == "runtime-checkpoint-requests-0002"
    assert old_plan["migration_sha256"] == DatabaseMigrations.checksum(:sqlite, 2)

    assert_raise RuntimeError, ~r/upgrade required/, fn ->
      DatabaseMigrations.boot!(db, :sqlite)
    end

    assert MigrationQuery.rows!(db, "SELECT * FROM kyuubiki_data_migrations ORDER BY version") ==
             before

    DatabaseMigrations.upgrade_candidate!(db, :sqlite)

    assert MigrationQuery.rows!(
             db,
             "SELECT * FROM kyuubiki_data_migrations WHERE version <= 2 ORDER BY version"
           ) == before

    assert MigrationQuery.rows!(db, "SELECT payload FROM kyuubiki_analysis_results") == payload
    assert DatabaseMigrations.plan!(db, :sqlite)["data_version"] == 3
    assert DatabaseShape.contracts(:sqlite)["kyuubiki_project_requests"]
  end

  test "a failed revision-three addition leaves the schema and ledger unchanged", %{db: db} do
    MigrationQuery.rows!(db, "CREATE INDEX kyuubiki_project_requests ON kyuubiki_jobs(status)")
    before = MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name")
    assert_raise MatchError, fn -> DatabaseMigrations.upgrade_candidate!(db, :sqlite) end
    assert MigrationQuery.rows!(db, "SELECT sql FROM sqlite_schema ORDER BY name") == before
    assert DatabaseMigrations.plan!(db, :sqlite)["data_version"] == 2
  end

  test "the ledger reader detects a future revision after the complete supported prefix", %{
    db: db
  } do
    DatabaseMigrations.upgrade_candidate!(db, :sqlite)

    MigrationQuery.rows!(
      db,
      "INSERT INTO kyuubiki_data_migrations VALUES (4, 'future', 'unknown', 4, '2026-10-12T00:00:00Z')"
    )

    assert_raise RuntimeError, ~r/history/, fn -> DatabaseMigrations.boot!(db, :sqlite) end
  end
end
