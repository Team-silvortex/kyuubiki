# Kyuubiki Python SDK

Minimal headless Python SDK for Kyuubiki public protocols.

```python
from kyuubiki_sdk import ControlPlaneClient, SolverRpcClient

cp = ControlPlaneClient("http://127.0.0.1:4000")
print(cp.health())

rpc = SolverRpcClient("127.0.0.1", 5001)
print(rpc.describe_agent())
```
