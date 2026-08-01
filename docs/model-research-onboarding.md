# Model Research Onboarding

This is the single research startup path for a large model integrating with
Kyuubiki. The machine-readable source is
[`model-research-bootstrap.json`](model-research-bootstrap.json); repository
ingestion should begin at [`../llms.txt`](../llms.txt).

## Definition of Ready

A model is ready to begin research only when it can:

- identify one configured Headless SDK and backend target
- load the bootstrap hard rules and stop conditions
- run the selected SDK preflight and receive `ready_for_planning: true`
- build a constrained collaboration request without provider credentials in
  its context
- normalize provider tool calls into
  `kyuubiki.model-workflow-proposal/v1`
- build and inspect a Headless plan before dispatch
- advance cross-turn work only from caller-verified receipts and bound job ids
- distinguish planning, real execution, numerical validation, and external
  qualification
- retain the required research evidence after execution

Reading documentation alone does not grant execution authority. It gives the
model enough contract knowledge to prepare safe, inspectable work.

## Bootstrap Sequence

1. Read `llms.txt` and `docs/model-research-bootstrap.json`.
2. Load every document listed under `required_documents`.
3. Select Rust, Python, or Elixir and run its
   `inspect_model_research_bootstrap` equivalent with a caller-owned resource
   resolver. Stop if the readiness report contains a blocker or missing path.
4. Retain the report as `kyuubiki.model-research-readiness-report/v1`; its
   authority is always `none_preflight_only`.
5. Keep provider networking, credentials, billing, and retry policy in the
   caller-owned integration layer.
6. Call `service_health` and inspect protocol compatibility.
7. Create the shared collaboration session fixture or a narrower derivative.
8. Project tools for the selected provider and request one dependency frontier.
9. Normalize the provider response and build a Headless plan.
10. Reject an invalid plan; request approval for every confirmation-gated step.
11. Have the caller issue a plan-bound approval using
    `schemas/model-plan-approval.schema.json`. The model may request this
    approval but may not create or infer it.
12. Use the selected SDK execution surface and its Session dispatcher to route
    through the ordinary Headless client and retain a
    `kyuubiki.model-research-execution-receipt/v1` receipt.
13. Verify that receipt independently, then create or advance
    `kyuubiki.model-research-frontier/v1`. Its generated proposal binds the
    exact returned `job_id`; the model does not supply that binding.
14. Repeat receipt verification and frontier advancement for `job_wait` and
    `result_fetch`.
15. Pass the authenticated `ready_to_validate` frontier, its bound result
    receipt, and the resolved workflow graph through the SDK validation handoff.
    Retain `kyuubiki.model-research-validation-report/v1`; attach a validated
    material research bundle when available.

One dependency frontier per turn is important. A model must not propose
`job_wait` with a guessed job id in the same turn that creates the job. It must
consume the real submission receipt first.

The frontier is deliberately a small checkpoint, not a result store. It retains
the research stage, next allowed action, real job binding, and minimal receipt
evidence. Full results remain in the ordinary result and artifact stores. A
failed or cancelled execution moves the frontier to `blocked`; it never skips
forward to validation. Persisted frontiers are also untrusted input: a
caller-owned frontier verifier must authenticate them before proposal creation
or state advancement.

## First Bounded Research

The first portable exercise uses the catalog-owned workflow:

```text
workflow.material-study-envelope-ranking-json
```

Start from:

- `schemas/examples.model-collaboration-session.json`
- `schemas/examples.model-workflow-proposal.json`
- `schemas/examples.material-envelope-catalog-request.json`

The initial proposal checks service health and submits the bounded catalog
workflow. `workflow_submit_catalog` is confirmation-gated. After dispatch, use
the returned `job_id` in a later `job_wait` proposal, then use the same real id
for `result_fetch`.

Use `schemas/model-research-frontier.schema.json` for persisted state and
`schemas/examples.model-research-frontier.json` as the portable fixture. Rust,
Python, and Elixir expose equivalent start, advance, and proposal helpers. They
require caller-owned receipt and frontier verifiers for the cross-turn chain.

Use `schemas/model-research-validation-report.schema.json` for the next handoff.
The three SDKs reject a mismatched graph, receipt, workflow, job binding,
unverified checkpoint, incomplete runtime, or invalid research bundle. A report
with `screening_bundle_validated` still carries
`screening_only_not_qualification` and `external_validation_required: true`.

Every official execution bridge rejects the entire plan before network access if an
exact gated step is not covered by a caller-issued approval. Runtime failures
are returned as partial receipts with `status: failed`; they are evidence of an
attempt, never evidence of completed execution.

The native Rust reference entry is
`sdks/rust/examples/execute_model_research_plan.rs`. From `sdks/rust`, provide
the configured control-plane URL and run it with the repository fixtures:

```bash
KYUUBIKI_BASE_URL=http://127.0.0.1:4000 KYUUBIKI_APPROVAL_ID="$CALLER_APPROVAL_ID" KYUUBIKI_APPROVAL_AUTHORITY="$CALLER_APPROVAL_AUTHORITY" cargo run --example execute_model_research_plan -- ../../schemas/examples.model-collaboration-session.json ../../schemas/examples.model-workflow-proposal.json ../../schemas/examples.model-plan-approval.json
```

Use the example approval only to verify the bounded fixture. Real integrations
must issue a fresh approval identity after caller review of the exact plan and
implement `ModelApprovalVerifier` against a trusted caller-owned policy or
credential boundary. The environment verifier in the example is illustrative,
not a production authorization system.

This fixture proves connectivity, policy, catalog dispatch, dependency
sequencing, and retained-result access. It is a screening workflow, not a claim
that the input rows came from independently qualified simulation or experiment.

## Research Completion

A research turn is complete only when the retained output records:

- terminal job state and real execution receipt
- verified research frontier and exact bound job identity
- verified model research validation report and workflow-result artifact keys
- exact caller approval identity for every gated action
- execution authority and provenance
- input contracts and artifact identities
- numerical baselines, convergence evidence, or explicit missing evidence
- quality gates and blocking reasons
- reliability posture and limitations
- reproducible project-relative commands
- next validation or next-round actions

Use the material research bundle contract and the automated material research
example as the reference evidence shape. If external calibration, experiment,
or an independent solver is still required, report that requirement rather
than upgrading the claim.

## Stop Instead of Guessing

Stop and return a structured blocker when:

- the backend is unavailable or protocol-incompatible
- the model asks for an action outside the projected catalog
- required payload data or a real job id is missing
- confirmation has not been granted
- execution provenance is absent
- a quality gate remains blocking
- the requested conclusion exceeds screening evidence

The correct output in these cases is a repair or validation plan, not a
fabricated result.

## Readiness Is Not Execution

Rust, Python, and Elixir expose the same preflight over a caller-provided
resource resolver. It validates project-relative paths, required documents,
the selected collaboration/execution/frontier/validation surfaces, shared
fixtures, hard rules, stop conditions, and the completion contract. Missing or
unsafe resources return a diagnostic report with `ready_for_planning: false`.

The report cannot approve, dispatch, or validate a solve. Its fixed
`execution_authority` is `none_preflight_only`; the ordinary approval, receipt,
frontier, and validation gates remain mandatory.
