defmodule KyuubikiWeb.OperatorTaskApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  test "prepares and verifies an operator task IR envelope" do
    task = fixture_task!()

    conn =
      :post
      |> conn("/api/v1/operator-tasks/prepare", Jason.encode!(%{"task" => task}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "verified"
    assert payload["operator_id"] == "transform.api_fixture"
    assert payload["program_id"] == "transform.api_fixture"
    assert payload["task_digest"] == get_in(task, ["integrity", "task_digest"])
  end

  test "rejects tampered operator task IR envelopes" do
    task = put_in(fixture_task!(), ["config", "alpha"], false)

    conn =
      :post
      |> conn("/api/v1/operator-tasks/prepare", Jason.encode!(%{"task" => task}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 422
    assert Jason.decode!(conn.resp_body)["error"] =~ "operator_task_digest_mismatch"
  end

  test "executes an operator task IR envelope through the service API" do
    {:ok, task} =
      OperatorTaskIR.build(
        "transform.evaluate_material_thermal_shock",
        %{
          "temperature_delta" => 160.0,
          "thermal_expansion" => 1.2e-5,
          "youngs_modulus" => 70.0e9,
          "poisson_ratio" => 0.33,
          "yield_strength" => 320.0e6
        },
        %{"constraint_factor" => 0.7}
      )

    conn =
      :post
      |> conn("/api/v1/operator-tasks/execute", Jason.encode!(%{"task" => task}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "executed"
    assert payload["operator_id"] == "transform.evaluate_material_thermal_shock"
    assert payload["result"]["material_thermal_shock_status"] == "pass"
  end

  defp fixture_task! do
    descriptor = %{
      "id" => "transform.api_fixture",
      "family" => "fixture",
      "kind" => "transform",
      "execution" => %{"package_ref" => "orchestra://operator-package/transform.api_fixture"}
    }

    {:ok, task} =
      OperatorTaskIR.build_from_descriptor(
        descriptor,
        %{"x" => 1},
        %{"alpha" => true},
        descriptor_authoring: %{
          "mode" => "rust_native",
          "runtime" => "rust",
          "source" => "api_fixture",
          "hot_reloadable" => false
        },
        task_id: "operator-task-api-fixture"
      )

    task
  end
end
