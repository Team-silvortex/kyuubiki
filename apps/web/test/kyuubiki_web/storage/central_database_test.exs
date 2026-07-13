defmodule KyuubikiWeb.Storage.CentralDatabaseTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Storage.CentralDatabase

  test "migration plan owns the central server preview tables" do
    plan = CentralDatabase.migration_plan()

    assert plan["schema_version"] == "kyuubiki.central-database-contract/v1"
    assert plan["mode"] == "schema_setup_preview"
    assert plan["destructive_changes_allowed"] == false

    assert MapSet.subset?(
             MapSet.new([
               "central_store_sources",
               "central_store_entries",
               "central_publishers",
               "central_publisher_tokens",
               "central_artifacts",
               "central_artifact_signatures"
             ]),
             MapSet.new(plan["managed_tables"])
           )
  end

  test "persistence domains group catalog publisher and artifact tables" do
    domains = Map.new(CentralDatabase.persistence_domains(), &{&1["id"], &1})

    assert domains["catalog_entries"]["status"] == "schema_ready_preview"
    assert "central_store_entries" in domains["catalog_entries"]["tables"]
    assert "central_publisher_tokens" in domains["publisher_accounts"]["tables"]
    assert "central_artifact_signatures" in domains["release_artifacts"]["tables"]
    assert "kyuubiki_jobs" in domains["existing_runtime_records"]["tables"]
  end

  test "create table SQL keeps credentials out of publisher token storage" do
    sql = CentralDatabase.create_table_sqls() |> Enum.join("\n")

    assert sql =~ "CREATE TABLE IF NOT EXISTS central_store_entries"
    assert sql =~ "PRIMARY KEY (kind, entry_id)"
    assert sql =~ "token_fingerprint TEXT NOT NULL"
    refute sql =~ "raw_token"
    refute sql =~ "token_secret"
  end

  test "status report exposes central schema coverage without connecting to the database" do
    report = CentralDatabase.status_report()

    assert report["schema_version"] == "kyuubiki.central-database-status/v1"
    assert report["contract_schema_version"] == "kyuubiki.central-database-contract/v1"
    assert report["managed_table_count"] == 6
    assert report["domain_count"] == 4
    assert report["coverage"]["publisher_accounts"]["table_count"] == 2
    assert "central_artifacts" in report["coverage"]["release_artifacts"]["tables"]
  end
end
