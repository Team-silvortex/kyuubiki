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

  def normalize_electrostatic_bar_1d(params),
    do: normalize_graph_model(params, :invalid_electrostatic_bar_model)

  def normalize_magnetostatic_bar_1d(params),
    do: normalize_graph_model(params, :invalid_magnetostatic_bar_model)

  def normalize_torsion_1d(params), do: normalize_graph_model(params, :invalid_torsion_model)
  def normalize_spring_1d(params), do: normalize_graph_model(params, :invalid_spring_model)
  def normalize_spring_2d(params), do: normalize_graph_model(params, :invalid_spring_2d_model)
  def normalize_spring_3d(params), do: normalize_graph_model(params, :invalid_spring_3d_model)
  def normalize_frame_2d(params), do: normalize_graph_model(params, :invalid_frame_model)
  def normalize_frame_3d(params), do: normalize_graph_model(params, :invalid_frame_3d_model)

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

  defp normalize_graph_model(%{"nodes" => nodes, "elements" => elements}, _error)
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_graph_model(%{nodes: nodes, elements: elements}, _error)
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
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
