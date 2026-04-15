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
```

Highlights:

- control-plane jobs/results/export CRUD
- direct solver-RPC access
- high-level `KyuubikiSession` for submit/wait flows
- `KyuubikiAgentClient` for run-study and chunk-browse flows
- retry policy, failure classification, and chunk iteration helpers
- reusable `KyuubikiAuth` header auth object
- thin JSON-first payload shape for AI-generated requests

Example:

- Run from [run_study.py](/Users/Shared/chroot/dev/kyuubiki/sdks/python/examples/run_study.py)
- Typical invocation:
  `PYTHONPATH=sdks/python KYUUBIKI_BASE_URL=http://127.0.0.1:4000 python3 sdks/python/examples/run_study.py`
- Smoke test:
  `PYTHONPATH=sdks/python python3 -m unittest discover -s sdks/python/tests`
