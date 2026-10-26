defmodule KyuubikiWeb.Playground.AgentExecutionCancellationTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Playground.AgentExecutionCancellation, as: Cancellation

  test "cancellation acknowledgement cannot coerce floating generations into an exact target" do
    for generation <- [1, 3, 9_007_199_254_740_992], matched <- [true, false] do
      target = target(generation)
      malformed = put_in(ack(target, matched), ~w(execution_target generation), generation / 1)
      result = Cancellation.deliver(%{}, target, fn _, _ -> {:ok, malformed} end)
      assert_invalid_ack(result)
    end
  end

  test "out of range observed generations cannot become active cancellation evidence" do
    for generation <- [0, -1, 3.0, "3", nil, 18_446_744_073_709_551_616] do
      observation = Cancellation.observe("job", "request", {:ok, descriptor(generation)})
      assert observation == %{"status" => "agent_observation_invalid", "outcome" => "unknown"}
    end
  end

  test "invalid raw protocol observations remain distinct from unreachable endpoints" do
    response =
      {:error, {:agent_transport_failure, :protocol, {:invalid_response, :duplicate_json_key}}}

    assert Cancellation.observe("job", "request", response) ==
             %{"status" => "agent_observation_invalid", "outcome" => "unknown"}
  end

  test "invalid raw protocol acknowledgements remain unconfirmed with no diagnostic echo" do
    response = {:error, {:agent_transport_failure, :protocol, "SECRET"}}
    result = Cancellation.deliver(%{}, target(3), fn _, _ -> response end)
    assert_invalid_ack(result)
    refute inspect(result) =~ "SECRET"
  end

  test "non object acknowledgements are invalid replies rather than unreachable endpoints" do
    for value <- [nil, [], false, 42, "SECRET"] do
      result = Cancellation.deliver(%{}, target(3), fn _, _ -> {:ok, value} end)
      assert_invalid_ack(result)
      refute inspect(result) =~ "SECRET"
    end
  end

  test "integer generation boundaries retain exact registration and negative acknowledgements" do
    for generation <- [1, 3, 9_007_199_254_740_993, 18_446_744_073_709_551_615],
        matched <- [true, false] do
      target = target(generation)
      observation = Cancellation.observe("job", "request", {:ok, descriptor(generation)})
      assert observation["status"] == "original_endpoint_reports_active_request"
      assert observation["execution_target"] === target
      assert observation["generation"] === generation
      assert observation["outcome"] == "unknown"

      acknowledgement = ack(target, matched)

      result =
        Cancellation.deliver(%{}, target, fn _, actual ->
          assert actual === target
          {:ok, acknowledgement}
        end)

      assert result["status"] == acknowledgement["status"]
      assert result["cancel_registered"] === matched
      assert result["agent_acknowledgement"] === acknowledgement
      refute result["agent_acknowledgement"]["execution_terminal_confirmed"]
      refute result["agent_acknowledgement"]["automatic_replay_authorized"]
    end
  end

  test "invalid acknowledgement targets cannot bypass field and identity validation" do
    target = target(3)
    original = ack(target, true)

    for malformed <- [
          put_in(original, ~w(execution_target generation), 0),
          put_in(original, ~w(execution_target generation), 18_446_744_073_709_551_616),
          put_in(original, ~w(execution_target process_instance_id), "other-process"),
          put_in(original, ~w(execution_target job_id), "job\n"),
          put_in(original, ["execution_target"], Map.delete(target, "request_id")),
          put_in(original, ["execution_target"], Map.put(target, "cancel_all", true))
        ] do
      assert_invalid_ack(Cancellation.deliver(%{}, target, fn _, _ -> {:ok, malformed} end))
    end
  end

  test "invalid caller targets cannot send and real transport failures stay unknown" do
    never = fn _, _ -> flunk("invalid target was sent") end
    result = Cancellation.deliver(%{}, target(3.0), never)
    assert result["status"] == "execution_target_invalid"
    refute result["delivery_attempted"]

    for failure <- [
          {:error, {:agent_transport_failure, :connect, :econnrefused}},
          {:error, {:agent_transport_failure, :receive, :timeout}},
          {:error, :timeout}
        ] do
      assert Cancellation.observe("job", "request", failure) ==
               %{"status" => "original_endpoint_unreachable", "outcome" => "unknown"}

      result = Cancellation.deliver(%{}, target(3), fn _, _ -> failure end)
      assert result["status"] == "cancellation_outcome_unknown"
      assert result["uncertainty_reason"] == "original_endpoint_unreachable"
      assert is_nil(result["cancel_registered"])
    end
  end

  defp assert_invalid_ack(result) do
    assert result["status"] == "cancellation_outcome_unknown"
    assert result["uncertainty_reason"] == "agent_acknowledgement_invalid"
    assert result["delivery_attempted"]
    assert is_nil(result["cancel_registered"])
    assert is_nil(result["agent_acknowledgement"])
  end

  defp target(generation),
    do: %{
      "process_instance_id" => "process",
      "request_id" => "request",
      "job_id" => "job",
      "generation" => generation
    }

  defp descriptor(generation) do
    %{
      "lifecycle" => %{"process_instance_id" => "process"},
      "solver_control" => %{
        "schema_version" => "kyuubiki.agent-solver-control/v1",
        "available" => true,
        "active" => [
          %{
            "request_id" => "request",
            "job_id" => "job",
            "generation" => generation,
            "cancel_requested" => false
          }
        ]
      }
    }
  end

  defp ack(target, matched),
    do: %{
      "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
      "execution_target" => target,
      "status" => if(matched, do: "requested", else: "target_not_observed"),
      "cancel_registered" => matched,
      "execution_terminal_confirmed" => false,
      "pending_cancellation_created" => false,
      "operator_package_cleanup_performed" => false,
      "automatic_replay_authorized" => false
    }
end
