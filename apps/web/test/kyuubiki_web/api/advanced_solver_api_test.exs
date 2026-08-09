defmodule KyuubikiWeb.Api.AdvancedSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.FemModelNormalizer

  @solid_tetra_request %{
    "nodes" => [
      %{"id" => "n0", "x" => 0.0, "y" => 0.0, "z" => 0.0},
      %{"id" => "n1", "x" => 1.0, "y" => 0.0, "z" => 0.0},
      %{"id" => "n2", "x" => 0.0, "y" => 1.0, "z" => 0.0},
      %{"id" => "n3", "x" => 0.0, "y" => 0.0, "z" => 1.0}
    ],
    "elements" => [
      %{
        "id" => "t0",
        "node_a" => 0,
        "node_b" => 1,
        "node_c" => 2,
        "node_d" => 3,
        "youngs_modulus" => 70.0e9,
        "poisson_ratio" => 0.33
      }
    ]
  }

  @transient_heat_request %{
    "nodes" => [
      %{"id" => "hot", "x" => 0.0, "fix_temperature" => true, "temperature" => 100.0},
      %{"id" => "mid", "x" => 0.5, "temperature" => 20.0},
      %{"id" => "cold", "x" => 1.0, "fix_temperature" => true, "temperature" => 0.0}
    ],
    "elements" => [
      %{
        "id" => "h0",
        "node_i" => 0,
        "node_j" => 1,
        "area" => 0.01,
        "conductivity" => 45.0,
        "density" => 7800.0,
        "specific_heat" => 500.0
      },
      %{
        "id" => "h1",
        "node_i" => 1,
        "node_j" => 2,
        "area" => 0.01,
        "conductivity" => 45.0,
        "density" => 7800.0,
        "specific_heat" => 500.0
      }
    ],
    "time_step" => 0.1,
    "steps" => 4
  }

  @transient_spring_request %{
    "nodes" => [
      %{"id" => "fixed", "x" => 0.0, "fix_x" => true, "mass" => 1.0},
      %{"id" => "tip", "x" => 1.0, "load_x" => 10.0, "mass" => 2.0}
    ],
    "elements" => [
      %{"id" => "s0", "node_i" => 0, "node_j" => 1, "stiffness" => 100.0, "damping" => 0.5}
    ],
    "time_step" => 0.01,
    "steps" => 10
  }

  @harmonic_spring_request %{
    "nodes" => [
      %{"id" => "fixed", "x" => 0.0, "fix_x" => true, "mass" => 1.0},
      %{"id" => "tip", "x" => 1.0, "load_x" => 10.0, "mass" => 2.0}
    ],
    "elements" => [
      %{"id" => "s0", "node_i" => 0, "node_j" => 1, "stiffness" => 100.0, "damping" => 1.0}
    ],
    "frequencies_hz" => [0.0, 0.5, 1.0]
  }

  @buckling_frame_request %{
    "frame" => %{
      "nodes" => [
        %{
          "id" => "base",
          "x" => 0.0,
          "y" => 0.0,
          "fix_x" => true,
          "fix_y" => true,
          "fix_rz" => false,
          "load_x" => 0.0,
          "load_y" => 0.0,
          "moment_z" => 0.0
        },
        %{
          "id" => "top",
          "x" => 0.0,
          "y" => 2.0,
          "fix_x" => true,
          "fix_y" => false,
          "fix_rz" => false,
          "load_x" => 0.0,
          "load_y" => -100_000.0,
          "moment_z" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "column",
          "node_i" => 0,
          "node_j" => 1,
          "area" => 0.01,
          "youngs_modulus" => 210.0e9,
          "moment_of_inertia" => 8.0e-6,
          "section_modulus" => 1.0e-4
        }
      ]
    },
    "mode_count" => 1
  }

  @p_delta_request %{
    "buckling" => @buckling_frame_request,
    "imperfection_amplitude" => 0.002,
    "imperfection_shape" => [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    "load_steps" => 4
  }

  @material_p_delta_request %{
    "stability" =>
      Map.merge(@p_delta_request, %{
        "kinematics" => "corotational",
        "maximum_load_factor" => 1.0
      }),
    "materials" => [
      %{"element_id" => "column", "yield_strength" => 250.0e6, "hardening_ratio" => 0.05}
    ]
  }

  @cohesive_interface_request %{
    "id" => "interface-0",
    "initial_stiffness" => 1000.0,
    "compression_stiffness" => 2000.0,
    "peak_traction" => 10.0,
    "failure_separation" => 0.05,
    "separation_history" => [0.0, 0.01, 0.03, 0.015]
  }

  @cohesive_interface_2d_request %{
    "nodes" => [
      %{"id" => "lower-i", "x" => 0.0, "y" => 0.0},
      %{"id" => "lower-j", "x" => 1.0, "y" => 0.0},
      %{"id" => "upper-i", "x" => 0.0, "y" => 0.0},
      %{"id" => "upper-j", "x" => 1.0, "y" => 0.0}
    ],
    "element" => %{
      "id" => "interface-0",
      "lower_i" => 0,
      "lower_j" => 1,
      "upper_i" => 2,
      "upper_j" => 3,
      "thickness" => 1.0
    },
    "material" => %{
      "normal_initial_stiffness" => 1000.0,
      "normal_compression_stiffness" => 2000.0,
      "normal_peak_traction" => 10.0,
      "normal_failure_separation" => 0.05,
      "shear_initial_stiffness" => 500.0,
      "shear_peak_traction" => 5.0,
      "shear_failure_separation" => 0.05
    },
    "displacement_history" => [
      %{
        "nodal_displacements" => [
          [0.0, 0.0],
          [0.0, 0.0],
          [0.03, 0.03],
          [0.03, 0.03]
        ]
      }
    ]
  }

  @cohesive_interface_mesh_2d_request %{
    "id" => "mesh.web",
    "nodes" => [
      %{"id" => "lower-i", "x" => 0.0, "y" => 0.0, "fixed" => [true, true], "load" => [0.0, 0.0]},
      %{"id" => "lower-j", "x" => 1.0, "y" => 0.0, "fixed" => [true, true], "load" => [0.0, 0.0]},
      %{
        "id" => "upper-i",
        "x" => 0.0,
        "y" => 0.0,
        "fixed" => [true, false],
        "load" => [0.0, 2.5]
      },
      %{"id" => "upper-j", "x" => 1.0, "y" => 0.0, "fixed" => [true, false], "load" => [0.0, 2.5]}
    ],
    "materials" => [
      %{"id" => "adhesive", "properties" => @cohesive_interface_2d_request["material"]}
    ],
    "elements" => [
      %{
        "id" => "interface-0",
        "lower_i" => 0,
        "lower_j" => 1,
        "upper_i" => 2,
        "upper_j" => 3,
        "thickness" => 1.0,
        "material_id" => "adhesive"
      }
    ],
    "load_steps" => 2,
    "max_iterations" => 12,
    "tolerance" => 1.0e-11
  }

  @cohesive_interface_mesh_3d_request %{
    "id" => "mesh.web.3d",
    "nodes" => [
      %{
        "id" => "lower-a",
        "x" => 0.0,
        "y" => 0.0,
        "z" => 0.0,
        "fixed" => [true, true, true],
        "load" => [0.0, 0.0, 0.0]
      },
      %{
        "id" => "lower-b",
        "x" => 1.0,
        "y" => 0.0,
        "z" => 0.0,
        "fixed" => [true, true, true],
        "load" => [0.0, 0.0, 0.0]
      },
      %{
        "id" => "lower-c",
        "x" => 0.0,
        "y" => 1.0,
        "z" => 0.0,
        "fixed" => [true, true, true],
        "load" => [0.0, 0.0, 0.0]
      },
      %{
        "id" => "upper-a",
        "x" => 0.0,
        "y" => 0.0,
        "z" => 0.0,
        "fixed" => [true, true, false],
        "load" => [0.0, 0.0, 0.8333333333333334]
      },
      %{
        "id" => "upper-b",
        "x" => 1.0,
        "y" => 0.0,
        "z" => 0.0,
        "fixed" => [true, true, false],
        "load" => [0.0, 0.0, 0.8333333333333334]
      },
      %{
        "id" => "upper-c",
        "x" => 0.0,
        "y" => 1.0,
        "z" => 0.0,
        "fixed" => [true, true, false],
        "load" => [0.0, 0.0, 0.8333333333333334]
      }
    ],
    "materials" => [
      %{
        "id" => "adhesive",
        "properties" => %{
          "normal_initial_stiffness" => 1000.0,
          "normal_compression_stiffness" => 2000.0,
          "normal_peak_traction" => 100.0,
          "normal_failure_separation" => 1.0,
          "shear_initial_stiffness" => 500.0,
          "shear_peak_traction" => 50.0,
          "shear_failure_separation" => 1.0
        }
      }
    ],
    "elements" => [
      %{
        "id" => "interface-0",
        "lower_a" => 0,
        "lower_b" => 1,
        "lower_c" => 2,
        "upper_a" => 3,
        "upper_b" => 4,
        "upper_c" => 5,
        "material_id" => "adhesive"
      }
    ],
    "load_steps" => 1,
    "tolerance" => 1.0e-11
  }

  @cases [
    {"/api/v1/fem/acoustic-bar-1d/jobs", "max_sound_pressure_level_db", %{}},
    {"/api/v1/fem/stokes-flow-plane-quad-2d/jobs", "max_velocity", %{}},
    {"/api/v1/fem/stokes-flow-plane-triangle-2d/jobs", "max_velocity", %{}},
    {"/api/v1/fem/nonlinear-spring-1d/jobs", "converged", %{}},
    {"/api/v1/fem/contact-gap-1d/jobs", "active_contact_count", %{"contacts" => []}},
    {"/api/v1/fem/cohesive-interface-1d/jobs", "max_damage", @cohesive_interface_request},
    {"/api/v1/fem/cohesive-interface-2d/jobs", "max_normal_damage",
     @cohesive_interface_2d_request},
    {"/api/v1/fem/cohesive-interface-mesh-2d/jobs", "converged",
     @cohesive_interface_mesh_2d_request},
    {"/api/v1/fem/cohesive-interface-mesh-3d/jobs", "converged",
     @cohesive_interface_mesh_3d_request},
    {"/api/v1/fem/modal-frame-2d/jobs", "natural_frequencies_hz", %{}},
    {"/api/v1/fem/buckling-beam-1d/jobs", "minimum_load_factor", %{}},
    {"/api/v1/fem/buckling-frame-2d/jobs", "minimum_load_factor", @buckling_frame_request},
    {"/api/v1/fem/frame-2d-p-delta/jobs", "max_imperfection_amplification", @p_delta_request},
    {"/api/v1/fem/frame-2d-material-p-delta/jobs", "yielded_element_count",
     @material_p_delta_request},
    {"/api/v1/fem/modal-frame-3d/jobs", "natural_frequencies_hz", %{}},
    {"/api/v1/fem/solid-tetra-3d/jobs", "max_von_mises_stress", @solid_tetra_request},
    {"/api/v1/fem/transient-heat-bar-1d/jobs", "final_time", @transient_heat_request},
    {"/api/v1/fem/transient-spring-1d/jobs", "max_velocity", @transient_spring_request},
    {"/api/v1/fem/harmonic-spring-1d/jobs", "peak_frequency_hz", @harmonic_spring_request}
  ]

  for {path, expected_key, extra_request} <- @cases do
    @path path
    @expected_key expected_key
    @extra_request extra_request

    test "submits #{path} through the orchestration API" do
      result = %{
        @expected_key => expected_value(@expected_key),
        "nodes" => [],
        "elements" => [],
        "input" => %{"nodes" => [], "elements" => []}
      }

      {:ok, _pid} =
        FakePlaygroundAgent.start_link([
          solver_progress("solving advanced model"),
          %{"ok" => true, "result" => result}
        ])

      configure_solver_agent()

      conn =
        :post
        |> conn(@path, Jason.encode!(Map.merge(base_request(), @extra_request)))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status == 202

      payload = Jason.decode!(conn.resp_body)
      result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

      assert result_payload["job"]["status"] == "completed"
      assert result_payload["result"][@expected_key] == expected_value(@expected_key)
    end
  end

  test "preserves an explicit cyclic material load schedule during normalization" do
    section_fibers = [
      %{"y" => -0.028_284_271_247_461_9, "area" => 0.005},
      %{"y" => 0.028_284_271_247_461_9, "area" => 0.005}
    ]

    request =
      @material_p_delta_request
      |> Map.update!("stability", fn stability ->
        stability
        |> Map.delete("maximum_load_factor")
        |> Map.delete("load_steps")
      end)
      |> Map.update!("materials", fn [material] ->
        [
          material
          |> Map.put("section_fibers", section_fibers)
          |> Map.put("longitudinal_integration_points", 4)
          |> Map.put("adaptive_longitudinal_integration", true)
          |> Map.put("longitudinal_integration_tolerance", 0.0005)
        ]
      end)
      |> Map.put("load_factor_schedule", [1.3, 0.0, -1.3])

    assert {:ok, normalized} =
             FemModelNormalizer.normalize_frame_2d_material_p_delta(request)

    assert normalized["load_factor_schedule"] == [1.3, 0.0, -1.3]
    assert normalized["stability"]["kinematics"] == "corotational"
    assert normalized["materials"] |> hd() |> Map.fetch!("section_fibers") == section_fibers
    assert normalized["materials"] |> hd() |> Map.fetch!("longitudinal_integration_points") == 4
    assert normalized["materials"] |> hd() |> Map.fetch!("adaptive_longitudinal_integration")

    assert normalized["materials"] |> hd() |> Map.fetch!("longitudinal_integration_tolerance") ==
             0.0005
  end

  defp base_request do
    %{
      "nodes" => [%{"id" => "n0", "x" => 0.0}, %{"id" => "n1", "x" => 1.0}],
      "elements" => [%{"id" => "e0", "node_i" => 0, "node_j" => 1}]
    }
  end

  defp expected_value("converged"), do: true
  defp expected_value("active_contact_count"), do: 0
  defp expected_value("natural_frequencies_hz"), do: [10.0]
  defp expected_value(_key), do: 1.0

  defp configure_solver_agent do
    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()
  end

  defp solver_progress(message) do
    %{
      "event" => "progress",
      "progress" => %{
        "job_id" => "advanced-solver-session",
        "stage" => "solving",
        "progress" => 0.5,
        "iteration" => 1,
        "message" => message
      }
    }
  end
end
