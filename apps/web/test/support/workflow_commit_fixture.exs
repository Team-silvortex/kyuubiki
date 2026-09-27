defmodule KyuubikiWeb.TestSupport.WorkflowCommitFixture do
  @moduledoc false

  import ExUnit.Assertions
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}

  def setup_coordinator do
    Store.reset()
    AnalysisResultStore.reset()
    restart_coordinator()
    assert %{"lease" => %{"status" => "owner"}} = WorkflowRecoveryCoordinator.snapshot()

    ExUnit.Callbacks.on_exit(fn ->
      Store.reset()
      AnalysisResultStore.reset()
      restart_coordinator()
    end)

    :ok
  end

  def claimed_job(opts \\ []) do
    id = "workflow-commit-#{System.unique_integer([:positive])}"

    assert {:ok, job} =
             Store.create(%{
               job_id: id,
               project_id: "workflow-commit",
               simulation_case_id: "case",
               status: :solving,
               execution_started_at: DateTime.utc_now(),
               progress: Keyword.get(opts, :progress, 0.1)
             })

    graph = %{"id" => id, "nodes" => [], "recovery_policy" => %{"retry_safety" => "idempotent"}}
    assert {:ok, pending} = WorkflowRecoveryEnvelope.new(graph, %{}, %{"job_id" => id}, %{})
    session = WorkflowRecoveryCoordinator.snapshot()["session_id"]
    assert {:ok, running, claim} = WorkflowRecoveryEnvelope.claim(pending, session, :initial)

    runtime = %{
      "workflow_id" => id,
      "current_node" => nil,
      "progress_events" => [],
      "artifacts" => %{},
      WorkflowRecoveryEnvelope.internal_key() => running
    }

    assert :ok = AnalysisResultStore.put(id, runtime)
    %{id: id, job: job, runtime: runtime, claim: claim}
  end

  def progress(node \\ "node-a"),
    do: %{"node_id" => node, "completed_nodes" => 1, "total_nodes" => 4}

  def terminal_job(status \\ :completed) do
    fixture = claimed_job()
    assert {:ok, job} = Store.apply_progress(%{job_id: fixture.id, stage: status, progress: 1.0})
    key = WorkflowRecoveryEnvelope.internal_key()
    recovery = WorkflowRecoveryEnvelope.transition(fixture.runtime[key], Atom.to_string(status))
    runtime = Map.put(fixture.runtime, key, recovery)
    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    %{fixture | job: job, runtime: runtime}
  end

  def result, do: %{"artifacts" => %{"output" => 42}, "failed_nodes" => []}

  def unchanged(fixture) do
    assert {:ok, job} = Store.get(fixture.id)
    assert job == fixture.job
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert runtime == fixture.runtime
  end

  def restart_coordinator do
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, WorkflowRecoveryCoordinator)

    assert {:ok, _pid} =
             Supervisor.restart_child(KyuubikiWeb.Supervisor, WorkflowRecoveryCoordinator)
  end
end
