export type CentralStoreEntryKind =
  | "operator"
  | "workflow_template"
  | "frontend_dsl_template"
  | "language_pack";

export type CentralStoreSource = {
  id: string;
  type: string;
  label: string;
  enabled: boolean;
  editable: boolean;
  status: "ready" | "configured" | "unavailable" | string;
  url?: string | null;
  path?: string | null;
  supports: CentralStoreEntryKind[];
};

export type CentralStoreEntry = {
  id: string;
  kind: CentralStoreEntryKind;
  title: string;
  summary?: string | null;
  version?: string | null;
  source_id: string;
  source_kind: string;
  package_ref?: string | null;
  domain?: string | null;
  category?: string | null;
  tags: string[];
  install: {
    mode: string;
    requires_download: boolean;
    target?: string | null;
  };
  payload?: Record<string, unknown>;
};

export type CentralStoreCapabilities = {
  operator_store: { status: string; backing?: string };
  workflow_template_store: { status: string; backing?: string };
  frontend_dsl_template_store: { status: string; backing?: string };
  language_pack_store: { status: string; backing?: string };
  login_system: { status: string; backing?: string };
  signed_downloads: { status: string };
  artifact_admission: { status: string; backing?: string };
  publisher_accounts: { status: string; backing?: string };
  publish_policy: { status: string; backing?: string };
  publish_readiness: { status: string; backing?: string };
  database_policy: { status: string; backing?: string };
  provenance_policy: { status: string; backing?: string };
};

export type CentralStoreCatalogPayload = {
  schema_version: "kyuubiki.central-store-catalog/v1";
  service: "kyuubiki-central";
  status: string;
  entries: CentralStoreEntry[];
  sources: CentralStoreSource[];
  summary: {
    entry_count: number;
    kinds: Partial<Record<CentralStoreEntryKind, number>>;
    sources: Record<string, number>;
    store_kinds: CentralStoreEntryKind[];
  };
  capabilities: CentralStoreCapabilities;
};

export type CentralStoreEntryEnvelope = {
  entry: CentralStoreEntry;
};

export type CentralAuthProvider = {
  id: "oidc" | "device_code" | "personal_access_token" | string;
  label: string;
  status: "planned" | "preview" | "ready" | string;
  intended_clients: string[];
};

export type CentralSessionPolicyPayload = {
  schema_version: "kyuubiki.central-session-policy/v1";
  status: string;
  current_auth: {
    mode: string;
    descriptor: Record<string, unknown>;
  };
  planned_auth: CentralAuthProvider[];
  session_rules: {
    store_download_requires_session: boolean;
    publish_requires_session: boolean;
    agent_registration_requires_cluster_identity: boolean;
    credential_storage: string;
  };
};

export type CentralPublishResourcePolicy = {
  kind: CentralStoreEntryKind;
  manifest_schema: string;
  required_evidence: string[];
  distribution_modes: string[];
  mutable_after_publish: boolean;
};

export type CentralPublishPolicyPayload = {
  schema_version: "kyuubiki.central-publish-policy/v1";
  status: string;
  accepting_submissions: boolean;
  reason: string;
  resource_kinds: CentralPublishResourcePolicy[];
  review_stages: string[];
  publisher_requirements: {
    login_required: boolean;
    publisher_account_required: boolean;
    personal_access_token_supported: boolean;
    device_code_supported: boolean;
    anonymous_publish_allowed: boolean;
  };
};

export type CentralPublisherPolicyPayload = {
  schema_version: "kyuubiki.central-publisher-policy/v1";
  status: string;
  accounts_enabled: boolean;
  token_issuance_enabled: boolean;
  storage_tables: string[];
  identity_modes: Array<{
    id: string;
    status: string;
    clients: string[];
  }>;
  account_lifecycle: {
    allowed_statuses: string[];
    default_status: string;
    manual_review_required: boolean;
    anonymous_publish_allowed: boolean;
  };
  token_policy: {
    raw_token_storage_allowed: boolean;
    stored_secret_material: string;
    fingerprint_storage_table: string;
    rotation_required: boolean;
    revocation_supported: boolean;
    required_scopes: string[];
  };
  blocking_reasons: string[];
};

