defmodule KyuubikiWeb.WorkflowNativeInvocationTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.WorkflowNativeInvocation

  @key "workflow_native_invocation_sha256"
  @context %{
    "job_id" => "parent",
    "control_mode" => "mesh",
    "workflow_input_binding_sha256" => String.duplicate("a", 64)
  }

  for field <- [:payload, :config, :context] do
    test "exact #{field} numbers remain distinct inside nested arrays" do
      pairs = [
        {1.0e-30, 2.0e-30},
        {1.0e-300, 2.0e-300},
        {0.0, 1.0e-30},
        {-0.0, -1.0e-30},
        {0.3333333333333333, 0.33333333333333337},
        {1, 1.0},
        {0.0, -0.0}
      ]

      for {a, b} <- pairs do
        original = %{payload: %{}, config: %{}, context: @context}

        original =
          Map.update!(original, unquote(field), &Map.put(&1, "numbers", [%{"value" => a}]))

        changed =
          Map.update!(original, unquote(field), &Map.put(&1, "numbers", [%{"value" => b}]))

        refute invocation_digest(original) == invocation_digest(changed)
      end
    end
  end

  test "key order is stable and JSON round trips retain an unchanged invocation" do
    payload = %{"b" => [1.0e-300, -0.0], "a" => %{"q" => 0.33333333333333337, "p" => true}}
    original = %{payload: payload, config: %{"b" => nil, "a" => 1.0e-30}, context: @context}

    reversed =
      Map.update!(original, :payload, &(&1 |> Map.to_list() |> Enum.reverse() |> Map.new()))

    assert invocation_digest(original) == invocation_digest(reversed)

    roundtrip =
      Map.new(original, fn {field, value} ->
        {field, value |> DurableJson.encode!() |> Jason.decode!()}
      end)

    assert invocation_digest(original) == invocation_digest(roundtrip)
  end

  test "computed binding is idempotent and overrides supplied values without granting authority" do
    payload = %{"value" => 1.0e-30}
    bound = WorkflowNativeInvocation.bind_context(@context, payload, %{})
    assert Map.delete(bound, @key) == @context
    assert bound[@key] =~ ~r/\A[0-9a-f]{64}\z/
    assert WorkflowNativeInvocation.bind_context(bound, payload, %{}) == bound

    for forged <- [nil, "forged", String.duplicate("0", 64), %{"input" => "not-authority"}] do
      assert WorkflowNativeInvocation.bind_context(Map.put(@context, @key, forged), payload, %{}) ==
               bound
    end
  end

  test "binding uses sorted lossless JSON, not fixed-decimal canonicalization or retained payloads" do
    payload = %{"secret" => "input-only", "value" => 1.0e-300}
    config = %{"threshold" => 2.0e-300}
    bound = WorkflowNativeInvocation.bind_context(@context, payload, config)

    exact = %{
      "schema_version" => "kyuubiki.workflow-native-invocation/v1",
      "input_artifact" => payload,
      "config" => config,
      "orchestration_context" => @context
    }

    expected =
      exact
      |> DurableJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)

    assert bound[@key] == expected
    refute DurableJson.encode!(bound) =~ "input-only"
    assert map_size(bound) == map_size(@context) + 1
  end

  defp invocation_digest(spec),
    do: WorkflowNativeInvocation.bind_context(spec.context, spec.payload, spec.config)[@key]
end
