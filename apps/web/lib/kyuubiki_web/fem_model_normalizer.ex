defmodule KyuubikiWeb.FemModelNormalizer do
  @moduledoc false

  @composite_payload_keys [
    {"electrostatic_model", :electrostatic_model},
    {"electric_conduction_model", :electric_conduction_model},
    {"heat_model", :heat_model},
    {"thermal_model", :thermal_model},
    {"electrothermal_loss", :electrothermal_loss},
    {"electrothermal_feedback", :electrothermal_feedback},
    {"electric_conduction_feedback", :electric_conduction_feedback},
    {"thermal_expansion_feedback", :thermal_expansion_feedback}
  ]

  def normalize_axial_bar(params), do: KyuubikiWeb.AxialBarInput.normalize(params)

  def normalize_acoustic_bar_1d(params),
    do: normalize_graph_model(params, :invalid_acoustic_model)

  def normalize_truss_2d(params), do: normalize_graph_model(params, :invalid_truss_model)

  def normalize_thermal_truss_2d(params),
    do: normalize_graph_model(params, :invalid_thermal_truss_model)

  def normalize_truss_3d(params), do: normalize_graph_model(params, :invalid_truss_3d_model)

  def normalize_thermal_truss_3d(params),
    do: normalize_graph_model(params, :invalid_thermal_truss_3d_model)

  def normalize_plane_triangle_2d(params), do: normalize_graph_model(params, :invalid_plane_model)

  def normalize_thermal_plane_triangle_2d(params),
    do: normalize_graph_model(params, :invalid_thermal_plane_model)

  def normalize_plane_quad_2d(params),
    do: normalize_graph_model(params, :invalid_plane_quad_model)

  def normalize_thermal_plane_quad_2d(params),
    do: normalize_graph_model(params, :invalid_thermal_plane_quad_model)

  def normalize_beam_1d(params), do: normalize_graph_model(params, :invalid_beam_model)

  def normalize_thermal_beam_1d(params),
    do: normalize_graph_model(params, :invalid_thermal_beam_model)

  def normalize_thermal_bar_1d(params),
    do: normalize_graph_model(params, :invalid_thermal_bar_model)

  def normalize_heat_bar_1d(params), do: normalize_graph_model(params, :invalid_heat_bar_model)

  def normalize_transient_heat_bar_1d(params),
    do: normalize_graph_model(params, :invalid_transient_heat_bar_model)

  def normalize_electrostatic_bar_1d(params),
    do: normalize_graph_model(params, :invalid_electrostatic_bar_model)

  def normalize_magnetostatic_bar_1d(params),
    do: normalize_graph_model(params, :invalid_magnetostatic_bar_model)

  def normalize_advection_diffusion_bar_1d(params) do
    with {:ok, model} <- normalize_graph_model(params, :invalid_advection_diffusion_bar_model),
         scheme <- Map.get(model, "scheme", Map.get(model, :scheme, "galerkin")),
         true <- scheme in ["galerkin", "upwind"],
         false <-
           Map.has_key?(model, "scheme") and Map.has_key?(model, :scheme) and
             model["scheme"] != model[:scheme] do
      if Map.has_key?(model, :scheme) do
        {:ok, model |> Map.delete(:scheme) |> Map.put("scheme", scheme)}
      else
        {:ok, model}
      end
    else
      _ -> {:error, :invalid_advection_diffusion_bar_model}
    end
  end

  def normalize_torsion_1d(params), do: normalize_graph_model(params, :invalid_torsion_model)
  def normalize_spring_1d(params), do: normalize_graph_model(params, :invalid_spring_model)

  def normalize_transient_spring_1d(params),
    do: normalize_graph_model(params, :invalid_transient_spring_model)

  def normalize_harmonic_spring_1d(params),
    do: normalize_graph_model(params, :invalid_harmonic_spring_model)

  def normalize_nonlinear_spring_1d(params),
    do: normalize_graph_model(params, :invalid_nonlinear_spring_model)

  def normalize_contact_gap_1d(%{
        "nodes" => nodes,
        "elements" => elements,
        "contacts" => contacts
      })
      when is_list(nodes) and is_list(elements) and is_list(contacts),
      do: {:ok, %{"nodes" => nodes, "elements" => elements, "contacts" => contacts}}

  def normalize_contact_gap_1d(%{nodes: nodes, elements: elements, contacts: contacts})
      when is_list(nodes) and is_list(elements) and is_list(contacts),
      do: {:ok, %{"nodes" => nodes, "elements" => elements, "contacts" => contacts}}

  def normalize_contact_gap_1d(_params), do: {:error, :invalid_contact_gap_model}

  def normalize_cohesive_interface_1d(
        %{
          "id" => id,
          "initial_stiffness" => initial_stiffness,
          "compression_stiffness" => compression_stiffness,
          "peak_traction" => peak_traction,
          "failure_separation" => failure_separation,
          "separation_history" => separation_history
        } = params
      )
      when is_binary(id) and is_list(separation_history),
      do:
        {:ok,
         Map.merge(params, %{
           "initial_stiffness" => initial_stiffness,
           "compression_stiffness" => compression_stiffness,
           "peak_traction" => peak_traction,
           "failure_separation" => failure_separation
         })}

  def normalize_cohesive_interface_1d(%{
        id: id,
        initial_stiffness: initial_stiffness,
        compression_stiffness: compression_stiffness,
        peak_traction: peak_traction,
        failure_separation: failure_separation,
        separation_history: separation_history
      })
      when is_binary(id) and is_list(separation_history),
      do:
        {:ok,
         %{
           "id" => id,
           "initial_stiffness" => initial_stiffness,
           "compression_stiffness" => compression_stiffness,
           "peak_traction" => peak_traction,
           "failure_separation" => failure_separation,
           "separation_history" => separation_history
         }}

  def normalize_cohesive_interface_1d(_params),
    do: {:error, :invalid_cohesive_interface_model}

  def normalize_cohesive_interface_2d(
        %{
          "nodes" => nodes,
          "element" => element,
          "material" => material,
          "displacement_history" => history
        } = params
      )
      when is_list(nodes) and is_map(element) and is_map(material) and is_list(history),
      do: {:ok, params}

  def normalize_cohesive_interface_2d(%{
        nodes: nodes,
        element: element,
        material: material,
        displacement_history: history
      })
      when is_list(nodes) and is_map(element) and is_map(material) and is_list(history),
      do:
        {:ok,
         %{
           "nodes" => nodes,
           "element" => element,
           "material" => material,
           "displacement_history" => history
         }}

  def normalize_cohesive_interface_2d(_params),
    do: {:error, :invalid_cohesive_interface_2d_model}

  def normalize_cohesive_interface_mesh_2d(
        %{"nodes" => nodes, "materials" => materials, "elements" => elements} = params
      )
      when is_list(nodes) and is_list(materials) and is_list(elements),
      do: {:ok, params}

  def normalize_cohesive_interface_mesh_2d(
        %{nodes: nodes, materials: materials, elements: elements} = params
      )
      when is_list(nodes) and is_list(materials) and is_list(elements),
      do:
        {:ok,
         params
         |> Map.delete(:nodes)
         |> Map.delete(:materials)
         |> Map.delete(:elements)
         |> Map.put("nodes", nodes)
         |> Map.put("materials", materials)
         |> Map.put("elements", elements)}

  def normalize_cohesive_interface_mesh_2d(_params),
    do: {:error, :invalid_cohesive_interface_mesh_2d_model}

  def normalize_cohesive_interface_mesh_3d(
        %{"nodes" => nodes, "materials" => materials, "elements" => elements} = params
      )
      when is_list(nodes) and is_list(materials) and is_list(elements),
      do: {:ok, params}

  def normalize_cohesive_interface_mesh_3d(
        %{nodes: nodes, materials: materials, elements: elements} = params
      )
      when is_list(nodes) and is_list(materials) and is_list(elements),
      do:
        {:ok,
         params
         |> Map.delete(:nodes)
         |> Map.delete(:materials)
         |> Map.delete(:elements)
         |> Map.put("nodes", nodes)
         |> Map.put("materials", materials)
         |> Map.put("elements", elements)}

  def normalize_cohesive_interface_mesh_3d(_params),
    do: {:error, :invalid_cohesive_interface_mesh_3d_model}

  def normalize_spring_2d(params), do: normalize_graph_model(params, :invalid_spring_2d_model)
  def normalize_spring_3d(params), do: normalize_graph_model(params, :invalid_spring_3d_model)
  def normalize_frame_2d(params), do: normalize_graph_model(params, :invalid_frame_model)
  def normalize_frame_3d(params), do: normalize_graph_model(params, :invalid_frame_3d_model)

  def normalize_solid_tetra_3d(params),
    do: normalize_graph_model(params, :invalid_solid_tetra_3d_model)

  def normalize_modal_frame_2d(params),
    do: normalize_graph_model(params, :invalid_modal_frame_model)

  def normalize_buckling_beam_1d(params),
    do: normalize_graph_model(params, :invalid_buckling_beam_model)

  def normalize_buckling_frame_2d(%{"frame" => frame} = params) when is_map(frame) do
    with {:ok, normalized_frame} <- normalize_graph_model(frame, :invalid_buckling_frame_model) do
      {:ok, Map.put(params, "frame", normalized_frame)}
    end
  end

  def normalize_buckling_frame_2d(%{frame: frame} = params) when is_map(frame) do
    with {:ok, normalized_frame} <- normalize_graph_model(frame, :invalid_buckling_frame_model) do
      {:ok,
       params
       |> Map.delete(:frame)
       |> Map.put("frame", normalized_frame)}
    end
  end

  def normalize_buckling_frame_2d(_params), do: {:error, :invalid_buckling_frame_model}

  def normalize_frame_2d_p_delta(%{"buckling" => buckling} = params) when is_map(buckling) do
    with {:ok, normalized_buckling} <- normalize_buckling_frame_2d(buckling) do
      {:ok, Map.put(params, "buckling", normalized_buckling)}
    end
  end

  def normalize_frame_2d_p_delta(%{buckling: buckling} = params) when is_map(buckling) do
    with {:ok, normalized_buckling} <- normalize_buckling_frame_2d(buckling) do
      {:ok,
       params
       |> Map.delete(:buckling)
       |> Map.put("buckling", normalized_buckling)}
    end
  end

  def normalize_frame_2d_p_delta(_params), do: {:error, :invalid_frame_2d_p_delta_model}

  def normalize_frame_2d_material_p_delta(
        %{
          "stability" => stability,
          "materials" => materials
        } = params
      )
      when is_map(stability) and is_list(materials) do
    with {:ok, normalized_stability} <- normalize_frame_2d_p_delta(stability) do
      {:ok, Map.put(params, "stability", normalized_stability)}
    end
  end

  def normalize_frame_2d_material_p_delta(%{stability: stability, materials: materials} = params)
      when is_map(stability) and is_list(materials) do
    with {:ok, normalized_stability} <- normalize_frame_2d_p_delta(stability) do
      {:ok, Map.put(params, :stability, normalized_stability)}
    end
  end

  def normalize_frame_2d_material_p_delta(_params),
    do: {:error, :invalid_frame_2d_material_p_delta_model}

  def normalize_modal_frame_3d(params),
    do: normalize_graph_model(params, :invalid_modal_frame_3d_model)

  def normalize_thermal_frame_2d(params),
    do: normalize_graph_model(params, :invalid_thermal_frame_model)

  def normalize_thermal_frame_3d(params),
    do: normalize_graph_model(params, :invalid_thermal_frame_3d_model)

  def normalize_electrostatic_plane_triangle_2d(params),
    do: normalize_graph_model(params, :invalid_electrostatic_plane_triangle_model)

  def normalize_electrostatic_plane_quad_2d(params),
    do: normalize_graph_model(params, :invalid_electrostatic_plane_quad_model)

  def normalize_electric_conduction_plane_quad_2d(
        %{"nodes" => nodes, "elements" => elements} = params
      )
      when is_list(nodes) and is_list(elements) do
    normalize_electric_conduction_interfaces(
      nodes,
      elements,
      Map.get(params, "contact_interfaces", []),
      Map.get(params, "terminals", [])
    )
  end

  def normalize_electric_conduction_plane_quad_2d(%{nodes: nodes, elements: elements} = params)
      when is_list(nodes) and is_list(elements) do
    normalize_electric_conduction_interfaces(
      nodes,
      elements,
      Map.get(params, :contact_interfaces, []),
      Map.get(params, :terminals, [])
    )
  end

  def normalize_electric_conduction_plane_quad_2d(_params),
    do: {:error, :invalid_electric_conduction_plane_quad_model}

  def normalize_composite_thermo_electric_panel(params) when is_map(params) do
    with {:ok, normalized} <- normalize_composite_payload_maps(params) do
      research = Map.get(params, "research", Map.get(params, :research))
      {:ok, if(is_nil(research), do: normalized, else: Map.put(normalized, "research", research))}
    end
  end

  def normalize_composite_thermo_electric_panel(_params),
    do: {:error, :invalid_composite_thermo_electric_panel_model}

  defp normalize_composite_payload_maps(params) do
    Enum.reduce_while(@composite_payload_keys, {:ok, %{}}, fn {string_key, atom_key},
                                                              {:ok, normalized} ->
      case Map.get(params, string_key, Map.get(params, atom_key)) do
        value when is_map(value) ->
          {:cont, {:ok, Map.put(normalized, string_key, value)}}

        _ ->
          {:halt, {:error, :invalid_composite_thermo_electric_panel_model}}
      end
    end)
  end

  defp normalize_electric_conduction_interfaces(nodes, elements, contacts, terminals)
       when is_list(contacts) and is_list(terminals),
       do:
         {:ok,
          %{
            "nodes" => nodes,
            "elements" => elements,
            "contact_interfaces" => contacts,
            "terminals" => terminals
          }}

  defp normalize_electric_conduction_interfaces(_nodes, _elements, _contacts, _terminals),
    do: {:error, :invalid_electric_conduction_plane_quad_model}

  def normalize_magnetostatic_plane_triangle_2d(params),
    do: normalize_graph_model(params, :invalid_magnetostatic_plane_triangle_model)

  def normalize_magnetostatic_plane_quad_2d(params),
    do: normalize_graph_model(params, :invalid_magnetostatic_plane_quad_model)

  def normalize_heat_plane_triangle_2d(params),
    do: normalize_graph_model(params, :invalid_heat_plane_triangle_model)

  def normalize_heat_plane_quad_2d(params),
    do: normalize_graph_model(params, :invalid_heat_plane_quad_model)

  def normalize_stokes_flow_plane_quad_2d(params),
    do: normalize_graph_model(params, :invalid_stokes_flow_plane_quad_model)

  def normalize_stokes_flow_plane_triangle_2d(params),
    do: normalize_graph_model(params, :invalid_stokes_flow_plane_triangle_model)

  defp normalize_graph_model(%{"nodes" => nodes, "elements" => elements} = params, error)
       when is_list(nodes) and is_list(elements) do
    with {:ok, nodes} <- KyuubikiWeb.GraphEntityInput.normalize(nodes, "n"),
         {:ok, elements} <- KyuubikiWeb.GraphEntityInput.normalize(elements, "e") do
      {:ok, params |> Map.put("nodes", nodes) |> Map.put("elements", elements)}
    else
      _ -> {:error, error}
    end
  end

  defp normalize_graph_model(%{nodes: nodes, elements: elements} = params, error)
       when is_list(nodes) and is_list(elements) do
    params
    |> Map.delete(:nodes)
    |> Map.delete(:elements)
    |> Map.put("nodes", nodes)
    |> Map.put("elements", elements)
    |> normalize_graph_model(error)
  end

  defp normalize_graph_model(_params, error), do: {:error, error}
end
