# Kyuubiki Python SDK

Headless Python SDK for Kyuubiki public protocols and AI-friendly automation.

```python
from kyuubiki_sdk import (
    ControlPlaneClient,
    KyuubikiAgentClient,
    KyuubikiAuth,
    KyuubikiRetryPolicy,
    KyuubikiSession,
    SolverRpcClient,
    build_workflow_dataset_contract,
    build_workflow_dataset_value,
    build_workflow_edge,
    build_workflow_graph,
    build_workflow_node,
    build_workflow_port,
    validate_workflow_dataset_contract,
    validate_workflow_graph,
)

auth = KyuubikiAuth.access_token("dev-token")
cp = ControlPlaneClient("http://127.0.0.1:4000", auth=auth)
print(cp.health())

rpc = SolverRpcClient("127.0.0.1", 5001)
print(rpc.describe_agent())

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
workflow_job = cp.submit_workflow_catalog_job(
    "workflow.heat-to-thermo-quad-2d",
    {"thermal_case": {"loadcase": "baseline"}},
)
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
)
```

Highlights:

- control-plane jobs/results/export CRUD
- control-plane workflow catalog, operator catalog, and workflow submission
- built-in workflow graph and dataset-contract validation helpers
- builder helpers for graph, node, edge, port, and dataset contract assembly
- direct solver-RPC access
- high-level `KyuubikiSession` for submit/wait flows
- `KyuubikiAgentClient` for run-study and chunk-browse flows
- retry policy, failure classification, and chunk iteration helpers
- reusable `KyuubikiAuth` header auth object
- thin JSON-first payload shape for AI-generated requests

Example:

- Run from [run_study.py](examples/run_study.py)
- Typical invocation:
  `PYTHONPATH=sdks/python KYUUBIKI_BASE_URL=http://127.0.0.1:4000 python3 sdks/python/examples/run_study.py`
- Smoke test:
  `PYTHONPATH=sdks/python python3 -m unittest discover -s sdks/python/tests`
