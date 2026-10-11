defmodule KyuubikiWeb.Orchestra.WorkflowDispatchRecoveryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryDispatchGuard, as: Guard
  alias KyuubikiWeb.Orchestra.WorkflowJobRunner
  alias KyuubikiWeb.Orchestra.{WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Fixture.setup_coordinator()
    root = Path.join(System.tmp_dir!(), "workflow-replay-#{System.unique_integer([:positive])}")
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})

    on_exit(fn ->
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    %{root: root}
  end

  for state <- ~w(dispatch_boundary_unconfirmed outcome_unknown),
      reason <- [:process_restart, :runner_loss],
      policy <- ["idempotent", "checkpointed"] do
    test "#{reason} cannot replay #{state} even with #{policy} policy", ctx do
      fixture = native_job(unquote(policy))
      attempt = dispatch(fixture.id, unquote(state))
      before = File.read!(Path.join(ctx.root, attempt <> ".json"))

      if unquote(reason) == :process_restart do
        assert %{"recovered" => 0, "blocked" => 1} = WorkflowRecoveryCoordinator.recover_now()
      else
        send(WorkflowRecoveryCoordinator, {:recover_job, fixture.id, :runner_loss})
        assert WorkflowRecoveryCoordinator.snapshot()["lease"]["status"] == "owner"
      end

      assert_blocked(fixture)
      assert File.read!(Path.join(ctx.root, attempt <> ".json")) == before
      assert File.ls!(ctx.root) == [attempt <> ".json"]
      assert %{"recovered" => 0, "blocked" => 0} = WorkflowRecoveryCoordinator.recover_now()

      assert {:error, :workflow_already_dispatched} =
               WorkflowRecoveryCoordinator.dispatch(fixture.id)
    end
  end

  defp assert_blocked(fixture) do
    assert {:ok, %{status: :failed, message: message}} = Store.get(fixture.id)
    assert message =~ "workflow_dispatch_outcome_unconfirmed"
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    recovery = runtime[WorkflowRecoveryEnvelope.internal_key()]
    assert recovery["state"] == "recovery_blocked"
    assert recovery["generation"] == fixture.claim["generation"]
    assert recovery["attempt"] == fixture.claim["attempt"]

    assert recovery["envelope"] ==
             fixture.runtime[WorkflowRecoveryEnvelope.internal_key()]["envelope"]

    assert List.last(recovery["history"])["next_action"] ==
             "inspect_original_dispatch_before_explicit_resubmit"

    assert :error = WorkflowJobRunner.running(fixture.id)
  end

  for state <- ~w(not_dispatched observed_executed observed_failed observed_blocked),
      reason <- [:process_restart, :runner_loss] do
    test "#{state} does not add an ambiguity fence to #{reason}" do
      fixture = native_job()
      dispatch(fixture.id, unquote(state))
      assert :ok = validate(fixture, unquote(reason))
      Fixture.unchanged(fixture)
    end
  end

  test "an older ambiguous attempt is not erased by a newer terminal observation" do
    fixture = native_job()
    dispatch(fixture.id, "outcome_unknown")
    dispatch(fixture.id, "observed_executed")
    assert {:error, :workflow_dispatch_outcome_unconfirmed} = validate(fixture)
    Fixture.unchanged(fixture)
  end

  test "actual parent identity blocks even if the child task name differs" do
    fixture = native_job()
    changed = put_in(request(fixture.id), ["params", "task_ir", "task_id"], "different-task")
    assert {:ok, _} = Journal.begin_dispatch(changed, endpoint())
    assert {:error, :workflow_dispatch_outcome_unconfirmed} = validate(fixture)
  end

  test "an explicit foreign parent is not inferred from a matching child task name" do
    fixture = native_job()
    foreign = put_in(request(fixture.id), ["params", "job_id"], "other-parent")
    assert {:ok, _} = Journal.begin_dispatch(foreign, endpoint())
    assert :ok = validate(fixture)
  end

  for format <- [:legacy, :unbound] do
    test "#{format} exact child identity denies replay without gaining execution authority",
         ctx do
      fixture = native_job()
      attempt = dispatch(fixture.id, "outcome_unknown")
      stop_supervised(Journal)
      path = Path.join(ctx.root, attempt <> ".json")
      {:ok, records} = Files.load(ctx.root)
      original = records[attempt]

      record =
        if unquote(format) == :legacy do
          original
          |> Map.delete("job_id")
          |> Map.put("schema_version", "kyuubiki.operator-task-dispatch-record/v1")
        else
          Map.put(original, "job_id", nil)
        end

      File.rm!(path)
      assert :ok = Files.write(ctx.root, record)
      before = File.read!(path)
      start_supervised!({Journal, root: ctx.root})
      assert {:error, :workflow_dispatch_outcome_unconfirmed} = validate(fixture)
      assert File.read!(path) == before
      assert is_nil(Files.job_id(record))
    end
  end

  test "unbound prefix collisions do not count as an exact graph child" do
    fixture = native_job()

    changed =
      request(fixture.id)
      |> update_in(["params"], &Map.delete(&1, "job_id"))
      |> update_in(["params", "task_ir", "task_id"], &(&1 <> ":other"))

    assert {:ok, _} = Journal.begin_dispatch(changed, endpoint())
    assert :ok = validate(fixture)
  end

  test "journal loss blocks native recovery without killing the coordinator" do
    fixture = native_job()
    pid = Process.whereis(WorkflowRecoveryCoordinator)
    stop_supervised(Journal)
    assert %{"recovered" => 0, "blocked" => 1} = WorkflowRecoveryCoordinator.recover_now()
    assert Process.whereis(WorkflowRecoveryCoordinator) == pid
    assert {:ok, %{status: :failed, message: message}} = Store.get(fixture.id)
    assert message =~ "workflow_dispatch_journal_unavailable"
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)

    assert List.last(runtime[WorkflowRecoveryEnvelope.internal_key()]["history"])["next_action"] ==
             "restore_dispatch_journal_before_recovery"

    assert :error = WorkflowJobRunner.running(fixture.id)
  end

  test "an invalid journal generation is not treated as empty", ctx do
    fixture = native_job()
    attempt = dispatch(fixture.id, "outcome_unknown")
    stop_supervised(Journal)
    path = Path.join(ctx.root, attempt <> ".json")
    File.write!(path, "{}")
    start_supervised!({Journal, root: ctx.root})

    assert {:error,
            {:workflow_dispatch_journal_unavailable, :operator_task_dispatch_journal_invalid}} =
             validate(fixture)

    assert File.read!(path) == "{}"
    Fixture.unchanged(fixture)
  end

  test "purely local recovery does not depend on journal availability, unlike native admission" do
    local = Fixture.claimed_job()
    native = native_job()
    stop_supervised(Journal)
    assert :ok = validate(local)
    pending = fresh_pending(native)

    assert {:error,
            {:workflow_dispatch_journal_unavailable, :operator_task_dispatch_journal_unavailable}} =
             Guard.claim(native.id, pending, "new-owner", :initial, 3)

    assert {:error, :workflow_already_dispatched} = validate(native, :initial)
  end

  test "ordinary startup cannot bypass an ambiguous dispatch after a rollback to pending" do
    fixture = native_job()
    dispatch(fixture.id, "outcome_unknown")

    runtime =
      Map.put(fixture.runtime, WorkflowRecoveryEnvelope.internal_key(), fresh_pending(fixture))

    assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, fixture.runtime, runtime)

    assert {:error, {:workflow_recovery_blocked, :workflow_dispatch_outcome_unconfirmed}} =
             WorkflowRecoveryCoordinator.dispatch(fixture.id)

    assert_blocked(%{fixture | runtime: runtime, claim: %{"generation" => 0, "attempt" => 0}})
  end

  test "a running generation relabelled pending cannot masquerade as first admission" do
    fixture = native_job()

    recycled =
      Map.put(fixture.runtime[WorkflowRecoveryEnvelope.internal_key()], "state", "pending")

    assert {:error, :workflow_already_dispatched} =
             Guard.claim(fixture.id, recycled, "new-owner", :initial, 3)

    Fixture.unchanged(fixture)
  end

  test "a clean native first admission still works with a healthy empty journal" do
    fixture = native_job()

    assert {:ok, claimed, _} =
             Guard.claim(fixture.id, fresh_pending(fixture), "new-owner", :initial, 3)

    assert claimed["generation"] == 1
    Fixture.unchanged(fixture)
  end

  test "envelope tampering is rejected before consulting the dispatch journal" do
    fixture = native_job()
    recovery = fixture.runtime[WorkflowRecoveryEnvelope.internal_key()]
    tampered = put_in(recovery, ["envelope", "orchestration_context", "job_id"], "other")
    stop_supervised(Journal)

    assert {:error, :workflow_recovery_digest_mismatch} =
             Guard.claim(fixture.id, tampered, "new-owner", :runner_loss, 3)
  end

  test "a digest-valid execution context cannot borrow another parent's identity" do
    fixture = native_job()

    assert {:error, :workflow_recovery_identity_mismatch} =
             Guard.claim(
               "other",
               fixture.runtime[WorkflowRecoveryEnvelope.internal_key()],
               "new-owner",
               :runner_loss,
               3
             )
  end

  for nodes <- [
        nil,
        "nodes",
        [nil],
        [%{"kind" => "extract", "operator_id" => "extract.thermal_result_diagnostics"}]
      ] do
    test "malformed native graph #{inspect(nodes)} fails without a recovery crash" do
      fixture = Fixture.claimed_job()
      graph = %{"id" => fixture.id, "nodes" => unquote(Macro.escape(nodes))}

      assert {:ok, recovery} =
               WorkflowRecoveryEnvelope.new(graph, %{}, %{"job_id" => fixture.id}, %{})

      assert {:error, :invalid_workflow_execution_envelope} =
               Guard.claim(fixture.id, recovery, "new-owner", :initial, 3)
    end
  end

  defp validate(fixture, reason \\ :process_restart) do
    case Guard.claim(
           fixture.id,
           fixture.runtime[WorkflowRecoveryEnvelope.internal_key()],
           "new-owner",
           reason,
           3
         ) do
      {:ok, _, _} -> :ok
      error -> error
    end
  end

  defp fresh_pending(fixture) do
    envelope = fixture.runtime[WorkflowRecoveryEnvelope.internal_key()]["envelope"]

    assert {:ok, pending} =
             WorkflowRecoveryEnvelope.new(
               envelope["graph"],
               envelope["input_artifacts"],
               envelope["orchestration_context"],
               envelope["response_options"]
             )

    pending
  end

  defp dispatch(job, state) do
    assert {:ok, attempt} = Journal.begin_dispatch(request(job), endpoint())

    if state != "dispatch_boundary_unconfirmed",
      do: assert(:ok = Journal.finish_dispatch(attempt, state))

    attempt
  end

  defp native_job(policy \\ "idempotent") do
    fixture = Fixture.claimed_job()

    graph = %{
      "id" => fixture.id,
      "nodes" => [%{"id" => "diagnose", "kind" => "extract", "operator_id" => operator()}],
      "edges" => [],
      "recovery_policy" => %{"retry_safety" => policy}
    }

    graph =
      if policy == "checkpointed" do
        put_in(graph, ["recovery_policy", "checkpoint"], %{
          "operator_task_batch_checkpoint_verification_contract" =>
            "kyuubiki.operator_task_batch_checkpoint_verification/v1",
          "status" => "verified",
          "checkpoint_digest" => String.duplicate("b", 64)
        })
      else
        graph
      end

    assert {:ok, pending} =
             WorkflowRecoveryEnvelope.new(graph, %{}, %{"job_id" => fixture.id}, %{})

    session = WorkflowRecoveryCoordinator.snapshot()["session_id"]
    assert {:ok, running, claim} = WorkflowRecoveryEnvelope.claim(pending, session, :initial)
    runtime = Map.put(fixture.runtime, WorkflowRecoveryEnvelope.internal_key(), running)
    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    %{fixture | runtime: runtime, claim: claim}
  end

  defp operator, do: "extract.thermal_result_diagnostics"
  defp endpoint, do: %{id: "owner", host: "127.0.0.1", port: 1}

  defp request(job),
    do: %{
      "id" => "original-rpc",
      "params" => %{
        "job_id" => job,
        "task_ir" => %{
          "task_id" => "#{job}:diagnose:#{operator()}",
          "integrity" => %{"task_digest" => String.duplicate("a", 64)},
          "operator" => %{"id" => operator()},
          "execution_program" => %{"program_id" => operator()}
        }
      }
    }
end
