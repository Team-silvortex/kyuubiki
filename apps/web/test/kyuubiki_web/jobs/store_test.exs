defmodule KyuubikiWeb.Jobs.StoreTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.AnalysisResultMemoryBackend
  alias KyuubikiWeb.Persistence
  alias KyuubikiWeb.Jobs.Store

  setup do
    data_dir =
      Path.join(System.tmp_dir!(), "kyuubiki-store-test-#{System.unique_integer([:positive])}")

    System.put_env("KYUUBIKI_DATA_DIR", data_dir)
    Store.reset()
    AnalysisResultStore.reset()

    on_exit(fn ->
      Persistence.clear!()
      System.delete_env("KYUUBIKI_DATA_DIR")
    end)

    :ok
  end

  test "creates a job and applies progress updates" do
    {:ok, job} =
      Store.create(%{
        job_id: "job-1",
        project_id: "project-1",
        simulation_case_id: "case-1"
      })

    assert job.status == :queued

    {:ok, updated} =
      Store.apply_progress(%{
        job_id: "job-1",
        stage: "solving",
        progress: 0.5,
        iteration: 12,
        residual: 1.0e-4
      })

    assert updated.status == :solving
    assert updated.progress == 0.5
    assert updated.iteration == 12
    assert updated.residual == 1.0e-4
  end

  test "reloads persisted jobs and results after store restart" do
    {:ok, _job} =
      Store.create(%{
        job_id: "job-2",
        project_id: "project-2",
        simulation_case_id: "case-2"
      })

    {:ok, _updated} =
      Store.apply_progress(%{
        job_id: "job-2",
        stage: "completed",
        progress: 1.0,
        iteration: 4,
        residual: 2.0e-1
      })

    :ok =
      AnalysisResultStore.put("job-2", %{
        "kind" => "axial_bar_1d",
        "max_displacement" => 1.23e-4
      })

    restart_application()

    assert {:ok, reloaded_job} = Store.get("job-2")
    assert reloaded_job.status == :completed
    assert reloaded_job.progress == 1.0
    assert reloaded_job.iteration == 4
    assert reloaded_job.residual == 2.0e-1

    assert {:ok, result} = AnalysisResultStore.get("job-2")
    assert result["kind"] == "axial_bar_1d"
    assert result["max_displacement"] == 1.23e-4
  end

  test "result store reports missing job constraints without crashing" do
    assert {:error, _reason} =
             AnalysisResultStore.put("missing-job", %{"status" => "orphaned_result"})
  end

  test "result store compare-and-swap permits only one concurrent writer" do
    job_id = "job-cas"

    assert {:ok, _job} =
             Store.create(%{
               job_id: job_id,
               project_id: "project-cas",
               simulation_case_id: "case-cas"
             })

    initial = %{"generation" => 1, "owner" => "session-a"}
    assert :ok = AnalysisResultStore.put(job_id, initial)

    results =
      1..8
      |> Enum.map(fn candidate ->
        Task.async(fn ->
          replacement = %{"generation" => 2, "owner" => "session-#{candidate}"}
          {candidate, AnalysisResultStore.compare_and_swap(job_id, initial, replacement)}
        end)
      end)
      |> Task.await_many(5_000)

    winners = for {candidate, :ok} <- results, do: candidate
    stale = for {_candidate, {:error, :stale_analysis_result}} <- results, do: :stale

    assert [_winner] = winners
    assert length(stale) == 7
    assert {:ok, %{"generation" => 2, "owner" => owner}} = AnalysisResultStore.get(job_id)
    assert owner == "session-#{hd(winners)}"
  end

  test "memory result store compare-and-swap is atomic and fail-closed" do
    start_supervised!({AnalysisResultMemoryBackend, []})

    initial = %{"generation" => 3}
    replacement = %{"generation" => 4}

    assert :ok = AnalysisResultMemoryBackend.put("memory-cas", initial)
    assert :ok = AnalysisResultMemoryBackend.compare_and_swap("memory-cas", initial, replacement)

    assert {:error, :stale_analysis_result} =
             AnalysisResultMemoryBackend.compare_and_swap("memory-cas", initial, %{
               "generation" => 5
             })

    assert {:error, {:result_not_found, "missing-memory-cas"}} =
             AnalysisResultMemoryBackend.compare_and_swap(
               "missing-memory-cas",
               initial,
               replacement
             )
  end

  defp restart_application do
    :ok = Application.stop(:kyuubiki_web)
    {:ok, _started} = Application.ensure_all_started(:kyuubiki_web)
  end
end
