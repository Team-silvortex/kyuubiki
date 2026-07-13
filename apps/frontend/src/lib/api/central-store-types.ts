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
  publisher_accounts: { status: string };
  publish_policy: { status: string; backing?: string };
  database_policy: { status: string; backing?: string };
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
