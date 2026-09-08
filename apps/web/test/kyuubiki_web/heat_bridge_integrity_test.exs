defmodule KyuubikiWeb.HeatBridgeIntegrityTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorHeatBridgeRuntime, as: Bridge

  defp fixture(shape) do
    {coordinates, cells} =
      if shape == :triangle,
        do: {[[0, 0], [1, 0], [1, 1], [4, 0]], [[0, 1, 2], [1, 3, 2]]},
        else: {[[0, 0], [1, 0], [4, 0], [0, 1], [1, 1], [4, 1]], [[0, 1, 4, 3], [1, 2, 5, 4]]}

    nodes =
      Enum.with_index(coordinates, fn [x, y], i ->
        %{"id" => "n#{i}", "x" => x, "y" => y, "temperature" => 30.0}
      end)

    elements =
      Enum.with_index(cells, fn indexes, i ->
        Enum.zip(["node_i", "node_j", "node_k", "node_l"], indexes)
        |> Map.new()
        |> Map.merge(%{
          "id" => "e#{i}",
          "average_temperature" => 30.0 + 20.0 * i,
          "area" => 1.0 + 2.0 * i
        })
      end)

    heat = %{"nodes" => nodes, "elements" => elements, "input" => %{"elements" => elements}}
    seed = %{"nodes" => nodes, "elements" => elements}
    {heat, seed}
  end

  defp run(shape, heat, seed, reduction \\ "mean") do
    with {:ok, contract} <-
           Bridge.resolve_heat_to_thermo_bridge_contract(%{
             "contract" => %{
               "source" => %{
                 "field" => "average_temperature",
                 "distribution" => "element_to_nodes"
               },
               "transform" => %{
                 "reference_temperature" => 20.0,
                 "scale" => 2.0,
                 "reduction" => reduction
               }
             }
           }) do
      method =
        if shape == :triangle,
          do: :bridge_heat_result_to_thermal_plane_triangle_model,
          else: :bridge_heat_result_to_thermal_plane_quad_model

      apply(Bridge, method, [heat, seed, contract])
    end
  end

  test "both shapes preserve unequal-area reduction with default indexes" do
    for shape <- [:triangle, :quad],
        {reduction, expected} <- [
          {"copy", 40.0},
          {"mean", 40.0},
          {"sum", 80.0},
          {"area_weighted_mean", 50.0},
          {"min", 20.0},
          {"max", 60.0}
        ] do
      {heat, seed} = fixture(shape)
      assert {:ok, result} = run(shape, heat, seed, reduction)
      assert_in_delta Enum.at(result["nodes"], 1)["temperature_delta"], expected, 1.0e-10
    end
  end

  test "truncated arrays and malformed indexes never default missing physical evidence" do
    for shape <- [:triangle, :quad] do
      {heat, seed} = fixture(shape)

      for index <- [-1, 999, "1", nil] do
        [first, second] = heat["elements"]
        bad = Map.put(heat, "elements", [Map.put(first, "node_j", index), second])
        assert {:error, _} = run(shape, bad, seed)
      end

      assert {:error, _} = run(shape, Map.put(heat, "nodes", tl(heat["nodes"])), seed)
      assert {:error, _} = run(shape, Map.put(heat, "elements", tl(heat["elements"])), seed)
      [first, second] = heat["elements"]

      assert {:error, _} =
               run(shape, Map.put(heat, "elements", [Map.put(first, "node_j", 0), second]), seed)
    end
  end

  test "invalid numeric fields are not coerced to zero" do
    {heat, seed} = fixture(:quad)

    for field <- ["average_temperature", "area"], bad <- [nil, "30", false] do
      [first, second] = heat["elements"]

      assert {:error, _} =
               run(:quad, Map.put(heat, "elements", [Map.put(first, field, bad), second]), seed)
    end
  end

  test "malformed contract sections and values return errors instead of raising or defaulting" do
    for section <- ["source", "transform", "target"], value <- [false, 1, [], "bad"] do
      assert {:error, _} =
               Bridge.resolve_heat_to_thermo_bridge_contract(%{"contract" => %{section => value}})
    end

    for field <- ["scale", "default_value", "reference_temperature"],
        value <- [nil, false, "2"] do
      assert {:error, _} =
               Bridge.resolve_heat_to_thermo_bridge_contract(%{
                 "contract" => %{"transform" => %{field => value}}
               })
    end

    for fields <- [[], ["node_i", 1], ["node_i", "node_i"]] do
      assert {:error, _} =
               Bridge.resolve_heat_to_thermo_bridge_contract(%{
                 "contract" => %{"source" => %{"node_index_fields" => fields}}
               })
    end
  end

  test "numeric overflow fails locally and the next request still works" do
    {heat, seed} = fixture(:quad)

    bad =
      Map.update!(heat, "elements", fn elements ->
        Enum.map(elements, &Map.put(&1, "average_temperature", 1.0e308))
      end)

    for reduction <- ["sum", "mean", "area_weighted_mean"] do
      assert {:error, _} = run(:quad, bad, seed, reduction)
    end

    assert {:ok, _} = run(:quad, heat, seed)
  end
end
