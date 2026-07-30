# Team Silvortex Store Model

Kyuubiki is a Team Silvortex application. Its hosted account identity is shared
with other Team Silvortex apps through the Team Silvortex official website
account project, while the open-source Kyuubiki runtime and self-hosted store
contracts remain deployable without depending on the hosted account plane.

This split is intentional:

- open-source users and research labs can self-host the store and supporting
  service surfaces.
- the hosted Team Silvortex center store can provide identity, billing,
  download quotas, subscriptions, publisher submission, and revenue sharing.
- the Team Silvortex official website account project owns account registration,
  session security, OIDC provider behavior, billing identity, and payout
  identity.
- Kyuubiki only integrates with that account system as a reviewed OIDC client
  and policy consumer.
- Hub, Workbench, Installer, SDKs, agents, and Orchestra should consume stable
  store contracts instead of hard-coding a single hosted service.

## Store Modes

### Open Source Self-Hosted Store

The open-source self-hosted store is the default compatibility contract for
institutions that need local control.

It should support:

- operator catalogs.
- workflow template catalogs.
- frontend DSL template catalogs.
- language-pack delivery.
- signed download indexes.
- readiness and provenance policy endpoints.

It should not require:

- a Team Silvortex account.
- hosted billing.
- hosted payout infrastructure.
- closed-source account services.

Self-hosted deployments may still choose to integrate their own identity,
approval, billing, or entitlement layers, but those are deployment decisions,
not base Kyuubiki runtime requirements.

### Hosted Team Silvortex Center Store

The hosted center store is the commercial ecosystem layer. It can use the
external Team Silvortex official website account and billing plane while still
exposing the same public resource contracts to Kyuubiki clients.

Hosted center store scope:

- operator distribution.
- computation-flow and workflow-template distribution.
- publisher onboarding.
- download metering.
- subscription entitlement.
- creator revenue sharing.

The hosted account system is shared across Team Silvortex applications and is
not unique to Kyuubiki. Kyuubiki should not implement a parallel account
database, password flow, billing identity store, or payout identity store.

## Monetization Boundary

The intended business posture is close to a creator-platform model:

- non-store Kyuubiki services remain free to use.
- the hosted center store meters operator downloads and workflow-template
  downloads.
- free users receive a small monthly download allowance.
- subscription unlocks unlimited hosted center-store downloads.
- creators are paid proportionally from metered downloads.

The first monetized resource kinds are:

- `operator`
- `workflow_template`

Language packs and frontend DSL templates can be centrally distributed, but
they are not the first paid resource classes unless the commercial policy is
changed explicitly.

## Publisher Eligibility And Revenue Sharing

Any user with a Team Silvortex account can become a publisher only after the
hosted center store has the required account and payout prerequisites.

Minimum publisher requirements:

- Team Silvortex account.
- legal payment method.
- publisher review.
- resource provenance.
- signature and security checks.
- resource-kind-specific evidence.

Revenue sharing is based on proportional download share from metered hosted
center-store downloads. Self-hosted private downloads are outside this payout
meter unless a deployment explicitly reports them through a future hosted
agreement.

## Architecture Red Lines

- Kyuubiki open-source runtime code must not require hosted billing to run.
- Self-hosted store contracts must stay open and inspectable.
- The Team Silvortex official website project owns the hosted account system;
  Kyuubiki only stores integration configuration and local session/cache state.
- Kyuubiki's hosted identity integration is an OIDC client contract:
  authorization-code with PKCE, exact redirect URIs, `state`, `nonce`, JWKS
  verification, and platform keychain or memory-only token storage.
- Kyuubiki must not use password grants, dynamic client registration, or a copy
  of the official website account database.
- Closed-source account, billing, quota, fraud, tax, and payout systems must
  stay behind hosted Team Silvortex service boundaries.
- Agent and Orchestra execution authority must not be derived from marketplace
  billing state.
- Publisher trust must come from review, provenance, signatures, and policy
  checks, not from download count alone.
- Repository files must not contain production account, billing, payout, or
  signing secrets.

## Current Implementation Status

The current `moxi 2.x` implementation is a preview contract, not a live hosted
marketplace.

Implemented as machine-readable policy:

- shared Team Silvortex account-system metadata in the session and publisher
  policies.
- explicit `official-website` provider metadata and `kyuubiki_oidc_client`
  integration role.
- `identity_integration` session-policy metadata for discovery, PKCE, request
  checks, scopes, token storage, and forbidden flows.
- hosted center-store download/session boundary.
- self-hosted store independence from Team Silvortex account requirements.
- commercial model metadata for the publish policy.
- publisher requirements for account and legal payment method.
- publisher payout-policy metadata for proportional download sharing.

Still intentionally not implemented:

- real Team Silvortex login.
- account creation, password, TOTP, recovery, and account database ownership.
- payment processing.
- subscription entitlement checks.
- creator payout execution.
- write-side public upload endpoints.

Related docs:

- `docs/central-server-components.md`
- `docs/operator-library-centralization.md`
- `docs/commercial-readiness-2.0.md`
