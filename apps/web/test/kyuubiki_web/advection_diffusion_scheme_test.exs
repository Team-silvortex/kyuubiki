defmodule KyuubikiWeb.AdvectionDiffusionSchemeTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.FemModelNormalizer

  @cases Path.expand("../../../../schemas/examples.advection-diffusion-scheme.json", __DIR__)
         |> File.read!()
         |> Jason.decode!()

  test "shared JSON scheme fixture agrees with typed Rust input" do
    assert length(@cases["accepted"]) == 3
    assert length(@cases["rejected"]) == 7

    for case_data <- @cases["accepted"] do
      model =
        if Map.has_key?(case_data, "scheme"),
          do: Map.put(@cases["model"], "scheme", case_data["scheme"]),
          else: @cases["model"]

      assert {:ok, normalized} = FemModelNormalizer.normalize_advection_diffusion_bar_1d(model)
      assert Map.get(normalized, "scheme", "galerkin") == case_data["expected"]

      assert {:ok, ^normalized} =
               FemModelNormalizer.normalize_advection_diffusion_bar_1d(normalized)
    end

    for case_data <- @cases["rejected"] do
      assert {:error, :invalid_advection_diffusion_bar_model} =
               FemModelNormalizer.normalize_advection_diffusion_bar_1d(
                 Map.put(@cases["model"], "scheme", case_data["scheme"])
               )
    end
  end

  test "internal atom keys normalize without ambiguous scheme selection" do
    model = Map.put(@cases["model"], :scheme, "upwind")
    assert {:ok, normalized} = FemModelNormalizer.normalize_advection_diffusion_bar_1d(model)
    assert normalized["scheme"] == "upwind"
    refute Map.has_key?(normalized, :scheme)

    assert {:ok, ^normalized} =
             FemModelNormalizer.normalize_advection_diffusion_bar_1d(
               Map.put(model, "scheme", "upwind")
             )

    assert {:error, :invalid_advection_diffusion_bar_model} =
             FemModelNormalizer.normalize_advection_diffusion_bar_1d(
               Map.put(model, "scheme", "galerkin")
             )

    assert {:error, :invalid_advection_diffusion_bar_model} =
             FemModelNormalizer.normalize_advection_diffusion_bar_1d(
               Map.put(model, :scheme, :upwind)
             )
  end
end
