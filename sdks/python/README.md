# Kyuubiki Python SDK

Headless Python SDK for Kyuubiki public protocols and AI-friendly automation.

## Role

The Python SDK is the research-automation line for scripts, notebooks, lab
pipelines, data analysis, and optimization loops. It consumes the same headless
contracts as the Rust and Elixir SDKs, so Python users can build their own
wrappers without depending on Workbench or Rust reference CLIs.

```python
from kyuubiki_sdk import (
    ControlPlaneClient,
    KyuubikiAgentClient,
    KyuubikiAuth,
    KyuubikiRetryPolicy,
    KyuubikiSession,
    SolverRpcClient,
    build_material_report_from_payload,
    build_contact_gap_1d_workflow,
    build_modal_frame_2d_workflow,
    build_nonlinear_spring_1d_workflow,
    build_workflow_dataset_contract,
    build_workflow_dataset_value,
    build_workflow_defaults,
    build_workflow_edge,
    build_workflow_graph,
    build_workflow_node,
    build_workflow_operator_fetch_entry,
    build_workflow_output_manifest,
    build_workflow_port,
    material_study_envelope_catalog_request,
    material_study_execution_plan_example,
    validate_workflow_dataset_contract,
    validate_workflow_graph,
    validate_workflow_result_against_graph,
)

auth = KyuubikiAuth.access_token("dev-token")
cp = ControlPlaneClient("http://127.0.0.1:4000", auth=auth)
print(cp.health())

rpc = SolverRpcClient("127.0.0.1", 5001)
print(rpc.describe_agent())
print(rpc.solve_study("modal_frame_2d", {"nodes": [], "elements": []}))
print(rpc.solve_study("nonlinear_spring_1d", {"nodes": [], "elements": []}))
print(rpc.solve_study("contact_gap_1d", {"nodes": [], "elements": [], "contacts": []}))

session = KyuubikiSession.from_endpoints(
    "http://127.0.0.1:4000",
    auth=auth,
    rpc_host="127.0.0.1",
    rpc_port=5001,
)

jobs = session.submit_jobs(
    [
        {"solve_kind": "truss_2d", "payload": {"model": {}, "case": {}}},
        {"solve_kind": "truss_3d", "payload": {"model": {}, "case": {}}},
    ]
)

print(jobs)

agent = KyuubikiAgentClient(session)
bundle = agent.run_study("truss_2d", {"model": {}, "case": {}}, timeout_s=60.0)
nodes_page = agent.browse_result_chunks(bundle["terminal"]["job"]["job_id"], "nodes", offset=0, limit=250)
retry_bundle = agent.run_study_with_retry(
    "truss_2d",
    {"model": {}, "case": {}},
    retry_policy=KyuubikiRetryPolicy(max_attempts=3),
)
for page in agent.iter_result_chunks(bundle["terminal"]["job"]["job_id"], "nodes", page_size=250):
    print(page["returned"])

catalog = cp.list_workflow_catalog()
operators = cp.list_workflow_operators()
structural_operators = cp.list_workflow_operators({"domain": "structural", "family": "solver"})
operator = cp.fetch_workflow_operator("solver.truss_2d")
workflow_descriptor = cp.fetch_workflow_catalog_workflow("workflow.heat-to-thermo-quad-2d")
workflow_job = cp.submit_workflow_catalog_job(
    "workflow.heat-to-thermo-quad-2d",
    {"thermal_case": {"loadcase": "baseline"}},
)
material_envelope = material_study_envelope_catalog_request()
material_envelope_job = cp.submit_workflow_catalog_job(
    material_envelope["workflow_id"],
    material_envelope["input_artifacts"],
)
material_plan = material_study_execution_plan_example()
assert material_plan["schema_version"] == "kyuubiki.material-study-execution-plan/v1"
workflow_run = agent.run_workflow_catalog(
    "workflow.heat-to-thermo-quad-2d",
    {"thermal_case": {"loadcase": "baseline"}},
    timeout_s=60.0,
)
workflow_runtime = workflow_run["workflow_runtime"]
validate_workflow_dataset_contract(
    {
        "schema_version": "kyuubiki.workflow-dataset/v1",
        "id": "dataset.demo/v1",
        "version": "1.0.0",
        "values": [{"id": "thermal_case", "data_class": "study_model", "element_type": "json_object", "shape": {}}],
    }
)
validate_workflow_graph(
    {
        "schema_version": "kyuubiki.workflow-graph/v1",
        "id": "workflow.demo",
        "name": "Demo workflow",
        "version": "1.0.0",
        "entry_nodes": ["input"],
        "nodes": [
            {"id": "input", "kind": "input", "inputs": [], "outputs": [{"id": "case", "artifact_type": "study_model/demo"}]},
            {"id": "output", "kind": "output", "inputs": [{"id": "case", "artifact_type": "study_model/demo"}], "outputs": []},
        ],
        "edges": [{"id": "edge-1", "from": {"node": "input", "port": "case"}, "to": {"node": "output", "port": "case"}, "artifact_type": "study_model/demo"}],
    }
)
dataset_contract = build_workflow_dataset_contract(
    "dataset.demo/v1",
    version="1.0.0",
    values=[build_workflow_dataset_value("thermal_case", data_class="study_model", element_type="json_object", shape={})],
)
graph = build_workflow_graph(
    "workflow.demo",
    name="Demo workflow",
    version="1.0.0",
    entry_nodes=["input"],
    nodes=[
        build_workflow_node("input", kind="input", inputs=[], outputs=[build_workflow_port("case", artifact_type="study_model/demo", dataset_value="thermal_case")]),
        build_workflow_node("output", kind="output", inputs=[build_workflow_port("case", artifact_type="study_model/demo", dataset_value="thermal_case")], outputs=[]),
    ],
    edges=[build_workflow_edge("edge-1", from_node="input", from_port="case", to_node="output", to_port="case", artifact_type="study_model/demo", dataset_value="thermal_case")],
    dataset_contract=dataset_contract,
    defaults=build_workflow_defaults(
        cache_policy="cached",
        orchestrated=False,
        dispatch_policy="central_fetch",
        placement_tags=["cpu"],
        required_capabilities=["solver.thermal"],
    ),
    dispatch_policy="central_fetch",
    operator_fetch_plan=[
        build_workflow_operator_fetch_entry(
            "input",
            operator_id="input.demo",
            package_ref="kyuubiki://operators/input.demo",
            version="1.0.0",
            integrity="sha256:demo",
            cache_scope="agent",
        )
    ],
    placement_tags=["mesh-enabled"],
    required_capabilities=["artifact-cache"],
)
modal_graph = build_modal_frame_2d_workflow()
nonlinear_graph = build_nonlinear_spring_1d_workflow(orchestrated=False)
contact_graph = build_contact_gap_1d_workflow()
workflow_graph_run = agent.run_workflow_graph(
    graph,
    {"thermal_case": {"loadcase": "baseline"}},
    timeout_s=60.0,
)
output_manifest = build_workflow_output_manifest(graph)
validated_outputs = validate_workflow_result_against_graph(graph, workflow_graph_run["result"])
material_report = build_material_report_from_payload(
    "dielectric-screening",
    {
        "result_payloads": [
            {"result": {"max_electric_field": 42.0e6, "max_flux_density": 1.2e-3}},
            {"result": {"max_electric_field": 38.0e6, "max_flux_density": 3.3e-3}},
            {"result": {"max_electric_field": 48.0e6, "max_flux_density": 0.9e-3}},
        ]
    },
)
```

