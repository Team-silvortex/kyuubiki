defmodule KyuubikiWeb.Api.CentralStoreApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "central catalog aggregates operators templates dsl templates and language packs" do
    conn =
      :get
      |> conn("/api/v1/central/catalog")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    entries = payload["entries"]
    summary = payload["summary"]

    assert payload["schema_version"] == "kyuubiki.central-store-catalog/v1"
    assert payload["service"] == "kyuubiki-central"
    assert summary["kinds"]["operator"] >= 8
    assert summary["kinds"]["workflow_template"] >= 3
    assert summary["kinds"]["frontend_dsl_template"] >= 1
    assert summary["kinds"]["language_pack"] == 60
    assert "language_pack" in summary["store_kinds"]

    assert Enum.any?(entries, &(&1["id"] == "solve.frame_3d"))
    assert Enum.any?(entries, &(&1["id"] == "frontend.dsl.layout_report"))
    assert Enum.any?(entries, &(&1["id"] == "workbench-fr-core-1.19"))
    assert Enum.any?(payload["sources"], &(&1["id"] == "builtin.language-packs"))
    assert payload["capabilities"]["login_system"]["status"] == "preview_contract"
  end

  test "central catalog filters and fetches language-pack entries" do
    list_conn =
      :get
      |> conn("/api/v1/central/catalog?kind=language_pack&q=zh-TW")
      |> Router.call(@opts)

    assert list_conn.status == 200
    language_packs = Jason.decode!(list_conn.resp_body)["entries"]
    assert length(language_packs) == 2
    assert Enum.all?(language_packs, &(&1["kind"] == "language_pack"))

    fetch_conn =
      :get
      |> conn("/api/v1/central/catalog/language_pack/workbench-zh-tw-core-1.19")
      |> Router.call(@opts)

    assert fetch_conn.status == 200
    entry = Jason.decode!(fetch_conn.resp_body)["entry"]
    assert entry["source_id"] == "builtin.language-packs"
    assert entry["install"]["target"] == "language-packs/workbench/zh-tw.json"
    assert entry["payload"]["language"] == "zh-TW"
  end

  test "central session policy exposes login roadmap without issuing credentials" do
    conn =
      :get
      |> conn("/api/v1/central/session-policy")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)

    assert payload["schema_version"] == "kyuubiki.central-session-policy/v1"
    assert payload["current_auth"]["mode"] == "local_orchestra_token"
    assert payload["session_rules"]["publish_requires_session"] == true

    assert payload["session_rules"]["credential_storage"] ==
             "client_platform_keychain_or_memory_only"

    assert Enum.any?(payload["planned_auth"], &(&1["id"] == "oidc"))
    assert Enum.any?(payload["planned_auth"], &(&1["id"] == "personal_access_token"))
  end

  test "central publish policy exposes resource admission requirements without accepting writes" do
    conn =
      :get
      |> conn("/api/v1/central/publish-policy")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    resource_kinds = MapSet.new(Enum.map(payload["resource_kinds"], & &1["kind"]))

    assert payload["schema_version"] == "kyuubiki.central-publish-policy/v1"
    assert payload["accepting_submissions"] == false
    assert payload["publisher_requirements"]["anonymous_publish_allowed"] == false
    assert "security_scan" in payload["review_stages"]

    assert MapSet.subset?(
             MapSet.new([
               "operator",
               "workflow_template",
               "frontend_dsl_template",
               "language_pack"
             ]),
             resource_kinds
           )

    operator = Enum.find(payload["resource_kinds"], &(&1["kind"] == "operator"))
    language_pack = Enum.find(payload["resource_kinds"], &(&1["kind"] == "language_pack"))

    assert "dynamic_smoke" in operator["required_evidence"]
    assert language_pack["manifest_schema"] == "schemas/language-pack.schema.json"
  end

  test "central database policy exposes server deployment persistence requirements" do
    conn =
      :get
      |> conn("/api/v1/central/database-policy")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    domain_ids = MapSet.new(Enum.map(payload["persistence_domains"], & &1["id"]))

    assert payload["schema_version"] == "kyuubiki.central-database-policy/v1"
    assert payload["active_backend"] in ["sqlite", "postgres", "memory"]
    assert "postgres" in payload["supported_backends"]
    assert "sqlite" in payload["supported_backends"]
    assert payload["server_test_profile"]["preferred_backend"] == "postgres"
    assert "DATABASE_URL" in payload["server_test_profile"]["required_env"]
    assert payload["migration_policy"]["destructive_changes_allowed"] == false
    assert "central_store_entries" in payload["migration_policy"]["managed_tables"]
    assert "central_artifact_signatures" in Enum.map(payload["table_specs"], & &1["name"])
    assert payload["backup_policy"]["postgres"] =~ "pg_dump"

    assert MapSet.subset?(
             MapSet.new(["catalog_entries", "publisher_accounts", "release_artifacts"]),
             domain_ids
           )
  end

  test "central catalog returns explicit not-found errors" do
    conn =
      :get
      |> conn("/api/v1/central/catalog/language_pack/missing-pack")
      |> Router.call(@opts)

    assert conn.status == 404
    payload = Jason.decode!(conn.resp_body)
    assert payload["error"] == "central_store_entry_not_found"
    assert payload["kind"] == "language_pack"
  end
end
