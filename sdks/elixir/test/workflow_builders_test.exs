defmodule KyuubikiSdk.WorkflowBuildersTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.WorkflowBuilders
  alias KyuubikiSdk.WorkflowContracts
  alias KyuubikiSdk.AdvancedSolverWorkflows

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
              shape:
                WorkflowBuilders.shape(%{
                  axes: [WorkflowBuilders.axis("elements", %{semantic: "mesh_element"})]
                }),
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
          WorkflowBuilders.node("input", "input", %{
            outputs: [
              WorkflowBuilders.port("case", "study_model/demo", %{dataset_value: "thermal_case"})
            ]
          }),
          WorkflowBuilders.node(
            "solve",
            "solve",
            %{
              operator_id: "solve.demo",
              inputs: [
                WorkflowBuilders.port("case", "study_model/demo", %{dataset_value: "thermal_case"})
              ],
              outputs: [
                WorkflowBuilders.port("result", "result/demo", %{dataset_value: "thermal_result"})
              ]
            }
          ),
          WorkflowBuilders.node("output", "output", %{
            inputs: [
              WorkflowBuilders.port("result", "result/demo", %{dataset_value: "thermal_result"})
            ]
          })
        ],
        [
          WorkflowBuilders.edge("edge-1", "input", "case", "solve", "case", "study_model/demo", %{
            dataset_value: "thermal_case"
          }),
          WorkflowBuilders.edge("edge-2", "solve", "result", "output", "result", "result/demo", %{
            dataset_value: "thermal_result"
          })
        ],
        %{
          dataset_contract: dataset,
          output_nodes: ["output"],
          defaults:
            WorkflowBuilders.defaults(%{
              cache_policy: "cached",
              orchestrated: false,
              dispatch_policy: "central_fetch",
              placement_tags: ["cpu"],
              required_capabilities: ["solver.thermal"]
            }),
          dispatch_policy: "central_fetch",
          operator_fetch_plan: [
            WorkflowBuilders.operator_fetch_entry("solve", "solve.demo", %{
              package_ref: "kyuubiki://operators/solve.demo",
              version: "1.0.0",
              integrity: "sha256:demo",
              cache_scope: "agent"
            })
          ],
          placement_tags: ["mesh-enabled"],
          required_capabilities: ["artifact-cache"]
        }
      )

    assert graph["schema_version"] == WorkflowContracts.workflow_graph_schema_version()
    assert graph["dispatch_policy"] == "central_fetch"
    assert graph["defaults"]["orchestrated"] == false
    assert {:ok, _} = WorkflowContracts.validate_graph(graph)
  end

  test "builds advanced solver workflow templates" do
    modal = AdvancedSolverWorkflows.modal_frame_2d()
    modal_3d = AdvancedSolverWorkflows.modal_frame_3d()
    nonlinear = AdvancedSolverWorkflows.nonlinear_spring_1d(%{orchestrated: false})
    contact = AdvancedSolverWorkflows.contact_gap_1d()
    magnetic = AdvancedSolverWorkflows.magnetostatic_plane_triangle_2d()
    magnetic_quad = AdvancedSolverWorkflows.magnetostatic_plane_quad_2d()

    assert get_in(modal, ["nodes", Access.at(1), "operator_id"]) == "solve.modal_frame_2d"
    assert get_in(modal, ["edges", Access.at(1), "artifact_type"]) == "result/modal_frame_2d"
    assert get_in(modal_3d, ["nodes", Access.at(1), "operator_id"]) == "solve.modal_frame_3d"
    assert get_in(modal_3d, ["edges", Access.at(1), "artifact_type"]) == "result/modal_frame_3d"

    assert get_in(nonlinear, ["nodes", Access.at(1), "operator_id"]) ==
             "solve.nonlinear_spring_1d"

    assert nonlinear["defaults"]["orchestrated"] == false
    assert get_in(contact, ["nodes", Access.at(1), "operator_id"]) == "solve.contact_gap_1d"
    assert get_in(contact, ["edges", Access.at(1), "artifact_type"]) == "result/contact_gap_1d"

    assert get_in(magnetic, ["nodes", Access.at(1), "operator_id"]) ==
             "solve.magnetostatic_plane_triangle_2d"

    assert get_in(magnetic, ["edges", Access.at(1), "artifact_type"]) ==
             "result/magnetostatic_plane_triangle_2d"

    assert get_in(magnetic_quad, ["nodes", Access.at(1), "operator_id"]) ==
             "solve.magnetostatic_plane_quad_2d"

    assert get_in(magnetic_quad, ["edges", Access.at(1), "artifact_type"]) ==
             "result/magnetostatic_plane_quad_2d"
  end
end
