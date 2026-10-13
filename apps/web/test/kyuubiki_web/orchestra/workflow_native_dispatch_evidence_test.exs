defmodule KyuubikiWeb.Orchestra.WorkflowNativeDispatchEvidenceTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore, WorkflowNativeOperatorRuntime}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowNativeDispatchEvidence, as: Evidence
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryDispatchGuard, as: Guard
  alias KyuubikiWeb.Orchestra.{OperatorTaskIR, OperatorTaskExecutionSummary, WorkflowJobRunner}
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @identity ~w(task_id task_digest operator_id program_id)

  setup do
    Fixture.setup_coordinator()
    root = Path.join(System.tmp_dir!(), "native-evidence-#{System.unique_integer([:positive])}")
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})

    on_exit(fn ->
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    %{root: root}
  end

  for state <-
        ~w(dispatch_boundary_unconfirmed outcome_unknown observed_executed observed_failed observed_blocked not_dispatched),
      loss <- [:missing_root, :empty_root] do
    test "#{state} cannot be forgotten across a journal restart with #{loss}", ctx do
      f = fixture()
      attempt = retain(f, unquote(state))
      {:ok, before} = AnalysisResultStore.get(f.id)
      assert :ok = stop_supervised(Journal)
      File.rm_rf!(ctx.root)
      if unquote(loss) == :empty_root, do: File.mkdir_p!(ctx.root)
      start_supervised!({Journal, root: ctx.root})

      # The journal alone still correctly supports a first-use empty directory.
      assert :ok = Journal.workflow_replay_fence(f.id, [f.request["task_id"]])

      assert {:error,
              {:workflow_dispatch_journal_unavailable, :workflow_dispatch_evidence_missing}} =
               Guard.claim(f.id, before[Envelope.internal_key()], "restart", :process_restart, 3)

      Fixture.restart_coordinator()
      Coordinator.recover_now()
      {:ok, job} = Store.get(f.id)
      {:ok, runtime} = AnalysisResultStore.get(f.id)
      recovery = runtime[Envelope.internal_key()]
      assert job.status == :failed
      assert job.message =~ "workflow_dispatch_evidence_missing"
      assert recovery["state"] == "recovery_blocked"
      assert recovery["generation"] == f.claim["generation"]
      assert recovery["attempt"] == f.claim["attempt"]
      assert recovery["envelope"] == before[Envelope.internal_key()]["envelope"]
      assert recovery[Evidence.key()] == before[Envelope.internal_key()][Evidence.key()]
      assert recovery[Evidence.key()]["entries"][attempt]
      assert :error = WorkflowJobRunner.running(f.id)

      assert List.last(recovery["history"])["next_action"] ==
               "restore_dispatch_journal_before_recovery"

      assert Coordinator.recover_now()["recovered"] == 0
      assert File.exists?(ctx.root) == (unquote(loss) == :empty_root)
      if unquote(loss) == :empty_root, do: assert(File.ls!(ctx.root) == [])
    end
  end

  test "another valid attempt for the same task cannot replace lost evidence", ctx do
    f = fixture()
    old = retain(f, "observed_executed")
    assert {:ok, replacement} = Journal.begin_dispatch(f.rpc, endpoint())
    assert :ok = Journal.finish_dispatch(replacement, "not_dispatched")
    assert :ok = stop_supervised(Journal)
    File.rm!(Path.join(ctx.root, old <> ".json"))
    start_supervised!({Journal, root: ctx.root})
    assert {:error, :workflow_dispatch_evidence_missing} = access(f, {:lookup, f.request})
    assert {:ok, [%{"attempt_id" => ^replacement}]} = Journal.lookup_task(f.request["task_id"])
  end

  test "a fresh native workflow with no dispatch remains recoverable without creating files",
       ctx do
    f = fixture()
    assert :ok = access(f, {:lookup, f.request})

    assert {:ok, _, _} =
             Guard.claim(f.id, f.runtime[Envelope.internal_key()], "restart", :runner_loss, 3)

    Fixture.unchanged(f)
    refute File.exists?(ctx.root)
  end

  test "committed native evidence survives history turnover before and after restart", ctx do
    f = fixture()
    retain(f, "observed_executed")
    standalone = put_in(f.rpc, ["params", "job_id"], "standalone")

    for _ <- 1..140 do
      {:ok, id} = Journal.begin_dispatch(standalone, endpoint())
      assert :ok = Journal.finish_dispatch(id, "not_dispatched")
    end

    assert :ok = access(f, {:lookup, f.request})
    assert :ok = stop_supervised(Journal)
    start_supervised!({Journal, root: ctx.root})
    assert :ok = access(f, {:lookup, f.request})
    {:ok, runtime} = AnalysisResultStore.get(f.id)

    assert {:ok, _, _} =
             Guard.claim(f.id, runtime[Envelope.internal_key()], "restart", :runner_loss, 3)
  end

  test "retained terminal observations still pass admission without authorizing a receipt" do
    for state <- ~w(observed_executed observed_failed observed_blocked not_dispatched) do
      f = fixture()
      retain(f, state)
      {:ok, runtime} = AnalysisResultStore.get(f.id)

      assert {:ok, _, _} =
               Guard.claim(f.id, runtime[Envelope.internal_key()], "restart", :runner_loss, 3)

      assert :ok = access(f, {:lookup, f.request})
    end
  end

  test "commits are private, idempotent and have no inputs, outputs, endpoints or timestamps",
       ctx do
    f = fixture()
    attempt = retain(f, "not_dispatched")
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    {:ok, job} = Store.get(f.id)
    saved = %{f | runtime: runtime, job: job}
    bucket = runtime[Envelope.internal_key()][Evidence.key()]

    assert Map.keys(bucket["entries"][attempt]) |> Enum.sort() ==
             Enum.sort(@identity ++ ~w(attempt_id producer_claim))

    files = File.ls!(ctx.root)
    bytes = File.read!(Path.join(ctx.root, attempt <> ".json"))
    assert :ok = access(f, {:commit, f.request, attempt})
    assert :ok = access(f, {:lookup, f.request})
    Fixture.unchanged(saved)
    assert File.ls!(ctx.root) == files
    assert File.read!(Path.join(ctx.root, attempt <> ".json")) == bytes
    {:ok, public} = Analysis.fetch_job(f.id)
    assert public["result"]["recovery"]["native_dispatch_evidence_count"] == 1
    refute Map.has_key?(public["result"], Envelope.internal_key())
    refute Jason.encode!(public) =~ attempt

    for state <- ~w(completed failed cancelled) do
      ended = Envelope.transition(runtime[Envelope.internal_key()], state)
      refute Map.has_key?(ended, Evidence.key())
      assert Envelope.public_summary(ended)["native_dispatch_evidence_count"] == 0
    end
  end

  for field <- ~w(generation attempt owner_session_id) do
    test "stale #{field} cannot write or read evidence" do
      f = fixture()
      {:ok, attempt} = Journal.begin_dispatch(f.rpc, endpoint())

      claim =
        Map.update!(f.claim, unquote(field), fn x ->
          if is_integer(x), do: x + 1, else: x <> "-stale"
        end)

      for action <- [{:lookup, f.request}, {:commit, f.request, attempt}] do
        assert {:error, :stale_workflow_execution_claim} =
                 Coordinator.native_dispatch_evidence(f.id, claim, action)

        Fixture.unchanged(f)
      end
    end
  end

  for fault <- [:removed, :digest, :parent, :envelope, :extra, :entry_shape] do
    test "#{fault} evidence blocks before advancing the workflow generation" do
      f = fixture()
      attempt = retain(f, "observed_executed")
      {:ok, runtime} = AnalysisResultStore.get(f.id)
      key = Envelope.internal_key()
      original = runtime[key][Evidence.key()]

      changed =
        case unquote(fault) do
          :removed ->
            Map.delete(runtime[key], Evidence.key())

          :digest ->
            put_in(runtime[key], [Evidence.key(), "sha256"], String.duplicate("0", 64))

          :parent ->
            Map.put(runtime[key], Evidence.key(), seal(Map.put(original, "job_id", "foreign")))

          :envelope ->
            Map.put(
              runtime[key],
              Evidence.key(),
              seal(Map.put(original, "envelope_sha256", String.duplicate("0", 64)))
            )

          :extra ->
            Map.put(
              runtime[key],
              Evidence.key(),
              seal(Map.put(original, "secret", "not-supported"))
            )

          :entry_shape ->
            Map.put(
              runtime[key],
              Evidence.key(),
              seal(put_in(original, ["entries", attempt, "producer_claim"], []))
            )
        end

      assert :ok = AnalysisResultStore.put(f.id, Map.put(runtime, key, changed))

      assert {:error, :workflow_native_dispatch_evidence_invalid} =
               Guard.claim(f.id, changed, "restart", :runner_loss, 3)

      assert %{"recovered" => 0, "blocked" => 1} = Coordinator.recover_now()
      {:ok, blocked} = AnalysisResultStore.get(f.id)
      assert blocked[key]["generation"] == f.claim["generation"]
      assert blocked[key]["state"] == "recovery_blocked"
      assert Map.get(blocked[key], Evidence.key()) == Map.get(changed, Evidence.key())
    end
  end

  test "legacy pure workflows do not acquire a native journal dependency" do
    f = Fixture.claimed_job()
    recovery = Map.delete(f.runtime[Envelope.internal_key()], Evidence.key())
    assert :ok = stop_supervised(Journal)
    assert {:ok, _, _} = Guard.claim(f.id, recovery, "restart", :runner_loss, 3)
  end

  test "evidence identity must belong to this graph, journal attempt and original invocation" do
    f = fixture()
    attempt = retain(f, "not_dispatched")
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    {:ok, job} = Store.get(f.id)
    saved = %{f | runtime: runtime, job: job}

    for request <- [
          Map.put(f.request, "task_id", "foreign"),
          Map.put(f.request, "extra", true),
          Map.put(f.request, "task_digest", "bad")
        ] do
      assert {:error, :invalid} = access(f, {:commit, request, attempt})
      Fixture.unchanged(saved)
    end

    changed = Map.put(f.request, "task_digest", String.duplicate("b", 64))
    assert {:error, :input_changed} = access(f, {:lookup, changed})
    assert {:error, :conflict} = access(f, {:commit, changed, attempt})

    assert {:error, :workflow_dispatch_evidence_missing} =
             access(f, {:commit, f.request, String.duplicate("0", 32)})

    Fixture.unchanged(saved)
  end

  test "loss between recovery lookup and reservation cannot be papered over by a new commit",
       ctx do
    f = fixture()
    retain(f, "not_dispatched")
    assert :ok = access(f, {:lookup, f.request})
    {:ok, before} = AnalysisResultStore.get(f.id)
    assert :ok = stop_supervised(Journal)
    File.rm_rf!(ctx.root)
    start_supervised!({Journal, root: ctx.root})
    {:ok, attempt} = Journal.begin_dispatch(f.rpc, endpoint())

    assert {:error, :workflow_dispatch_evidence_missing} =
             access(f, {:commit, f.request, attempt})

    assert {:ok, ^before} = AnalysisResultStore.get(f.id)

    assert {:ok, [%{"state" => "dispatch_boundary_unconfirmed"}]} =
             Journal.lookup_task(f.request["task_id"])
  end

  test "a rejected transaction does not retain a reservation or permit a request" do
    f = fixture()
    {:ok, attempt} = Journal.begin_dispatch(f.rpc, endpoint())

    {:ok, future} =
      Store.apply_progress(%{
        job_id: f.id,
        stage: :solving,
        progress: 0.2,
        emitted_at: DateTime.add(DateTime.utc_now(), 60, :second)
      })

    assert {:error, _} = access(f, {:commit, f.request, attempt})
    Fixture.unchanged(%{f | job: future})
  end

  test "the actual native runtime routes a bound evidence callback to dispatch" do
    f = fixture()
    callback = fn action -> access(f, action) end

    assert {:error, {:native_workflow_recovery_blocked, :dispatch_evidence_commit_failed}} =
             WorkflowNativeOperatorRuntime.run(
               "extract",
               operator(),
               %{},
               %{},
               native_node(),
               %{"job_id" => f.id},
               client: __MODULE__.FailingEvidenceClient,
               native_dispatch_evidence: callback
             )

    assert_receive {:reserved_callback_result, {:error, :workflow_dispatch_evidence_missing}}
    Fixture.unchanged(f)
  end

  defmodule FailingEvidenceClient do
    def run_operator_task_ir(_task, opts, _progress) do
      callback = Keyword.fetch!(opts, :on_dispatch_reserved)
      send(self(), {:reserved_callback_result, callback.(String.duplicate("0", 32))})
      {:error, :operator_task_dispatch_evidence_unavailable}
    end
  end

  test "entry and byte bounds reject oversize evidence without evicting history" do
    f = fixture()
    attempt = retain(f, "not_dispatched")
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    recovery = runtime[Envelope.internal_key()]
    bucket = recovery[Evidence.key()]
    entry = bucket["entries"][attempt]

    entries =
      Map.new(1..512, fn n ->
        id = n |> Integer.to_string(16) |> String.downcase() |> String.pad_leading(32, "0")
        {id, Map.put(entry, "attempt_id", id)}
      end)

    full = Map.put(recovery, Evidence.key(), seal(Map.put(bucket, "entries", entries)))
    assert :ok = Evidence.verify(full, f.id)

    over =
      update_in(full, [Evidence.key(), "entries"], &Map.put(&1, attempt, entry))
      |> Map.update!(Evidence.key(), &seal/1)

    assert {:error, :invalid} = Evidence.verify(over, f.id)

    large =
      Map.new(entries, fn {id, value} ->
        {id, Map.put(value, "program_id", String.duplicate("x", 1024))}
      end)

    over_bytes = Map.put(recovery, Evidence.key(), seal(Map.put(bucket, "entries", large)))
    assert {:error, :invalid} = Evidence.verify(over_bytes, f.id)
    assert {:ok, ^runtime} = AnalysisResultStore.get(f.id)
  end

  test "malformed evidence queries fail locally without poisoning the journal" do
    f = fixture()

    for invalid <- [nil, false, [%{}], [nil], [f.request], List.duplicate(%{}, 513)] do
      assert {:error, :operator_task_dispatch_journal_invalid} =
               Journal.verify_evidence(f.id, invalid)

      assert {:error, :operator_task_dispatch_journal_invalid} =
               Journal.workflow_replay_fence(f.id, [], invalid, Journal)
    end

    assert :ok = Journal.workflow_replay_fence(f.id, [])
    assert {:ok, _} = Journal.begin_dispatch(f.rpc, endpoint())
  end

  defp fixture do
    f =
      Fixture.claimed_job(
        graph: %{
          "id" => "native-evidence",
          "nodes" => [native_node()],
          "recovery_policy" => %{"retry_safety" => "idempotent"}
        }
      )

    {:ok, task} =
      OperatorTaskIR.build(operator(), %{}, %{}, task_id: "#{f.id}:diagnose:#{operator()}")

    {:ok, summary} = OperatorTaskExecutionSummary.build(task)

    Map.merge(f, %{
      request: Map.take(summary, @identity),
      rpc: %{"id" => f.id, "params" => %{"task_ir" => task, "job_id" => f.id}}
    })
  end

  defp retain(f, state) do
    {:ok, attempt} = Journal.begin_dispatch(f.rpc, endpoint())
    assert :ok = access(f, {:commit, f.request, attempt})

    if state != "dispatch_boundary_unconfirmed",
      do: assert(:ok = Journal.finish_dispatch(attempt, state))

    attempt
  end

  defp access(f, action), do: Coordinator.native_dispatch_evidence(f.id, f.claim, action)
  defp endpoint, do: %{id: "owner", host: "127.0.0.1", port: 1}
  defp operator, do: "extract.thermal_result_diagnostics"
  defp native_node, do: %{"id" => "diagnose", "kind" => "extract", "operator_id" => operator()}

  defp seal(bucket),
    do:
      Map.put(
        bucket,
        "sha256",
        :crypto.hash(:sha256, DurableJson.encode!(Map.delete(bucket, "sha256")))
        |> Base.encode16(case: :lower)
      )
end
