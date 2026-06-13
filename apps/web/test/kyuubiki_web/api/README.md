# API Tests

This directory holds HTTP/router-facing integration tests for the orchestrator
API surface.

- `router_test.exs`
  Core end-to-end API behavior across projects, jobs, results, agents, and health.

- `job_submission_api_test.exs`
  Dedicated HTTP submission and cancellation coverage for lightweight job entrypoints.

- `control_plane_api_test.exs`
  Dedicated control-plane health, protocol, remote-agent registry, and Hub workload coverage.

- `job_audit_api_test.exs`
  Dedicated persisted jobs, results CRUD/export, and security-event audit coverage.

- `workflow_catalog_api_test.exs`
  Dedicated workflow catalog listing, filtering, and async catalog job coverage.

- `operator_catalog_api_test.exs`
  Dedicated operator catalog listing, filtering, and descriptor coverage.

- `cluster_security_api_test.exs`
  Cluster-route protection coverage for tokens, timestamps, allowlists, and fingerprints.

- `project_model_api_test.exs`
  Dedicated CRUD coverage for projects, models, and model versions.

- `workflow_runtime_api_test.exs`
  Dedicated workflow-graph runtime coverage for the core synchronous run path.

- `workflow_condition_api_test.exs`
  Dedicated workflow-graph condition, branch skipping, and merge-path coverage.

- `workflow_async_bridge_api_test.exs`
  Dedicated workflow-graph asynchronous job and explicit bridge-contract coverage.

- `frame_beam_solver_api_test.exs`
  Dedicated orchestration coverage for frame and beam solver job families.

- `torsion_spring_solver_api_test.exs`
  Dedicated orchestration coverage for torsion and spring solver job families.

- `truss_solver_api_test.exs`
  Dedicated orchestration coverage for version-bound, 2D, 3D, and failure-path truss solver jobs.

- `workflow_catalog_triangle_job_test.exs`
  Focused async workflow-catalog execution coverage for the triangle coupled path.

- `test/support/workflow_api_test_support.exs`
  Shared workflow API test helpers for fake agent sessions, polling async jobs, and reusable workflow input artifacts.

- `test/support/api_router_case.exs`
  Shared router-test case template for API setup, cleanup, aliases, and Plug helpers.
