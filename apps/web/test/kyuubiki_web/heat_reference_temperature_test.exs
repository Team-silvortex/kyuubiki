defmodule KyuubikiWeb.HeatReferenceTemperatureTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorHeatBridgeRuntime, as: Bridge

  test "temperature origin and scale preserve the same expansion for both shapes" do
    for shape <- [:quad, :triangle],
        reference <- [20.0, 293.15],
        rise <- [0.0, 20.0],
        distribution <- ["node_to_node", "element_to_nodes"] do
      nodes = [%{"id" => "n", "x" => 0.0, "y" => 0.0, "temperature" => reference + rise}]

      heat = %{
        "nodes" => nodes,
        "input" => %{"elements" => [%{}]},
        "elements" => [%{"node_i" => 0, "area" => 1.0, "average_temperature" => reference + rise}]
      }

      seed = %{"nodes" => nodes, "elements" => [%{}]}
      field = if distribution == "node_to_node", do: "temperature", else: "average_temperature"

      assert {:ok, contract} =
               Bridge.resolve_heat_to_thermo_bridge_contract(%{
                 "contract" => %{
                   "source" => %{
                     "field" => field,
                     "distribution" => distribution,
                     "node_index_fields" => ["node_i"]
                   },
                   "transform" => %{"reference_temperature" => reference, "scale" => 2.0}
                 }
               })

      method =
        if shape == :quad,
          do: :bridge_heat_result_to_thermal_plane_quad_model,
          else: :bridge_heat_result_to_thermal_plane_triangle_model

      assert {:ok, %{"nodes" => [node]}} = apply(Bridge, method, [heat, seed, contract])
      assert_in_delta node["temperature_delta"], 2.0 * rise, 1.0e-10
    end
  end

  test "malformed references and nontemperature sources do not silently discard configuration" do
    for value <- [nil, "20", true] do
      assert {:error, :invalid_bridge_reference_temperature} =
               Bridge.resolve_heat_to_thermo_bridge_contract(%{
                 "contract" => %{"transform" => %{"reference_temperature" => value}}
               })
    end

    assert {:error, :invalid_bridge_reference_temperature_source} =
             Bridge.resolve_heat_to_thermo_bridge_contract(%{
               "contract" => %{
                 "source" => %{"field" => "heat_load"},
                 "transform" => %{"reference_temperature" => 20.0}
               }
             })

    assert {:ok, contract} = Bridge.resolve_heat_to_thermo_bridge_contract(%{})
    assert contract.reference_temperature == 0.0

    assert {:ok, contract} =
             Bridge.resolve_heat_to_thermo_bridge_contract(%{"contract" => %{"transform" => nil}})

    assert contract.reference_temperature == 0.0
  end
end
