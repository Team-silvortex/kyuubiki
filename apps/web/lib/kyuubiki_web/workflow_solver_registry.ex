defmodule KyuubikiWeb.WorkflowSolverRegistry do
  @moduledoc false

  @solver_specs [
    {"solve.frame_3d", :solve_frame_3d, "mechanical", "frame_3d",
     "Solve a 3D frame model with six-DOF nodes and verified baseline coverage.",
     ["verified", "mechanical", "frame", "3d"]},
    {"solve.thermal_frame_3d", :solve_thermal_frame_3d, "thermo_mechanical", "thermal_frame_3d",
     "Solve a thermal 3D frame model with restrained expansion and temperature gradients.",
     ["verified", "thermo_mechanical", "frame", "3d"]},
    {"solve.electrostatic_bar_1d", :solve_electrostatic_bar_1d, "electromagnetic",
     "electrostatic_bar_1d",
     "Solve a 1D electrostatic bar model and expose potential, field, and flux results.",
     ["verified", "electromagnetic", "electrostatic", "bar", "1d"]},
    {"solve.electrostatic_plane_triangle_2d", :solve_electrostatic_plane_triangle_2d,
     "electromagnetic", "electrostatic_plane_triangle_2d",
     "Solve a 2D electrostatic triangle model and expose potential, field, and flux results.",
     ["verified", "electromagnetic", "electrostatic", "plane", "triangle", "2d"]},
    {"solve.electrostatic_plane_quad_2d", :solve_electrostatic_plane_quad_2d, "electromagnetic",
     "electrostatic_plane_quad_2d",
     "Solve a 2D electrostatic quad model and expose potential, field, and flux results.",
     ["verified", "electromagnetic", "electrostatic", "plane", "quad", "2d"]},
    {"solve.heat_plane_quad_2d", :solve_heat_plane_quad_2d, "thermal", "heat_plane_quad_2d",
     "Solve a 2D heat-conduction quad model and expose verified temperature/flux fields.",
     ["verified", "thermal", "heat", "plane", "quad", "2d"]},
    {"solve.thermal_truss_3d", :solve_thermal_truss_3d, "thermo_mechanical", "thermal_truss_3d",
     "Solve a thermal 3D truss model with expansion-driven axial response.",
     ["verified", "thermo_mechanical", "truss", "3d"]},
    {"solve.bar_1d", :solve_bar_1d, "mechanical", "bar_1d",
     "Solve a 1D axial bar model with nodal displacement and stress output.",
     ["verified", "mechanical", "bar", "1d"]},
    {"solve.thermal_bar_1d", :solve_thermal_bar_1d, "thermo_mechanical", "thermal_bar_1d",
     "Solve a 1D thermal bar model with coupled displacement and stress response.",
     ["verified", "thermo_mechanical", "bar", "1d"]},
    {"solve.heat_bar_1d", :solve_heat_bar_1d, "thermal", "heat_bar_1d",
     "Solve a 1D heat-conduction bar model with nodal temperature and flux output.",
     ["verified", "thermal", "heat", "bar", "1d"]},
    {"solve.heat_plane_triangle_2d", :solve_heat_plane_triangle_2d, "thermal",
     "heat_plane_triangle_2d",
     "Solve a 2D heat-conduction triangle model and expose temperature gradients.",
     ["verified", "thermal", "heat", "plane", "triangle", "2d"]},
    {"solve.thermal_truss_2d", :solve_thermal_truss_2d, "thermo_mechanical", "thermal_truss_2d",
     "Solve a thermal 2D truss model with expansion-driven axial response.",
     ["verified", "thermo_mechanical", "truss", "2d"]},
    {"solve.beam_1d", :solve_beam_1d, "mechanical", "beam_1d",
     "Solve a 1D beam bending model with displacement and moment response.",
     ["verified", "mechanical", "beam", "1d"]},
    {"solve.thermal_beam_1d", :solve_thermal_beam_1d, "thermo_mechanical", "thermal_beam_1d",
     "Solve a 1D thermal beam model with gradient-driven deformation.",
     ["verified", "thermo_mechanical", "beam", "1d"]},
    {"solve.torsion_1d", :solve_torsion_1d, "mechanical", "torsion_1d",
     "Solve a 1D torsion model with twist and torque distribution output.",
     ["verified", "mechanical", "torsion", "1d"]},
    {"solve.spring_1d", :solve_spring_1d, "mechanical", "spring_1d",
     "Solve a 1D spring chain model with support reaction response.",
     ["verified", "mechanical", "spring", "1d"]},
    {"solve.spring_2d", :solve_spring_2d, "mechanical", "spring_2d",
     "Solve a 2D spring network model with planar support response.",
     ["verified", "mechanical", "spring", "2d"]},
    {"solve.spring_3d", :solve_spring_3d, "mechanical", "spring_3d",
     "Solve a 3D spring network model with spatial support response.",
     ["verified", "mechanical", "spring", "3d"]},
    {"solve.truss_2d", :solve_truss_2d, "mechanical", "truss_2d",
     "Solve a 2D truss model with nodal displacement and axial force output.",
     ["verified", "mechanical", "truss", "2d"]},
    {"solve.truss_3d", :solve_truss_3d, "mechanical", "truss_3d",
     "Solve a 3D truss model with nodal displacement and axial force output.",
     ["verified", "mechanical", "truss", "3d"]},
    {"solve.plane_triangle_2d", :solve_plane_triangle_2d, "mechanical", "plane_triangle_2d",
     "Solve a 2D plane-stress triangle model with stress field output.",
     ["verified", "mechanical", "plane", "triangle", "2d"]},
    {"solve.thermal_plane_triangle_2d", :solve_thermal_plane_triangle_2d, "thermo_mechanical",
     "thermal_plane_triangle_2d",
     "Solve a 2D thermal plane triangle model with thermo-mechanical stress output.",
     ["verified", "thermo_mechanical", "plane", "triangle", "2d"]},
    {"solve.plane_quad_2d", :solve_plane_quad_2d, "mechanical", "plane_quad_2d",
     "Solve a 2D plane-stress quad model with stress field output.",
     ["verified", "mechanical", "plane", "quad", "2d"]},
    {"solve.thermal_plane_quad_2d", :solve_thermal_plane_quad_2d, "thermo_mechanical",
     "thermal_plane_quad_2d",
     "Solve a 2D thermal plane quad model with thermo-mechanical stress output.",
     ["verified", "thermo_mechanical", "plane", "quad", "2d"]},
    {"solve.frame_2d", :solve_frame_2d, "mechanical", "frame_2d",
     "Solve a 2D frame model with bending, axial, and rotational response.",
     ["verified", "mechanical", "frame", "2d"]},
    {"solve.thermal_frame_2d", :solve_thermal_frame_2d, "thermo_mechanical", "thermal_frame_2d",
     "Solve a thermal 2D frame model with restrained expansion and gradients.",
     ["verified", "thermo_mechanical", "frame", "2d"]}
  ]

  @solvers Enum.map(@solver_specs, fn {id, method, domain, family, summary, capability_tags} ->
             %{
               id: id,
               method: method,
               domain: domain,
               family: family,
               summary: summary,
               capability_tags: capability_tags
             }
           end)

  @solver_by_id Map.new(@solvers, &{&1.id, &1})

  def list, do: @solvers

  def fetch(operator_id) when is_binary(operator_id), do: Map.fetch(@solver_by_id, operator_id)
  def fetch(_operator_id), do: :error

  def descriptor(%{
        id: id,
        domain: domain,
        family: family,
        summary: summary,
        capability_tags: tags
      }) do
    %{
      "id" => id,
      "version" => "1.0.0",
      "domain" => domain,
      "family" => family,
      "kind" => "solver",
      "summary" => summary,
      "capability_tags" => tags,
      "origin" => "built_in",
      "input_schema" => %{"schema" => "kyuubiki.operator.#{family}.input", "version" => "1"},
      "output_schema" => %{"schema" => "kyuubiki.operator.#{family}.output", "version" => "1"},
      "inputs" => [
        port_descriptor(
          "model",
          "model/#{family}",
          "Primary operator model input",
          "model",
          "kyuubiki.operator.#{family}.input"
        )
      ],
      "outputs" => [
        port_descriptor(
          "result",
          "result/#{family}",
          "Primary operator result output",
          "result",
          "kyuubiki.operator.#{family}.output"
        )
      ],
      "validation" => %{
        "baseline_status" => "verified",
        "baseline_cases" => ["#{family}_baseline"],
        "smoke_paths" => ["workflow_graph", "orchestrated_api"]
      }
    }
  end

  defp port_descriptor(id, artifact_type, description, dataset_value, schema_ref) do
    %{
      "id" => id,
      "artifact_type" => artifact_type,
      "description" => description,
      "dataset_value" => dataset_value,
      "schema_ref" => %{"schema" => schema_ref, "version" => "1"}
    }
  end
end
