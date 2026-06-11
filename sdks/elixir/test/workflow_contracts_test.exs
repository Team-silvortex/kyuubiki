defmodule KyuubikiSdk.WorkflowContractsTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.WorkflowContracts

  test "validates reference workflow examples" do
    dataset =
      "../../../schemas/examples.workflow-dataset.json"
      |> Path.expand(__DIR__)
      |> File.read!()
      |> Jason.decode!()

    graph =
      "../../../schemas/examples.workflow-graph.json"
      |> Path.expand(__DIR__)
      |> File.read!()
      |> Jason.decode!()

    assert {:ok, _} = WorkflowContracts.validate_dataset_contract(dataset)
    assert {:ok, _} = WorkflowContracts.validate_graph(graph)
  end

  test "rejects unknown dataset value reference" do
    graph =
      "../../../schemas/examples.workflow-graph.json"
      |> Path.expand(__DIR__)
      |> File.read!()
      |> Jason.decode!()
      |> put_in(["edges", Access.at(0), "dataset_value"], "missing_value")

    assert {:error, error} = WorkflowContracts.validate_graph(graph)
    assert error.type == :validation
    assert error.message =~ "missing_value"
  end
end
