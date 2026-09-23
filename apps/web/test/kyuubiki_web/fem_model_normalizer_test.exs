defmodule KyuubikiWeb.FemModelNormalizerTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.FemModelNormalizer

  test "preserves transient solver controls beyond graph topology" do
    params = %{
      "nodes" => [%{"id" => "fixed"}, %{"id" => "tip"}],
      "elements" => [%{"id" => "s0", "node_i" => 0, "node_j" => 1}],
      "time_step" => 0.01,
      "steps" => 10,
      "history_stride" => 4
    }

    assert {:ok, normalized} = FemModelNormalizer.normalize_transient_spring_1d(params)
    assert normalized["time_step"] == 0.01
    assert normalized["steps"] == 10
    assert normalized["history_stride"] == 4
  end

  test "preserves harmonic frequency sweep controls" do
    params = %{
      nodes: [%{id: "fixed"}, %{id: "tip"}],
      elements: [%{id: "s0", node_i: 0, node_j: 1}],
      frequencies_hz: [0.0, 0.5, 1.0]
    }

    assert {:ok, normalized} = FemModelNormalizer.normalize_harmonic_spring_1d(params)
    assert normalized["nodes"] == params.nodes
    assert normalized["elements"] == params.elements
    assert normalized[:frequencies_hz] == [0.0, 0.5, 1.0]
  end

  test "preserves corotational continuation controls" do
    params = %{
      "buckling" => %{
        "frame" => %{
          "nodes" => [%{"id" => "base"}, %{"id" => "top"}],
          "elements" => [%{"id" => "column", "node_i" => 0, "node_j" => 1}]
        },
        "mode_count" => 1
      },
      "imperfection_amplitude" => 0.002,
      "kinematics" => "corotational",
      "max_iterations" => 24,
      "tolerance" => 1.0e-9,
      "max_step_cutbacks" => 6
    }

    assert {:ok, normalized} = FemModelNormalizer.normalize_frame_2d_p_delta(params)
    assert normalized["kinematics"] == "corotational"
    assert normalized["max_iterations"] == 24
    assert normalized["tolerance"] == 1.0e-9
    assert normalized["max_step_cutbacks"] == 6
  end

  test "assigns stable node and element ids when a compact mesh omits them" do
    params = %{
      "nodes" => [%{"x" => 0.0, "y" => 0.0}, %{"id" => "hot", "x" => 1.0, "y" => 0.0}],
      "elements" => [%{"node_i" => 0, "node_j" => 1, "node_k" => 1, "node_l" => 0}]
    }

    assert {:ok, normalized} = FemModelNormalizer.normalize_heat_plane_quad_2d(params)
    assert Enum.map(normalized["nodes"], & &1["id"]) == ["n0", "hot"]
    assert Enum.map(normalized["elements"], & &1["id"]) == ["e0"]
  end

  test "passes thermal contact data without merging coincident temperature nodes" do
    nodes =
      for {x, y} <- [{0, 0}, {1, 0}, {1, 1}, {0, 1}, {1, 0}, {2, 0}, {2, 1}, {1, 1}] do
        %{
          "x" => x,
          "y" => y,
          "fix_temperature" => x in [0, 2],
          "temperature" => if(x == 0, do: 20.0, else: 0.0),
          "heat_load" => 0.0
        }
      end

    contacts = [
      %{
        "id" => "bond",
        "side_a" => [1, 2],
        "side_b" => [4, 7],
        "thermal_resistance_m2_k_w" => 1.0
      }
    ]

    quads =
      for offset <- [0, 4] do
        %{
          "node_i" => offset,
          "node_j" => offset + 1,
          "node_k" => offset + 2,
          "node_l" => offset + 3,
          "conductivity" => 2.0,
          "thickness" => 0.5
        }
      end

    triangles =
      for quad <- quads, side <- [[0, 1, 2], [0, 2, 3]] do
        [i, j, k] = Enum.map(side, &(quad["node_i"] + &1))
        %{"node_i" => i, "node_j" => j, "node_k" => k, "conductivity" => 2.0, "thickness" => 0.5}
      end

    for {normalize, elements} <- [
          {&FemModelNormalizer.normalize_heat_plane_quad_2d/1, quads},
          {&FemModelNormalizer.normalize_heat_plane_triangle_2d/1, triangles}
        ] do
      params = %{"nodes" => nodes, "elements" => elements, "contact_interfaces" => contacts}
      assert {:ok, normalized} = normalize.(params)
      assert normalized["contact_interfaces"] == contacts
      assert Enum.map(normalized["nodes"], & &1["id"]) == Enum.map(0..7, &"n#{&1}")
      assert Enum.map(normalized["nodes"], &Map.delete(&1, "id")) == nodes
    end
  end
end
