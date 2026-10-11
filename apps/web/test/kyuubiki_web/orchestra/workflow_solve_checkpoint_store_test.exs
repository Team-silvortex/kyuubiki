defmodule KyuubikiWeb.Orchestra.WorkflowSolveCheckpointStoreTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.Orchestra.WorkflowSolveCheckpoint, as: Checkpoint
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture
  alias KyuubikiWeb.TestSupport.WorkflowSolveCheckpointFixture, as: SolveFixture
  alias KyuubikiWeb.WorkflowGraphRunner

  setup do
    Fixture.setup_coordinator()
  end

  test "success is committed before downstream publication and later reused read-only" do
    fixture = SolveFixture.claimed()
    result = %{"value" => 1.0, "small" => 1.0e-30}
    assert :execute = access(fixture, {:lookup, fixture.request})
    assert :ok = access(fixture, {:commit, fixture.request, result})
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert {:ok, job} = Store.get(fixture.id)
    key = WorkflowRecoveryEnvelope.internal_key()
    assert job.status == fixture.job.status
    assert job.progress == fixture.job.progress
    assert job.execution_started_at == fixture.job.execution_started_at
    assert runtime["artifacts"] == %{}
    assert runtime["current_node"] == nil
    assert runtime["progress_events"] == []
    assert runtime[key][Checkpoint.key()]["entries"]["solve"]["producer_claim"] == fixture.claim
    assert :ok = access(fixture, {:commit, fixture.request, result})
    Fixture.unchanged(%{fixture | runtime: runtime, job: job})

    {:ok, replay, claim} =
      WorkflowRecoveryEnvelope.claim(
        runtime[key],
        fixture.claim["owner_session_id"],
        :runner_loss
      )

    resumed = Map.put(runtime, key, replay)
    assert :ok = AnalysisResultStore.put(fixture.id, resumed)
    resumed_fixture = %{fixture | runtime: resumed, claim: claim, job: job}
    assert {:ok, ^result} = access(resumed_fixture, {:lookup, fixture.request})
    Fixture.unchanged(resumed_fixture)
    assert {:ok, public} = Analysis.fetch_job(fixture.id)
    assert public["result"]["recovery"]["solve_checkpoint_count"] == 1
    refute Map.has_key?(public["result"], key)

    assert {:error, :active_workflow_result_is_read_only} =
             Analysis.update_result(fixture.id, %{"forged" => true})
  end

  test "small artifacts retain integral float spelling after SQL and JSON round trips" do
    fixture = SolveFixture.claimed()
    output = %{"values" => [1.0, 0.0, -1.0, 2.5e-30]}
    assert :ok = access(fixture, {:commit, fixture.request, output})
    {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    key = WorkflowRecoveryEnvelope.internal_key()
    json = runtime[key][Checkpoint.key()]["entries"]["solve"]["result_json"]
    assert json =~ "1.0"
    assert is_float(Jason.decode!(json)["values"] |> hd())
    assert :ok = Checkpoint.verify(runtime[key], fixture.id)
  end

  for field <- ~w(generation attempt owner_session_id) do
    test "stale #{field} cannot read or commit an artifact" do
      fixture = SolveFixture.claimed()

      stale =
        Map.update!(fixture.claim, unquote(field), fn value ->
          if is_integer(value), do: value + 1, else: value <> "-stale"
        end)

      for action <- [{:lookup, fixture.request}, {:commit, fixture.request, 42}] do
        assert {:error, :stale_workflow_execution_claim} =
                 WorkflowRecoveryCoordinator.solve_checkpoint(fixture.id, stale, action)

        Fixture.unchanged(fixture)
      end
    end
  end

  for status <- [:completed, :cancelled, :failed] do
    test "a #{status} job cannot read or publish solve checkpoints" do
      fixture = Fixture.terminal_job(unquote(status))

      for action <- [{:lookup, %{}}, {:commit, %{}, 42}] do
        assert {:error, {:workflow_job_terminal, unquote(status)}} = access(fixture, action)
        Fixture.unchanged(fixture)
      end
    end
  end

  test "a displaced lease cannot read, retain or block a solve artifact" do
    fixture = SolveFixture.claimed()
    name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
    {:ok, previous} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(previous)
    {:ok, takeover} = LeaseStore.acquire(name, "another-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)
    assert {:error, :orchestra_lease_lost} = access(fixture, {:lookup, fixture.request})
    assert {:error, _} = access(fixture, {:commit, fixture.request, 42})

    assert {:error, _} =
             WorkflowRecoveryCoordinator.block_solve_checkpoint(
               fixture.id,
               fixture.claim,
               :invalid
             )

    Fixture.unchanged(fixture)
  end

  test "conflicting or malformed checkpoints never overwrite completed evidence" do
    fixture = SolveFixture.claimed()
    assert :ok = access(fixture, {:commit, fixture.request, 42})
    {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    {:ok, job} = Store.get(fixture.id)
    saved = %{fixture | runtime: runtime, job: job}
    assert {:error, :conflict} = access(saved, {:commit, fixture.request, 99})

    for request <- [
          Map.put(fixture.request, "node_id", "other"),
          Map.put(fixture.request, "node_sha256", String.duplicate("0", 64)),
          Map.put(fixture.request, "extra", "untrusted")
        ] do
      assert {:error, :invalid} = access(saved, {:commit, request, 99})
      Fixture.unchanged(saved)
    end
  end

  test "corrupt retained artifacts block replay before claim advancement" do
    fixture = SolveFixture.claimed()
    assert :ok = access(fixture, {:commit, fixture.request, 42})
    {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    key = WorkflowRecoveryEnvelope.internal_key()
    corrupt = put_in(runtime, [key, Checkpoint.key(), "entries", "solve", "result_json"], "99")
    assert :ok = AnalysisResultStore.put(fixture.id, corrupt)
    assert %{"blocked" => 1, "recovered" => 0} = WorkflowRecoveryCoordinator.recover_now()
    {:ok, blocked} = AnalysisResultStore.get(fixture.id)
    assert blocked[key]["state"] == "recovery_blocked"
    assert blocked[key]["generation"] == fixture.claim["generation"]
    assert blocked[key]["envelope"] == runtime[key]["envelope"]
    assert blocked[key][Checkpoint.key()] == corrupt[key][Checkpoint.key()]
    {:ok, job} = Store.get(fixture.id)
    assert job.message =~ "workflow_solve_checkpoint_invalid"
  end

  test "oversize output omits a checkpoint without a runtime or job write" do
    fixture = SolveFixture.claimed()
    large = [String.duplicate("x", 270_000), String.duplicate("x", 270_000)]
    assert :ok = access(fixture, {:commit, fixture.request, large})
    Fixture.unchanged(fixture)
  end

  for reason <- ~w(invalid input_changed conflict storage_unavailable commit_failed)a do
    test "#{reason} retains the envelope in a blocked job rather than a successful result" do
      fixture = SolveFixture.claimed()
      assert :ok = access(fixture, {:commit, fixture.request, 42})
      {:ok, runtime} = AnalysisResultStore.get(fixture.id)
      key = WorkflowRecoveryEnvelope.internal_key()

      assert :ok =
               WorkflowRecoveryCoordinator.block_solve_checkpoint(
                 fixture.id,
                 fixture.claim,
                 unquote(reason)
               )

      {:ok, ended} = AnalysisResultStore.get(fixture.id)
      assert ended[key]["state"] == "recovery_blocked"
      assert ended[key][Checkpoint.key()] == runtime[key][Checkpoint.key()]
      assert ended[key]["envelope"] == runtime[key]["envelope"]

      assert List.last(ended[key]["history"])["next_action"] ==
               "inspect_solve_checkpoint_before_explicit_resubmit"

      assert WorkflowRecoveryCoordinator.recover_now()["recovered"] == 0
    end
  end

  test "invalid blocking reasons cannot be persisted or echoed" do
    fixture = SolveFixture.claimed()

    for reason <- ["secret", %{token: "secret"}, nil, :unknown] do
      assert {:error, :invalid_workflow_solve_checkpoint_reason} =
               WorkflowRecoveryCoordinator.block_solve_checkpoint(
                 fixture.id,
                 fixture.claim,
                 reason
               )

      Fixture.unchanged(fixture)
    end
  end

  test "on_error skip cannot publish a checkpoint failure" do
    port = %{"id" => "payload", "artifact_type" => "payload/json"}

    edge = fn from, to ->
      %{
        "id" => from <> to,
        "artifact_type" => "payload/json",
        "from" => %{"node" => from, "port" => "payload"},
        "to" => %{"node" => to, "port" => "payload"}
      }
    end

    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "skip-checkpoint",
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [port]},
        Map.put(SolveFixture.solve_node(), "config", %{"on_error" => "skip"}),
        %{"id" => "output", "kind" => "output", "inputs" => [port], "outputs" => []}
      ],
      "edges" => [edge.("input", "solve"), edge.("solve", "output")]
    }

    error = {:workflow_solve_checkpoint_blocked, :commit_failed}

    assert {:error, {:workflow_node_error, "solve", ^error}} =
             WorkflowGraphRunner.run(graph, %{"input" => %{}},
               execute_solve: fn _, _, _ -> {:error, error} end,
               execute_transform: fn _, _, _ -> flunk("no transform") end,
               execute_extract: fn _, _, _ -> flunk("no extract") end,
               execute_export: fn _, _, _ -> flunk("no export") end
             )
  end

  defp access(fixture, action),
    do: WorkflowRecoveryCoordinator.solve_checkpoint(fixture.id, fixture.claim, action)
end
