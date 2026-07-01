defmodule KyuubikiWeb.TestSupport.WorkflowCatalogBenchmarkFixtures do
  @moduledoc false

  alias KyuubikiWeb.TestSupport.WorkflowApi
  alias KyuubikiWeb.TestSupport.WorkflowApiFixtures

  def heat_thermo_benchmark_frames do
    [
      [
        %{
          "ok" => true,
          "result" => %{
            "max_temperature" => 70.0,
            "max_heat_flux" => 3000.0,
            "nodes" => [
              %{
                "id" => "h0",
                "x" => 0.0,
                "y" => 0.0,
                "temperature" => 70.0,
                "heat_load" => 300.0
              },
              %{
                "id" => "h1",
                "x" => 1.0,
                "y" => 0.0,
                "temperature" => 45.0,
                "heat_load" => 300.0
              },
              %{
                "id" => "h2",
                "x" => 1.0,
                "y" => 1.0,
                "temperature" => 20.0,
                "heat_load" => 300.0
              },
              %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0}
            ],
            "elements" => [
              %{
                "id" => "hq0",
                "temperature_gradient_x" => -25.0,
                "temperature_gradient_y" => -50.0,
                "heat_flux_x" => -1800.0,
                "heat_flux_y" => 2400.0
              }
            ],
            "input" => %{
              "elements" => [
                %{
                  "id" => "hq0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "node_l" => 3,
                  "thickness" => 0.02,
                  "conductivity" => 45.0
                }
              ]
            }
          }
        }
      ],
      [
        %{
          "ok" => true,
          "result" => %{
            "max_displacement" => 0.0015,
            "max_stress" => 1200.0,
            "max_temperature_delta" => 70.0,
            "nodes" => [
              %{
                "id" => "h0",
                "temperature_delta" => 70.0,
                "displacement_x" => 0.0,
                "displacement_y" => 0.0
              },
              %{
                "id" => "h1",
                "temperature_delta" => 45.0,
                "displacement_x" => 0.001,
                "displacement_y" => 0.0
              },
              %{
                "id" => "h2",
                "temperature_delta" => 20.0,
                "displacement_x" => 0.0015,
                "displacement_y" => 0.0
              },
              %{
                "id" => "h3",
                "temperature_delta" => 20.0,
                "displacement_x" => 0.0012,
                "displacement_y" => 0.0
              }
            ],
            "elements" => [
              %{
                "id" => "tq0",
                "von_mises_stress" => 1200.0,
                "stress_x" => -1200.0,
                "stress_y" => -1200.0
              }
            ]
          }
        }
      ]
    ]
  end

  def electrostatic_quad_triangle_compare_input_artifacts do
    WorkflowApi.electrostatic_plane_quad_input_artifacts()
    |> Map.merge(WorkflowApi.electrostatic_plane_triangle_input_artifacts())
  end

  def electrostatic_quad_triangle_compare_frames do
    WorkflowApiFixtures.electrostatic_quad_summary_frames() ++
      [List.first(electrostatic_heat_thermo_triangle_frames())]
  end

  def electrostatic_heat_thermo_triangle_frames do
    [
      [
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
            "input" => %{
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
          }
        }
      ],
      [
        %{
          "ok" => true,
          "result" => %{
            "max_temperature" => 75.0,
            "max_heat_flux" => 900.0,
            "nodes" => [
              %{
                "id" => "t0",
                "x" => 0.0,
                "y" => 0.0,
                "temperature" => 75.0,
                "heat_load" => 500.0
              },
              %{
                "id" => "t1",
                "x" => 1.0,
                "y" => 0.0,
                "temperature" => 55.0,
                "heat_load" => 500.0
              },
              %{"id" => "t2", "x" => 0.0, "y" => 1.0, "temperature" => 35.0, "heat_load" => 500.0}
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
      ],
      [
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
      ]
    ]
  end
end