Highlights:

- control-plane jobs/results/export CRUD
- control-plane workflow catalog, operator catalog, and workflow submission
- direct workflow catalog descriptor fetch plus auto graph resolution for catalog runs
- material envelope catalog workflow helper for Python automation clients
- material study execution plan contract fixture for schedulers that need to
  inspect `--plan-study`-style output before solver dispatch
- operator catalog filtering plus single-operator descriptor fetch
- expanded solve-kind coverage across structural, thermal,
  thermo-mechanical, electrostatic, modal, and nonlinear study families
- built-in workflow graph and dataset-contract validation helpers
- distributed workflow execution-hint fields for dispatch policy, operator fetch
  plan, placement tags, and required capabilities
- workflow output manifest and result validation helpers
- material-study catalog, headless result extraction, and report ranking helpers
- builder helpers for graph, node, edge, port, and dataset contract assembly
- direct solver-RPC access
- advanced solver workflow templates for modal frame, nonlinear spring, and
  contact gap runs
- high-level `KyuubikiSession` for submit/wait flows
- `KyuubikiAgentClient` for run-study, workflow-run, and chunk-browse flows
- retry policy, failure classification, and chunk iteration helpers
- reusable `KyuubikiAuth` header auth object
- thin JSON-first payload shape for AI-generated requests

Example:

- Run from [run_study.py](examples/run_study.py)
- Material envelope workflow example:
  [run_material_envelope.py](examples/run_material_envelope.py)
- Material study execution-plan example:
  [plan_material_study.py](examples/plan_material_study.py)
- Advanced solver example: [run_advanced_solvers.py](examples/run_advanced_solvers.py)
- Typical invocation:
  `PYTHONPATH=sdks/python KYUUBIKI_BASE_URL=http://127.0.0.1:4000 python3 sdks/python/examples/run_study.py`
- Material envelope invocation:
  `PYTHONPATH=sdks/python KYUUBIKI_BASE_URL=http://127.0.0.1:4000 python3 sdks/python/examples/run_material_envelope.py`
- Material study plan invocation:
  `PYTHONPATH=sdks/python python3 sdks/python/examples/plan_material_study.py`
- Smoke test:
  `PYTHONPATH=sdks/python python3 -m unittest discover -s sdks/python/tests`
