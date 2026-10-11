defmodule KyuubikiWeb.TestSupport.WorkflowSolveCheckpointFixture do
  @moduledoc false
  alias KyuubikiWeb.Orchestra.{WorkflowRecoveryEnvelope, WorkflowSolveCheckpoint}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  def solve_node(id \\ "solve") do
    port = %{"id" => "payload", "artifact_type" => "payload/json"}

    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => "solve.bar_1d",
      "inputs" => [port],
      "outputs" => [port],
      "config" => %{}
    }
  end

  def graph(count \\ 1),
    do: %{"id" => "solve-checkpoint", "nodes" => for(i <- 1..count, do: solve_node("solve-#{i}"))}

  def claimed do
    fixture = Fixture.claimed_job(graph: %{"id" => "solve-checkpoint", "nodes" => [solve_node()]})
    recovery = fixture.runtime[WorkflowRecoveryEnvelope.internal_key()]

    request =
      WorkflowSolveCheckpoint.request(
        solve_node(),
        %{"value" => 41},
        recovery["envelope"]["orchestration_context"]
      )

    Map.put(fixture, :request, request)
  end

  def running(count \\ 1) do
    {:ok, pending} = WorkflowRecoveryEnvelope.new(graph(count), %{}, %{"job_id" => "parent"}, %{})
    {:ok, recovery, claim} = WorkflowRecoveryEnvelope.claim(pending, "owner", :initial)
    {recovery, claim}
  end

  def request(recovery, index \\ 1, input \\ %{"value" => 41}) do
    node = Enum.at(recovery["envelope"]["graph"]["nodes"], index - 1)
    WorkflowSolveCheckpoint.request(node, input, recovery["envelope"]["orchestration_context"])
  end
end
