defmodule KyuubikiWeb.FemModelNormalizer do
  @moduledoc false

  def normalize_axial_bar(params) when is_map(params) do
    with {:ok, length} <- fetch_number(params, ["length", :length]),
         {:ok, area} <- fetch_number(params, ["area", :area]),
         {:ok, elements} <- fetch_number(params, ["elements", :elements]),
         {:ok, tip_force} <- fetch_number(params, ["tip_force", :tip_force]),
         {:ok, youngs_modulus_gpa} <-
           fetch_number(params, ["youngs_modulus_gpa", :youngs_modulus_gpa]) do
      {:ok,
       %{
         "length" => length,
         "area" => area,
         "elements" => round(elements),
         "tip_force" => tip_force,
         "youngs_modulus" => youngs_modulus_gpa * 1.0e9
       }}
    end
  end

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

  def normalize_modal_frame_3d(params),
    do: normalize_graph_model(params, :invalid_modal_frame_3d_model)

  def normalize_thermal_frame_2d(params),
    do: normalize_graph_model(params, :invalid_thermal_frame_model)

  def normalize_thermal_frame_3d(params),
    do: normalize_graph_model(params, :invalid_thermal_frame_3d_model)

  def normalize_electrostatic_plane_triangle_2d(%{"nodes" => nodes, "elements" => elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_electrostatic_plane_triangle_2d(%{nodes: nodes, elements: elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{nodes: nodes, elements: elements}}

  def normalize_electrostatic_plane_triangle_2d(_params),
    do: {:error, :invalid_electrostatic_plane_triangle_model}

  def normalize_electrostatic_plane_quad_2d(%{"nodes" => nodes, "elements" => elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_electrostatic_plane_quad_2d(%{nodes: nodes, elements: elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{nodes: nodes, elements: elements}}

  def normalize_electrostatic_plane_quad_2d(_params),
    do: {:error, :invalid_electrostatic_plane_quad_model}

  def normalize_magnetostatic_plane_triangle_2d(%{"nodes" => nodes, "elements" => elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_magnetostatic_plane_triangle_2d(%{nodes: nodes, elements: elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{nodes: nodes, elements: elements}}

  def normalize_magnetostatic_plane_triangle_2d(_params),
    do: {:error, :invalid_magnetostatic_plane_triangle_model}

  def normalize_magnetostatic_plane_quad_2d(%{"nodes" => nodes, "elements" => elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_magnetostatic_plane_quad_2d(%{nodes: nodes, elements: elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{nodes: nodes, elements: elements}}

  def normalize_magnetostatic_plane_quad_2d(_params),
    do: {:error, :invalid_magnetostatic_plane_quad_model}

  def normalize_heat_plane_triangle_2d(%{"nodes" => nodes, "elements" => elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_heat_plane_triangle_2d(%{nodes: nodes, elements: elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{nodes: nodes, elements: elements}}

  def normalize_heat_plane_triangle_2d(_params),
    do: {:error, :invalid_heat_plane_triangle_model}

  def normalize_heat_plane_quad_2d(%{"nodes" => nodes, "elements" => elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_heat_plane_quad_2d(%{nodes: nodes, elements: elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{nodes: nodes, elements: elements}}

  def normalize_heat_plane_quad_2d(_params), do: {:error, :invalid_heat_plane_quad_model}

  def normalize_stokes_flow_plane_quad_2d(%{"nodes" => nodes, "elements" => elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_stokes_flow_plane_quad_2d(%{nodes: nodes, elements: elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_stokes_flow_plane_quad_2d(_params),
    do: {:error, :invalid_stokes_flow_plane_quad_model}

  def normalize_stokes_flow_plane_triangle_2d(%{"nodes" => nodes, "elements" => elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_stokes_flow_plane_triangle_2d(%{nodes: nodes, elements: elements})
      when is_list(nodes) and is_list(elements),
      do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  def normalize_stokes_flow_plane_triangle_2d(_params),
    do: {:error, :invalid_stokes_flow_plane_triangle_model}

  defp normalize_graph_model(%{"nodes" => nodes, "elements" => elements} = params, _error)
       when is_list(nodes) and is_list(elements) do
    {:ok, params}
  end

  defp normalize_graph_model(%{nodes: nodes, elements: elements} = params, _error)
       when is_list(nodes) and is_list(elements) do
    {:ok,
     params
     |> Map.delete(:nodes)
     |> Map.delete(:elements)
     |> Map.put("nodes", nodes)
     |> Map.put("elements", elements)}
  end

  defp normalize_graph_model(_params, error), do: {:error, error}

  defp fetch_number(params, [key | rest]) do
    case Map.fetch(params, key) do
      {:ok, value} -> cast_number(value)
      :error -> fetch_number(params, rest)
    end
  end

  defp fetch_number(_params, []), do: {:error, :missing_parameter}

  defp cast_number(value) when is_integer(value), do: {:ok, value * 1.0}
  defp cast_number(value) when is_float(value), do: {:ok, value}

  defp cast_number(value) when is_binary(value) do
    case Float.parse(value) do
      {parsed, ""} -> {:ok, parsed}
      _ -> {:error, :invalid_parameter}
    end
  end

  defp cast_number(_value), do: {:error, :invalid_parameter}
end
