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

  test "validates execution hints" do
    graph =
      "../../../schemas/examples.workflow-graph.json"
      |> Path.expand(__DIR__)
      |> File.read!()
      |> Jason.decode!()
      |> Map.put("dispatch_policy", "central_fetch")
      |> Map.put("placement_tags", ["mesh-enabled"])
      |> Map.put("required_capabilities", ["artifact-cache"])
      |> Map.put("operator_fetch_plan", [
        %{
          "node_id" => "thermal_solve",
          "operator_id" => "solve.thermal.steady_state",
          "package_ref" => "kyuubiki://operators/solve.thermal.steady_state",
          "version" => "1.0.0",
          "integrity" => "sha256:demo",
          "cache_scope" => "agent"
        }
      ])
      |> Map.put("defaults", %{
        "cache_policy" => "cached",
        "orchestrated" => false,
        "dispatch_policy" => "central_fetch",
        "placement_tags" => ["cpu"],
        "required_capabilities" => ["solver.thermal"]
      })
      |> put_in(["nodes", Access.at(1), "placement_tags"], ["gpu-preferred"])
      |> put_in(["nodes", Access.at(1), "required_capabilities"], ["solver.thermal"])

    assert {:ok, validated} = WorkflowContracts.validate_graph(graph)
    assert validated["dispatch_policy"] == "central_fetch"
  end

  test "rejects invalid dispatch policy" do
    graph =
      "../../../schemas/examples.workflow-graph.json"
      |> Path.expand(__DIR__)
      |> File.read!()
      |> Jason.decode!()
      |> Map.put("dispatch_policy", "mystery_mode")

    assert {:error, error} = WorkflowContracts.validate_graph(graph)
    assert error.type == :validation
    assert error.message =~ "dispatch_policy"
  end
end
