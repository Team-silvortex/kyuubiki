defmodule KyuubikiSdk.AdvancedSolverWorkflows do
  @moduledoc "Headless workflow templates for advanced physical solvers."

  alias KyuubikiSdk.WorkflowBuilders

  def modal_frame_2d(attrs \\ %{}) do
    single_solver_workflow(
      attr(attrs, :graph_id, "workflow.modal-frame-2d"),
      %{
        name: attr(attrs, :name, "Modal frame 2d"),
        version: attr(attrs, :version, "1.0.0"),
        operator_id: "solve.modal_frame_2d",
        input_artifact_type: "study_model/modal_frame_2d",
        result_artifact_type: "result/modal_frame_2d",
        orchestrated: attr(attrs, :orchestrated, true)
      }
    )
  end

  def modal_frame_3d(attrs \\ %{}) do
    single_solver_workflow(
      attr(attrs, :graph_id, "workflow.modal-frame-3d"),
      %{
        name: attr(attrs, :name, "Modal frame 3d"),
        version: attr(attrs, :version, "1.0.0"),
        operator_id: "solve.modal_frame_3d",
        input_artifact_type: "study_model/modal_frame_3d",
        result_artifact_type: "result/modal_frame_3d",
        orchestrated: attr(attrs, :orchestrated, true)
      }
    )
  end

  def nonlinear_spring_1d(attrs \\ %{}) do
    single_solver_workflow(
      attr(attrs, :graph_id, "workflow.nonlinear-spring-1d"),
      %{
        name: attr(attrs, :name, "Nonlinear spring 1d"),
        version: attr(attrs, :version, "1.0.0"),
        operator_id: "solve.nonlinear_spring_1d",
        input_artifact_type: "study_model/nonlinear_spring_1d",
        result_artifact_type: "result/nonlinear_spring_1d",
        orchestrated: attr(attrs, :orchestrated, true)
      }
    )
  end

  def contact_gap_1d(attrs \\ %{}) do
    single_solver_workflow(
      attr(attrs, :graph_id, "workflow.contact-gap-1d"),
      %{
        name: attr(attrs, :name, "Contact gap 1d"),
        version: attr(attrs, :version, "1.0.0"),
        operator_id: "solve.contact_gap_1d",
        input_artifact_type: "study_model/contact_gap_1d",
        result_artifact_type: "result/contact_gap_1d",
        orchestrated: attr(attrs, :orchestrated, true)
      }
    )
  end

  def magnetostatic_plane_triangle_2d(attrs \\ %{}) do
    single_solver_workflow(
      attr(attrs, :graph_id, "workflow.magnetostatic-plane-triangle-2d"),
      %{
        name: attr(attrs, :name, "Magnetostatic plane triangle 2d"),
        version: attr(attrs, :version, "1.0.0"),
        operator_id: "solve.magnetostatic_plane_triangle_2d",
        input_artifact_type: "study_model/magnetostatic_plane_triangle_2d",
        result_artifact_type: "result/magnetostatic_plane_triangle_2d",
        orchestrated: attr(attrs, :orchestrated, true)
      }
    )
  end

  def magnetostatic_plane_quad_2d(attrs \\ %{}) do
    single_solver_workflow(
      attr(attrs, :graph_id, "workflow.magnetostatic-plane-quad-2d"),
      %{
        name: attr(attrs, :name, "Magnetostatic plane quad 2d"),
        version: attr(attrs, :version, "1.0.0"),
        operator_id: "solve.magnetostatic_plane_quad_2d",
        input_artifact_type: "study_model/magnetostatic_plane_quad_2d",
        result_artifact_type: "result/magnetostatic_plane_quad_2d",
        orchestrated: attr(attrs, :orchestrated, true)
      }
    )
  end

  def single_solver_workflow(graph_id, attrs) do
    input_type = attr(attrs, :input_artifact_type)
    result_type = attr(attrs, :result_artifact_type)

    WorkflowBuilders.graph(
      graph_id,
      attr(attrs, :name),
      attr(attrs, :version, "1.0.0"),
      ["input"],
      [
        WorkflowBuilders.node("input", "input", %{
          outputs: [WorkflowBuilders.port("model", input_type)]
        }),
        WorkflowBuilders.node("solve", "solve", %{
          operator_id: attr(attrs, :operator_id),
          inputs: [WorkflowBuilders.port("model", input_type)],
          outputs: [WorkflowBuilders.port("result", result_type)]
        }),
        WorkflowBuilders.node("output", "output", %{
          inputs: [WorkflowBuilders.port("result", result_type)]
        })
      ],
      [
        WorkflowBuilders.edge("input_to_solve", "input", "model", "solve", "model", input_type),
        WorkflowBuilders.edge(
          "solve_to_output",
          "solve",
          "result",
          "output",
          "result",
          result_type
        )
      ],
      %{
        output_nodes: ["output"],
        defaults:
          WorkflowBuilders.defaults(%{
            cache_policy: "cached",
            orchestrated: attr(attrs, :orchestrated, true)
          })
      }
    )
  end

  defp attr(attrs, key, default \\ nil) do
    Map.get(attrs, key, Map.get(attrs, Atom.to_string(key), default))
  end
end
