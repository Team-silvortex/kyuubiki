defmodule KyuubikiWeb.CentralStore do
  @moduledoc """
  Future center-server facade for marketplace, auth, and distributable UI assets.
  """

  alias KyuubikiWeb.AssetStore
  alias KyuubikiWeb.Security
  alias KyuubikiWeb.Storage.CentralDatabase

  @repo_root Path.expand("../../../../", __DIR__)
  @language_catalog_path Path.join(@repo_root, "language-packs/catalog.json")
  @language_pack_root Path.join(@repo_root, "language-packs")

  @store_kinds [
    "operator",
    "workflow_template",
    "frontend_dsl_template",
    "language_pack"
  ]

  def catalog(filters \\ %{}) when is_map(filters) do
    asset_catalog = AssetStore.catalog(filters)
    language_entries = language_pack_entries(filters)
    entries = asset_catalog["entries"] ++ language_entries

    %{
      "schema_version" => "kyuubiki.central-store-catalog/v1",
      "service" => "kyuubiki-central",
      "status" => "local_preview",
      "entries" => entries,
      "sources" => sources(asset_catalog["sources"]),
      "summary" => summary(entries),
      "capabilities" => capabilities()
    }
  end

  def fetch(kind, id) when kind != "language_pack" and is_binary(id) do
    AssetStore.fetch(kind, id)
  end

  def fetch("language_pack", id) when is_binary(id) do
    language_pack_entries(%{})
    |> Enum.find(&(&1["id"] == id))
    |> case do
      nil -> {:error, {:central_store_entry_not_found, "language_pack", id}}
      entry -> {:ok, %{"entry" => entry}}
    end
  end

  def session_policy do
    %{
      "schema_version" => "kyuubiki.central-session-policy/v1",
      "status" => "preview",
      "current_auth" => %{
        "mode" => "local_orchestra_token",
        "descriptor" => Security.descriptor()
      },
      "planned_auth" => [
        %{
          "id" => "oidc",
          "label" => "OpenID Connect",
          "status" => "planned",
          "intended_clients" => ["hub", "workbench", "headless_sdk"]
        },
        %{
          "id" => "device_code",
          "label" => "Device-code login",
          "status" => "planned",
          "intended_clients" => ["cli", "headless_sdk", "remote_agent"]
        },
        %{
          "id" => "personal_access_token",
          "label" => "Personal access token",
          "status" => "planned",
          "intended_clients" => ["headless_sdk", "installer", "ci"]
        }
      ],
      "session_rules" => %{
        "store_download_requires_session" => false,
        "publish_requires_session" => true,
        "agent_registration_requires_cluster_identity" => true,
        "credential_storage" => "client_platform_keychain_or_memory_only"
      }
    }
  end

  def publish_policy do
    %{
      "schema_version" => "kyuubiki.central-publish-policy/v1",
      "status" => "preview_contract",
      "accepting_submissions" => false,
      "reason" => "publisher accounts, signing keys, and provenance checks are planned",
      "resource_kinds" => Enum.map(@store_kinds, &publish_resource_policy/1),
      "review_stages" => [
        "manifest_shape",
        "schema_validation",
        "provenance_check",
        "security_scan",
        "compatibility_smoke",
        "signature_attestation",
        "catalog_indexing"
      ],
      "publisher_requirements" => %{
        "login_required" => true,
        "publisher_account_required" => true,
        "personal_access_token_supported" => true,
        "device_code_supported" => true,
        "anonymous_publish_allowed" => false
      }
    }
  end

  def publisher_policy do
    %{
      "schema_version" => "kyuubiki.central-publisher-policy/v1",
      "status" => "preview_contract",
      "accounts_enabled" => false,
      "token_issuance_enabled" => false,
      "storage_tables" => ["central_publishers", "central_publisher_tokens"],
      "identity_modes" => [
        %{
          "id" => "oidc",
          "status" => "planned",
          "clients" => ["hub", "workbench", "headless_sdk"]
        },
        %{
          "id" => "device_code",
          "status" => "planned",
          "clients" => ["cli", "installer", "headless_sdk"]
        },
        %{
          "id" => "personal_access_token",
          "status" => "planned",
          "clients" => ["ci", "installer", "headless_sdk"]
        }
      ],
      "account_lifecycle" => %{
        "allowed_statuses" => ["pending_review", "active", "suspended", "revoked"],
        "default_status" => "pending_review",
        "manual_review_required" => true,
        "anonymous_publish_allowed" => false
      },
      "token_policy" => %{
        "raw_token_storage_allowed" => false,
        "stored_secret_material" => "fingerprint_only",
        "fingerprint_storage_table" => "central_publisher_tokens",
        "rotation_required" => true,
        "revocation_supported" => true,
        "required_scopes" => [
          "central:catalog:read",
          "central:publish:submit",
          "central:artifact:upload",
          "central:artifact:yank",
          "central:security:recall"
        ]
      },
      "blocking_reasons" => [
        "hosted_login_not_enabled",
        "publisher_review_queue_not_enabled",
        "token_issuer_not_configured"
      ]
    }
  end

  def publish_readiness do
    provenance_gates = Map.new(provenance_policy()["resource_gates"], &{&1["kind"], &1})

    %{
      "schema_version" => "kyuubiki.central-publish-readiness/v1",
      "status" => "blocked_preview",
      "accepting_submissions" => false,
      "blocking_reasons" => [
        "publisher_accounts_not_enabled",
        "signing_keys_not_configured",
        "write_side_review_queue_not_enabled"
      ],
      "resource_readiness" =>
        Enum.map(@store_kinds, &publish_resource_readiness(&1, provenance_gates[&1])),
      "required_storage_tables" => [
        "central_publishers",
        "central_artifacts",
        "central_artifact_signatures"
      ],
      "next_unlocks" => [
        "enable publisher account storage, review, and token scopes",
        "configure detached artifact signing keys outside the repository",
        "enable write-side review queue after provenance smoke passes"
      ]
    }
  end

  def database_policy do
    %{
      "schema_version" => "kyuubiki.central-database-policy/v1",
      "status" => "preview_contract",
      "active_backend" => Atom.to_string(KyuubikiWeb.Storage.backend()),
      "repo_module" => repo_module_name(),
      "supported_backends" => ["sqlite", "postgres"],
      "server_test_profile" => %{
        "preferred_backend" => "postgres",
        "local_backend" => "sqlite",
        "required_env" => ["KYUUBIKI_STORAGE_BACKEND", "DATABASE_URL"],
        "smoke_commands" => [
          "MODE=cloud BACKEND=postgres make check-central-database-readiness",
          "RUN_DB_SMOKE=1 MODE=cloud BACKEND=postgres make test-central-database-smoke"
        ]
      },
      "persistence_domains" => CentralDatabase.persistence_domains(),
      "migration_policy" => CentralDatabase.migration_plan(),
      "table_specs" => CentralDatabase.table_specs(),
      "backup_policy" => %{
        "sqlite" => "copy database file after stopping writers",
        "postgres" => "pg_dump plus retained catalog manifest snapshot",
        "retention" => "deployment-defined"
      }
    }
  end

  def provenance_policy do
    %{
      "schema_version" => "kyuubiki.central-provenance-policy/v1",
      "status" => "preview_contract",
      "accepting_artifact_uploads" => false,
      "artifact_contract" => %{
        "digest_algorithms" => ["sha256", "blake3"],
        "metadata_fields" => [
          "artifact_id",
          "kind",
          "entry_id",
          "version",
          "digest",
          "size_bytes"
        ],
        "immutable_fields" => ["artifact_id", "kind", "entry_id", "version", "digest"]
      },
      "resource_gates" => Enum.map(@store_kinds, &provenance_resource_gate/1),
      "required_attestations" => [
        %{"id" => "manifest_schema", "status" => "required", "applies_to" => @store_kinds},
        %{"id" => "compatibility_smoke", "status" => "required", "applies_to" => @store_kinds},
        %{"id" => "security_scan", "status" => "required", "applies_to" => @store_kinds},
        %{"id" => "sbom", "status" => "planned_required", "applies_to" => ["operator"]},
        %{"id" => "signature", "status" => "planned_required", "applies_to" => @store_kinds}
      ],
      "signature_policy" => %{
        "mode" => "detached_signature_preview",
        "accepted_key_kinds" => ["ed25519", "minisign", "cosign"],
        "detached_signature_required" => true
      },
      "revocation_policy" => %{
        "supports_yank" => true,
        "supports_security_recall" => true,
        "retains_audit_log" => true
      },
      "download_rules" => %{
        "anonymous_download_allowed" => true,
        "checksum_required_before_install" => true,
        "installer_must_verify_signature" => true
      }
    }
  end

  def artifact_admission_policy do
    %{
      "schema_version" => "kyuubiki.central-artifact-admission-policy/v1",
      "status" => "blocked_preview",
      "accepting_uploads" => false,
      "write_endpoint_enabled" => false,
      "resource_kinds" => Enum.map(@store_kinds, &artifact_resource_admission/1),
      "artifact_envelope" => %{
        "required_fields" => [
          "artifact_id",
          "kind",
          "entry_id",
          "version",
          "digest",
          "size_bytes",
          "manifest",
          "attestations"
        ],
        "digest_algorithms" => ["sha256", "blake3"],
        "immutable_fields" => ["artifact_id", "kind", "entry_id", "version", "digest"],
        "storage_tables" => ["central_artifacts", "central_artifact_signatures"]
      },
      "publisher_token_policy" => %{
        "required_scopes" => ["central:publish:submit", "central:artifact:upload"],
        "raw_token_storage_allowed" => false,
        "fingerprint_storage_table" => "central_publisher_tokens",
        "credential_storage" => "client_platform_keychain_or_memory_only"
      },
      "review_queue" => %{
        "status" => "planned_required",
        "stages" => [
          "manifest_shape",
          "schema_validation",
          "provenance_check",
          "security_scan",
          "compatibility_smoke",
          "signature_attestation",
          "catalog_indexing"
        ],
        "manual_approval_required" => true
      },
      "blocking_reasons" => [
        "publisher_accounts_not_enabled",
        "signing_keys_not_configured",
        "write_side_review_queue_not_enabled",
        "artifact_upload_endpoint_disabled"
      ]
    }
  end

  def publish_pipeline do
    stages = publish_pipeline_stages()
    blocked = Enum.filter(stages, &(&1["status"] != "ready"))

    %{
      "schema_version" => "kyuubiki.central-publish-pipeline/v1",
      "status" => "blocked_preview",
      "mode" => "read_only_contract",
      "accepting_writes" => false,
      "stage_count" => length(stages),
      "blocked_stage_count" => length(blocked),
      "stages" => stages,
      "handoff_contracts" => [
        "kyuubiki.central-publisher-policy/v1",
        "kyuubiki.central-artifact-admission-policy/v1",
        "kyuubiki.central-provenance-policy/v1",
        "kyuubiki.central-database-contract/v1",
        "kyuubiki.central-store-catalog/v1"
      ],
      "unlock_order" => Enum.map(blocked, & &1["id"])
    }
  end

  defp provenance_resource_gate(kind) do
    publish_policy = publish_resource_policy(kind)

    %{
      "kind" => kind,
      "publish_evidence" => publish_policy["required_evidence"],
      "provenance_attestations" => provenance_attestations_for(kind),
      "installer_checks" => [
        "manifest_schema",
        "artifact_digest",
        "detached_signature",
        "compatibility_smoke"
      ],
      "storage_tables" => ["central_artifacts", "central_artifact_signatures"]
    }
  end

  defp provenance_attestations_for("operator") do
    ["manifest_schema", "compatibility_smoke", "security_scan", "sbom", "signature"]
  end

  defp provenance_attestations_for(_kind) do
    ["manifest_schema", "compatibility_smoke", "security_scan", "signature"]
  end

  defp artifact_resource_admission(kind) do
    publish_policy = publish_resource_policy(kind)
    provenance = provenance_resource_gate(kind)

    %{
      "kind" => kind,
      "status" => "blocked_preview",
      "manifest_schema" => publish_policy["manifest_schema"],
      "required_evidence" => publish_policy["required_evidence"],
      "required_attestations" => provenance["provenance_attestations"],
      "installer_checks" => provenance["installer_checks"],
      "distribution_modes" => publish_policy["distribution_modes"],
      "mutable_after_publish" => false,
      "yank_supported" => true,
      "security_recall_supported" => true
    }
  end

  defp publish_pipeline_stages do
    [
      publish_pipeline_stage(
        "publisher_identity",
        "Publisher identity and account review",
        "blocked_preview",
        "central-publisher-policy",
        ["publisher_accounts_not_enabled", "token_issuer_not_configured"]
      ),
      publish_pipeline_stage(
        "artifact_envelope",
        "Immutable artifact envelope and upload preflight",
        "blocked_preview",
        "central-artifact-admission-policy",
        ["artifact_upload_endpoint_disabled"]
      ),
      publish_pipeline_stage(
        "signature_attestation",
        "Detached artifact signature verification",
        "blocked_preview",
        "central-provenance-policy",
        ["signing_keys_not_configured"]
      ),
      publish_pipeline_stage(
        "review_queue",
        "Manual and automated review queue",
        "blocked_preview",
        "central-publish-readiness",
        ["write_side_review_queue_not_enabled"]
      ),
      publish_pipeline_stage(
        "catalog_indexing",
        "Catalog entry indexing after approval",
        "blocked_preview",
        "central-store-catalog",
        ["central_write_api_disabled"]
      ),
      publish_pipeline_stage(
        "recall_and_yank",
        "Yank and security recall controls",
        "blocked_preview",
        "central-provenance-policy",
        ["write_audit_log_not_enabled"]
      ),
      publish_pipeline_stage(
        "download_verification",
        "Installer download verification contract",
        "ready",
        "central-provenance-policy",
        []
      )
    ]
  end

  defp publish_pipeline_stage(id, label, status, contract, blockers) do
    %{
      "id" => id,
      "label" => label,
      "status" => status,
      "contract" => contract,
      "blocking_reasons" => blockers,
      "writes_enabled" => false
    }
  end

  def database_status do
    CentralDatabase.status_report()
  end

  def capabilities do
    %{
      "operator_store" => %{"status" => "ready", "backing" => "AssetStore.operator"},
      "workflow_template_store" => %{
        "status" => "ready",
        "backing" => "AssetStore.workflow_template"
      },
      "frontend_dsl_template_store" => %{
        "status" => "ready",
        "backing" => "AssetStore.frontend_dsl_template"
      },
      "language_pack_store" => %{"status" => "ready", "backing" => "language-packs/catalog.json"},
      "login_system" => %{"status" => "preview_contract", "backing" => "Security.descriptor"},
      "signed_downloads" => %{"status" => "planned"},
      "artifact_admission" => %{
        "status" => "blocked_preview",
        "backing" => "CentralStore.artifact_admission_policy"
      },
      "publish_pipeline" => %{
        "status" => "blocked_preview",
        "backing" => "CentralStore.publish_pipeline"
      },
      "publisher_accounts" => %{
        "status" => "preview_contract",
        "backing" => "CentralStore.publisher_policy"
      },
      "publish_policy" => %{
        "status" => "preview_contract",
        "backing" => "CentralStore.publish_policy"
      },
      "publish_readiness" => %{
        "status" => "blocked_preview",
        "backing" => "CentralStore.publish_readiness"
      },
      "database_policy" => %{
        "status" => "preview_contract",
        "backing" => "CentralStore.database_policy"
      },
      "provenance_policy" => %{
        "status" => "preview_contract",
        "backing" => "CentralStore.provenance_policy"
      }
    }
  end

  defp publish_resource_policy("operator") do
    %{
      "kind" => "operator",
      "manifest_schema" => "schemas/operator-package-dynamic-smoke.schema.json",
      "required_evidence" => ["manifest", "dynamic_smoke", "operator_task_ir"],
      "distribution_modes" => ["signed_archive", "orchestra_pull"],
      "mutable_after_publish" => false
    }
  end

  defp publish_resource_policy("workflow_template") do
    %{
      "kind" => "workflow_template",
      "manifest_schema" => "schemas/workflow-graph.schema.json",
      "required_evidence" => ["workflow_graph", "dataset_contract", "catalog_smoke"],
      "distribution_modes" => ["catalog_entry", "project_install"],
      "mutable_after_publish" => false
    }
  end

  defp publish_resource_policy("frontend_dsl_template") do
    %{
      "kind" => "frontend_dsl_template",
      "manifest_schema" => "kyuubiki.frontend-dsl/v1",
      "required_evidence" => ["dsl_document", "ui_automation_contract", "sandbox_review"],
      "distribution_modes" => ["project_template", "workbench_install"],
      "mutable_after_publish" => false
    }
  end

  defp publish_resource_policy("language_pack") do
    %{
      "kind" => "language_pack",
      "manifest_schema" => "schemas/language-pack.schema.json",
      "required_evidence" => ["language_pack", "locale_target", "surface_validation", "unsafe_text_scan"],
      "distribution_modes" => ["language_pack_catalog", "local_import"],
      "mutable_after_publish" => false
    }
  end

  defp publish_resource_readiness(kind, provenance_gate) do
    policy = publish_resource_policy(kind)

    %{
      "kind" => kind,
      "status" => "blocked_preview",
      "publish_evidence" => policy["required_evidence"],
      "provenance_attestations" => provenance_gate["provenance_attestations"],
      "installer_checks" => provenance_gate["installer_checks"],
      "blocking_reasons" => [
        "publisher_identity_required",
        "artifact_signature_required",
        "central_write_api_disabled"
      ]
    }
  end

  defp repo_module_name do
    case KyuubikiWeb.Storage.repo_module() do
      nil -> nil
      module -> Atom.to_string(module)
    end
  end

  defp language_pack_entries(filters) do
    filters = normalize_filters(filters)

    @language_catalog_path
    |> File.read()
    |> case do
      {:ok, raw} -> Jason.decode!(raw)["packs"] || []
      {:error, _reason} -> []
    end
    |> Enum.map(&language_pack_entry/1)
    |> Enum.filter(&matches_filters?(&1, filters))
    |> Enum.sort_by(&{&1["kind"], &1["id"]})
  rescue
    _ -> []
  end

  defp language_pack_entry(pack) do
    %{
      "id" => pack["id"],
      "kind" => "language_pack",
      "title" => pack["name"],
      "summary" => "#{pack["surface"]} language pack for #{pack["language"]}",
      "version" => current_language_pack_version(pack["path"]),
      "source_id" => "builtin.language-packs",
      "source_kind" => "builtin",
      "package_ref" => "language-pack://#{pack["surface"]}/#{pack["language"]}",
      "domain" => "localization",
      "category" => pack["surface"],
      "tags" => ["language_pack", pack["surface"], pack["language"], pack["status"]],
      "install" => %{
        "mode" => "catalog_file",
        "requires_download" => false,
        "target" => "language-packs/#{pack["path"]}"
      },
      "payload" => pack
    }
  end

  defp current_language_pack_version(relative_path) when is_binary(relative_path) do
    @language_pack_root
    |> Path.join(relative_path)
    |> File.read()
    |> case do
      {:ok, raw} -> Jason.decode!(raw)["version"] || "unknown"
      {:error, _reason} -> "unknown"
    end
  rescue
    _ -> "unknown"
  end

  defp sources(asset_sources) do
    asset_sources ++
      [
        %{
          "id" => "builtin.language-packs",
          "type" => "builtin",
          "label" => "Built-in Language Packs",
          "enabled" => true,
          "editable" => false,
          "status" => "ready",
          "supports" => ["language_pack"]
        }
      ]
  end

  defp summary(entries) do
    %{
      "entry_count" => length(entries),
      "kinds" => summarize_by(entries, "kind"),
      "sources" => summarize_by(entries, "source_id"),
      "store_kinds" => @store_kinds
    }
  end

  defp normalize_filters(filters) do
    Map.new(filters, fn {key, value} -> {to_string(key), value} end)
  end

  defp matches_filters?(entry, filters) do
    matches?(entry["kind"], filters["kind"]) and
      matches?(entry["source_id"], filters["source_id"]) and
      matches_query?(entry, filters["q"])
  end

  defp matches?(_actual, value) when value in [nil, ""], do: true
  defp matches?(actual, value), do: actual == value

  defp matches_query?(_entry, value) when value in [nil, ""], do: true

  defp matches_query?(entry, value) when is_binary(value) do
    [entry["id"], entry["title"], entry["summary"], entry["domain"], entry["category"]]
    |> Enum.concat(entry["tags"] || [])
    |> Enum.reject(&is_nil/1)
    |> Enum.join(" ")
    |> String.downcase()
    |> String.contains?(String.downcase(value))
  end

  defp summarize_by(entries, key) do
    Enum.reduce(entries, %{}, fn entry, acc ->
      Map.update(acc, entry[key] || "unknown", 1, &(&1 + 1))
    end)
  end
end
