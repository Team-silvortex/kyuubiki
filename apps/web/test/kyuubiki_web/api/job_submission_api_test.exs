defmodule KyuubikiWeb.Api.JobSubmissionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "submits a spring 2d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/spring-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "y" => 0.0, "fix_x" => true, "fix_y" => true, "load_x" => 0.0, "load_y" => 0.0},
            %{"id" => "n1", "x" => 1.0, "y" => 0.0, "fix_x" => false, "fix_y" => false, "load_x" => 1000.0, "load_y" => 0.0}
          ],
          "elements" => [%{"id" => "s0", "node_i" => 0, "node_j" => 1, "stiffness" => 25_000.0}]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert Jason.decode!(conn.resp_body)["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a spring 3d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/spring-3d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "y" => 0.0, "z" => 0.0, "fix_x" => true, "fix_y" => true, "fix_z" => true, "load_x" => 0.0, "load_y" => 0.0, "load_z" => 0.0},
            %{"id" => "n1", "x" => 1.0, "y" => 0.0, "z" => 0.0, "fix_x" => false, "fix_y" => true, "fix_z" => true, "load_x" => 1000.0, "load_y" => 0.0, "load_z" => 0.0}
          ],
          "elements" => [%{"id" => "s0", "node_i" => 0, "node_j" => 1, "stiffness" => 25_000.0}]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert Jason.decode!(conn.resp_body)["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal truss 2d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-truss-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "y" => 0.0, "fix_x" => true, "fix_y" => true, "load_x" => 0.0, "load_y" => 0.0, "temperature_delta" => 40.0},
            %{"id" => "n1", "x" => 1.0, "y" => 0.0, "fix_x" => true, "fix_y" => true, "load_x" => 0.0, "load_y" => 0.0, "temperature_delta" => 40.0}
          ],
          "elements" => [%{"id" => "tt0", "node_i" => 0, "node_j" => 1, "area" => 0.01, "youngs_modulus" => 210.0e9, "thermal_expansion" => 12.0e-6}]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert Jason.decode!(conn.resp_body)["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal truss 3d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-truss-3d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "y" => 0.0, "z" => 0.0, "fix_x" => true, "fix_y" => true, "fix_z" => true, "load_x" => 0.0, "load_y" => 0.0, "load_z" => 0.0, "temperature_delta" => 40.0},
            %{"id" => "n1", "x" => 1.0, "y" => 0.0, "z" => 0.0, "fix_x" => true, "fix_y" => true, "fix_z" => true, "load_x" => 0.0, "load_y" => 0.0, "load_z" => 0.0, "temperature_delta" => 40.0},
            %{"id" => "n2", "x" => 0.0, "y" => 1.0, "z" => 0.0, "fix_x" => true, "fix_y" => true, "fix_z" => true, "load_x" => 0.0, "load_y" => 0.0, "load_z" => 0.0, "temperature_delta" => 0.0}
          ],
          "elements" => [%{"id" => "tt0", "node_i" => 0, "node_j" => 1, "area" => 0.01, "youngs_modulus" => 210.0e9, "thermal_expansion" => 12.0e-6}]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert Jason.decode!(conn.resp_body)["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal beam 1d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-beam-1d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "fix_y" => true, "fix_rz" => true, "load_y" => 0.0, "moment_z" => 0.0},
            %{"id" => "n1", "x" => 2.0, "fix_y" => true, "fix_rz" => true, "load_y" => 0.0, "moment_z" => 0.0}
          ],
          "elements" => [
            %{"id" => "tb0", "node_i" => 0, "node_j" => 1, "youngs_modulus" => 2.1e11, "moment_of_inertia" => 8.0e-6, "section_modulus" => 1.6e-4, "thermal_expansion" => 1.2e-5, "section_depth" => 0.2, "distributed_load_y" => 0.0, "temperature_gradient_y" => 40.0}
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert Jason.decode!(conn.resp_body)["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal frame 2d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-frame-2d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "y" => 0.0, "fix_x" => true, "fix_y" => true, "fix_rz" => true, "load_x" => 0.0, "load_y" => 0.0, "moment_z" => 0.0, "temperature_delta" => 35.0},
            %{"id" => "n1", "x" => 2.0, "y" => 0.0, "fix_x" => true, "fix_y" => true, "fix_rz" => true, "load_x" => 0.0, "load_y" => 0.0, "moment_z" => 0.0, "temperature_delta" => 35.0}
          ],
          "elements" => [
            %{"id" => "tf0", "node_i" => 0, "node_j" => 1, "area" => 0.02, "youngs_modulus" => 2.1e11, "moment_of_inertia" => 8.0e-6, "section_modulus" => 1.6e-4, "thermal_expansion" => 1.2e-5, "section_depth" => 0.2, "temperature_gradient_y" => 30.0}
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert Jason.decode!(conn.resp_body)["job"]["status"] in ["queued", "running", "completed"]
  end

  test "submits a thermal frame 3d job" do
    conn =
      :post
      |> conn(
        "/api/v1/fem/thermal-frame-3d/jobs",
        Jason.encode!(%{
          "nodes" => [
            %{"id" => "n0", "x" => 0.0, "y" => 0.0, "z" => 0.0, "fix_x" => true, "fix_y" => true, "fix_z" => true, "fix_rx" => true, "fix_ry" => true, "fix_rz" => true, "load_x" => 0.0, "load_y" => 0.0, "load_z" => 0.0, "moment_x" => 0.0, "moment_y" => 0.0, "moment_z" => 0.0, "temperature_delta" => 35.0},
            %{"id" => "n1", "x" => 2.0, "y" => 0.0, "z" => 0.0, "fix_x" => true, "fix_y" => true, "fix_z" => true, "fix_rx" => true, "fix_ry" => true, "fix_rz" => true, "load_x" => 0.0, "load_y" => 0.0, "load_z" => 0.0, "moment_x" => 0.0, "moment_y" => 0.0, "moment_z" => 0.0, "temperature_delta" => 35.0}
          ],
          "elements" => [
            %{"id" => "tf3-0", "node_i" => 0, "node_j" => 1, "area" => 0.02, "youngs_modulus" => 2.1e11, "shear_modulus" => 8.0e10, "torsion_constant" => 1.0e-5, "moment_of_inertia_y" => 8.0e-6, "moment_of_inertia_z" => 6.0e-6, "section_modulus_y" => 1.6e-4, "section_modulus_z" => 1.2e-4, "thermal_expansion" => 1.2e-5, "section_depth_y" => 0.2, "section_depth_z" => 0.15, "temperature_gradient_y" => 30.0, "temperature_gradient_z" => 20.0}
          ]
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 202
    assert Jason.decode!(conn.resp_body)["job"]["status"] in ["queued", "running", "completed"]
  end

  test "cancels an active job through the API" do
    {:ok, job} =
      Store.create(%{
        job_id: "job-cancel",
        project_id: "project-cancel",
        simulation_case_id: "case-cancel"
      })

    assert job.status == :queued

    Store.apply_progress(%{
      job_id: "job-cancel",
      stage: "solving",
      progress: 0.6,
      message: "solving structural system"
    })

    cancel_conn =
      :post
      |> conn("/api/v1/jobs/job-cancel/cancel")
      |> Router.call(@opts)

    assert cancel_conn.status == 200

    payload = Jason.decode!(cancel_conn.resp_body)
    assert payload["job"]["status"] == "cancelled"
    assert payload["job"]["message"] == "job cancelled by operator"

    assert {:ok, cancelled_job} = Store.get("job-cancel")
    assert cancelled_job.status == :cancelled
  end
end
