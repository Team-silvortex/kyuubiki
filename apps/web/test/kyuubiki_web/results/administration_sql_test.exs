defmodule KyuubikiWeb.Results.AdministrationSqlTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.{LeaseStore, WorkflowRecoveryCoordinator}
  alias KyuubikiWeb.TestSupport.{AnalysisCommitFault, ResultReadProbe, StorageOutageFixture}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @moduletag skip: not Storage.sqlite?()

  setup do
    StorageOutageFixture.setup()
  end

  for action <- [:replace, :delete] do
    test "#{action} and a concurrent workflow claim cannot both consume a mutable snapshot" do
      fixture = Fixture.claimed_job()
      plain = %{"editable" => true}
      assert :ok = AnalysisResultStore.put(fixture.id, plain)
      probe = ResultReadProbe.pause(fixture.id)

      editor =
        Task.async(fn ->
          case unquote(action) do
            :replace -> Analysis.update_result(fixture.id, %{"edited" => true})
            :delete -> Analysis.delete_result(fixture.id)
          end
        end)

      assert_receive {:result_read_paused, ^probe, reader, protected?}, 2_000
      on_exit(probe, fn -> send(reader, :resume_result_read) end)

      publisher =
        Task.async(fn ->
          Store.apply_progress_with_result(
            %{job_id: fixture.id, stage: :solving, progress: 0.2},
            fixture.job,
            plain,
            fixture.runtime
          )
        end)

      # An unprotected old read permits a real second transaction to finish;
      # an immediate transaction must release its reader before the writer can proceed.
      published = if not protected?, do: Task.await(publisher, 2_000)
      send(reader, :resume_result_read)
      on_exit(probe, fn -> :ok end)
      edited = Task.await(editor, 2_000)
      published = if protected?, do: Task.await(publisher, 2_000), else: published
      assert Enum.count([edited, published], &match?({:ok, _}, &1)) == 1

      case published do
        {:ok, job} ->
          Fixture.unchanged(%{fixture | job: job})

        {:error, _} ->
          assert {:ok, job} = Store.get(fixture.id)
          assert job == fixture.job

          case unquote(action) do
            :replace -> assert {:ok, %{"edited" => true}} = AnalysisResultStore.get(fixture.id)
            :delete -> assert :error = AnalysisResultStore.get(fixture.id)
          end
      end
    end
  end

  for action <- [:replace, :delete], failed_write <- [:job, :result] do
    test "#{action} rolls back when the #{failed_write} write fails and permits retry" do
      fixture = Fixture.terminal_job()

      restore =
        case {unquote(action), unquote(failed_write)} do
          {_, :job} ->
            AnalysisCommitFault.reject_job_update(fixture.id)

          {:replace, :result} ->
            AnalysisCommitFault.reject_result_update(fixture.id)

          {:delete, :result} ->
            AnalysisCommitFault.reject_delete(fixture.id, "kyuubiki_analysis_results")
        end

      coordinator = Process.whereis(WorkflowRecoveryCoordinator)
      assert {:error, {:completion_persistence_failed, _}} = edit(fixture.id, unquote(action))
      Fixture.unchanged(fixture)
      assert Process.whereis(WorkflowRecoveryCoordinator) == coordinator
      restore.()
      assert {:ok, _} = edit(fixture.id, unquote(action))
      assert {:ok, job} = Store.get(fixture.id)
      assert job == fixture.job
    end
  end

  for {action, operation, mutation} <- [
        {:replace, "UPDATE", {:replace, %{"reviewed" => true}}},
        {:delete, "DELETE", :delete}
      ] do
    @tag capture_log: true
    test "process loss after result #{action} rolls back before lease commit" do
      fixture = Fixture.terminal_job()

      assert {:ok, lease} =
               LeaseStore.acquire("result-edit-probe", LeaseStore.instance_id(), 120_000)

      on_exit(fn -> LeaseStore.release(lease) end)
      probe = AnalysisCommitFault.pause_after_result_change(fixture.id, unquote(operation))
      edit = fn -> Store.edit_result(fixture.id, unquote(Macro.escape(mutation))) end
      {pid, ref} = spawn_monitor(fn -> LeaseStore.with_lease(lease, edit) end)
      on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)

      assert_receive {:completion_paused, ^probe, ^pid}, 2_000
      Process.exit(pid, :kill)
      assert_receive {:DOWN, ^ref, :process, ^pid, :killed}, 2_000
      Fixture.unchanged(fixture)
      assert {:ok, _} = LeaseStore.with_lease(lease, edit)
    end
  end

  defp edit(id, :replace), do: Analysis.update_result(id, %{"reviewed" => true})
  defp edit(id, :delete), do: Analysis.delete_result(id)
end
