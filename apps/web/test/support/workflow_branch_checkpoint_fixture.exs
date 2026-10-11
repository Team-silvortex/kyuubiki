Code.require_file("workflow_commit_fixture.exs", __DIR__)

defmodule KyuubikiWeb.TestSupport.WorkflowBranchCheckpointFixture do
  @moduledoc false
  alias KyuubikiWeb.Orchestra.{WorkflowBranchCheckpoint, WorkflowRecoveryEnvelope}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture

  def condition(id \\ "branch") do
    %{
      "id" => id,
      "kind" => "condition",
      "config" => %{
        "predicate" => %{"path" => "value", "operator" => "gt", "value" => 0},
        "on_error" => "skip"
      },
      "inputs" => [port("payload")],
      "outputs" => [port("if_true"), port("if_false")]
    }
  end

  def decision(value \\ true),
    do: %{
      "chosen_output" => if(value, do: "if_true", else: "if_false"),
      "predicate_result" => value
    }

  def claimed do
    fixture =
      WorkflowCommitFixture.claimed_job(
        graph: %{"id" => "branch-fixture", "nodes" => [condition()]}
      )

    Map.put(
      fixture,
      :request,
      WorkflowBranchCheckpoint.request(condition(), %{"value" => 1.0}, %{"job_id" => fixture.id})
    )
  end

  def running(count \\ 1) do
    nodes = Enum.map(1..count, &condition("branch-#{&1}"))
    graph = %{"id" => "branch-fixture", "nodes" => nodes}
    {:ok, pending} = WorkflowRecoveryEnvelope.new(graph, %{}, %{"job_id" => "branch-job"}, %{})
    {:ok, recovery, claim} = WorkflowRecoveryEnvelope.claim(pending, "first-owner", :initial)
    %{recovery: recovery, claim: claim, nodes: nodes, context: %{"job_id" => "branch-job"}}
  end

  def graph do
    input = %{"id" => "input", "kind" => "input", "outputs" => [port("payload")]}

    solve = fn id ->
      %{
        "id" => id,
        "kind" => "solve",
        "operator_id" => "solve.bar_1d",
        "inputs" => [port("payload")],
        "outputs" => [port("payload")]
      }
    end

    transform = fn id, inputs ->
      %{
        "id" => id,
        "kind" => "transform",
        "operator_id" => "transform.first_available",
        "inputs" => Enum.map(inputs, &port/1),
        "outputs" => [port("payload")]
      }
    end

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "branch-checkpoint-restart",
      "nodes" => [
        input,
        solve.("source"),
        condition(),
        transform.("true-path", ["payload"]),
        transform.("false-path", ["payload"]),
        transform.("join", ["left", "right"]),
        solve.("finish"),
        %{"id" => "output", "kind" => "output", "inputs" => [port("payload")], "outputs" => []}
      ],
      "edges" => [
        edge("input", "payload", "source"),
        edge("source", "payload", "branch"),
        edge("branch", "if_true", "true-path"),
        edge("branch", "if_false", "false-path"),
        edge("true-path", "payload", "join", "left"),
        edge("false-path", "payload", "join", "right"),
        edge("join", "payload", "finish"),
        edge("finish", "payload", "output")
      ]
    }
  end

  defp port(id), do: %{"id" => id, "artifact_type" => "payload/json"}

  defp edge(from, output, to, input \\ "payload"),
    do: %{
      "id" => from <> to,
      "artifact_type" => "payload/json",
      "from" => %{"node" => from, "port" => output},
      "to" => %{"node" => to, "port" => input}
    }
end
