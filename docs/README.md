# Documentation Map

Choose a task first; the reference catalog below is not a required reading order.
The [HTML book](book.html) is the human/WebView entrypoint.
[Current status](current-line.md#version-and-evidence-ledger) separates package
metadata, the last recorded installation, Git checkpoints and source follow-ups.

## Choose A Path

| Goal | Start here | Continue with |
| --- | --- | --- |
| Understand the product | [Book chapter 1](book-ch01-what-is-kyuubiki.html) | [Current architecture](current-architecture-map.md) |
| Finish a first study | [First research tutorial](tutorial-first-research.html) | [Failure and recovery](tutorial-recovery.html) |
| Compose operators | [Workflow tutorial](tutorial-composite-workflow.html) | [Workflow contracts](book-ch05-workflow-and-operators.html) |
| Integrate a research agent | [Model entry](../llms.txt), [bootstrap](model-research-bootstrap.json) | [Research onboarding](model-research-onboarding.html) |
| Implement an operator | [Rust Operator SDK tutorial](tutorial-operator-sdk.html) | [Reliability policy](operator-reliability.md) |
| Automate the GUI | [PWDT tutorial](tutorial-pwdt-automation.html) | [UI contract](ui-automation-contract.html) |
| Deploy or update | [Runtime chapter](book-ch04-runtime-modes.html) | [Installer remote control](installer-remote-control.md) |
| Assess trust or debug a failure | [Testing and CI](testing-and-ci.md) | [Weakness roadmap](weakness-roadmap.md), [accuracy plan](accuracy-plan.md) |

## Ownership And Evidence

- Repository topic documents own behavior; the [ownership map](runtime-doc-ownership.md)
  identifies the primary owner when architecture/runtime pages overlap.
- HTML chapter pages explain those topics to users. The [Hub shelf](../apps/hub-gui/ui/docs/index.html)
  is a short mirror, not a second release diary or independent source of truth.
- JSON schemas and manifests define checked contracts. A registered command is
  a plan until its execution has evidence.
- Dated reports and release records retain their platform, version, backend
  and exclusions. Current summaries link to them rather than adding overlapping counts.
- Historical planning files are not active deployment instructions. Use the
  current version ledger and live tensor before interpreting old readiness claims.

## Reference Catalog

### Book And Tutorials

Start with the book for context, or a tutorial for a concrete completion contract.

- [book.html](book.html)
- [book-manifest.json](book-manifest.json)
- [navigation-matrix.html](navigation-matrix.html)
- [book-ch01-what-is-kyuubiki.html](book-ch01-what-is-kyuubiki.html)
- [book-ch02-version-line.html](book-ch02-version-line.html)
- [book-ch03-architecture-boundaries.html](book-ch03-architecture-boundaries.html)
- [book-ch04-runtime-modes.html](book-ch04-runtime-modes.html)
- [book-ch05-workflow-and-operators.html](book-ch05-workflow-and-operators.html)
- [book-ch06-sdk-surfaces.html](book-ch06-sdk-surfaces.html)
- [book-ch07-trust-and-safety.html](book-ch07-trust-and-safety.html)
- [book-ch08-reading-paths.html](book-ch08-reading-paths.html)
- [tutorials.html](tutorials.html)
- [tutorial-first-research.html](tutorial-first-research.html)
- [tutorial-composite-workflow.html](tutorial-composite-workflow.html)
- [tutorial-pwdt-automation.html](tutorial-pwdt-automation.html)
- [tutorial-headless-rust.html](tutorial-headless-rust.html)
- [tutorial-operator-sdk.html](tutorial-operator-sdk.html)
- [tutorial-recovery.html](tutorial-recovery.html)

### Current Status And Historical Records

Only current-line.md and version-line.md own active version posture. Checkpoint archives, Moxi/Tamamono notes and 1.x release plans retain their historical scope.

- [current-line.md](current-line.md)
- [version-line.md](version-line.md)
- [daji-3.0.0.md](daji-3.0.0.md)
- [daji-3.6-to-3.7-history.md](daji-3.6-to-3.7-history.md)
- [daji-3.5-history.md](daji-3.5-history.md)
- [daji-3.3-to-3.4-history.md](daji-3.3-to-3.4-history.md)
- [moxi-closeout.md](moxi-closeout.md)
- [moxi-handoff.md](moxi-handoff.md)
- [moxi-handoff.manifest.json](moxi-handoff.manifest.json)
- [moxi-minor-lines.md](moxi-minor-lines.md)
- [tamamono-minor-lines.md](tamamono-minor-lines.md)
- [release-prep-1.7.md](release-prep-1.7.md)
- [release-prep-1.8.md](release-prep-1.8.md)
- [release-prep-1.9-to-1.20.md](release-prep-1.9-to-1.20.md)

### Architecture And Documentation Ownership

Responsibility and authority maps belong here; benchmark diaries and runtime procedures have separate owners.

- [current-architecture-map.md](current-architecture-map.md)
- [system-overview.md](system-overview.md)
- [module-architecture.md](module-architecture.md)
- [architecture-extension-standard.md](architecture-extension-standard.md)
- [architecture-red-lines.md](architecture-red-lines.md)
- [app-runtime-boundaries.md](app-runtime-boundaries.md)
- [hub-architecture.md](hub-architecture.md)
- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [agent-control-authority.md](agent-control-authority.md)
- [mobile-gui-runtime-boundary.md](mobile-gui-runtime-boundary.md)
- [repository-structure.md](repository-structure.md)
- [project-architecture-organization.md](project-architecture-organization.md)
- [philosophy.md](philosophy.md)
- [documentation-system.md](documentation-system.md)
- [runtime-doc-ownership.md](runtime-doc-ownership.md)
- [maintenance.md](maintenance.md)
- [development.md](development.md)
- [toolchain-contract.md](toolchain-contract.md)

### Headless And Operator SDKs

Headless controls the backend; the Rust-only Worker SDK extends operators. Model collaboration is an adapter, not execution authority.

- [headless-sdks.md](headless-sdks.md)
- [operator-sdk.md](operator-sdk.md)
- [headless-agent-contract.md](headless-agent-contract.md)
- [protocols.md](protocols.md)
- [model-collaboration-sdk.html](model-collaboration-sdk.html)
- [model-collaboration-sdk.md](model-collaboration-sdk.md)
- [model-research-onboarding.html](model-research-onboarding.html)
- [model-research-onboarding.md](model-research-onboarding.md)
- [model-research-bootstrap.json](model-research-bootstrap.json)

### Physics Workflows And Research Contracts

Execution coverage, result identity and numerical qualification are distinct. Research examples are bounded reference journeys, not material certification.

- [physics-coverage-map.md](physics-coverage-map.md)
- [operator-category-taxonomy.md](operator-category-taxonomy.md)
- [operator-library-centralization.md](operator-library-centralization.md)
- [workflow-graph.md](workflow-graph.md)
- [workflow-dataset.md](workflow-dataset.md)
- [operator-task-ir-digest.md](operator-task-ir-digest.md)
- [thermal-contact-operator.md](thermal-contact-operator.md)
- [axial-bar-input-normalization.md](axial-bar-input-normalization.md)
- [graph-entity-input-normalization.md](graph-entity-input-normalization.md)
- [advection-diffusion-service-chain.md](advection-diffusion-service-chain.md)
- [advection-diffusion-upwind.md](advection-diffusion-upwind.md)
- [advection-diffusion-research-chain.md](advection-diffusion-research-chain.md)
- [automated-material-research-example.md](automated-material-research-example.md)
- [automated-material-research-example.manifest.json](automated-material-research-example.manifest.json)
- [research-layered-thermal.html](research-layered-thermal.html)
- [material-research-roadmap.md](material-research-roadmap.md)
- [material-score-contract.md](material-score-contract.md)
- [material-score-contract.manifest.json](material-score-contract.manifest.json)
- [kcore-exchange-format.html](kcore-exchange-format.html)
- [kcore-exchange-format.md](kcore-exchange-format.md)
- [headless-research-input-identity.md](headless-research-input-identity.md)
- [headless-saved-model-sources.md](headless-saved-model-sources.md)
- [headless-model-artifact-identity.md](headless-model-artifact-identity.md)
- [headless-result-artifact-readback.md](headless-result-artifact-readback.md)
- [solver-matrix-optimization-pack.html](solver-matrix-optimization-pack.html)
- [solver-matrix-optimization-pack.md](solver-matrix-optimization-pack.md)

### GUI PWDT And Localization

PWDT automates the fixed product-owned GUI; it is not the Python Headless SDK. Installed UI acceptance remains version/platform-specific.

- [frontend-style.md](frontend-style.md)
- [frontend-implementation.md](frontend-implementation.md)
- [ui-architecture-migration.md](ui-architecture-migration.md)
- [ui-automation-contract.html](ui-automation-contract.html)
- [ui-automation-contract.json](ui-automation-contract.json)
- [language-packs.md](language-packs.md)
- [rendering-roadmap.html](rendering-roadmap.html)
- [acceptance-immersive-modeling.html](acceptance-immersive-modeling.html)
- [browser-storage-security-audit.html](browser-storage-security-audit.html)

### Operations Installation And Data

Installer owns deployment, lifecycle, integrity and update visibility. Runtime procedures must keep secrets and host-local configuration outside Git.

- [operations.md](operations.md)
- [security.md](security.md)
- [packaging-and-deployment.md](packaging-and-deployment.md)
- [desktop-release-checklist.md](desktop-release-checklist.md)
- [installer-remote-control.md](installer-remote-control.md)
- [remote-deployment-roadmap.html](remote-deployment-roadmap.html)
- [remote-pilot.md](remote-pilot.md)
- [central-server-components.md](central-server-components.md)
- [store-deployment-boundary.md](store-deployment-boundary.md)
- [component-integrity-protocol.html](component-integrity-protocol.html)
- [installation-integrity-contract.html](installation-integrity-contract.html)
- [update-catalog.html](update-catalog.html)
- [data-lifecycle.html](data-lifecycle.html)
- [troubleshooting.md](troubleshooting.md)

### Validation And Readiness

Test totals, instrumented code coverage, tensor depth and release permission answer different questions.

- [testing-and-ci.md](testing-and-ci.md)
- [accuracy-plan.md](accuracy-plan.md)
- [accuracy-baselines.md](accuracy-baselines.md)
- [operator-reliability.md](operator-reliability.md)
- [weakness-roadmap.md](weakness-roadmap.md)
- [minimal-industrial-closure.md](minimal-industrial-closure.md)
- [minimal-industrial-closure.manifest.json](minimal-industrial-closure.manifest.json)
- [commercial-readiness-2.0.md](commercial-readiness-2.0.md)
- [commercial-readiness-2.0.manifest.json](commercial-readiness-2.0.manifest.json)
- [fem-blender-roadmap.md](fem-blender-roadmap.md)

## Maintain This Map

Keep one owner per topic. Update source contracts first, then relevant HTML
chapters and short Hub mirrors. New local documentation belongs in this catalog;
new central reading paths also belong in [book-manifest.json](book-manifest.json).

Current status is a concise handoff. Put experiments in dated reports and move
superseded checkpoint narratives into the history files without relabeling their
results. Keep old checkpoint anchors reachable. Do not delete retained evidence
to reduce duplication in a mirror.

Run the focused documentation gates after changes:

```sh
make check-doc-inventory
make check-doc-book
make check-version-line
make audit-project-organization
```

These validate structure and metadata, not the truth of numerical or installed
acceptance claims. Use [testing-and-ci.md](testing-and-ci.md) for execution lanes
and [documentation-system.md](documentation-system.md) for curation rules.
