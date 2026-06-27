defmodule KyuubikiWeb.WorkflowOperatorRoadmapRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "magnetostatic quad bridge maps field values into heat loads" do
    assert {:ok, bridged} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.magnetostatic_field_to_heat_quad_2d",
               magnetostatic_result(),
               %{
                 "seed_model" => heat_seed_model(),
                 "contract" => %{
                   "source" => %{"field" => "magnetic_flux_density_magnitude"},
                   "transform" => %{"scale" => 2.0, "reduction" => "mean"},
                   "target" => %{"field" => "heat_load"}
                 }
               }
             )

    assert Enum.all?(bridged["nodes"], &(Map.fetch!(&1, "heat_load") == 6.0))
  end

  defp magnetostatic_result do
    %{
      "nodes" => [
        %{"id" => "n0", "x" => 0.0, "y" => 0.0, "vector_potential" => 0.0},
        %{"id" => "n1", "x" => 1.0, "y" => 0.0, "vector_potential" => 0.0},
        %{"id" => "n2", "x" => 1.0, "y" => 1.0, "vector_potential" => 1.0},
        %{"id" => "n3", "x" => 0.0, "y" => 1.0, "vector_potential" => 1.0}
      ],
      "elements" => [
        %{
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "area" => 1.0,
          "magnetic_flux_density_magnitude" => 3.0
        }
      ]
    }
  end

  defp heat_seed_model do
    %{
      "nodes" => [
        %{
          "id" => "n0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n2",
          "x" => 1.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n3",
          "x" => 0.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "h0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "thickness" => 0.1,
          "conductivity" => 1.0
        }
      ]
    }
  end
end
