defmodule KyuubikiWeb.Api.AdvancedSolverApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  @cases [
    {"/api/v1/fem/acoustic-bar-1d/jobs", "max_sound_pressure_level_db", %{}},
    {"/api/v1/fem/stokes-flow-plane-quad-2d/jobs", "max_velocity", %{}},
    {"/api/v1/fem/nonlinear-spring-1d/jobs", "converged", %{}},
    {"/api/v1/fem/contact-gap-1d/jobs", "active_contact_count", %{"contacts" => []}},
    {"/api/v1/fem/modal-frame-2d/jobs", "natural_frequencies_hz", %{}},
    {"/api/v1/fem/modal-frame-3d/jobs", "natural_frequencies_hz", %{}}
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
