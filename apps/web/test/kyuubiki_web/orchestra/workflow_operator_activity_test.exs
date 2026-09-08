defmodule KyuubikiWeb.Orchestra.WorkflowOperatorActivityTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore, WorkflowOperatorRuntime}
  alias KyuubikiWeb.Jobs.{Store, Watchdog}
  alias KyuubikiWeb.Orchestra.{WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}

  defmodule HeartbeatClient do
    def request(_method, payload, on_progress, opts) do
      owner = :persistent_term.get({__MODULE__, :owner})
      send(owner, {:operator_started, self(), opts[:job_id]})
      hold(owner, payload, on_progress)
    end

    defp hold(owner, payload, on_progress) do
      receive do
        {:heartbeat, ref} ->
          result = on_progress.(%{"stage" => "solving", "progress" => 0.9, "iteration" => 99})
          send(owner, {:heartbeat, ref, result})
          hold(owner, payload, on_progress)

        :finish ->
          {:ok, payload}
      after
        15_000 -> {:error, :fixture_timeout}
      end
    end
  end

  setup do
    runtime = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])
    watchdog = Application.get_env(:kyuubiki_web, Watchdog, [])
    Store.reset()
    AnalysisResultStore.reset()
    :persistent_term.put({HeartbeatClient, :owner}, self())

    Application.put_env(
      :kyuubiki_web,
      WorkflowOperatorRuntime,
      Keyword.put(runtime, :solve_runtime_client, HeartbeatClient)
    )

    Application.put_env(
      :kyuubiki_web,
      Watchdog,
      Keyword.merge(watchdog, stale_job_ms: 1_500, job_timeout_ms: 60_000)
    )

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, runtime)
      Application.put_env(:kyuubiki_web, Watchdog, watchdog)
      :persistent_term.erase({HeartbeatClient, :owner})
      Store.reset()
      AnalysisResultStore.reset()
    end)

    :ok
  end

  test "operator heartbeat refreshes liveness without advancing nodes or resetting execution time" do
    {id, pid} = start_workflow()
    {:ok, initial} = Store.get(id)
    Process.sleep(1_700)
    heartbeat(pid)
    assert %{stalled: 0, timed_out: 0} = Watchdog.scan_now()
    {:ok, alive} = Store.get(id)
    assert alive.status == :solving
    assert alive.progress == initial.progress
    assert alive.iteration == initial.iteration
    assert alive.execution_started_at == initial.execution_started_at
    assert DateTime.compare(alive.updated_at, initial.updated_at) == :gt
    assert {:ok, runtime} = AnalysisResultStore.get(id)
    assert length(runtime["progress_events"]) == 1

    # Heartbeats do not disable the independent stale-work or execution timeout guards.
    Process.sleep(1_700)
    assert %{stalled: 1} = Watchdog.scan_now()
    assert {:ok, %{status: :failed}} = Store.get(id)
    send(pid, :finish)
  end

  test "throttled activity still checks ownership and cannot touch cancelled or stale claims" do
    {id, pid} = start_workflow()
    heartbeat(pid)
    {:ok, before} = Store.get(id)
    {:ok, runtime} = AnalysisResultStore.get(id)
    recovery = runtime[WorkflowRecoveryEnvelope.internal_key()]
    claim = Map.take(recovery, ["generation", "attempt", "owner_session_id"])
    activity = %{"event" => "operator_activity", "node_id" => "solve"}
    assert :ok = WorkflowRecoveryCoordinator.record_progress(id, claim, activity)
    assert {:ok, ^before} = Store.get(id)

    assert {:error, :stale_workflow_execution_claim} =
             WorkflowRecoveryCoordinator.record_progress(
               id,
               Map.put(claim, "generation", 0),
               activity
             )

    assert {:ok, ^before} = Store.get(id)
    assert :ok = WorkflowRecoveryCoordinator.cancel(id)
    assert {:error, _} = WorkflowRecoveryCoordinator.record_progress(id, claim, activity)
    send(pid, :finish)
  end

  test "fresh operator heartbeat cannot extend the original execution deadline" do
    {id, pid} = start_workflow()
    {:ok, original} = Store.get(id)
    Process.sleep(200)
    heartbeat(pid)
    config = Application.get_env(:kyuubiki_web, Watchdog, [])
    Application.put_env(:kyuubiki_web, Watchdog, Keyword.put(config, :job_timeout_ms, 100))
    assert %{stalled: 0, timed_out: 1} = Watchdog.scan_now()
    assert {:ok, finished} = Store.get(id)
    assert finished.status == :failed
    assert finished.execution_started_at == original.execution_started_at
    assert finished.message =~ "watchdog timed out job"
    send(pid, :finish)
  end

  defp heartbeat(pid) do
    ref = make_ref()
    send(pid, {:heartbeat, ref})
    assert_receive {:heartbeat, ^ref, :ok}, 2_000
  end

  defp start_workflow do
    model = %{"id" => "model", "artifact_type" => "model/bar_1d"}
    result = %{"id" => "result", "artifact_type" => "result/bar_1d"}

    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "research.operator-activity",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [model]},
        %{
          "id" => "solve",
          "kind" => "solve",
          "operator_id" => "solve.bar_1d",
          "inputs" => [model],
          "outputs" => [result]
        },
        %{"id" => "output", "kind" => "output", "inputs" => [result], "outputs" => []}
      ],
      "edges" => [
        %{
          "id" => "a",
          "from" => %{"node" => "input", "port" => "model"},
          "to" => %{"node" => "solve", "port" => "model"},
          "artifact_type" => "model/bar_1d"
        },
        %{
          "id" => "b",
          "from" => %{"node" => "solve", "port" => "result"},
          "to" => %{"node" => "output", "port" => "result"},
          "artifact_type" => "result/bar_1d"
        }
      ]
    }

    {:ok, submitted} =
      Analysis.submit_workflow_graph(%{
        "graph" => graph,
        "input_artifacts" => %{"input" => %{"value" => 41}}
      })

    id = submitted["job"]["job_id"]
    assert_receive {:operator_started, pid, ^id}, 2_000
    on_exit(fn -> send(pid, :finish) end)
    {id, pid}
  end
end
