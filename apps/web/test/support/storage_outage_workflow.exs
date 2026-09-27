defmodule KyuubikiWeb.TestSupport.StorageOutageWorkflow do
  @moduledoc false

  alias KyuubikiWeb.{Analysis, WorkflowOperatorRuntime}
  @state {__MODULE__, :state}

  def configure(owner, mode), do: :persistent_term.put(@state, {owner, mode})

  def install(owner) do
    original = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])
    updated = Keyword.put(original, :solve_runtime_client, __MODULE__)
    Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, updated)
    configure(owner, :hold)

    ExUnit.Callbacks.on_exit(fn ->
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original)
      :persistent_term.erase(@state)
    end)
  end

  def request("solve_bar_1d", payload, _on_progress, opts) do
    {owner, mode} = :persistent_term.get(@state)
    send(owner, {:outage_solver_request, Keyword.get(opts, :job_id), mode})

    case mode do
      :succeed -> {:ok, Map.put(payload, "recovered", true)}
      :hold -> receive do: (:finish -> {:ok, payload})
    end
  end

  def submit(policy) do
    model = %{"id" => "model", "artifact_type" => "model/bar_1d"}
    result = %{"id" => "result", "artifact_type" => "result/bar_1d"}

    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.storage-outage",
      "recovery_policy" => %{"retry_safety" => policy},
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [model]},
        %{
          "id" => "solve",
          "kind" => "solve",
          "operator_id" => "solve.bar_1d",
          "inputs" => [model],
          "outputs" => [result]
        },
        %{"id" => "output", "kind" => "output", "inputs" => [result], "outputs" => []}
      ],
      "edges" => [edge("e0", "input", "solve", model), edge("e1", "solve", "output", result)]
    }

    Analysis.submit_workflow_graph(%{
      "graph" => graph,
      "input_artifacts" => %{"input" => %{"value" => 41}}
    })
  end

  defp edge(id, from, to, port) do
    %{
      "id" => id,
      "from" => %{"node" => from, "port" => port["id"]},
      "to" => %{"node" => to, "port" => port["id"]},
      "artifact_type" => port["artifact_type"]
    }
  end
end
