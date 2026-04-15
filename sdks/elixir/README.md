# Kyuubiki Elixir SDK

Headless Elixir SDK for Kyuubiki control-plane and solver-rpc protocols.

```elixir
auth = KyuubikiSdk.Auth.access_token("dev-token")
client = KyuubikiSdk.ControlPlaneClient.new("http://127.0.0.1:4000", auth: auth)
{:ok, health} = KyuubikiSdk.ControlPlaneClient.health(client)

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
pages = Enum.take(KyuubikiSdk.AgentClient.stream_result_chunks(session, bundle.terminal["job"]["job_id"], "nodes", page_size: 250), 2)
```

Highlights:

- jobs/results/export CRUD on the control plane
- direct framed TCP solver-RPC access
- `KyuubikiSdk.Session` for submit/batch/wait flows
- `KyuubikiSdk.AgentClient` for run-study and chunk-browse flows
- retry, failure classification, and chunk streaming helpers
- `KyuubikiSdk.Auth` and structured `KyuubikiSdk.Error`
- BEAM-friendly thin wrapper over the public protocol

Example:

- Run from [run_study.exs](/Users/Shared/chroot/dev/kyuubiki/sdks/elixir/examples/run_study.exs)
- Typical invocation:
  `cd sdks/elixir && KYUUBIKI_BASE_URL=http://127.0.0.1:4000 mix run examples/run_study.exs`
