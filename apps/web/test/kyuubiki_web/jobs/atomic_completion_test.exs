defmodule KyuubikiWeb.Jobs.AtomicCompletionTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store

  setup do
    Store.reset()
    AnalysisResultStore.reset()
    :ok
  end

  test "completion publishes the worker, result and terminal state together" do
    job = create_job("complete")
    result = %{"displacement" => [0.0, 0.25]}

    assert {:ok, completed} = Store.complete_with_result(job.job_id, "worker-final", result)
    assert completed.status == :completed
    assert completed.progress == 1.0
    assert completed.worker_id == "worker-final"
    assert completed.iteration == job.iteration
    assert completed.residual == job.residual
    assert completed.execution_started_at == job.execution_started_at
    assert completed.project_id == job.project_id
    assert {:ok, ^completed} = Store.get(job.job_id)
    assert {:ok, ^result} = AnalysisResultStore.get(job.job_id)
  end

  for stage <- [:completed, :failed, :cancelled] do
    test "a #{stage} job cannot receive a late result or worker assignment" do
      job = create_job("terminal")

      assert {:ok, terminal} =
               Store.apply_progress(%{job_id: job.job_id, stage: unquote(stage), progress: 1.0})

      assert {:error, {:job_already_terminal, unquote(stage)}} =
               Store.complete_with_result(job.job_id, "late-worker", %{"late" => true})

      assert {:ok, ^terminal} = Store.get(job.job_id)
      assert :error = AnalysisResultStore.get(job.job_id)
    end
  end

  test "a duplicate completion cannot overwrite the first result" do
    job = create_job("duplicate")
    first = %{"winner" => 1}
    assert {:ok, completed} = Store.complete_with_result(job.job_id, "worker-1", first)

    assert {:error, {:job_already_terminal, :completed}} =
             Store.complete_with_result(job.job_id, "worker-2", %{"winner" => 2})

    assert {:ok, ^completed} = Store.get(job.job_id)
    assert {:ok, ^first} = AnalysisResultStore.get(job.job_id)
  end

  test "existing result conflicts roll back the attempted job and worker update" do
    job = create_job("conflict")
    existing = %{"provenance" => "already-stored"}
    assert :ok = AnalysisResultStore.put(job.job_id, existing)
    id = job.job_id

    assert {:error, {:result_already_exists, ^id}} =
             Store.complete_with_result(id, "replacement", %{"different" => true})

    assert {:ok, ^job} = Store.get(id)
    assert {:ok, ^existing} = AnalysisResultStore.get(id)
  end

  test "missing jobs and malformed completion arguments never publish a result" do
    assert {:error, {:job_not_found, "missing"}} =
             Store.complete_with_result("missing", "worker", %{})

    assert :error = AnalysisResultStore.get("missing")
    job = create_job("invalid")

    for {worker, result} <- [{"", %{}}, {nil, %{}}, {"worker", nil}, {"worker", []}] do
      assert {:error, :invalid_solver_completion} =
               Store.complete_with_result(job.job_id, worker, result)
    end

    assert {:ok, ^job} = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)
  end

  test "an unencodable result rolls back and does not poison the following completion" do
    job = create_job("encoding")

    assert {:error, {:completion_persistence_failed, _reason}} =
             Store.complete_with_result(job.job_id, "bad-worker", %{"invalid" => fn -> :bad end})

    assert {:ok, ^job} = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)

    assert {:ok, %{status: :completed}} =
             Store.complete_with_result(job.job_id, "good-worker", %{"valid" => true})
  end

  test "a future progress timestamp does not break final completion ordering" do
    job = create_job("clock")
    future = DateTime.add(DateTime.utc_now(), 60, :second)

    assert {:ok, _} =
             Store.apply_progress(%{
               job_id: job.job_id,
               stage: :postprocessing,
               progress: 0.99,
               emitted_at: future
             })

    assert {:ok, completed} = Store.complete_with_result(job.job_id, "worker", %{})
    assert DateTime.compare(completed.updated_at, future) != :lt
    assert {:ok, %{}} = AnalysisResultStore.get(job.job_id)
  end

  test "concurrent publishers elect exactly one matching worker and result" do
    job = create_job("publishers")

    outcomes =
      1..8
      |> Enum.map(fn index ->
        Task.async(fn ->
          worker = "worker-#{index}"
          {worker, Store.complete_with_result(job.job_id, worker, %{"worker" => worker})}
        end)
      end)
      |> Task.await_many(10_000)

    assert [{winner, {:ok, completed}}] = Enum.filter(outcomes, &match?({_, {:ok, _}}, &1))
    assert completed.worker_id == winner

    assert Enum.count(outcomes, &match?({_, {:error, {:job_already_terminal, :completed}}}, &1)) ==
             7

    assert {:ok, %{"worker" => ^winner}} = AnalysisResultStore.get(job.job_id)
  end

  for stage <- [:cancelled, :failed] do
    test "racing #{stage} against completion never leaves a mixed terminal snapshot" do
      for index <- 1..12 do
        job = create_job("race-#{index}")
        owner = self()

        tasks =
          for operation <- [:publish, :stop] do
            Task.async(fn ->
              send(owner, {:ready, self()})

              receive do
                :go ->
                  case operation do
                    :publish ->
                      Store.complete_with_result(job.job_id, "winner", %{"ok" => true})

                    :stop ->
                      Store.apply_progress(%{
                        job_id: job.job_id,
                        stage: unquote(stage),
                        progress: 1.0
                      })
                  end
              end
            end)
          end

        for _ <- tasks, do: assert_receive({:ready, _pid})
        for task <- tasks, do: send(task.pid, :go)
        outcomes = Task.await_many(tasks, 10_000)
        assert Enum.count(outcomes, &match?({:ok, _}, &1)) == 1
        assert {:ok, final} = Store.get(job.job_id)

        case final.status do
          :completed ->
            assert final.worker_id == "winner"
            assert {:ok, %{"ok" => true}} = AnalysisResultStore.get(job.job_id)

          unquote(stage) ->
            assert final.worker_id == job.worker_id
            assert :error = AnalysisResultStore.get(job.job_id)
        end
      end
    end
  end

  defp create_job(prefix) do
    assert {:ok, job} =
             Store.create(%{
               job_id: "#{prefix}-#{System.unique_integer([:positive])}",
               project_id: "atomic-completion",
               simulation_case_id: "case",
               status: :solving,
               progress: 0.9,
               worker_id: "original-worker",
               execution_started_at: DateTime.utc_now(),
               iteration: 7,
               residual: 0.001
             })

    job
  end
end
