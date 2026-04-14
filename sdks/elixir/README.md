# Kyuubiki Elixir SDK

Headless protocol SDK for Kyuubiki.

```elixir
client = KyuubikiSdk.ControlPlaneClient.new("http://127.0.0.1:4000")
{:ok, health} = KyuubikiSdk.ControlPlaneClient.health(client)

rpc = KyuubikiSdk.SolverRpcClient.new("127.0.0.1", 5001)
{:ok, descriptor} = KyuubikiSdk.SolverRpcClient.describe_agent(rpc)
```
