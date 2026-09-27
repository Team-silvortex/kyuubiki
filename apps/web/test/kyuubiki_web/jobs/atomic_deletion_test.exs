defmodule KyuubikiWeb.Jobs.AtomicDeletionTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.TestSupport.StorageOutageFixture

  setup do
    StorageOutageFixture.setup()
  end

  test "joint deletion returns the removed snapshot without inventing a cancellation receipt" do
    job = job()
    assert :ok = AnalysisResultStore.put(job.job_id, %{"retained" => 42})
    assert {:ok, ^job} = Store.delete_with_result(job.job_id)
    assert :error = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)
  end

  test "missing jobs return not found without changing existing work" do
    job = job()

    assert {:error, {:job_not_found, "missing-deletion"}} =
             Store.delete_with_result("missing-deletion")

    assert {:ok, ^job} = Store.get(job.job_id)
  end

  test "invalid deletion identifiers are rejected without modifying either collection" do
    job = job()
    assert :ok = AnalysisResultStore.put(job.job_id, %{"retained" => true})

    for id <- [nil, "", 0, %{}, []] do
      assert {:error, :invalid_analysis_deletion} = Store.delete_with_result(id)
    end

    assert {:ok, ^job} = Store.get(job.job_id)
    assert {:ok, %{"retained" => true}} = AnalysisResultStore.get(job.job_id)
  end

  @tag skip: not Storage.memory?()
  test "a missing job cannot authorize deletion of an orphan result" do
    assert :ok = AnalysisResultStore.put("orphan-deletion", %{"retained" => true})

    assert {:error, {:job_not_found, "orphan-deletion"}} =
             Store.delete_with_result("orphan-deletion")

    assert {:ok, %{"retained" => true}} = AnalysisResultStore.get("orphan-deletion")
  end

  test "concurrent deletions produce exactly one successful removal" do
    job = job()
    assert :ok = AnalysisResultStore.put(job.job_id, %{"retained" => true})

    replies = race(List.duplicate(fn -> Store.delete_with_result(job.job_id) end, 8))
    assert Enum.count(replies, &match?({:ok, ^job}, &1)) == 1
    assert Enum.count(replies, &match?({:error, _}, &1)) == 7
    assert :error = Store.get(job.job_id)
    assert :error = AnalysisResultStore.get(job.job_id)
  end

  for operation <- [:initialization, :completion] do
    test "racing deletion against #{operation} cannot leave a result without a job" do
      for _ <- 1..12 do
        job = job()

        publish = fn ->
          case unquote(operation) do
            :initialization -> Store.initialize_result(job.job_id, %{"pending" => true})
            :completion -> Store.complete_with_result(job.job_id, "worker", %{"finished" => true})
          end
        end

        id = job.job_id
        [deleted, published] = race([fn -> Store.delete_with_result(id) end, publish])

        assert match?({:ok, %{job_id: ^id}}, published) or
                 published == {:error, {:job_not_found, id}}

        case deleted do
          {:ok, _job} ->
            :ok

          {:error, {:stale_job_snapshot, ^id}} ->
            assert {:ok, _job} = Store.delete_with_result(id)
        end

        assert :error = Store.get(job.job_id)
        assert :error = AnalysisResultStore.get(job.job_id)
      end
    end
  end

  defp race(callbacks) do
    owner = self()

    tasks =
      Enum.map(callbacks, fn callback ->
        Task.async(fn ->
          send(owner, {:ready, self()})
          receive do: (:go -> callback.())
        end)
      end)

    for _ <- tasks, do: assert_receive({:ready, _pid}, 2_000)
    for task <- tasks, do: send(task.pid, :go)
    Task.await_many(tasks, 10_000)
  end

  defp job do
    assert {:ok, job} =
             Store.create(%{
               job_id: "atomic-deletion-#{System.unique_integer([:positive])}",
               project_id: "deletion",
               simulation_case_id: "case"
             })

    job
  end
end
