defmodule KyuubikiWeb.WorkflowOperatorBridgeRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "bridges electrostatic triangle results with area weighted mean reduction" do
    electrostatic_result = %{
      "nodes" => [
        %{"id" => "e0", "x" => 0.0, "y" => 0.0},
        %{"id" => "e1", "x" => 1.0, "y" => 0.0},
        %{"id" => "e2", "x" => 0.0, "y" => 1.0},
        %{"id" => "e3", "x" => 1.0, "y" => 1.0}
      ],
      "elements" => [
        %{
          "id" => "et0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "area" => 1.0,
          "electric_field_magnitude" => 2.0
        },
        %{
          "id" => "et1",
          "node_i" => 1,
          "node_j" => 3,
          "node_k" => 2,
          "area" => 3.0,
          "electric_field_magnitude" => 10.0
        }
      ]
    }

    config = %{
      "seed_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h2", "x" => 0.0, "y" => 1.0, "heat_load" => 0.0},
          %{"id" => "h3", "x" => 1.0, "y" => 1.0, "heat_load" => 0.0}
        ],
        "elements" => [
          %{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2},
          %{"id" => "ht1", "node_i" => 1, "node_j" => 3, "node_k" => 2}
        ]
      },
      "contract" => %{
        "source" => %{
          "field" => "electric_field_magnitude",
          "distribution" => "element_to_nodes",
          "node_index_fields" => ["node_i", "node_j", "node_k"]
        },
        "transform" => %{
          "scale" => 1.0,
          "reduction" => "area_weighted_mean",
          "default_value" => 0.0
        },
        "target" => %{"field" => "heat_load"}
      }
    }

    assert {:ok, %{"nodes" => nodes}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.electrostatic_field_to_heat_triangle_2d",
               electrostatic_result,
               config
             )

    assert Enum.map(nodes, & &1["heat_load"]) == [2.0, 8.0, 8.0, 10.0]
  end

  test "bridges electrostatic nodal potential into heat node temperature" do
    electrostatic_result = %{
      "nodes" => [
        %{"id" => "e0", "x" => 0.0, "y" => 0.0, "potential" => 12.0},
        %{"id" => "e1", "x" => 1.0, "y" => 0.0, "potential" => 8.0}
      ],
      "elements" => [%{"id" => "et0", "node_i" => 0, "node_j" => 1}]
    }

    config = %{
      "seed_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 0.0, "heat_load" => 0.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 0.0, "heat_load" => 0.0}
        ],
        "elements" => [%{"id" => "ht0", "node_i" => 0, "node_j" => 1}]
      },
      "contract" => %{
        "source" => %{"field" => "potential", "distribution" => "node_to_node"},
        "transform" => %{"scale" => 2.0, "default_value" => 0.0},
        "target" => %{"field" => "temperature"}
      }
    }

    assert {:ok, %{"nodes" => nodes}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.electrostatic_field_to_heat_quad_2d",
               electrostatic_result,
               config
             )

    assert Enum.map(nodes, & &1["temperature"]) == [24.0, 16.0]
  end

  test "rejects invalid bridge target fields across coupled runtimes" do
    assert {:error, :invalid_bridge_contract_target_field} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.electrostatic_field_to_heat_triangle_2d",
               %{"nodes" => [], "elements" => []},
               %{
                 "seed_model" => %{"nodes" => [], "elements" => []},
                 "contract" => %{"target" => %{"field" => "temperature_delta"}}
               }
             )

    assert {:error, :invalid_bridge_contract_target_field} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.temperature_field_to_thermo_triangle_2d",
               %{"nodes" => [], "input" => %{"elements" => []}, "elements" => []},
               %{
                 "seed_model" => %{"nodes" => [], "elements" => []},
                 "contract" => %{"target" => %{"field" => "temperature"}}
               }
             )
  end
end
