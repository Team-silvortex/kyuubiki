# Kyuubiki Elixir SDK

Headless Elixir SDK for Kyuubiki control-plane and solver-rpc protocols.

```elixir
auth = KyuubikiSdk.Auth.access_token("dev-token")
client = KyuubikiSdk.ControlPlaneClient.new("http://127.0.0.1:4000", auth: auth)
{:ok, health} = KyuubikiSdk.ControlPlaneClient.health(client)
{:ok, operators} = KyuubikiSdk.ControlPlaneClient.list_workflow_operators(client)
{:ok, structural_operators} = KyuubikiSdk.ControlPlaneClient.list_workflow_operators(client, domain: "structural", family: "solver")
{:ok, operator} = KyuubikiSdk.ControlPlaneClient.fetch_workflow_operator(client, "solver.truss_2d")
{:ok, workflow_descriptor} = KyuubikiSdk.ControlPlaneClient.fetch_workflow_catalog_workflow(client, "workflow.heat-to-thermo-quad-2d")

rpc = KyuubikiSdk.SolverRpcClient.new("127.0.0.1", 5001)
{:ok, descriptor} = KyuubikiSdk.SolverRpcClient.describe_agent(rpc)

session =
  KyuubikiSdk.new_session(
    base_url: "http://127.0.0.1:4000",
    auth: auth,
    rpc_host: "127.0.0.1",
    rpc_port: 5001
  )

{:ok, submitted} = KyuubikiSdk.Session.submit_job(session, "truss_2d", %{"model" => %{}, "case" => %{}})
{:ok, bundle} = KyuubikiSdk.AgentClient.run_study(session, "truss_2d", %{"model" => %{}, "case" => %{}}, timeout: 60_000)
{:ok, nodes_page} = KyuubikiSdk.AgentClient.browse_result_chunks(session, bundle.terminal["job"]["job_id"], "nodes", offset: 0, limit: 250)
{:ok, retried} = KyuubikiSdk.AgentClient.run_study_with_retry(session, "truss_2d", %{"model" => %{}, "case" => %{}}, max_attempts: 3)
{:ok, workflow_run} = KyuubikiSdk.AgentClient.run_workflow_catalog(session, "workflow.heat-to-thermo-quad-2d", %{"thermal_case" => %{"loadcase" => "baseline"}}, timeout: 60_000)
workflow_runtime = workflow_run.workflow_runtime
pages = Enum.take(KyuubikiSdk.AgentClient.stream_result_chunks(session, bundle.terminal["job"]["job_id"], "nodes", page_size: 250), 2)

dataset =
  KyuubikiSdk.workflow_dataset_contract(
    "dataset.demo/v1",
    "1.0.0",
    [KyuubikiSdk.workflow_dataset_value("thermal_case", "study_model", "json_object")]
  )

graph =
  KyuubikiSdk.workflow_graph(
    "workflow.demo",
    "Demo workflow",
    "1.0.0",
    ["input"],
    [
      KyuubikiSdk.workflow_node("input", "input", %{outputs: [KyuubikiSdk.workflow_port("case", "study_model/demo", %{dataset_value: "thermal_case"})]}),
      KyuubikiSdk.workflow_node("output", "output", %{inputs: [KyuubikiSdk.workflow_port("case", "study_model/demo", %{dataset_value: "thermal_case"})]})
    ],
    [KyuubikiSdk.workflow_edge("edge-1", "input", "case", "output", "case", "study_model/demo", %{dataset_value: "thermal_case"})],
    %{
      dataset_contract: dataset,
      defaults:
        KyuubikiSdk.workflow_defaults(%{
          cache_policy: "cached",
          orchestrated: false,
          dispatch_policy: "central_fetch",
          placement_tags: ["cpu"],
          required_capabilities: ["solver.thermal"]
        }),
      dispatch_policy: "central_fetch",
      operator_fetch_plan: [
        KyuubikiSdk.workflow_operator_fetch_entry("input", "input.demo", %{
          package_ref: "kyuubiki://operators/input.demo",
          version: "1.0.0",
          integrity: "sha256:demo",
          cache_scope: "agent"
        })
      ],
      placement_tags: ["mesh-enabled"],
      required_capabilities: ["artifact-cache"]
    }
  )

{:ok, output_manifest} = KyuubikiSdk.build_workflow_output_manifest(graph)
{:ok, validated_outputs} = KyuubikiSdk.validate_workflow_result_against_graph(graph, workflow_run.result)
```

Highlights:

- jobs/results/export CRUD on the control plane
- operator catalog listing, filtering, and descriptor fetch
- workflow catalog descriptor fetch plus auto graph resolution for catalog runs
- expanded solve-kind coverage across structural, thermal,
  thermo-mechanical, and electrostatic study families
- direct framed TCP solver-RPC access
- `KyuubikiSdk.Session` for submit/batch/wait flows
- `KyuubikiSdk.AgentClient` for run-study, workflow-run, and chunk-browse flows
- retry, failure classification, and chunk streaming helpers
- `KyuubikiSdk.Auth` and structured `KyuubikiSdk.Error`
- workflow contract validation and builder helpers
- distributed workflow execution-hint fields for dispatch policy, operator fetch
  plan, placement tags, and required capabilities
- workflow output manifest and result validation helpers
- BEAM-friendly thin wrapper over the public protocol

Example:

- Run from [run_study.exs](examples/run_study.exs)
- Typical invocation:
  `cd sdks/elixir && KYUUBIKI_BASE_URL=http://127.0.0.1:4000 mix run examples/run_study.exs`
- Smoke test:
  `cd sdks/elixir && mix test`
