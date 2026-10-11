defmodule KyuubikiWeb.Orchestra.WorkflowSolveCheckpointRestartTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope
  alias KyuubikiWeb.Orchestra.WorkflowSolveCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.WorkflowOperatorRuntime

  defmodule Client do
    def configure(owner, mode, first \\ :success),
      do: :persistent_term.put(__MODULE__, {owner, mode, first})

    def clear, do: :persistent_term.erase(__MODULE__)

    def request("solve_bar_1d", payload, _progress, opts) do
      {owner, mode, first} = :persistent_term.get(__MODULE__)
      send(owner, {:solve_checkpoint_request, opts[:job_id], payload["value"], mode})

      if payload["value"] == 1 do
        case first do
          :failure ->
            {:error, :controlled_solver_failure}

          :oversize ->
            {:ok, Map.put(payload, "padding", List.duplicate(String.duplicate("x", 270_000), 2))}

          :success ->
            {:ok, %{"value" => 1.0, "tiny" => 1.0e-30}}
        end
      else
        case mode do
          :hold ->
            receive do
              :release -> {:ok, payload}
            after
              60_000 -> {:error, :hold_timeout}
            end

          :succeed ->
            {:ok, payload}

          :crash_once ->
            configure(owner, :succeed, first)
            exit(:controlled_second_node_loss)
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

    Client.configure(self(), :hold)
    Store.reset()
    AnalysisResultStore.reset()

    on_exit(fn ->
      Application.ensure_all_started(:kyuubiki_web)
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original)
      Client.clear()
      Store.reset()
      AnalysisResultStore.reset()
    end)

    :ok
  end

  test "restart reuses the completed first solve and only resumes the unfinished second solve" do
    id = submit()
    assert_receive {:solve_checkpoint_request, ^id, 1, :hold}, 2_000
    assert_receive {:solve_checkpoint_request, ^id, 2, :hold}, 2_000
    {:ok, original} = AnalysisResultStore.get(id)
    key = WorkflowRecoveryEnvelope.internal_key()
    assert map_size(original[key][Checkpoint.key()]["entries"]) == 1
    restart(:success)
    assert_receive {:solve_checkpoint_request, ^id, 2, :succeed}, 2_000
    refute_receive {:solve_checkpoint_request, ^id, 1, :succeed}, 100
    result = terminal(id, "completed")
    assert result["artifacts"]["output-a.payload"] === %{"value" => 1.0, "tiny" => 1.0e-30}
    assert result["recovery"]["generation"] == 2
    assert result["recovery"]["solve_checkpoint_count"] == 0
    {:ok, final} = AnalysisResultStore.get(id)
    refute Map.has_key?(final[key], Checkpoint.key())
  end

  @tag capture_log: true
  test "runner loss retains the completed sibling artifact before reclaiming the job" do
    Client.configure(self(), :crash_once)
    id = submit()
    assert_receive {:solve_checkpoint_request, ^id, 1, :crash_once}, 2_000
    assert_receive {:solve_checkpoint_request, ^id, 2, :crash_once}, 2_000
    assert_receive {:solve_checkpoint_request, ^id, 2, :succeed}, 2_000
    refute_receive {:solve_checkpoint_request, ^id, 1, :succeed}, 100
    assert terminal(id, "completed")["recovery"]["generation"] == 2
  end

  test "restart blocks a damaged completed artifact before invoking either solver" do
    id = submit()
    assert_receive {:solve_checkpoint_request, ^id, 2, :hold}, 2_000
    {:ok, runtime} = AnalysisResultStore.get(id)
    key = WorkflowRecoveryEnvelope.internal_key()
    corrupt = put_in(runtime, [key, Checkpoint.key(), "entries", "solve-a", "result_json"], "{}")
    :ok = AnalysisResultStore.put(id, corrupt)
    restart(:success)
    result = terminal(id, "failed")
    refute_receive {:solve_checkpoint_request, ^id, _, :succeed}, 100
    assert result["recovery"]["state"] == "recovery_blocked"
    assert result["recovery"]["generation"] == 1
    assert result["recovery"]["envelope_retained"] == true
  end

  test "a successful checkpoint cannot override the graph's side-effect replay restriction" do
    graph = Map.put(graph(), "recovery_policy", %{"retry_safety" => "checkpoint_required"})
    id = submit(graph)
    assert_receive {:solve_checkpoint_request, ^id, 2, :hold}, 2_000
    restart(:success)
    result = terminal(id, "failed")
    refute_receive {:solve_checkpoint_request, ^id, _, :succeed}, 100
    assert result["recovery"]["state"] == "recovery_blocked"
    assert result["recovery"]["generation"] == 1
    assert result["recovery"]["solve_checkpoint_count"] == 1
  end

  test "oversize results still use the original idempotent replay policy, without extra retention" do
    Client.configure(self(), :hold, :oversize)
    id = submit()
    assert_receive {:solve_checkpoint_request, ^id, 2, :hold}, 2_000
    {:ok, original} = Analysis.fetch_job(id)
    assert original["result"]["recovery"]["solve_checkpoint_count"] == 0
    restart(:oversize)
    assert_receive {:solve_checkpoint_request, ^id, 1, :succeed}, 2_000
    assert_receive {:solve_checkpoint_request, ^id, 2, :succeed}, 2_000
    assert terminal(id, "completed")["recovery"]["generation"] == 2
  end

  test "failed skipped solves are not restored as successful checkpoints" do
    Client.configure(self(), :hold, :failure)
    id = submit()
    assert_receive {:solve_checkpoint_request, ^id, 2, :hold}, 2_000
    {:ok, original} = Analysis.fetch_job(id)
    assert original["result"]["recovery"]["solve_checkpoint_count"] == 0
    restart(:failure)
    assert_receive {:solve_checkpoint_request, ^id, 1, :succeed}, 2_000
    assert_receive {:solve_checkpoint_request, ^id, 2, :succeed}, 2_000
    result = terminal(id, "completed")
    assert result["failed_nodes"] == ["solve-a"]
    assert result["skipped_nodes"] == ["output-a"]
    refute Map.has_key?(result["artifacts"], "output-a.payload")
  end

  defp restart(first) do
    :ok = Application.stop(:kyuubiki_web)
    Client.configure(self(), :succeed, first)
    assert {:ok, _} = Application.ensure_all_started(:kyuubiki_web)
  end

  defp submit(graph \\ graph()) do
    {:ok, admitted} =
      Analysis.submit_workflow_graph(%{
        "graph" => graph,
        "input_artifacts" => %{"input-a" => %{"value" => 1}, "input-b" => %{"value" => 2}}
      })

    admitted["job"]["job_id"]
  end

  defp graph do
    port = %{"id" => "payload", "artifact_type" => "payload/json"}
    input = fn id -> %{"id" => id, "kind" => "input", "outputs" => [port]} end

    solve = fn id ->
      %{
        "id" => id,
        "kind" => "solve",
        "operator_id" => "solve.bar_1d",
        "inputs" => [port],
        "outputs" => [port],
        "config" => %{"on_error" => "skip"}
      }
    end

    output = fn id -> %{"id" => id, "kind" => "output", "inputs" => [port], "outputs" => []} end

    edge = fn from, to ->
      %{
        "id" => from <> to,
        "artifact_type" => "payload/json",
        "from" => %{"node" => from, "port" => "payload"},
        "to" => %{"node" => to, "port" => "payload"}
      }
    end

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "checkpoint-restart",
      "nodes" => [
        input.("input-a"),
        solve.("solve-a"),
        input.("input-b"),
        solve.("solve-b"),
        output.("output-a"),
        output.("output-b")
      ],
      "edges" => [
        edge.("input-a", "solve-a"),
        edge.("solve-a", "output-a"),
        edge.("input-b", "solve-b"),
        edge.("solve-b", "output-b")
      ]
    }
  end

  defp terminal(id, status, attempts \\ 300)
  defp terminal(_id, _status, 0), do: flunk("checkpoint workflow did not finish")

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
