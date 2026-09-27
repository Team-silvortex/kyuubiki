defmodule KyuubikiWeb.Jobs.ProgressConcurrencyTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Jobs.{Store, Watchdog}
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.JobRecord
  alias KyuubikiWeb.TestSupport.JobQueryRace

  import Ecto.Query

  @moduletag skip: not Storage.sql?()

  setup do
    Store.reset()
    original = Application.get_env(:kyuubiki_web, Watchdog, [])

    Application.put_env(:kyuubiki_web, Watchdog,
      scan_interval_ms: 60_000,
      stale_job_ms: 5_000,
      job_timeout_ms: 60_000
    )

    Watchdog.scan_now()

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, Watchdog, original)
      Store.reset()
    end)

    :ok
  end

  for terminal <- [:completed, :failed, :cancelled] do
    test "a late progress writer cannot overwrite concurrent #{terminal}" do
      terminal = unquote(terminal)
      job = create_job("terminal-race")
      timestamp = DateTime.add(job.updated_at, 1, :second)

      race =
        JobQueryRace.after_read({:job, job.job_id}, fn ->
          Store.apply_progress(%{
            job_id: job.job_id,
            stage: terminal,
            progress: 1.0,
            message: "final receipt",
            emitted_at: timestamp
          })
        end)

      assert {:error, {:terminal_job_mutation, ^terminal, :solving}} =
               Store.apply_progress(%{
                 job_id: job.job_id,
                 stage: :solving,
                 progress: 0.5,
                 emitted_at: timestamp
               })

      assert_receive {:interleaved, ^race, {:ok, final}}
      assert {:ok, ^final} = Store.get(job.job_id)
    end
  end

  test "equal timestamps still detect concurrent progress regression" do
    job = create_job("equal-timestamp")
    timestamp = job.updated_at

    race =
      JobQueryRace.after_read({:job, job.job_id}, fn ->
        Store.apply_progress(%{
          job_id: job.job_id,
          stage: :solving,
          progress: 0.8,
          emitted_at: timestamp
        })
      end)

    assert {:error, {:progress_regression, 0.8, 0.5}} =
             Store.apply_progress(%{
               job_id: job.job_id,
               stage: :solving,
               progress: 0.5,
               emitted_at: timestamp
             })

    assert_receive {:interleaved, ^race, {:ok, final}}
    assert {:ok, ^final} = Store.get(job.job_id)
  end

  test "a valid later event retries against current metadata and worker ownership" do
    job = create_job("merge-race")
    timestamp = DateTime.add(DateTime.utc_now(), 1, :second)

    race =
      JobQueryRace.after_read({:job, job.job_id}, fn ->
        {:ok, _} = Store.assign_worker(job.job_id, "worker-new")
        Store.update_metadata(job.job_id, %{"project_id" => "project-new", "message" => "kept"})
      end)

    assert {:ok, updated} =
             Store.apply_progress(%{
               job_id: job.job_id,
               stage: :solving,
               progress: 0.9,
               emitted_at: timestamp
             })

    assert_receive {:interleaved, ^race, {:ok, _}}
    assert updated.project_id == "project-new"
    assert updated.worker_id == "worker-new"
    assert updated.message == "kept"
    assert updated.progress == 0.9
    assert {:ok, ^updated} = Store.get(job.job_id)
  end

  test "worker assignment cannot revert a concurrent completion receipt" do
    job = create_job("worker-race")

    race =
      JobQueryRace.after_read({:job, job.job_id}, fn ->
        Store.apply_progress(%{
          job_id: job.job_id,
          stage: :completed,
          progress: 1.0,
          residual: 0.01,
          iteration: 12,
          message: "completed during assignment"
        })
      end)

    assert {:ok, updated} = Store.assign_worker(job.job_id, "worker-final")
    assert_receive {:interleaved, ^race, {:ok, final}}
    assert updated.status == :completed
    assert updated.progress == 1.0
    assert updated.residual == final.residual
    assert updated.iteration == final.iteration
    assert updated.message == final.message
    assert updated.worker_id == "worker-final"
    assert {:ok, ^updated} = Store.get(job.job_id)
  end

  test "terminal replay does not overwrite concurrently edited metadata" do
    job = create_job("replay-race")
    event = %{job_id: job.job_id, stage: :completed, progress: 1.0}
    assert {:ok, _} = Store.apply_progress(event)

    race =
      JobQueryRace.after_read({:job, job.job_id}, fn ->
        Store.update_metadata(job.job_id, %{"message" => "reviewed"})
      end)

    assert {:ok, %{status: :completed}} = Store.apply_progress(event)
    assert_receive {:interleaved, ^race, {:ok, final}}
    assert {:ok, ^final} = Store.get(job.job_id)
  end

  test "watchdog does not fail a task whose same-progress heartbeat arrived after its scan" do
    job = create_job("heartbeat-race", -10)

    race =
      JobQueryRace.after_read(:list, fn ->
        Store.apply_progress(%{
          job_id: job.job_id,
          stage: :solving,
          progress: job.progress,
          message: "still computing"
        })
      end)

    assert %{stalled: 0, timed_out: 0} = Watchdog.scan_now()
    assert_receive {:interleaved, ^race, {:ok, final}}
    assert {:ok, ^final} = Store.get(job.job_id)
    assert %{stalled: 0, timed_out: 0} = Watchdog.scan_now()
  end

  test "progress retries preserve the original implicit event time" do
    job = create_job("original-event-time")

    race =
      JobQueryRace.after_read({:job, job.job_id}, fn ->
        Store.apply_progress(%{
          job_id: job.job_id,
          stage: :solving,
          progress: job.progress,
          message: "newer heartbeat"
        })
      end)

    assert {:error, {:stale_progress_event, current, emitted}} =
             Store.apply_progress(%{job_id: job.job_id, stage: :solving, progress: 0.5})

    assert DateTime.compare(emitted, current) == :lt
    assert_receive {:interleaved, ^race, {:ok, final}}
    assert {:ok, ^final} = Store.get(job.job_id)
  end

  for terminal <- [:completed, :failed, :cancelled] do
    test "watchdog does not count a concurrent #{terminal} receipt as its own failure" do
      terminal = unquote(terminal)
      job = create_job("watchdog-terminal", -10)

      race =
        JobQueryRace.after_read(:list, fn ->
          Store.apply_progress(%{
            job_id: job.job_id,
            stage: terminal,
            progress: terminal_progress(terminal, job.progress),
            message: "worker terminal receipt"
          })
        end)

      assert %{stalled: 0, timed_out: 0} = Watchdog.scan_now()
      assert_receive {:interleaved, ^race, {:ok, final}}
      assert {:ok, ^final} = Store.get(job.job_id)
    end
  end

  test "an expired queue snapshot cannot fail work that has just started execution" do
    timestamp = DateTime.add(DateTime.utc_now(), -120, :second)

    {:ok, job} =
      Store.create(%{
        job_id: "queue-race",
        project_id: "concurrency-test",
        simulation_case_id: "case",
        status: :queued,
        queue_timeout_ms: 1_000,
        execution_timeout_ms: 60_000,
        created_at: timestamp,
        updated_at: timestamp
      })

    race =
      JobQueryRace.after_read(:list, fn ->
        Store.apply_progress(%{job_id: job.job_id, stage: :solving, progress: 0.0})
      end)

    assert %{stalled: 0, timed_out: 0} = Watchdog.scan_now()
    assert_receive {:interleaved, ^race, {:ok, running}}
    assert {:ok, ^running} = Store.get(job.job_id)
    assert running.execution_started_at != nil
    assert %{stalled: 0, timed_out: 0} = Watchdog.scan_now()
  end

  test "equal-time stage changes cannot be reverted by another writer" do
    job = create_job("stage-race")
    event = %{job_id: job.job_id, progress: 0.5, emitted_at: job.updated_at}

    race =
      JobQueryRace.after_read({:job, job.job_id}, fn ->
        Store.apply_progress(Map.put(event, :stage, :postprocessing))
      end)

    assert {:error, {:stage_regression, :postprocessing, :solving}} =
             Store.apply_progress(Map.put(event, :stage, :solving))

    assert_receive {:interleaved, ^race, {:ok, final}}
    assert {:ok, ^final} = Store.get(job.job_id)
  end

  test "metadata receipt includes a completion that won the write race" do
    job = create_job("metadata-race")

    race =
      JobQueryRace.after_read({:job, job.job_id}, fn ->
        Store.apply_progress(%{job_id: job.job_id, stage: :completed, progress: 1.0})
      end)

    assert {:ok, %{status: :completed, progress: 1.0} = updated} =
             Store.update_metadata(job.job_id, %{"model_version_id" => "version-reviewed"})

    assert_receive {:interleaved, ^race, {:ok, _}}
    assert updated.model_version_id == "version-reviewed"
    assert {:ok, ^updated} = Store.get(job.job_id)
  end

  for operation <- [:metadata, :worker] do
    test "#{operation} writes report deletion rather than raising or recreating a job" do
      job = create_job("deleted-write")
      job_id = job.job_id
      race = JobQueryRace.after_read({:job, job_id}, fn -> Store.delete(job_id) end)

      result =
        case unquote(operation) do
          :metadata -> Store.update_metadata(job_id, %{"message" => "too late"})
          :worker -> Store.assign_worker(job_id, "worker-late")
        end

      assert {:error, {:job_not_found, ^job_id}} = result
      assert_receive {:interleaved, ^race, {:ok, _}}
      assert :error = Store.get(job_id)
    end
  end

  test "persistent contention stops after four snapshot attempts without partial progress" do
    job = create_job("bounded-retries")
    job_id = job.job_id
    repo = Storage.repo_module!()

    race =
      JobQueryRace.after_read(
        {:job, job_id},
        fn ->
          message = "writer-#{System.unique_integer([:positive])}"
          query = from(record in JobRecord, where: record.job_id == ^job_id)
          apply(repo, :update_all, [query, [set: [message: message]]])
        end,
        repeat: true
      )

    assert {:error, {:job_write_conflict, ^job_id}} =
             Store.apply_progress(%{job_id: job_id, stage: :solving, progress: 0.9})

    :telemetry.detach(race)
    for _ <- 1..4, do: assert_receive({:interleaved, ^race, {1, _}})
    refute_receive {:interleaved, ^race, _}, 0
    assert {:ok, current} = Store.get(job_id)
    assert current.progress == job.progress
    assert current.updated_at == job.updated_at
    assert current.message =~ "writer-"
  end

  test "concurrent tasks cannot reopen a completed job" do
    job = create_job("task-writers")
    timestamp = DateTime.add(DateTime.utc_now(), 1, :second)

    results =
      1..16
      |> Task.async_stream(
        fn index ->
          Store.apply_progress(%{
            job_id: job.job_id,
            stage: if(index == 8, do: :completed, else: :solving),
            progress: if(index == 8, do: 1.0, else: 0.5),
            emitted_at: timestamp
          })
        end,
        max_concurrency: 4,
        timeout: 5_000
      )
      |> Enum.to_list()

    for result <- results do
      assert match?({:ok, {:ok, _}}, result) or
               match?({:ok, {:error, {:terminal_job_mutation, :completed, :solving}}}, result)
    end

    assert {:ok, %{status: :completed, progress: 1.0}} = Store.get(job.job_id)
  end

  defp terminal_progress(:completed, _progress), do: 1.0
  defp terminal_progress(_terminal, progress), do: progress

  defp create_job(id, seconds_ago \\ -1) do
    timestamp = DateTime.add(DateTime.utc_now(), seconds_ago, :second)

    {:ok, job} =
      Store.create(%{
        job_id: id,
        project_id: "concurrency-test",
        simulation_case_id: "case",
        status: :solving,
        progress: 0.1,
        created_at: timestamp,
        execution_started_at: timestamp,
        updated_at: timestamp
      })

    job
  end
end
