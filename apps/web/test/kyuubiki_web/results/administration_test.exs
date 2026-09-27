defmodule KyuubikiWeb.Results.AdministrationTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryCoordinator, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.TestSupport.StorageOutageFixture
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    StorageOutageFixture.setup()
  end

  test "updating a missing job cannot create an orphan result" do
    assert {:error, {:job_not_found, "missing-result-owner"}} =
             Analysis.update_result("missing-result-owner", %{"value" => 1})

    assert :error = AnalysisResultStore.get("missing-result-owner")
    assert Store.list() == []
  end

  test "updating a missing result is not an upsert" do
    job = plain_job()
    assert {:error, {:result_not_found, _}} = Analysis.update_result(job.job_id, %{"value" => 1})
    assert :error = AnalysisResultStore.get(job.job_id)
    assert {:ok, ^job} = Store.get(job.job_id)
  end

  for key <- ["_workflow_recovery", :_workflow_recovery] do
    test "public editing rejects injected internal recovery metadata through #{inspect(key)}" do
      job = plain_job()
      original = %{"value" => 1}
      assert :ok = AnalysisResultStore.put(job.job_id, original)
      forged = %{unquote(key) => %{"state" => "pending"}, "value" => 2}

      assert {:error, :workflow_recovery_metadata_is_read_only} =
               Analysis.update_result(job.job_id, forged)

      assert {:ok, ^original} = AnalysisResultStore.get(job.job_id)
      assert {:ok, ^job} = Store.get(job.job_id)
    end
  end

  for shape <- [:terminal_claim, :malformed_claim, :legacy], action <- [:replace, :delete] do
    test "#{action} cannot unlock an active workflow with a #{shape} recovery record" do
      fixture = Fixture.claimed_job()
      key = WorkflowRecoveryEnvelope.internal_key()

      runtime =
        case unquote(shape) do
          :terminal_claim -> put_in(fixture.runtime, [key, "state"], "completed")
          :malformed_claim -> Map.put(fixture.runtime, key, %{})
          :legacy -> Map.delete(fixture.runtime, key)
        end

      assert :ok = AnalysisResultStore.put(fixture.id, runtime)
      assert {:error, :active_workflow_result_is_read_only} = edit(fixture.id, unquote(action))
      Fixture.unchanged(%{fixture | runtime: runtime})
    end
  end

  for action <- [:replace, :delete] do
    test "#{action} cannot erase a running recovery claim merely because the job failed" do
      fixture = Fixture.claimed_job()

      assert {:ok, job} =
               Store.apply_progress(%{job_id: fixture.id, stage: :failed, progress: 1.0})

      assert {:error, :active_workflow_result_is_read_only} = edit(fixture.id, unquote(action))
      Fixture.unchanged(%{fixture | job: job})
    end

    test "#{action} cannot bypass a revoked Orchestra lease" do
      fixture = Fixture.terminal_job()
      name = WorkflowRecoveryCoordinator.snapshot()["lease"]["lease_name"]
      assert {:ok, old} = LeaseStore.current(name)
      assert :ok = LeaseStore.release(old)
      assert {:ok, takeover} = LeaseStore.acquire(name, "other-orchestra", 60_000)
      on_exit(fn -> LeaseStore.release(takeover) end)

      assert {:error, :orchestra_lease_lost} = edit(fixture.id, unquote(action))
      Fixture.unchanged(fixture)
    end
  end

  for status <- [:completed, :failed, :cancelled] do
    test "#{status} workflow results remain editable without replacing recovery identity" do
      fixture = Fixture.terminal_job(unquote(status))
      key = WorkflowRecoveryEnvelope.internal_key()
      assert {:ok, payload} = Analysis.update_result(fixture.id, %{"reviewed" => true})
      assert payload["result"]["reviewed"] == true
      assert payload["result"]["workflow_id"] == fixture.runtime["workflow_id"]
      assert payload["result"]["recovery"]["state"] == Atom.to_string(unquote(status))
      assert {:ok, stored} = AnalysisResultStore.get(fixture.id)
      assert stored[key] == fixture.runtime[key]
      assert {:ok, %{"deleted" => true}} = Analysis.delete_result(fixture.id)
      assert :error = AnalysisResultStore.get(fixture.id)
      assert {:ok, job} = Store.get(fixture.id)
      assert job == fixture.job
    end
  end

  test "unencodable edits return an error without corrupting the terminal receipt" do
    fixture = Fixture.terminal_job()
    coordinator = Process.whereis(WorkflowRecoveryCoordinator)
    assert {:error, _} = Analysis.update_result(fixture.id, %{"bad" => fn -> :not_json end})
    Fixture.unchanged(fixture)
    assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
    assert {:ok, _} = Analysis.update_result(fixture.id, %{"reviewed" => true})
  end

  for recovery <- [nil, %{}, %{"state" => "unrecognized"}], action <- [:replace, :delete] do
    test "#{action} rejects malformed terminal recovery #{inspect(recovery)}" do
      fixture = Fixture.terminal_job()
      key = WorkflowRecoveryEnvelope.internal_key()
      runtime = Map.put(fixture.runtime, key, unquote(Macro.escape(recovery)))
      assert :ok = AnalysisResultStore.put(fixture.id, runtime)
      assert {:error, :active_workflow_result_is_read_only} = edit(fixture.id, unquote(action))
      Fixture.unchanged(%{fixture | runtime: runtime})
    end
  end

  test "blocked recovery on a failed job preserves its retained envelope during review" do
    fixture = Fixture.claimed_job()
    assert {:ok, job} = Store.apply_progress(%{job_id: fixture.id, stage: :failed, progress: 1.0})
    key = WorkflowRecoveryEnvelope.internal_key()
    recovery = WorkflowRecoveryEnvelope.transition(fixture.runtime[key], "recovery_blocked")
    assert is_map(recovery["envelope"])
    runtime = Map.put(fixture.runtime, key, recovery)
    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    assert {:ok, _} = Analysis.update_result(fixture.id, %{"reviewed" => true})
    assert {:ok, result} = AnalysisResultStore.get(fixture.id)
    assert result[key] == runtime[key]
    assert {:ok, _} = Analysis.delete_result(fixture.id)
    assert {:ok, ^job} = Store.get(fixture.id)
  end

  test "terminal legacy workflow results preserve identity and remain removable" do
    fixture = Fixture.terminal_job()
    runtime = Map.delete(fixture.runtime, WorkflowRecoveryEnvelope.internal_key())
    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    assert {:ok, payload} = Analysis.update_result(fixture.id, %{"reviewed" => true})
    assert payload["result"]["workflow_id"] == fixture.runtime["workflow_id"]
    assert {:ok, _} = Analysis.delete_result(fixture.id)
  end

  test "invalid storage mutation requests leave the complete snapshot untouched" do
    fixture = Fixture.terminal_job()

    for {id, action} <- [
          {fixture.id, :clear},
          {fixture.id, {:replace, []}},
          {fixture.id, {:replace, DateTime.utc_now()}},
          {nil, :delete},
          {"", :delete}
        ] do
      assert {:error, :invalid_analysis_result_edit} = Store.edit_result(id, action)
      Fixture.unchanged(fixture)
    end
  end

  test "simultaneous result deletions have exactly one winner and never delete the job" do
    fixture = Fixture.terminal_job()

    outcomes =
      1..8
      |> Task.async_stream(fn _ -> Store.edit_result(fixture.id, :delete) end, max_concurrency: 8)
      |> Enum.map(fn {:ok, result} -> result end)

    assert Enum.count(outcomes, &match?({:ok, _}, &1)) == 1
    assert Enum.count(outcomes, &match?({:error, {:result_not_found, _}}, &1)) == 7
    assert :error = AnalysisResultStore.get(fixture.id)
    assert {:ok, job} = Store.get(fixture.id)
    assert job == fixture.job
  end

  test "a result replacement racing deletion cannot resurrect the removed record" do
    for _ <- 1..12 do
      fixture = Fixture.terminal_job()

      outcomes =
        [{:replace, %{"reviewed" => true}}, :delete]
        |> Task.async_stream(&Store.edit_result(fixture.id, &1), max_concurrency: 2)
        |> Enum.map(fn {:ok, result} -> result end)

      assert [edited, {:ok, _removed}] = outcomes
      assert match?({:ok, _}, edited) or match?({:error, {:result_not_found, _}}, edited)
      assert :error = AnalysisResultStore.get(fixture.id)
      assert {:ok, job} = Store.get(fixture.id)
      assert job == fixture.job
      assert {:error, {:result_not_found, _}} = Analysis.update_result(fixture.id, %{})
    end
  end

  defp edit(id, :replace), do: Analysis.update_result(id, %{"reviewed" => true})
  defp edit(id, :delete), do: Analysis.delete_result(id)

  defp plain_job do
    assert {:ok, job} =
             Store.create(%{
               job_id: "result-admin-#{System.unique_integer([:positive])}",
               project_id: "results",
               simulation_case_id: "case"
             })

    job
  end
end
