defmodule KyuubikiWeb.TestSupport.WorkflowApiFixtures do
  @moduledoc false

  def guarded_quad_frames(:blocked), do: [[quad_electrostatic_result(:blocked)]]

  def guarded_quad_frames(:continued) do
    [
      [quad_electrostatic_result(:continued)],
      [quad_heat_result()],
      [quad_thermo_result()]
    ]
  end

  def electrostatic_quad_summary_frames, do: [[quad_catalog_summary_result()]]

  def guarded_triangle_frames(:blocked), do: [[triangle_electrostatic_result(:blocked)]]

  def guarded_triangle_frames(:continued) do
    [
      [triangle_electrostatic_result(:continued)],
      [triangle_heat_result()],
      [triangle_thermo_result()]
    ]
  end

  def heat_to_thermo_quad_input_artifacts do
    %{
      "heat_model" => %{
        "nodes" => [
          %{
            "id" => "h0",
            "x" => 0.0,
            "y" => 0.0,
            "fix_temperature" => true,
            "temperature" => 100.0,
            "heat_load" => 0.0
          },
          %{
            "id" => "h1",
            "x" => 1.0,
            "y" => 0.0,
            "fix_temperature" => false,
            "temperature" => 0.0,
            "heat_load" => 0.0
          },
          %{
            "id" => "h2",
            "x" => 1.0,
            "y" => 1.0,
            "fix_temperature" => true,
            "temperature" => 20.0,
            "heat_load" => 0.0
          },
          %{
            "id" => "h3",
            "x" => 0.0,
            "y" => 1.0,
            "fix_temperature" => true,
            "temperature" => 20.0,
            "heat_load" => 0.0
          }
        ],
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
  end

  def electrostatic_plane_quad_input_artifacts do
    %{
      "electrostatic_model" => %{
        "nodes" => [
          %{
            "id" => "n0",
            "x" => 0.0,
            "y" => 0.0,
            "fix_potential" => true,
            "potential" => 10.0,
            "charge_density" => 0.0
          },
          %{
            "id" => "n1",
            "x" => 1.0,
            "y" => 0.0,
            "fix_potential" => true,
            "potential" => 0.0,
            "charge_density" => 0.0
          },
          %{
            "id" => "n2",
            "x" => 1.0,
            "y" => 1.0,
            "fix_potential" => true,
            "potential" => 0.0,
            "charge_density" => 0.0
          },
          %{
            "id" => "n3",
            "x" => 0.0,
            "y" => 1.0,
            "fix_potential" => true,
            "potential" => 10.0,
            "charge_density" => 0.0
          }
        ],
        "elements" => [
          %{
            "id" => "epq0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "node_l" => 3,
            "thickness" => 0.05,
            "permittivity" => 2.0
          }
        ]
      }
    }
  end

  def electrostatic_plane_triangle_input_artifacts do
    %{
      "electrostatic_plane_triangle_model" => %{
        "nodes" => [
          %{"id" => "e0", "x" => 0.0, "y" => 0.0, "fix_potential" => true, "potential" => 12.0},
          %{"id" => "e1", "x" => 1.0, "y" => 0.0, "fix_potential" => true, "potential" => 0.0},
          %{
            "id" => "e2",
            "x" => 0.0,
            "y" => 1.0,
            "fix_potential" => false,
            "charge_density" => 0.0
          }
        ],
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
  end

  defp quad_electrostatic_result(mode) do
    case mode do
      :blocked ->
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "y" => 0.0,
                "potential" => 10.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 1.0,
                "y" => 0.0,
                "potential" => 0.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 1.0,
                "y" => 1.0,
                "potential" => 0.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 3,
                "id" => "n3",
                "x" => 0.0,
                "y" => 1.0,
                "potential" => 10.0,
                "charge_density" => 0.0
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "epq0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "area" => 1.0,
                "average_potential" => 5.0,
                "potential_gradient_x" => -10.0,
                "potential_gradient_y" => 0.0,
                "electric_field_x" => 10.0,
                "electric_field_y" => 0.0,
                "electric_field_magnitude" => 10.0,
                "electric_flux_density_x" => 20.0,
                "electric_flux_density_y" => 0.0,
                "electric_flux_density_magnitude" => 20.0
              }
            ],
            "max_potential" => 10.0,
            "max_electric_field" => 10.0,
            "max_flux_density" => 20.0,
            "input" => %{
              "elements" => [
                %{
                  "id" => "epq0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "node_l" => 3,
                  "thickness" => 0.05,
                  "permittivity" => 2.0
                }
              ]
            }
          }
        }

      :continued ->
        %{
          "ok" => true,
          "result" => %{
            "nodes" => [
              %{
                "index" => 0,
                "id" => "n0",
                "x" => 0.0,
                "y" => 0.0,
                "potential" => 6.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 1,
                "id" => "n1",
                "x" => 1.0,
                "y" => 0.0,
                "potential" => 0.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 2,
                "id" => "n2",
                "x" => 1.0,
                "y" => 1.0,
                "potential" => 0.0,
                "charge_density" => 0.0
              },
              %{
                "index" => 3,
                "id" => "n3",
                "x" => 0.0,
                "y" => 1.0,
                "potential" => 6.0,
                "charge_density" => 0.0
              }
            ],
            "elements" => [
              %{
                "index" => 0,
                "id" => "epq0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "area" => 1.0,
                "average_potential" => 3.0,
                "potential_gradient_x" => -6.0,
                "potential_gradient_y" => 0.0,
                "electric_field_x" => 6.0,
                "electric_field_y" => 0.0,
                "electric_field_magnitude" => 6.0,
                "electric_flux_density_x" => 12.0,
                "electric_flux_density_y" => 0.0,
                "electric_flux_density_magnitude" => 12.0
              }
            ],
            "max_potential" => 6.0,
            "max_electric_field" => 6.0,
            "max_flux_density" => 12.0,
            "input" => %{
              "elements" => [
                %{
                  "id" => "epq0",
                  "node_i" => 0,
                  "node_j" => 1,
                  "node_k" => 2,
                  "node_l" => 3,
                  "thickness" => 0.05,
                  "permittivity" => 2.0
                }
              ]
            }
          }
        }
    end
  end

  defp quad_catalog_summary_result do
    %{
      "ok" => true,
      "result" => %{
        "nodes" => [
          %{
            "index" => 0,
            "id" => "n0",
            "x" => 0.0,
            "y" => 0.0,
            "potential" => 10.0,
            "charge_density" => 0.0
          },
          %{
            "index" => 1,
            "id" => "n1",
            "x" => 1.0,
            "y" => 0.0,
            "potential" => 0.0,
            "charge_density" => 0.0
          },
          %{
            "index" => 2,
            "id" => "n2",
            "x" => 1.0,
            "y" => 1.0,
            "potential" => 0.0,
            "charge_density" => 0.0
          },
          %{
            "index" => 3,
            "id" => "n3",
            "x" => 0.0,
            "y" => 1.0,
            "potential" => 10.0,
            "charge_density" => 0.0
          }
        ],
        "elements" => [
          %{
            "index" => 0,
            "id" => "epq0",
            "node_i" => 0,
            "node_j" => 1,
            "node_k" => 2,
            "node_l" => 3,
            "area" => 1.0,
            "average_potential" => 5.0,
            "potential_gradient_x" => -10.0,
            "potential_gradient_y" => 0.0,
            "electric_field_x" => 10.0,
            "electric_field_y" => 0.0,
            "electric_field_magnitude" => 10.0,
            "electric_flux_density_x" => 20.0,
            "electric_flux_density_y" => 0.0,
            "electric_flux_density_magnitude" => 20.0
          }
        ],
        "max_potential" => 10.0,
        "max_electric_field" => 10.0,
        "max_flux_density" => 20.0,
        "input" => %{"nodes" => [], "elements" => []}
      }
    }
  end

  defp quad_heat_result do
    %{
      "ok" => true,
      "result" => %{
        "max_temperature" => 70.0,
        "max_heat_flux" => 1500.0,
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "temperature" => 70.0, "heat_load" => 300.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "temperature" => 45.0, "heat_load" => 300.0},
          %{"id" => "h2", "x" => 1.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0},
          %{"id" => "h3", "x" => 0.0, "y" => 1.0, "temperature" => 20.0, "heat_load" => 300.0}
        ],
        "elements" => [
          %{"id" => "hq0", "temperature_gradient_x" => -25.0, "temperature_gradient_y" => -50.0}
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
  end

  defp quad_thermo_result do
    %{
      "ok" => true,
      "result" => %{
        "max_displacement" => 0.0015,
        "max_stress" => 18_000_000.0,
        "max_temperature_delta" => 70.0,
        "nodes" => [
          %{"id" => "h0", "temperature_delta" => 70.0},
          %{"id" => "h1", "temperature_delta" => 45.0},
          %{"id" => "h2", "temperature_delta" => 20.0},
          %{"id" => "h3", "temperature_delta" => 20.0}
        ],
        "elements" => [
          %{
            "id" => "tq0",
            "stress_x" => -18_000_000.0,
            "stress_y" => -18_000_000.0,
            "mechanical_strain_x" => -1.8e-4,
            "mechanical_strain_y" => -1.8e-4
          }
        ]
      }
    }
  end

  defp triangle_electrostatic_result(mode) do
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
    end
  end

  defp triangle_heat_result do
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

  defp triangle_thermo_result do
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
end
