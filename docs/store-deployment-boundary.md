# Store Deployment Boundary

Kyuubiki ships portable catalog, package, provenance, signature, publishing,
and self-hosting contracts. These contracts describe how the product discovers
and verifies resources; they do not select a hosted vendor or encode a vendor's
identity, entitlement, pricing, payment, or payout policy.

## Self-hosted baseline

The open-source baseline supports local and institution-operated stores without
an external identity dependency. A deployment can expose operator, workflow,
frontend-DSL, and language-pack catalogs while retaining local control of its
database, signing roots, review policy, and access rules.

The self-hosted baseline must not require:

- a vendor account;
- a hosted subscription or entitlement service;
- a vendor billing or payout service;
- credentials for an unrelated control plane.

## External store adapters

A deployment may configure an external catalog or hosted distribution service.
That integration is an adapter layered on the portable Kyuubiki contracts. The
adapter may impose its own authentication and authorization policy, but the
open-source runtime treats those decisions as distribution-plane results rather
than solver or orchestrator authority.

The generic identity-provider contract supports deployment-configured OIDC with
authorization code and PKCE. Subjects are keyed by issuer plus subject. Email is
a mutable claim and must not become a local identity key. Password grants and
dynamic client registration remain outside the supported contract.

## Credential boundary

Provider configuration comes from deployment environment and secret storage.
The repository contains no client secret, signing key, production issuer,
database credential, or infrastructure address. Native applications use the
platform keychain when credentials must persist; server-side applications use
server session and secret-manager boundaries; browser-only fallback material is
memory-only.

## Runtime authority

Catalog access, package eligibility, or external account state never grants
solver execution, agent registration, or orchestra control. Packages still
pass the local provenance, signature, compatibility, and admission checks before
they enter a Kyuubiki runtime.

## Public contracts

- `kyuubiki.central-session-policy/v2`
- `kyuubiki.central-publish-policy/v2`
- `kyuubiki.central-publisher-policy/v2`

Deployment-specific services may consume these contracts. Their private policy
and implementation belong to their owning repositories rather than this core.
