defmodule KyuubikiWeb.Orchestra.WorkflowProvenanceRestartTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.{Analysis, AnalysisResultStore, WorkflowOperatorRuntime}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowSolveCheckpoint, as: SolveCheckpoint
  alias KyuubikiWeb.TestSupport.WorkflowBranchCheckpointFixture, as: Fixture

  defmodule Client do
    def configure(owner, mode, value),
      do: :persistent_term.put(__MODULE__, {owner, mode, value})

    def clear, do: :persistent_term.erase(__MODULE__)

    def request("solve_bar_1d", payload, _progress, opts) do
      {owner, mode, value} = :persistent_term.get(__MODULE__)
      send(owner, {:provenance_solver, opts[:job_id], payload, mode})

      case payload["step"] do
        "source" ->
          {:ok,
           %{
             "step" => "finish",
             "value" => value,
             "padding" => List.duplicate(String.duplicate("x", 270_000), 2)
           }}

        "finish" ->
          {:ok, %{"step" => "hold"}}

        "hold" when mode == :hold ->
          receive do
            :release -> {:ok, %{"step" => "done"}}
          after
            60_000 -> {:error, :hold_timeout}
          end

        "hold" ->
          {:ok, %{"step" => "done"}}
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

    on_exit(fn ->
      Application.ensure_all_started(:kyuubiki_web)
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original)
      Client.clear()
      Store.reset()
      AnalysisResultStore.reset()
    end)

    :ok
  end

  for reverse <- [false, true] do
    @reverse reverse
    test "equal payload from a changed fallback source blocks solve reuse, reversed=#{reverse}" do
      {id, original} = paused("not-numeric", @reverse)
      restart(1.0)
      assert_receive {:provenance_solver, ^id, %{"step" => "source"}, :succeed}, 2_000
      payload = terminal(id)
      assert payload["job"]["status"] == "failed"
      result = payload["result"]
      assert result["recovery"]["state"] == "recovery_blocked"
      assert result["recovery"]["generation"] == 2
      assert result["recovery"]["solve_checkpoint_count"] == 1

      assert List.last(result["recovery"]["history"])["reason"] ==
               "workflow_solve_checkpoint_input_changed"

      refute_receive {:provenance_solver, ^id, %{"step" => "finish"}, :succeed}, 50
      refute_receive {:provenance_solver, ^id, %{"step" => "hold"}, :succeed}, 50
      {:ok, ended} = AnalysisResultStore.get(id)
      key = Envelope.internal_key()
      assert ended[key][SolveCheckpoint.key()] == original[key][SolveCheckpoint.key()]
      assert ended["artifacts"] == %{}
    end

    test "unused failed alternative may change without invalidating the same fallback, reversed=#{reverse}" do
      {id, _original} = paused("first-invalid", @reverse)
      restart("second-invalid")
      assert_receive {:provenance_solver, ^id, %{"step" => "source"}, :succeed}, 2_000
      assert_receive {:provenance_solver, ^id, %{"step" => "hold"}, :succeed}, 2_000
      refute_receive {:provenance_solver, ^id, %{"step" => "finish"}, :succeed}, 50
      payload = terminal(id)
      assert payload["job"]["status"] == "completed"
      result = payload["result"]
      assert result["artifacts"]["join.payload"] == %{"step" => "finish"}
      assert result["artifacts"]["output.payload"] == %{"step" => "done"}
      join = Enum.find(result["node_runs"], &(&1["node_id"] == "join"))
      assert join["consumed_artifacts"] == ["backup.payload"]
      assert result["failed_nodes"] == ["branch"]
      assert result["recovery"]["solve_checkpoint_count"] == 0
    end
  end

  defp paused(value, reverse) do
    Client.configure(self(), :hold, value)

    {:ok, admitted} =
      Analysis.submit_workflow_graph(%{
        "graph" => graph(reverse),
        "input_artifacts" => %{
          "input" => %{"step" => "source"},
          "backup" => %{"step" => "finish"}
        }
      })

    id = admitted["job"]["job_id"]
    assert_receive {:provenance_solver, ^id, %{"step" => "source"}, :hold}, 2_000
    assert_receive {:provenance_solver, ^id, %{"step" => "finish"} = input, :hold}, 2_000
    assert input == %{"step" => "finish"}
    assert_receive {:provenance_solver, ^id, %{"step" => "hold"}, :hold}, 2_000
    {:ok, original} = AnalysisResultStore.get(id)
    recovery = original[Envelope.internal_key()]
    assert Envelope.public_summary(recovery)["solve_checkpoint_count"] == 1
    assert Envelope.public_summary(recovery)["branch_checkpoint_count"] == 0
    entry = recovery[SolveCheckpoint.key()]["entries"]["finish"]
    assert Jason.decode!(entry["result_json"]) == %{"step" => "hold"}
    {id, original}
  end

  defp restart(value) do
    :ok = Application.stop(:kyuubiki_web)
    Client.configure(self(), :succeed, value)
    assert {:ok, _} = Application.ensure_all_started(:kyuubiki_web)
  end

  defp terminal(id, attempts \\ 300)
  defp terminal(_id, 0), do: flunk("provenance workflow did not reach a terminal state")

  defp terminal(id, attempts) do
    case Analysis.fetch_job(id) do
      {:ok, %{"job" => %{"status" => status}} = payload}
      when status in ["completed", "failed", "cancelled"] ->
        payload

      _ ->
        Process.sleep(10)
        terminal(id, attempts - 1)
    end
  end

  defp graph(reverse) do
    original = Fixture.graph()

    nodes =
      original["nodes"]
      |> Enum.reject(&(&1["id"] == "false-path"))
      |> Enum.map(fn
        %{"id" => "true-path"} = node ->
          %{node | "operator_id" => "transform.normalize_summary_fields"}
          |> Map.put("config", %{"rules" => [%{"source" => "step"}]})

        %{"id" => "finish"} = node ->
          Map.put(node, "config", %{"on_error" => "skip"})

        node ->
          node
      end)

    backup = %{"id" => "backup", "kind" => "input", "outputs" => [port()]}

    hold = %{
      "id" => "hold",
      "kind" => "solve",
      "operator_id" => "solve.bar_1d",
      "inputs" => [port()],
      "outputs" => [port()]
    }

    edges =
      original["edges"]
      |> Enum.reject(
        &(get_in(&1, ["from", "node"]) == "false-path" or
            get_in(&1, ["to", "node"]) == "false-path")
      )
      |> Enum.map(fn edge ->
        if get_in(edge, ["from", "node"]) == "finish",
          do: put_in(edge, ["to", "node"], "hold"),
          else: edge
      end)

    nodes = nodes ++ [backup, hold]

    %{
      original
      | "id" => "provenance-restart",
        "nodes" => if(reverse, do: Enum.reverse(nodes), else: nodes),
        "edges" => edges ++ [edge("backup", "join", "right"), edge("hold", "output")]
    }
  end

  defp port, do: %{"id" => "payload", "artifact_type" => "payload/json"}

  defp edge(from, to, port \\ "payload"),
    do: %{
      "id" => "#{from}-#{to}",
      "artifact_type" => "payload/json",
      "from" => %{"node" => from, "port" => "payload"},
      "to" => %{"node" => to, "port" => port}
    }
end
