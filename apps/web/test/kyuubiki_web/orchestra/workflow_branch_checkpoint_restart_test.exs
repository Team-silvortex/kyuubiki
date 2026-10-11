defmodule KyuubikiWeb.Orchestra.WorkflowBranchCheckpointRestartTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.{Analysis, AnalysisResultStore, WorkflowOperatorRuntime}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowBranchCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.TestSupport.WorkflowBranchCheckpointFixture, as: Fixture

  defmodule Client do
    def configure(owner, mode, value, oversize \\ false),
      do: :persistent_term.put(__MODULE__, {owner, mode, value, oversize})

    def clear, do: :persistent_term.erase(__MODULE__)

    def request("solve_bar_1d", payload, _progress, opts) do
      {owner, mode, value, oversize} = :persistent_term.get(__MODULE__)
      send(owner, {:branch_solver, opts[:job_id], payload["step"], mode})

      if payload["step"] == "source" do
        result = %{"step" => "finish", "value" => value}

        {:ok,
         if(oversize,
           do: Map.put(result, "padding", List.duplicate(String.duplicate("x", 270_000), 2)),
           else: result
         )}
      else
        if mode == :hold do
          receive do
            :release -> {:ok, payload}
          after
            60_000 -> {:error, :hold_timeout}
          end
        else
          {:ok, payload}
        end
      end
    end
  end

  setup do
    original = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])

    Application.put_env(
      :kyuubiki_web,
      WorkflowOperatorRuntime,
      Keyword.put(original, :solve_runtime_client, Client)
    )

    Store.reset()
    AnalysisResultStore.reset()
    Client.configure(self(), :hold, 1.0)

    on_exit(fn ->
      Application.ensure_all_started(:kyuubiki_web)
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original)
      Client.clear()
      Store.reset()
      AnalysisResultStore.reset()
    end)

    :ok
  end

  for {value, chosen, skipped} <- [
        {1.0, "if_true", "false-path"},
        {-1.0, "if_false", "true-path"}
      ] do
    test "restart retains #{chosen}, rebuilds the join and never executes #{skipped}" do
      Client.configure(self(), :hold, unquote(value))
      id = submit()
      assert_receive {:branch_solver, ^id, "finish", :hold}, 2_000
      {:ok, original} = AnalysisResultStore.get(id)
      key = Envelope.internal_key()
      bucket = original[key][Checkpoint.key()]
      assert bucket["entries"]["branch"]["decision"]["chosen_output"] == unquote(chosen)
      restart(unquote(-value))
      assert_receive {:branch_solver, ^id, "finish", :succeed}, 2_000
      refute_receive {:branch_solver, ^id, "source", :succeed}, 100
      result = terminal(id, "completed")
      assert result["artifacts"]["output.payload"]["value"] === unquote(value)
      assert result["skipped_nodes"] == [unquote(skipped)]

      assert result["branch_decisions"] == [
               Map.put(Fixture.decision(unquote(value > 0)), "node_id", "branch")
             ]

      assert result["recovery"]["generation"] == 2
      assert result["recovery"]["branch_checkpoint_count"] == 0
      {:ok, final} = AnalysisResultStore.get(id)
      refute Map.has_key?(final[key], Checkpoint.key())
    end
  end

  test "replayed upstream drift blocks before either branch or join can dispatch despite on_error skip" do
    Client.configure(self(), :hold, 1.0, true)
    id = submit()
    assert_receive {:branch_solver, ^id, "finish", :hold}, 2_000
    {:ok, original} = AnalysisResultStore.get(id)
    key = Envelope.internal_key()
    assert Envelope.public_summary(original[key])["solve_checkpoint_count"] == 0
    restart(-1.0, true)
    assert_receive {:branch_solver, ^id, "source", :succeed}, 2_000
    refute_receive {:branch_solver, ^id, "finish", :succeed}, 100
    result = terminal(id, "failed")
    assert result["recovery"]["state"] == "recovery_blocked"
    assert result["recovery"]["generation"] == 2
    assert result["recovery"]["branch_checkpoint_count"] == 1

    assert List.last(result["recovery"]["history"])["reason"] ==
             "workflow_branch_checkpoint_input_changed"

    {:ok, ended} = AnalysisResultStore.get(id)
    assert ended[key][Checkpoint.key()] == original[key][Checkpoint.key()]
    assert ended["artifacts"] == %{}
  end

  test "corrupted decisions block restart before any solver is invoked" do
    id = submit()
    assert_receive {:branch_solver, ^id, "finish", :hold}, 2_000
    {:ok, runtime} = AnalysisResultStore.get(id)
    key = Envelope.internal_key()

    corrupted =
      put_in(
        runtime,
        [key, Checkpoint.key(), "entries", "branch", "decision", "predicate_result"],
        false
      )

    :ok = AnalysisResultStore.put(id, corrupted)
    restart(1.0)
    result = terminal(id, "failed")
    refute_receive {:branch_solver, ^id, _, :succeed}, 100
    assert result["recovery"]["state"] == "recovery_blocked"
    assert result["recovery"]["generation"] == 1
    assert result["recovery"]["envelope_retained"]
  end

  test "branch evidence cannot override an explicit side-effect replay restriction" do
    id =
      submit(
        Map.put(Fixture.graph(), "recovery_policy", %{"retry_safety" => "checkpoint_required"})
      )

    assert_receive {:branch_solver, ^id, "finish", :hold}, 2_000
    restart(1.0)
    result = terminal(id, "failed")
    refute_receive {:branch_solver, ^id, _, :succeed}, 100
    assert result["recovery"]["generation"] == 1
    assert result["recovery"]["branch_checkpoint_count"] == 1
  end

  defp restart(value, oversize \\ false) do
    :ok = Application.stop(:kyuubiki_web)
    Client.configure(self(), :succeed, value, oversize)
    assert {:ok, _} = Application.ensure_all_started(:kyuubiki_web)
  end

  defp submit(graph \\ Fixture.graph()) do
    {:ok, admitted} =
      Analysis.submit_workflow_graph(%{
        "graph" => graph,
        "input_artifacts" => %{"input" => %{"step" => "source"}}
      })

    admitted["job"]["job_id"]
  end

  defp terminal(id, status, attempts \\ 300)
  defp terminal(_id, _status, 0), do: flunk("branch workflow did not finish")

  defp terminal(id, status, attempts) do
    case Analysis.fetch_job(id) do
      {:ok, %{"job" => %{"status" => ^status}, "result" => result}} ->
        result

      _ ->
        Process.sleep(10)
        terminal(id, status, attempts - 1)
    end
  end
end
