defmodule KyuubikiWeb.TestSupport.WorkflowApiTriangleFixtures do
  @moduledoc false

  def electrostatic_result(mode) do
    case mode do
      :blocked ->
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "e0", "x" => 0.0, "y" => 0.0, "potential" => 12.0},
              %{"index" => 1, "id" => "e1", "x" => 1.0, "y" => 0.0, "potential" => 0.0},
              %{"index" => 2, "id" => "e2", "x" => 0.0, "y" => 1.0, "potential" => 6.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "et0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "area" => 0.5,
                "electric_field_x" => 6.0,
                "electric_field_y" => 8.0,
                "electric_field_magnitude" => 10.0,
                "electric_flux_density_x" => 12.0,
                "electric_flux_density_y" => 16.0,
                "electric_flux_density_magnitude" => 20.0
              }
            ],
            "max_potential" => 12.0,
            "max_electric_field" => 10.0,
            "max_flux_density" => 20.0,
            "input" => input()
          }
        }

      :continued ->
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{"index" => 0, "id" => "e0", "x" => 0.0, "y" => 0.0, "potential" => 6.0},
              %{"index" => 1, "id" => "e1", "x" => 1.0, "y" => 0.0, "potential" => 0.0},
              %{"index" => 2, "id" => "e2", "x" => 0.0, "y" => 1.0, "potential" => 3.0}
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "et0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "area" => 0.5,
                "electric_field_x" => 4.8,
                "electric_field_y" => 3.6,
                "electric_field_magnitude" => 6.0,
                "electric_flux_density_x" => 9.6,
                "electric_flux_density_y" => 7.2,
                "electric_flux_density_magnitude" => 12.0
              }
            ],
            "max_potential" => 6.0,
            "max_electric_field" => 6.0,
            "max_flux_density" => 12.0,
            "input" => input()
          }
        }
    end
  end

  def heat_result do
    %{
      "ok" => true,
      "result" => %{
        "max_temperature" => 75.0,
        "max_heat_flux" => 900.0,
        "nodes" => [
          %{"id" => "t0", "x" => 0.0, "y" => 0.0, "temperature" => 75.0, "heat_load" => 300.0},
          %{"id" => "t1", "x" => 1.0, "y" => 0.0, "temperature" => 55.0, "heat_load" => 300.0},
          %{"id" => "t2", "x" => 0.0, "y" => 1.0, "temperature" => 35.0, "heat_load" => 300.0}
        ],
        "elements" => [
          %{
            "id" => "ht0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "temperature_gradient_x" => 20.0,
            "temperature_gradient_y" => -40.0,
            "heat_flux_x" => -450.0,
            "heat_flux_y" => 900.0
          }
        ],
        "input" => %{
          "elements" => [
            %{
              "id" => "ht0",
              "node_i" => 0,
              "node_j" => 1,
              "node_k" => 2,
              "thickness" => 0.02,
              "conductivity" => 45.0
            }
          ]
        }
      }
    }
  end

  def thermo_result do
    %{
      "ok" => true,
      "result" => %{
        "max_displacement" => 0.0025,
        "max_stress" => 22_500_000.0,
        "max_temperature_delta" => 75.0,
        "nodes" => [
          %{"id" => "t0", "temperature_delta" => 75.0},
          %{"id" => "t1", "temperature_delta" => 55.0},
          %{"id" => "t2", "temperature_delta" => 35.0}
        ],
        "elements" => [
          %{
            "id" => "tt0",
            "stress_x" => -22_500_000.0,
            "stress_y" => -18_000_000.0,
            "mechanical_strain_x" => -2.2e-4,
            "mechanical_strain_y" => -1.8e-4
          }
        ]
      }
    }
  end

  defp input do
    %{
      "elements" => [
        %{
          "id" => "et0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "thickness" => 0.05,
          "permittivity" => 2.0
        }
      ]
    }
  end
end
