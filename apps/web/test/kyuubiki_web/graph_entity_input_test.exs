defmodule KyuubikiWeb.GraphEntityInputTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.{FemModelNormalizer, GraphEntityInput}

  @fixture Path.expand(
             "../../../../schemas/examples.graph-entity-input-normalization.json",
             __DIR__
           )
  @cases @fixture |> File.read!() |> Jason.decode!()
  @normalizers %{
    "solve_thermal_bar_1d" => &FemModelNormalizer.normalize_thermal_bar_1d/1,
    "solve_heat_bar_1d" => &FemModelNormalizer.normalize_heat_bar_1d/1,
    "solve_transient_heat_bar_1d" => &FemModelNormalizer.normalize_transient_heat_bar_1d/1,
    "solve_electrostatic_bar_1d" => &FemModelNormalizer.normalize_electrostatic_bar_1d/1,
    "solve_magnetostatic_bar_1d" => &FemModelNormalizer.normalize_magnetostatic_bar_1d/1,
    "solve_advection_diffusion_bar_1d" =>
      &FemModelNormalizer.normalize_advection_diffusion_bar_1d/1,
    "solve_heat_plane_triangle_2d" => &FemModelNormalizer.normalize_heat_plane_triangle_2d/1,
    "solve_heat_plane_quad_2d" => &FemModelNormalizer.normalize_heat_plane_quad_2d/1,
    "solve_electrostatic_plane_triangle_2d" =>
      &FemModelNormalizer.normalize_electrostatic_plane_triangle_2d/1,
    "solve_electrostatic_plane_quad_2d" =>
      &FemModelNormalizer.normalize_electrostatic_plane_quad_2d/1
  }

  defp patched(base, case_data) do
    base =
      if case_data["append_element"],
        do: Map.update!(base, "elements", &(&1 ++ [hd(&1)])),
        else: base

    Enum.reduce(case_data["edits"], base, fn edit, input ->
      Map.update!(input, edit["collection"], fn entities ->
        List.update_at(entities, edit["index"], fn entity ->
          if Map.has_key?(edit, "replacement"),
            do: edit["replacement"],
            else: Map.put(entity, "id", edit["id"])
        end)
      end)
    end)
  end

  test "shared accepted graph inputs preserve or generate unique ids" do
    assert map_size(@cases["models"]) == 10
    assert map_size(@normalizers) == 10
    assert Map.keys(@cases["models"]) -- Map.keys(@normalizers) == []

    assert length(@cases["accepted"]) == 6

    for {action, normalize} <- @normalizers, case_data <- @cases["accepted"] do
      base = @cases["models"][action]
      input = patched(base, case_data)
      assert {:ok, normalized} = normalize.(input), "#{action}/#{case_data["id"]}"

      for {collection, prefix} <- [{"nodes", "n"}, {"elements", "e"}] do
        for {entity, index} <- Enum.with_index(input[collection]) do
          id = Map.get(entity, "id", "")
          expected = if id == "", do: "#{prefix}#{index}", else: id
          assert Enum.at(normalized[collection], index) == Map.put(entity, "id", expected)
        end
      end

      assert {:ok, ^normalized} = normalize.(normalized)
    end
  end

  test "shared invalid graph ids fail without silent repair" do
    assert length(@cases["rejected"]) == 18

    for {action, normalize} <- @normalizers, case_data <- @cases["rejected"] do
      assert {:error, _} = normalize.(patched(@cases["models"][action], case_data)),
             "#{action}/#{case_data["id"]}"
    end
  end

  test "atom keys preserve controls and use the same identifier policy" do
    assert {:ok, normalized} =
             FemModelNormalizer.normalize_transient_heat_bar_1d(%{
               nodes: [%{x: 0}, %{id: "fixed", x: 1}],
               elements: [%{id: "", node_i: 0, node_j: 1}],
               time_step: 0.1,
               steps: 3,
               history_stride: 1
             })

    assert normalized["nodes"] == [%{:x => 0, "id" => "n0"}, %{id: "fixed", x: 1}]
    assert normalized["elements"] == [%{:node_i => 0, :node_j => 1, "id" => "e0"}]
    assert normalized[:time_step] == 0.1
    assert normalized[:steps] == 3
    assert normalized[:history_stride] == 1
    assert {:error, _} = GraphEntityInput.normalize([%{id: nil}], "n")
    assert {:error, _} = GraphEntityInput.normalize([%{id: "n1"}, %{}], "n")
    assert {:error, _} = GraphEntityInput.normalize([%{"id" => nil, :id => "valid"}], "n")
  end

  test "broader graph normalizer callers reject ambiguous ids too" do
    for normalize <- [
          &FemModelNormalizer.normalize_spring_1d/1,
          &FemModelNormalizer.normalize_truss_2d/1,
          &FemModelNormalizer.normalize_stokes_flow_plane_quad_2d/1
        ] do
      assert {:error, _} = normalize.(%{"nodes" => [%{}, %{"id" => "n0"}], "elements" => []})
    end
  end
end