export type CentralPublishReadinessPayload = {
  schema_version: "kyuubiki.central-publish-readiness/v1";
  status: string;
  accepting_submissions: boolean;
  blocking_reasons: string[];
  resource_readiness: Array<{
    kind: CentralStoreEntryKind;
    status: string;
    publish_evidence: string[];
    provenance_attestations: string[];
    installer_checks: string[];
    blocking_reasons: string[];
  }>;
  required_storage_tables: string[];
  next_unlocks: string[];
};

export type CentralDatabasePersistenceDomain = {
  id: string;
  status: "ready" | "planned" | "schema_ready_preview" | string;
  tables: string[];
  owned_kinds: string[];
};

export type CentralDatabaseTableSpec = {
  name: string;
  domain: string;
  purpose: string;
};

export type CentralDatabasePolicyPayload = {
  schema_version: "kyuubiki.central-database-policy/v1";
  status: string;
  active_backend: "sqlite" | "postgres" | "memory" | string;
  repo_module: string | null;
  supported_backends: Array<"sqlite" | "postgres" | string>;
  server_test_profile: {
    preferred_backend: string;
    local_backend: string;
    required_env: string[];
    smoke_commands: string[];
  };
  persistence_domains: CentralDatabasePersistenceDomain[];
  migration_policy: {
    schema_version: "kyuubiki.central-database-contract/v1";
    mode: string;
    future_mode: string;
    startup_schema_check: boolean;
    destructive_changes_allowed: boolean;
    managed_tables: string[];
  };
  table_specs: CentralDatabaseTableSpec[];
  backup_policy: {
    sqlite: string;
    postgres: string;
    retention: string;
  };
};

export type CentralDatabaseCoverage = {
  domain: string;
  table_count: number;
  tables: string[];
  status: "schema_ready_preview" | "missing" | string;
};

export type CentralDatabaseStatusPayload = {
  schema_version: "kyuubiki.central-database-status/v1";
  contract_schema_version: "kyuubiki.central-database-contract/v1";
  status: "schema_ready_preview" | "memory_preview" | string;
  backend: "sqlite" | "postgres" | "memory" | string;
  sql_enabled: boolean;
  repo_module: string | null;
  managed_table_count: number;
  managed_tables: string[];
  domain_count: number;
  domains: CentralDatabasePersistenceDomain[];
  coverage: Record<string, CentralDatabaseCoverage>;
};

export type CentralProvenancePolicyPayload = {
  schema_version: "kyuubiki.central-provenance-policy/v1";
  status: string;
  accepting_artifact_uploads: boolean;
  artifact_contract: {
    digest_algorithms: string[];
    metadata_fields: string[];
    immutable_fields: string[];
  };
  resource_gates: Array<{
    kind: CentralStoreEntryKind;
    publish_evidence: string[];
    provenance_attestations: string[];
    installer_checks: string[];
    storage_tables: string[];
  }>;
  required_attestations: Array<{
    id: string;
    status: string;
    applies_to: CentralStoreEntryKind[];
  }>;
  signature_policy: {
    mode: string;
    accepted_key_kinds: string[];
    detached_signature_required: boolean;
  };
  revocation_policy: {
    supports_yank: boolean;
    supports_security_recall: boolean;
    retains_audit_log: boolean;
  };
  download_rules: {
    anonymous_download_allowed: boolean;
    checksum_required_before_install: boolean;
    installer_must_verify_signature: boolean;
  };
};

export type CentralArtifactAdmissionPolicyPayload = {
  schema_version: "kyuubiki.central-artifact-admission-policy/v1";
  status: string;
  accepting_uploads: boolean;
  write_endpoint_enabled: boolean;
  resource_kinds: Array<{
    kind: CentralStoreEntryKind;
    status: string;
    manifest_schema: string;
    required_evidence: string[];
    required_attestations: string[];
    installer_checks: string[];
    distribution_modes: string[];
    mutable_after_publish: boolean;
    yank_supported: boolean;
    security_recall_supported: boolean;
  }>;
  artifact_envelope: {
    required_fields: string[];
    digest_algorithms: string[];
    immutable_fields: string[];
    storage_tables: string[];
  };
  publisher_token_policy: {
    required_scopes: string[];
    raw_token_storage_allowed: boolean;
    fingerprint_storage_table: string;
    credential_storage: string;
  };
  review_queue: {
    status: string;
    stages: string[];
    manual_approval_required: boolean;
  };
  blocking_reasons: string[];
};
