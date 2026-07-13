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
