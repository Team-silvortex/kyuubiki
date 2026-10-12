defmodule KyuubikiWeb.WorkflowQualitySweepExpansionAdmissionTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-quality-sweep-expansion-admission.json",
              __DIR__
            )
            |> File.read!()
            |> Jason.decode!()

  for fixture <- @fixtures do
    test "direct expansion: #{fixture["id"]}" do
      fixture = unquote(Macro.escape(fixture))
      payload = fixture["payload"] |> Jason.encode!() |> Jason.decode!()

      result =
        Runtime.run_transform_operator(
          "transform.expand_parameter_sweep",
          payload,
          fixture["config"]
        )

      if fixture["error"] do
        assert {:error, reason} = result
        assert is_binary(reason)
      else
        assert {:ok, expanded} = result
        for {key, expected} <- fixture["expected"], do: assert(expanded[key] == expected)
      end
    end
  end
end
