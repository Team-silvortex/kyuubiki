defmodule KyuubikiWeb.WorkflowTemplateRegistry do
  @moduledoc false

  alias KyuubikiWeb.WorkflowTemplateCustomEntries
  alias KyuubikiWeb.WorkflowSolverRegistry

  @summary_specs [
    %{
      id: "workflow.bar-1d-summary-json",
      name: "Bar 1D summary JSON",
      operator_id: "solve.bar_1d",
      entry_node_id: "bar_1d_model"
    },
    %{
      id: "workflow.acoustic-bar-1d-summary-json",
      name: "Acoustic bar 1D summary JSON",
      operator_id: "solve.acoustic_bar_1d",
      entry_node_id: "acoustic_bar_1d_model"
    },
    %{
      id: "workflow.thermal-bar-1d-summary-json",
      name: "Thermal bar 1D summary JSON",
      operator_id: "solve.thermal_bar_1d",
      entry_node_id: "thermal_bar_1d_model"
    },
    %{
      id: "workflow.heat-bar-1d-summary-json",
      name: "Heat bar 1D summary JSON",
      operator_id: "solve.heat_bar_1d",
      entry_node_id: "heat_bar_1d_model"
    },
    %{
      id: "workflow.electrostatic-plane-triangle-summary-json",
      name: "Electrostatic plane triangle summary JSON",
      operator_id: "solve.electrostatic_plane_triangle_2d",
      entry_node_id: "electrostatic_plane_triangle_model"
    },
    %{
      id: "workflow.heat-plane-triangle-summary-json",
      name: "Heat plane triangle summary JSON",
      operator_id: "solve.heat_plane_triangle_2d",
      entry_node_id: "heat_plane_triangle_2d_model"
    },
    %{
      id: "workflow.thermal-truss-2d-summary-json",
      name: "Thermal truss 2D summary JSON",
      operator_id: "solve.thermal_truss_2d",
      entry_node_id: "thermal_truss_2d_model"
    },
    %{
      id: "workflow.thermal-truss-3d-summary-json",
      name: "Thermal truss 3D summary JSON",
      operator_id: "solve.thermal_truss_3d",
      entry_node_id: "thermal_truss_3d_model"
    },
    %{
      id: "workflow.truss-2d-summary-json",
      name: "Truss 2D summary JSON",
      operator_id: "solve.truss_2d",
      entry_node_id: "truss_2d_model"
    },
    %{
      id: "workflow.truss-3d-summary-json",
      name: "Truss 3D summary JSON",
      operator_id: "solve.truss_3d",
      entry_node_id: "truss_3d_model"
    },
    %{
      id: "workflow.beam-1d-summary-json",
      name: "Beam 1D summary JSON",
      operator_id: "solve.beam_1d",
      entry_node_id: "beam_1d_model"
    },
    %{
      id: "workflow.thermal-beam-1d-summary-json",
      name: "Thermal beam 1D summary JSON",
      operator_id: "solve.thermal_beam_1d",
      entry_node_id: "thermal_beam_1d_model"
    },
    %{
      id: "workflow.torsion-1d-summary-json",
      name: "Torsion 1D summary JSON",
      operator_id: "solve.torsion_1d",
      entry_node_id: "torsion_1d_model"
    },
    %{
      id: "workflow.spring-1d-summary-json",
      name: "Spring 1D summary JSON",
      operator_id: "solve.spring_1d",
      entry_node_id: "spring_1d_model"
    },
    %{
      id: "workflow.spring-2d-summary-json",
      name: "Spring 2D summary JSON",
      operator_id: "solve.spring_2d",
      entry_node_id: "spring_2d_model"
    },
    %{
      id: "workflow.spring-3d-summary-json",
      name: "Spring 3D summary JSON",
      operator_id: "solve.spring_3d",
      entry_node_id: "spring_3d_model"
    },
    %{
      id: "workflow.frame-2d-summary-json",
      name: "Frame 2D summary JSON",
      operator_id: "solve.frame_2d",
      entry_node_id: "frame_2d_model"
    },
    %{
      id: "workflow.frame-3d-summary-json",
      name: "Frame 3D summary JSON",
      operator_id: "solve.frame_3d",
      entry_node_id: "frame_3d_model"
    },
    %{
      id: "workflow.thermal-frame-2d-summary-json",
      name: "Thermal frame 2D summary JSON",
      operator_id: "solve.thermal_frame_2d",
      entry_node_id: "thermal_frame_2d_model"
    },
    %{
      id: "workflow.thermal-frame-3d-summary-json",
      name: "Thermal frame 3D summary JSON",
      operator_id: "solve.thermal_frame_3d",
      entry_node_id: "thermal_frame_3d_model"
    },
    %{
      id: "workflow.modal-frame-2d-summary-json",
      name: "Modal frame 2D summary JSON",
      operator_id: "solve.modal_frame_2d",
      entry_node_id: "modal_frame_2d_model"
    },
    %{
      id: "workflow.modal-frame-3d-summary-json",
      name: "Modal frame 3D summary JSON",
      operator_id: "solve.modal_frame_3d",
      entry_node_id: "modal_frame_3d_model"
    },
    %{
      id: "workflow.nonlinear-spring-1d-summary-json",
      name: "Nonlinear spring 1D summary JSON",
      operator_id: "solve.nonlinear_spring_1d",
      entry_node_id: "nonlinear_spring_1d_model"
    },
    %{
      id: "workflow.contact-gap-1d-summary-json",
      name: "Contact gap 1D summary JSON",
      operator_id: "solve.contact_gap_1d",
      entry_node_id: "contact_gap_1d_model"
    },
    %{
      id: "workflow.plane-triangle-2d-summary-json",
      name: "Plane triangle 2D summary JSON",
      operator_id: "solve.plane_triangle_2d",
      entry_node_id: "plane_triangle_2d_model"
    },
    %{
      id: "workflow.thermal-plane-triangle-2d-summary-json",
      name: "Thermal plane triangle 2D summary JSON",
      operator_id: "solve.thermal_plane_triangle_2d",
      entry_node_id: "thermal_plane_triangle_2d_model"
    },
    %{
      id: "workflow.plane-quad-2d-summary-json",
      name: "Plane quad 2D summary JSON",
      operator_id: "solve.plane_quad_2d",
      entry_node_id: "plane_quad_2d_model"
    }
  ]

  def list do
    WorkflowTemplateCustomEntries.list() ++
      Enum.map(@summary_specs, &build_summary_workflow_entry/1)
  end

  def fetch(workflow_id) when is_binary(workflow_id) do
    Enum.find(list(), &(&1["id"] == workflow_id))
  end

  def graph_by_id(workflow_id) when is_binary(workflow_id) do
    case fetch(workflow_id) do
      %{"graph" => graph} -> {:ok, graph}
      nil -> {:error, {:workflow_not_found, workflow_id}}
    end
  end

  defp build_summary_workflow_entry(%{
         id: workflow_id,
         name: name,
         operator_id: operator_id,
         entry_node_id: entry_node_id
       }) do
    {:ok, solver} = WorkflowSolverRegistry.fetch(operator_id)
    descriptor = WorkflowSolverRegistry.descriptor(solver)
    input_port = List.first(descriptor["inputs"])
    output_port = List.first(descriptor["outputs"])
    graph = summary_graph(workflow_id, name, operator_id, entry_node_id, input_port, output_port)

    %{
      "id" => workflow_id,
      "name" => name,
      "version" => "1.0.0",
      "summary" =>
        "Solves a #{solver.family |> String.replace("_", " ")} study, extracts top-level summary metrics, and exports them as JSON.",
      "domains" => [solver.domain],
      "capability_tags" =>
        (solver.capability_tags ++ ["summary", "json", "default_template"]) |> Enum.uniq(),
      "graph" => graph,
      "entry_inputs" => [
        %{
          "node_id" => entry_node_id,
          "artifact_type" => input_port["artifact_type"],
          "description" => "#{name} workflow entry artifact."
        }
      ],
      "output_artifacts" => [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "#{name} workflow JSON summary artifact."
        }
      ]
    }
  end

  defp summary_graph(workflow_id, name, operator_id, entry_node_id, input_port, output_port) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => workflow_id,
      "name" => name,
      "version" => "1.0.0",
      "entry_nodes" => [entry_node_id],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        %{
          "id" => entry_node_id,
          "kind" => "input",
          "outputs" => [build_graph_port(input_port)]
        },
        %{
          "id" => "solve_main",
          "kind" => "solve",
          "operator_id" => operator_id,
          "inputs" => [build_graph_port(input_port)],
          "outputs" => [build_graph_port(output_port)]
        },
        %{
          "id" => "extract_summary",
          "kind" => "extract",
          "operator_id" => "extract.result_summary",
          "inputs" => [
            %{
              "id" => "result",
              "artifact_type" => output_port["artifact_type"]
            }
          ],
          "outputs" => [
            %{"id" => "summary", "artifact_type" => "report/summary"}
          ]
        },
        %{
          "id" => "export_json",
          "kind" => "export",
          "operator_id" => "export.summary_json",
          "inputs" => [
            %{"id" => "summary", "artifact_type" => "report/summary"}
          ],
          "outputs" => [
            %{"id" => "json", "artifact_type" => "export/json"}
          ]
        },
        %{
          "id" => "json_output",
          "kind" => "output",
          "inputs" => [%{"id" => "json", "artifact_type" => "export/json"}],
          "outputs" => []
        }
      ],
      "edges" => [
        edge(
          "e0",
          entry_node_id,
          input_port["id"],
          "solve_main",
          input_port["id"],
          input_port["artifact_type"]
        ),
        edge(
          "e1",
          "solve_main",
          output_port["id"],
          "extract_summary",
          "result",
          output_port["artifact_type"]
        ),
        edge("e2", "extract_summary", "summary", "export_json", "summary", "report/summary"),
        edge("e3", "export_json", "json", "json_output", "json", "export/json")
      ]
    }
  end

  defp build_graph_port(port) do
    %{
      "id" => port["id"],
      "artifact_type" => port["artifact_type"],
      "description" => port["description"],
      "dataset_value" => port["dataset_value"]
    }
  end

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
  end
end
