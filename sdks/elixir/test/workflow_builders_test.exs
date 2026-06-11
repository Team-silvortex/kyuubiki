defmodule KyuubikiSdk.WorkflowBuildersTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.WorkflowBuilders
  alias KyuubikiSdk.WorkflowContracts

  test "builds valid dataset contract" do
    contract =
      WorkflowBuilders.dataset_contract(
        "dataset.demo/v1",
        "1.0.0",
        [
          WorkflowBuilders.dataset_value(
            "thermal_case",
            "study_model",
            "json_object",
            %{
              shape: WorkflowBuilders.shape(%{axes: [WorkflowBuilders.axis("elements", %{semantic: "mesh_element"})]}),
              encoding: "json",
              schema_ref: WorkflowBuilders.schema_ref("kyuubiki.operator.demo.input", "1")
            }
          )
        ]
      )

    assert contract["schema_version"] == WorkflowContracts.workflow_dataset_schema_version()
    assert {:ok, _} = WorkflowContracts.validate_dataset_contract(contract)
  end

  test "builds valid graph" do
    dataset =
      WorkflowBuilders.dataset_contract(
        "dataset.demo/v1",
        "1.0.0",
        [
          WorkflowBuilders.dataset_value("thermal_case", "study_model", "json_object"),
          WorkflowBuilders.dataset_value("thermal_result", "result", "json_object")
        ]
      )

    graph =
      WorkflowBuilders.graph(
        "workflow.demo",
        "Demo workflow",
        "1.0.0",
        ["input"],
        [
          WorkflowBuilders.node("input", "input", %{outputs: [WorkflowBuilders.port("case", "study_model/demo", %{dataset_value: "thermal_case"})]}),
          WorkflowBuilders.node(
            "solve",
            "solve",
            %{
              operator_id: "solve.demo",
              inputs: [WorkflowBuilders.port("case", "study_model/demo", %{dataset_value: "thermal_case"})],
              outputs: [WorkflowBuilders.port("result", "result/demo", %{dataset_value: "thermal_result"})]
            }
          ),
          WorkflowBuilders.node("output", "output", %{inputs: [WorkflowBuilders.port("result", "result/demo", %{dataset_value: "thermal_result"})]})
        ],
        [
          WorkflowBuilders.edge("edge-1", "input", "case", "solve", "case", "study_model/demo", %{dataset_value: "thermal_case"}),
          WorkflowBuilders.edge("edge-2", "solve", "result", "output", "result", "result/demo", %{dataset_value: "thermal_result"})
        ],
        %{dataset_contract: dataset, output_nodes: ["output"]}
      )

    assert graph["schema_version"] == WorkflowContracts.workflow_graph_schema_version()
    assert {:ok, _} = WorkflowContracts.validate_graph(graph)
  end
end
