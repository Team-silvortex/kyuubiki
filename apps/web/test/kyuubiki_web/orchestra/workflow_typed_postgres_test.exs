defmodule KyuubikiWeb.Orchestra.WorkflowTypedPostgresTest do
  use ExUnit.Case, async: false

  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.{AnalysisResultStore, PostgresRepo, Storage, TaskDigestV2}
  alias KyuubikiWeb.Jobs.Store

  alias KyuubikiWeb.Orchestra.{
    LeaseStore,
    WorkflowBranchCheckpoint,
    WorkflowCheckpointStore,
    WorkflowRecoveryEnvelope,
    WorkflowSolveCheckpoint
  }

  alias KyuubikiWeb.Storage.{DatabaseMigrations, DurableJson, MigrationQuery}
  alias KyuubikiWeb.TestSupport.WorkflowBranchCheckpointFixture, as: Branch
  alias KyuubikiWeb.TestSupport.WorkflowSolveCheckpointFixture, as: Solve

  @moduletag skip:
               not Storage.postgres?() or
                 System.get_env("KYUUBIKI_TEST_DATABASE_URL") in [nil, ""]

  setup do
    schema = "typed_recovery_" <> Base.encode16(:crypto.strong_rand_bytes(12), case: :lower)
    url = System.fetch_env!("KYUUBIKI_TEST_DATABASE_URL")
    {:ok, pid} = start_repo(url, schema)
    previous = PostgresRepo.put_dynamic_repo(pid)
    Process.unlink(pid)

    on_exit(fn ->
      PostgresRepo.put_dynamic_repo(pid)
      MigrationQuery.rows!(PostgresRepo, "DROP SCHEMA IF EXISTS #{schema} CASCADE")
      Supervisor.stop(pid)
      PostgresRepo.put_dynamic_repo(previous)
    end)

    MigrationQuery.rows!(PostgresRepo, "CREATE SCHEMA #{schema}")
    DatabaseMigrations.boot!(PostgresRepo, :postgres)
    {:ok, lease} = LeaseStore.acquire("typed-recovery", "first-owner", 60_000)
    %{schema: schema, url: url, lease: lease, fixture: fixture()}
  end

  test "lease reads and writes use the currently selected repository schema", context do
    assert [[context.schema]] == MigrationQuery.rows!(PostgresRepo, "SELECT current_schema()")

    assert [["typed-recovery", "first-owner", 1]] ==
             MigrationQuery.rows!(PostgresRepo, """
             SELECT lease_name, owner_instance_id, fencing_token FROM kyuubiki_orchestra_leases
             """)

    assert {:ok, lease} = LeaseStore.current("typed-recovery")
    assert lease == context.lease
    assert {:ok, renewed} = LeaseStore.renew(lease, 60_000)
    assert renewed.fencing_token == 1
    assert :protected == LeaseStore.with_lease(renewed, fn -> :protected end)
    assert :ok = LeaseStore.release(renewed)
    assert {:ok, next} = LeaseStore.acquire("typed-recovery", "next-owner", 60_000)
    assert next.fencing_token == 2
  end

  test "JSONB numeric equality cannot conflate typed execution strings", %{fixture: fixture} do
    for {left, right} <- [{1, 1.0}, {-0.0, 0.0}] do
      assert [[true]] =
               MigrationQuery.rows!(PostgresRepo, "SELECT $1::text::jsonb = $2::text::jsonb", [
                 DurableJson.encode!(left),
                 DurableJson.encode!(right)
               ])

      a = execution(fixture.id, left)
      b = execution(fixture.id, right)
      assert a["envelope_sha256"] != b["envelope_sha256"]

      assert [[false]] =
               MigrationQuery.rows!(PostgresRepo, "SELECT $1::text::jsonb = $2::text::jsonb", [
                 DurableJson.encode!(a["envelope"]),
                 DurableJson.encode!(b["envelope"])
               ])
    end
  end

  test "PostgreSQL reads preserve canonical execution and bitwise numeric identity", %{
    fixture: fixture
  } do
    saved = snapshot(fixture.id)
    key = WorkflowRecoveryEnvelope.internal_key()
    assert saved.runtime[key]["envelope"] === fixture.runtime[key]["envelope"]
    assert :ok = WorkflowRecoveryEnvelope.verify(saved.runtime[key])
    {:ok, decoded} = WorkflowRecoveryEnvelope.execution(saved.runtime[key])
    assert_numbers(decoded["input_artifacts"]["input"])
    assert_numbers(hd(decoded["graph"]["nodes"])["config"]["witness"])
  end

  test "typed compare-and-swap rejects numeric identity changes without writing", %{
    fixture: fixture
  } do
    key = WorkflowRecoveryEnvelope.internal_key()

    for {left, right} <- [{1, 1.0}, {-0.0, 0.0}, {1.0e-30, 2.0e-30}, {1.0, 1.0000000000000002}] do
      runtime = Map.put(fixture.runtime, key, execution(fixture.id, left))
      assert :ok = AnalysisResultStore.put(fixture.id, runtime)
      before = snapshot(fixture.id)
      wrong = put_in(before.runtime, [key, "envelope"], execution(fixture.id, right)["envelope"])

      assert {:error, :stale_analysis_result} =
               AnalysisResultStore.compare_and_swap(fixture.id, wrong, %{"forged" => true})

      assert snapshot(fixture.id) === before

      replacement = Map.put(before.runtime, "checked", true)
      assert :ok = AnalysisResultStore.compare_and_swap(fixture.id, before.runtime, replacement)
      assert {:ok, ^replacement} = AnalysisResultStore.get(fixture.id)
    end
  end

  test "a fresh repository pool restores both checkpoints under a new claim", context do
    fixture = context.fixture
    assert :ok = access(context, WorkflowSolveCheckpoint, {:commit, fixture.solve, numbers()})

    assert :ok =
             access(
               context,
               WorkflowBranchCheckpoint,
               {:commit, fixture.branch, Branch.decision()}
             )

    before = snapshot(fixture.id)

    {:ok, reader} = start_repo(context.url, context.schema)
    previous = PostgresRepo.put_dynamic_repo(reader)

    try do
      assert snapshot(fixture.id) === before
      resumed = reclaim(fixture.id)
      context = put_in(context, [:fixture, :claim], resumed.claim)
      assert {:ok, output} = access(context, WorkflowSolveCheckpoint, {:lookup, fixture.solve})
      assert_numbers(output)

      assert {:ok, decision} =
               access(context, WorkflowBranchCheckpoint, {:lookup, fixture.branch})

      assert decision == Branch.decision()
      assert snapshot(fixture.id) === Map.delete(resumed, :claim)
    after
      PostgresRepo.put_dynamic_repo(previous)
      Supervisor.stop(reader)
    end
  end

  for checkpoint <- [WorkflowSolveCheckpoint, WorkflowBranchCheckpoint],
      table <- ["kyuubiki_jobs", "kyuubiki_analysis_results"] do
    test "#{inspect(checkpoint)} rolls back a rejected #{table} write and permits retry",
         context do
      checkpoint = unquote(checkpoint)
      fixture = context.fixture
      {request, output} = checkpoint_action(fixture, checkpoint)
      before = snapshot(fixture.id)
      drop = reject_update(unquote(table), fixture.id)

      assert {:error, _} = access(context, checkpoint, {:commit, request, output})
      assert snapshot(fixture.id) === before
      drop.()
      assert :ok = access(context, checkpoint, {:commit, request, output})
      saved = snapshot(fixture.id)
      assert saved.job.status == before.job.status
      assert saved.job.progress == before.job.progress
      assert saved.runtime["artifacts"] == %{}
      assert saved.runtime["current_node"] == nil
    end
  end

  test "old claims and displaced leases cannot read or commit checkpoints", context do
    fixture = context.fixture
    before = snapshot(fixture.id)

    for field <- ~w(generation attempt owner_session_id),
        checkpoint <- [WorkflowSolveCheckpoint, WorkflowBranchCheckpoint] do
      claim =
        Map.update!(fixture.claim, field, &if(is_integer(&1), do: &1 + 1, else: &1 <> "-old"))

      stale = put_in(context, [:fixture, :claim], claim)
      {request, output} = checkpoint_action(fixture, checkpoint)

      for action <- [{:lookup, request}, {:commit, request, output}] do
        assert {:error, :stale_workflow_execution_claim} = access(stale, checkpoint, action)
        assert snapshot(fixture.id) === before
      end
    end

    assert :ok = LeaseStore.release(context.lease)
    assert {:ok, _} = LeaseStore.acquire("typed-recovery", "replacement-owner", 60_000)

    for checkpoint <- [WorkflowSolveCheckpoint, WorkflowBranchCheckpoint] do
      assert {:error, :orchestra_lease_lost} = access(context, checkpoint, {:lookup, %{}})
      assert snapshot(fixture.id) === before
    end
  end

  test "terminal joint publication prunes private execution and both checkpoint payloads",
       context do
    fixture = context.fixture
    assert :ok = access(context, WorkflowSolveCheckpoint, {:commit, fixture.solve, numbers()})

    assert :ok =
             access(
               context,
               WorkflowBranchCheckpoint,
               {:commit, fixture.branch, Branch.decision()}
             )

    before = snapshot(fixture.id)
    key = WorkflowRecoveryEnvelope.internal_key()
    recovery = WorkflowRecoveryEnvelope.transition(before.runtime[key], "completed")
    terminal = Map.put(before.runtime, key, recovery)

    assert {:ok, _} =
             LeaseStore.with_lease(context.lease, fn ->
               Store.apply_progress_with_result(
                 %{job_id: fixture.id, stage: :completed, progress: 1.0},
                 before.job,
                 before.runtime,
                 terminal
               )
             end)

    saved = snapshot(fixture.id)
    assert saved.job.status == :completed
    refute Map.has_key?(saved.runtime[key], "envelope")
    refute Map.has_key?(saved.runtime[key], WorkflowSolveCheckpoint.key())
    refute Map.has_key?(saved.runtime[key], WorkflowBranchCheckpoint.key())

    assert WorkflowRecoveryEnvelope.public_summary(saved.runtime[key])[
             "native_task_digest_encoding"
           ] ==
             TaskDigestV2.encoding()
  end

  defp start_repo(url, schema),
    do:
      PostgresRepo.start_link(
        name: nil,
        url: url,
        pool_size: 2,
        parameters: [search_path: schema]
      )

  defp numbers, do: [1, 1.0, -0.0, 0.0, 1.0e-30, 1.0000000000000002]

  defp checkpoint_action(fixture, WorkflowSolveCheckpoint), do: {fixture.solve, numbers()}

  defp checkpoint_action(fixture, WorkflowBranchCheckpoint),
    do: {fixture.branch, Branch.decision()}

  defp assert_numbers(values) do
    assert values === numbers()

    assert Enum.map(Enum.filter(values, &is_float/1), &<<&1::float-64>>) ==
             Enum.map(Enum.filter(numbers(), &is_float/1), &<<&1::float-64>>)
  end

  defp execution(id, input) do
    solve = put_in(Solve.solve_node(), ["config", "witness"], numbers())

    graph = %{
      "id" => id,
      "defaults" => %{"native_task_digest_encoding" => TaskDigestV2.encoding()},
      "nodes" => [solve, Branch.condition()]
    }

    {:ok, pending} =
      WorkflowRecoveryEnvelope.new(graph, %{"input" => input}, %{"job_id" => id}, %{})

    {:ok, recovery, _} = WorkflowRecoveryEnvelope.claim(pending, "session-a", :initial)
    recovery
  end

  defp fixture do
    id = "typed-#{System.unique_integer([:positive])}"
    recovery = execution(id, numbers())
    {:ok, decoded} = WorkflowRecoveryEnvelope.execution(recovery)

    runtime = %{
      "artifacts" => %{},
      "current_node" => nil,
      "progress_events" => [],
      WorkflowRecoveryEnvelope.internal_key() => recovery
    }

    {:ok, _} =
      Store.create(%{
        job_id: id,
        project_id: "typed",
        simulation_case_id: "case",
        status: :solving,
        progress: 0.1,
        execution_started_at: DateTime.utc_now()
      })

    :ok = AnalysisResultStore.put(id, runtime)
    [solve, branch] = decoded["graph"]["nodes"]
    context = decoded["orchestration_context"]

    %{
      id: id,
      runtime: runtime,
      claim: Map.take(recovery, ~w(generation attempt owner_session_id)),
      solve: WorkflowSolveCheckpoint.request(solve, numbers(), context),
      branch: WorkflowBranchCheckpoint.request(branch, %{"value" => 1.0}, context)
    }
  end

  defp snapshot(id) do
    {:ok, job} = Store.get(id)
    {:ok, runtime} = AnalysisResultStore.get(id)
    %{job: job, runtime: runtime}
  end

  defp reclaim(id) do
    before = snapshot(id)
    key = WorkflowRecoveryEnvelope.internal_key()

    {:ok, next, claim} =
      WorkflowRecoveryEnvelope.claim(before.runtime[key], "session-b", :process_restart)

    runtime = Map.put(before.runtime, key, next)
    :ok = AnalysisResultStore.compare_and_swap(id, before.runtime, runtime)
    Map.put(%{job: before.job, runtime: runtime}, :claim, claim)
  end

  defp access(context, checkpoint, action),
    do:
      WorkflowCheckpointStore.access(
        context.fixture.id,
        context.fixture.claim,
        action,
        context.lease,
        checkpoint
      )

  defp reject_update(table, id) do
    name = "reject_typed_#{System.unique_integer([:positive])}"

    SQL.query!(PostgresRepo.get_dynamic_repo(), """
    CREATE FUNCTION #{name}() RETURNS trigger LANGUAGE plpgsql AS $$
    BEGIN RAISE EXCEPTION 'injected typed checkpoint write failure'; END $$
    """)

    SQL.query!(PostgresRepo.get_dynamic_repo(), """
    CREATE TRIGGER #{name} BEFORE UPDATE ON #{table}
    FOR EACH ROW WHEN (NEW.job_id = '#{id}') EXECUTE FUNCTION #{name}()
    """)

    fn -> SQL.query!(PostgresRepo.get_dynamic_repo(), "DROP TRIGGER #{name} ON #{table}") end
  end
end
