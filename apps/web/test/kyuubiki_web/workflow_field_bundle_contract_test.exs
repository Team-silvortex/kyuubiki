defmodule KyuubikiWeb.WorkflowFieldBundleContractTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.{WorkflowOperatorRuntime, WorkflowReportingRuntime, WorkflowSummaryRuntime}

  @root Path.expand("../../../..", __DIR__)
  @cases Enum.flat_map(["field", "bundle"], fn family ->
           @root
           |> Path.join("tests/fixtures/workflow-#{family}-contract.json")
           |> File.read!()
           |> Jason.decode!()
         end)

  for fixture <- @cases do
    @fixture fixture
    test "shared Rust/Elixir contract: #{fixture["id"]}" do
      %{"operator" => operator, "payload" => payload, "config" => config} = @fixture

      for run <- [&dispatch/3, &direct/3] do
        result = run.(operator, payload, config)

        if @fixture["error_contains"] do
          assert {:error, reason} = result
          assert inspect(reason) =~ @fixture["error_contains"]
        else
          assert {:ok, output} = result
          subset(output, @fixture["output"])
          assert Jason.decode!(Jason.encode!(output)) == output
        end
      end
    end
  end

  defp dispatch("extract." <> _ = operator, payload, config),
    do: WorkflowOperatorRuntime.run_extract_operator(operator, payload, config)

  defp dispatch(operator, payload, config),
    do: WorkflowOperatorRuntime.run_transform_operator(operator, payload, config)

  defp direct("extract.field_statistics", payload, config),
    do: WorkflowReportingRuntime.extract_field_statistics(payload, config)

  defp direct("extract.field_hotspots", payload, config),
    do: WorkflowReportingRuntime.extract_field_hotspots(payload, config)

  defp direct("transform.compose_diagnostics_bundle", payload, config),
    do: WorkflowSummaryRuntime.compose_diagnostics_bundle(payload, config)

  defp direct("transform.evaluate_diagnostics_bundle_guard", payload, config),
    do: WorkflowSummaryRuntime.evaluate_diagnostics_bundle_guard(payload, config)

  defp subset(actual, expected) when is_map(expected) do
    assert is_map(actual)

    for {key, value} <- expected do
      assert Map.has_key?(actual, key), "missing output #{key}"
      subset(actual[key], value)
    end
  end

  defp subset(actual, expected) when is_number(expected) and is_number(actual) do
    scale = max(abs(actual), abs(expected))

    assert scale == 0 or abs(actual / scale - expected / scale) <= 1.0e-12,
           "#{actual} != #{expected}"
  end

  defp subset(actual, expected), do: assert(actual == expected)
end
