defmodule KyuubikiWeb.Api.MagnetostaticSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  @bar_result %{
    "nodes" => [
      %{
        "index" => 0,
        "id" => "n0",
        "x" => 0.0,
        "magnetic_potential" => 4.0,
        "magnetomotive_source" => 0.0
      },
      %{
        "index" => 1,
        "id" => "n1",
        "x" => 1.0,
        "magnetic_potential" => 0.0,
        "magnetomotive_source" => 0.0
      }
    ],
    "elements" => [
      %{
        "index" => 0,
        "id" => "mb0",
        "node_i" => 0,
        "node_j" => 1,
        "length" => 1.0,
        "average_magnetic_potential" => 2.0,
        "magnetic_potential_gradient" => -4.0,
        "magnetic_field_strength" => 4.0,
        "magnetic_flux_density" => 8.0,
        "stored_energy" => 0.16
      }
    ],
    "max_magnetic_potential" => 4.0,
    "max_magnetic_field_strength" => 4.0,
    "max_flux_density" => 8.0,
    "total_stored_energy" => 0.16,
    "input" => %{"nodes" => [], "elements" => []}
  }

  @plane_nodes [
    %{
      "index" => 0,
      "id" => "n0",
      "x" => 0.0,
      "y" => 0.0,
      "vector_potential" => 4.0,
      "current_density" => 0.0
    },
    %{
      "index" => 1,
      "id" => "n1",
      "x" => 1.0,
      "y" => 0.0,
      "vector_potential" => 0.0,
      "current_density" => 0.0
    },
    %{
      "index" => 2,
      "id" => "n2",
      "x" => 0.0,
      "y" => 1.0,
      "vector_potential" => 4.0,
      "current_density" => 0.0
    }
  ]

  @triangle_element %{
    "index" => 0,
    "id" => "mt0",
    "node_i" => 0,
    "node_j" => 1,
    "node_k" => 2,
    "area" => 0.5,
    "average_vector_potential" => 2.6666666666666665,
    "vector_potential_gradient_x" => -4.0,
    "vector_potential_gradient_y" => 0.0,
    "magnetic_field_strength_x" => 0.0,
    "magnetic_field_strength_y" => 4.0,
    "magnetic_field_strength_magnitude" => 4.0,
    "magnetic_flux_density_x" => 0.0,
    "magnetic_flux_density_y" => 8.0,
    "magnetic_flux_density_magnitude" => 8.0,
    "stored_energy" => 0.08
  }

  test "runs a magnetostatic bar job through the orchestration API" do
    result =
      submit_solver_job("/api/v1/fem/magnetostatic-bar-1d/jobs", bar_request(), @bar_result)

    [element] = result["elements"]

    assert result["max_magnetic_potential"] == 4.0
    assert result["max_magnetic_field_strength"] == 4.0
    assert result["max_flux_density"] == 8.0
    assert result["total_stored_energy"] == 0.16
    assert Enum.map(result["nodes"], & &1["magnetic_potential"]) == [4.0, 0.0]
    assert element["magnetic_field_strength"] == 4.0
    assert element["magnetic_flux_density"] == 8.0
    assert element["stored_energy"] == 0.16
  end

  test "runs a magnetostatic plane triangle job through the orchestration API" do
    result =
      submit_solver_job(
        "/api/v1/fem/magnetostatic-plane-triangle-2d/jobs",
        plane_triangle_request(),
        plane_result([@triangle_element])
      )

    [element] = result["elements"]

    assert result["max_vector_potential"] == 4.0
    assert element["area"] == 0.5
    assert element["magnetic_field_strength_magnitude"] == 4.0
    assert element["magnetic_flux_density_magnitude"] == 8.0
    assert element["stored_energy"] == 0.08
  end

  test "runs a magnetostatic plane quad job through the orchestration API" do
    quad_element = Map.merge(@triangle_element, %{"id" => "mq0", "node_l" => 3, "area" => 1.0})

    result =
      submit_solver_job(
        "/api/v1/fem/magnetostatic-plane-quad-2d/jobs",
        plane_quad_request(),
        plane_result([quad_element])
      )

    [element] = result["elements"]

    assert result["max_vector_potential"] == 4.0
    assert element["node_l"] == 3
    assert element["area"] == 1.0
    assert element["magnetic_field_strength_magnitude"] == 4.0
    assert element["magnetic_flux_density_magnitude"] == 8.0
  end

  defp submit_solver_job(path, request, result) do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        solver_progress("solving magnetostatic model"),
        %{"ok" => true, "result" => result}
      ])

    configure_solver_agent()

    conn =
      :post
      |> conn(path, Jason.encode!(request))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202

    payload = Jason.decode!(conn.resp_body)
    result_payload = WorkflowApi.wait_for_job(payload["job"]["job_id"], @opts)

    assert result_payload["job"]["status"] == "completed"
    result_payload["result"]
  end

  defp bar_request do
    %{
      "nodes" => [
        %{
          "id" => "n0",
          "x" => 0.0,
          "fix_magnetic_potential" => true,
          "magnetic_potential" => 4.0,
          "magnetomotive_source" => 0.0
        },
        %{
          "id" => "n1",
          "x" => 1.0,
          "fix_magnetic_potential" => true,
          "magnetic_potential" => 0.0,
          "magnetomotive_source" => 0.0
        }
      ],
      "elements" => [
        %{"id" => "mb0", "node_i" => 0, "node_j" => 1, "area" => 0.02, "permeability" => 2.0}
      ]
    }
  end

  defp plane_triangle_request do
    %{
      "nodes" => plane_request_nodes(),
      "elements" => [
        %{
          "id" => "mt0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "thickness" => 0.05,
          "permeability" => 2.0
        }
      ]
    }
  end

  defp plane_quad_request do
    %{
      "nodes" => plane_request_nodes() ++ [%{"id" => "n3", "x" => 1.0, "y" => 1.0}],
      "elements" => [
        %{
          "id" => "mq0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "thickness" => 0.05,
          "permeability" => 2.0
        }
      ]
    }
  end

  defp plane_request_nodes do
    [
      %{"id" => "n0", "x" => 0.0, "y" => 0.0, "fix_vector_potential" => true},
      %{"id" => "n1", "x" => 1.0, "y" => 0.0, "fix_vector_potential" => true},
      %{"id" => "n2", "x" => 0.0, "y" => 1.0, "fix_vector_potential" => true}
    ]
  end

  defp plane_result(elements) do
    %{
      "nodes" => @plane_nodes,
      "elements" => elements,
      "max_vector_potential" => 4.0,
      "max_magnetic_field_strength" => 4.0,
      "max_flux_density" => 8.0,
      "total_stored_energy" => 0.08,
      "input" => %{"nodes" => [], "elements" => []}
    }
  end

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
        "job_id" => "solver-session",
        "stage" => "solving",
        "progress" => 0.5,
        "iteration" => 1,
        "message" => message
      }
    }
  end
end
