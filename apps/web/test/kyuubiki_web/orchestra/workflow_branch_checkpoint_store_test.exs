defmodule KyuubikiWeb.Orchestra.WorkflowBranchCheckpointStoreTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.{Analysis, AnalysisResultStore}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.Orchestra.WorkflowBranchCheckpoint, as: Checkpoint
  alias KyuubikiWeb.TestSupport.WorkflowBranchCheckpointFixture, as: BranchFixture
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Fixture.setup_coordinator()
  end

  test "branch commits and later reads preserve job progress, deadline and private evidence" do
    fixture = BranchFixture.claimed()
    decision = BranchFixture.decision()
    assert :execute = access(fixture, {:lookup, fixture.request})
    assert :ok = access(fixture, {:commit, fixture.request, decision})
    {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    {:ok, job} = Store.get(fixture.id)
    assert %{job | updated_at: fixture.job.updated_at} == fixture.job
    assert runtime["artifacts"] == %{}
    assert runtime["current_node"] == nil
    key = WorkflowRecoveryEnvelope.internal_key()
    assert runtime[key][Checkpoint.key()]["entries"]["branch"]["producer_claim"] == fixture.claim
    assert :ok = access(fixture, {:commit, fixture.request, decision})
    saved = %{fixture | runtime: runtime, job: job}
    Fixture.unchanged(saved)

    {:ok, resumed, claim} =
      WorkflowRecoveryEnvelope.claim(
        runtime[key],
        fixture.claim["owner_session_id"],
        :runner_loss
      )

    runtime = Map.put(runtime, key, resumed)
    :ok = AnalysisResultStore.put(fixture.id, runtime)
    resumed_fixture = %{saved | runtime: runtime, claim: claim}
    assert {:ok, ^decision} = access(resumed_fixture, {:lookup, fixture.request})
    Fixture.unchanged(resumed_fixture)
    {:ok, public} = Analysis.fetch_job(fixture.id)
    assert public["result"]["recovery"]["branch_checkpoint_count"] == 1
    refute Map.has_key?(public["result"], key)

    assert {:error, :active_workflow_result_is_read_only} =
             Analysis.update_result(fixture.id, %{})
  end

  for field <- ~w(generation attempt owner_session_id) do
    test "stale #{field} cannot read, commit or block a branch" do
      fixture = BranchFixture.claimed()

      stale =
        Map.update!(fixture.claim, unquote(field), fn value ->
          if is_integer(value), do: value + 1, else: value <> "-stale"
        end)

      for action <- [
            {:lookup, fixture.request},
            {:commit, fixture.request, BranchFixture.decision()}
          ] do
        assert {:error, :stale_workflow_execution_claim} =
                 WorkflowRecoveryCoordinator.branch_checkpoint(fixture.id, stale, action)

        Fixture.unchanged(fixture)
      end

      assert {:error, :stale_workflow_execution_claim} =
               WorkflowRecoveryCoordinator.block_branch_checkpoint(fixture.id, stale, :invalid)

      Fixture.unchanged(fixture)
    end
  end

  for status <- [:completed, :failed, :cancelled] do
    test "a #{status} job cannot read or publish branch checkpoints" do
      fixture = Fixture.terminal_job(unquote(status))

      for action <- [{:lookup, %{}}, {:commit, %{}, BranchFixture.decision()}] do
        assert {:error, {:workflow_job_terminal, unquote(status)}} = access(fixture, action)
        Fixture.unchanged(fixture)
      end
    end
  end

  test "lease takeover fences branch reads, writes and blocking transitions" do
    fixture = BranchFixture.claimed()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    {:ok, previous} = LeaseStore.current(name)
    :ok = LeaseStore.release(previous)
    {:ok, takeover} = LeaseStore.acquire(name, "another-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)
    assert {:error, :orchestra_lease_lost} = access(fixture, {:lookup, fixture.request})
    assert {:error, _} = access(fixture, {:commit, fixture.request, BranchFixture.decision()})

    assert {:error, _} =
             WorkflowRecoveryCoordinator.block_branch_checkpoint(
               fixture.id,
               fixture.claim,
               :invalid
             )

    Fixture.unchanged(fixture)
  end

  test "corrupt branch records block before claim advancement and retain original evidence" do
    fixture = BranchFixture.claimed()
    :ok = access(fixture, {:commit, fixture.request, BranchFixture.decision()})
    {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    key = WorkflowRecoveryEnvelope.internal_key()

    corrupt =
      put_in(
        runtime,
        [key, Checkpoint.key(), "entries", "branch", "decision", "chosen_output"],
        "if_false"
      )

    :ok = AnalysisResultStore.put(fixture.id, corrupt)
    assert %{"blocked" => 1, "recovered" => 0} = WorkflowRecoveryCoordinator.recover_now()
    {:ok, blocked} = AnalysisResultStore.get(fixture.id)
    assert blocked[key]["state"] == "recovery_blocked"
    assert blocked[key]["generation"] == 1
    assert blocked[key][Checkpoint.key()] == corrupt[key][Checkpoint.key()]
    assert blocked[key]["envelope"] == runtime[key]["envelope"]
    {:ok, job} = Store.get(fixture.id)
    assert job.message =~ "workflow_branch_checkpoint_invalid"
  end

  test "conflicting decisions and malformed requests never overwrite successful evidence" do
    fixture = BranchFixture.claimed()
    :ok = access(fixture, {:commit, fixture.request, BranchFixture.decision()})
    {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    {:ok, job} = Store.get(fixture.id)
    saved = %{fixture | runtime: runtime, job: job}

    assert {:error, :conflict} =
             access(fixture, {:commit, fixture.request, BranchFixture.decision(false)})

    for req <- [
          Map.put(fixture.request, "extra", 42),
          Map.put(fixture.request, "node_id", "other"),
          %{}
        ] do
      assert {:error, :invalid} = access(fixture, {:commit, req, BranchFixture.decision()})
      Fixture.unchanged(saved)
    end
  end

  for reason <- ~w(invalid input_changed conflict storage_unavailable commit_failed)a do
    test "#{reason} retains branch evidence in a blocked job" do
      fixture = BranchFixture.claimed()
      :ok = access(fixture, {:commit, fixture.request, BranchFixture.decision()})
      {:ok, runtime} = AnalysisResultStore.get(fixture.id)
      key = WorkflowRecoveryEnvelope.internal_key()

      assert :ok =
               WorkflowRecoveryCoordinator.block_branch_checkpoint(
                 fixture.id,
                 fixture.claim,
                 unquote(reason)
               )

      {:ok, ended} = AnalysisResultStore.get(fixture.id)
      assert ended[key]["state"] == "recovery_blocked"
      assert ended[key][Checkpoint.key()] == runtime[key][Checkpoint.key()]
      assert ended[key]["envelope"] == runtime[key]["envelope"]

      assert List.last(ended[key]["history"])["next_action"] ==
               "inspect_branch_checkpoint_before_explicit_resubmit"

      assert WorkflowRecoveryCoordinator.recover_now()["recovered"] == 0
    end
  end

  test "invalid failure reasons are neither stored nor echoed" do
    fixture = BranchFixture.claimed()

    for reason <- ["secret", %{"token" => "secret"}, nil, :unknown] do
      assert {:error, :invalid_workflow_branch_checkpoint_reason} =
               WorkflowRecoveryCoordinator.block_branch_checkpoint(
                 fixture.id,
                 fixture.claim,
                 reason
               )

      Fixture.unchanged(fixture)
    end
  end

  test "on_error skip cannot publish a branch checkpoint failure" do
    owner = self()
    graph = condition_graph()

    callback = fn
      {:lookup, _} -> :execute
      {:commit, _, _} -> {:error, :commit_failed}
    end

    error = {:workflow_branch_checkpoint_blocked, :commit_failed}

    assert {:error, {:workflow_node_error, "branch", ^error}} =
             KyuubikiWeb.Orchestra.Engine.execute_workflow_graph(
               graph,
               %{"input" => %{"value" => 1}},
               %{},
               fn event -> send(owner, {:branch_progress, event["node_id"]}) end,
               %{},
               branch_checkpoint: callback
             )

    refute_receive {:branch_progress, "branch"}
    refute_receive {:branch_progress, "output"}
  end

  test "invalid condition executors return node errors instead of crashing the graph runner" do
    graph = condition_graph()

    graph =
      Map.update!(
        graph,
        "nodes",
        &Enum.map(&1, fn node ->
          if node["kind"] == "condition",
            do: Map.update!(node, "config", fn config -> Map.delete(config, "on_error") end),
            else: node
        end)
      )

    for {callback, reason} <- [
          {:invalid, :invalid_condition_executor},
          {fn _, _ -> nil end, :invalid_condition_result},
          {fn _, _ -> {:ok, %{}} end, :invalid_condition_decision},
          {fn _, _ -> {:error, :predicate_failed} end, :predicate_failed}
        ] do
      assert {:error, {:workflow_node_error, "branch", ^reason}} =
               KyuubikiWeb.WorkflowGraphRunner.run(graph, %{"input" => %{"value" => 1}},
                 execute_condition: callback,
                 execute_solve: fn _, _, _ -> flunk("no solve") end,
                 execute_transform: fn _, _, _ -> flunk("no transform") end,
                 execute_extract: fn _, _, _ -> flunk("no extract") end,
                 execute_export: fn _, _, _ -> flunk("no export") end
               )
    end
  end

  defp condition_graph do
    graph = BranchFixture.graph()

    %{
      graph
      | "nodes" => Enum.filter(graph["nodes"], &(&1["id"] in ["input", "branch", "output"])),
        "edges" => [
          %{
            "id" => "input-branch",
            "artifact_type" => "payload/json",
            "from" => %{"node" => "input", "port" => "payload"},
            "to" => %{"node" => "branch", "port" => "payload"}
          },
          %{
            "id" => "branch-output",
            "artifact_type" => "payload/json",
            "from" => %{"node" => "branch", "port" => "if_true"},
            "to" => %{"node" => "output", "port" => "payload"}
          }
        ]
    }
  end

  defp access(fixture, action),
    do: WorkflowRecoveryCoordinator.branch_checkpoint(fixture.id, fixture.claim, action)
end
