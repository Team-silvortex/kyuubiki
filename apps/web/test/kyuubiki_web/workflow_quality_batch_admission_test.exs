defmodule KyuubikiWeb.WorkflowQualityBatchAdmissionTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowQualityObjectiveRuntime, as: Quality

  for cases <- [
        [nil],
        [%{"model" => %{}}, nil],
        [%{"id" => "a", "model" => %{}}, %{"id" => "a", "model" => %{}}],
        [%{"id" => " ", "model" => %{}}],
        [%{"id" => "a", "case_id" => "b", "model" => %{}}]
      ] do
    test "invalid cases cannot produce a partial or empty successful batch #{inspect(cases)}" do
      assert {:error, reason} =
               Quality.compose_quality_execution_batch(
                 %{"cases" => unquote(Macro.escape(cases))},
                 config()
               )

      assert is_binary(reason)
    end
  end

  for {key, value} <- [
        {"operator_config", []},
        {"dataset_contract", nil},
        {"orchestration_context", "bad"},
        {"placement_tags", ["thermal", 1]},
        {"required_capabilities", [nil]},
        {"merge_case_config", "false"},
        {"payload_source", nil},
        {"rpc_mode", "unknown"},
        {"task_id_prefix", " "}
      ] do
    test "batch config #{key} cannot be silently replaced" do
      assert {:error, reason} =
               Quality.compose_quality_execution_batch(
                 %{"cases" => [%{"model" => %{}}]},
                 Map.put(config(), unquote(key), unquote(Macro.escape(value)))
               )

      assert reason =~ unquote(key)
    end
  end

  test "declared counts and disabled or blocked flags cannot authorize batch generation" do
    for extra <- [
          %{"case_count" => 2},
          %{"expansion_enabled" => false},
          %{"expansion_budget_ready" => false},
          %{"sweep_budget" => %{"case_budget_exceeded" => true}}
        ] do
      assert {:error, _} =
               Quality.compose_quality_execution_batch(
                 Map.merge(%{"cases" => [%{"model" => %{}}]}, extra),
                 config()
               )
    end
  end

  test "case aliases retain identity and malformed per-case config cannot be ignored" do
    assert {:ok, batch} =
             Quality.compose_quality_execution_batch(
               %{"cases" => [%{"case_id" => "alias", "model" => %{}}]},
               config()
             )

    assert hd(batch["tasks"])["case_id"] == "alias"

    assert {:error, _} =
             Quality.compose_quality_execution_batch(
               %{"cases" => [%{"model" => %{}, "config" => []}]},
               Map.put(config(), "merge_case_config", true)
             )
  end

  defp config, do: %{"operator_id" => "transform.evaluate_material_thermal_shock"}

  test "generated batch is a JSON exchange artifact not an Elixir keyword structure" do
    assert {:ok, batch} =
             Quality.compose_quality_execution_batch(%{"cases" => [%{"model" => %{}}]}, config())

    assert {:ok, encoded} = Jason.encode(batch)
    assert Jason.decode!(encoded) == batch

    assert hd(batch["tasks"])["agent_rpc"]["routing_opts"]["job_id"] ==
             hd(batch["tasks"])["task_id"]
  end
end
