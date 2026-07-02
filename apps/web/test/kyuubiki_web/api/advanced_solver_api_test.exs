defmodule KyuubikiWeb.Api.AdvancedSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

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

  @cases [
    {"/api/v1/fem/acoustic-bar-1d/jobs", "max_sound_pressure_level_db", %{}},
    {"/api/v1/fem/stokes-flow-plane-quad-2d/jobs", "max_velocity", %{}},
    {"/api/v1/fem/nonlinear-spring-1d/jobs", "converged", %{}},
    {"/api/v1/fem/contact-gap-1d/jobs", "active_contact_count", %{"contacts" => []}},
    {"/api/v1/fem/modal-frame-2d/jobs", "natural_frequencies_hz", %{}},
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
