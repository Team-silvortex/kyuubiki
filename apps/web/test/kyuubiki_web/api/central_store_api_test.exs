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
    assert payload["capabilities"]["publisher_accounts"]["status"] == "preview_contract"
    assert payload["capabilities"]["artifact_admission"]["status"] == "blocked_preview"
    assert payload["capabilities"]["publish_pipeline"]["status"] == "blocked_preview"
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

  test "central publisher policy exposes account and token safety without issuance" do
    conn =
      :get
      |> conn("/api/v1/central/publisher-policy")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    modes = MapSet.new(Enum.map(payload["identity_modes"], & &1["id"]))

    assert payload["schema_version"] == "kyuubiki.central-publisher-policy/v1"
    assert payload["accounts_enabled"] == false
    assert payload["token_issuance_enabled"] == false
    assert "central_publishers" in payload["storage_tables"]
    assert "central_publisher_tokens" in payload["storage_tables"]
    assert payload["account_lifecycle"]["manual_review_required"] == true
    assert payload["account_lifecycle"]["anonymous_publish_allowed"] == false
    assert payload["token_policy"]["raw_token_storage_allowed"] == false
    assert payload["token_policy"]["stored_secret_material"] == "fingerprint_only"
    assert payload["token_policy"]["revocation_supported"] == true
    assert "central:security:recall" in payload["token_policy"]["required_scopes"]
    assert "hosted_login_not_enabled" in payload["blocking_reasons"]
    assert MapSet.subset?(MapSet.new(["oidc", "device_code", "personal_access_token"]), modes)
  end

  test "central publish readiness explains preview blockers per resource kind" do
    conn =
      :get
      |> conn("/api/v1/central/publish-readiness")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    resource_readiness = Map.new(payload["resource_readiness"], &{&1["kind"], &1})

    assert payload["schema_version"] == "kyuubiki.central-publish-readiness/v1"
    assert payload["accepting_submissions"] == false
    assert payload["status"] == "blocked_preview"
    assert "central_artifact_signatures" in payload["required_storage_tables"]
    assert "signing_keys_not_configured" in payload["blocking_reasons"]
    assert "artifact_signature_required" in resource_readiness["operator"]["blocking_reasons"]
    assert "sbom" in resource_readiness["operator"]["provenance_attestations"]
    assert "language_pack" in Map.keys(resource_readiness)
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

  test "central provenance policy exposes download verification requirements without uploads" do
    conn =
      :get
      |> conn("/api/v1/central/provenance-policy")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    attestations = MapSet.new(Enum.map(payload["required_attestations"], & &1["id"]))
    resource_gates = Map.new(payload["resource_gates"], &{&1["kind"], &1})

    assert payload["schema_version"] == "kyuubiki.central-provenance-policy/v1"
    assert payload["accepting_artifact_uploads"] == false
    assert "sha256" in payload["artifact_contract"]["digest_algorithms"]
    assert "digest" in payload["artifact_contract"]["immutable_fields"]
    assert payload["signature_policy"]["detached_signature_required"] == true
    assert payload["download_rules"]["checksum_required_before_install"] == true
    assert payload["revocation_policy"]["supports_security_recall"] == true

    assert resource_gates["operator"]["storage_tables"] == [
             "central_artifacts",
             "central_artifact_signatures"
           ]

    assert "sbom" in resource_gates["operator"]["provenance_attestations"]
    assert "detached_signature" in resource_gates["workflow_template"]["installer_checks"]

    assert MapSet.subset?(
             MapSet.new(["manifest_schema", "compatibility_smoke", "security_scan", "signature"]),
             attestations
           )
  end

  test "central artifact admission policy exposes upload preflight without write access" do
    conn =
      :get
      |> conn("/api/v1/central/artifact-admission-policy")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    resource_kinds = Map.new(payload["resource_kinds"], &{&1["kind"], &1})

    assert payload["schema_version"] == "kyuubiki.central-artifact-admission-policy/v1"
    assert payload["accepting_uploads"] == false
    assert payload["write_endpoint_enabled"] == false
    assert "artifact_upload_endpoint_disabled" in payload["blocking_reasons"]
    assert "digest" in payload["artifact_envelope"]["immutable_fields"]
    assert "central_artifact_signatures" in payload["artifact_envelope"]["storage_tables"]
    assert payload["publisher_token_policy"]["raw_token_storage_allowed"] == false
    assert "central:artifact:upload" in payload["publisher_token_policy"]["required_scopes"]
    assert payload["review_queue"]["manual_approval_required"] == true
    assert "sbom" in resource_kinds["operator"]["required_attestations"]
    assert resource_kinds["language_pack"]["security_recall_supported"] == true
  end

  test "central publish pipeline exposes ordered write-side blockers without writes" do
    conn =
      :get
      |> conn("/api/v1/central/publish-pipeline")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    stage_ids = Enum.map(payload["stages"], & &1["id"])
    stages = Map.new(payload["stages"], &{&1["id"], &1})

    assert payload["schema_version"] == "kyuubiki.central-publish-pipeline/v1"
    assert payload["mode"] == "read_only_contract"
    assert payload["accepting_writes"] == false
    assert payload["stage_count"] == length(payload["stages"])
    assert payload["blocked_stage_count"] == length(payload["unlock_order"])
    assert "kyuubiki.central-provenance-policy/v1" in payload["handoff_contracts"]

    assert stage_ids == [
             "publisher_identity",
             "artifact_envelope",
             "signature_attestation",
             "review_queue",
             "catalog_indexing",
             "recall_and_yank",
             "download_verification"
           ]

    assert "signing_keys_not_configured" in stages["signature_attestation"]["blocking_reasons"]
    assert stages["download_verification"]["status"] == "ready"
    assert Enum.all?(payload["stages"], &(&1["writes_enabled"] == false))
  end

  test "central database status exposes schema preview coverage" do
    conn =
      :get
      |> conn("/api/v1/central/database-status")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)

    assert payload["schema_version"] == "kyuubiki.central-database-status/v1"
    assert payload["contract_schema_version"] == "kyuubiki.central-database-contract/v1"
    assert payload["backend"] in ["sqlite", "postgres", "memory"]
    assert payload["managed_table_count"] == 6
    assert "central_store_entries" in payload["managed_tables"]
    assert payload["coverage"]["catalog_entries"]["status"] == "schema_ready_preview"
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
