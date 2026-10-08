defmodule KyuubikiWeb.AxialBarInputTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.FemModelNormalizer

  @fixture Path.expand("../../../../schemas/examples.axial-bar-input-normalization.json", __DIR__)
  @cases @fixture |> File.read!() |> Jason.decode!()

  test "shared accepted inputs normalize to canonical SI" do
    assert length(@cases["accepted"]) == 10

    for case_data <- @cases["accepted"] do
      input = Map.merge(@cases["base_input"], case_data["patch"])
      expected = Map.merge(@cases["base_expected"], case_data["expected_patch"])
      assert {:ok, normalized} = FemModelNormalizer.normalize_axial_bar(input), case_data["id"]
      assert normalized == expected, case_data["id"]
    end
  end

  test "shared invalid inputs never choose units or round counts silently" do
    assert length(@cases["rejected"]) == 26

    for case_data <- @cases["rejected"] do
      input = Map.merge(@cases["base_input"], case_data["patch"])

      assert {:error, :invalid_axial_bar_model} = FemModelNormalizer.normalize_axial_bar(input),
             case_data["id"]
    end
  end

  test "missing required fields and explicit null fail without exceptions" do
    input = Map.put(@cases["base_input"], "youngs_modulus_gpa", 210)

    for key <- ["length", "area", "elements", "tip_force"] do
      assert {:error, _} = FemModelNormalizer.normalize_axial_bar(Map.delete(input, key))
    end

    for invalid <- [nil, [], false, "model"] do
      assert {:error, _} = FemModelNormalizer.normalize_axial_bar(invalid)
    end

    assert {:error, _} =
             FemModelNormalizer.normalize_axial_bar(Map.put(input, "area", Integer.pow(10, 400)))
  end

  test "internal atom keys follow the same public contract" do
    assert {:ok, normalized} =
             FemModelNormalizer.normalize_axial_bar(%{
               length: "1",
               area: "0.01",
               elements: "4",
               tip_force: "1000",
               youngs_modulus_gpa: "210"
             })

    assert normalized == @cases["base_expected"]
  end
end
