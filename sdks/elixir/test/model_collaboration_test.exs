defmodule KyuubikiSdk.ModelCollaborationTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.ModelCollaboration

  defp session(allow_sensitive \\ false, service_only \\ true) do
    %{
      "schema_version" => ModelCollaboration.session_schema_version(),
      "session_id" => "session.elixir-model",
      "workflow_id" => "workflow.elixir-model",
      "objective" => "Discover the runtime and submit one bounded solve",
      "language" => "en",
      "created_at" => "2026-08-01T00:00:00Z",
      "policy" =>
        ModelCollaboration.default_policy()
        |> Map.put("allow_sensitive", allow_sensitive)
        |> Map.put("service_only", service_only)
    }
  end

  test "default catalog is read-only and service-owned" do
    tools = ModelCollaboration.tools()
    assert Enum.any?(tools, &(&1["action"] == "service_health"))
    assert Enum.all?(tools, &(&1["risk"] == "normal"))
    assert Enum.all?(tools, &(&1["runtime"] == "service"))
  end

  test "service-only policy excludes direct solver" do
    service_tools = ModelCollaboration.tools(session(true)["policy"])
    assert Enum.any?(service_tools, &(&1["action"] == "fem_submit"))
    refute Enum.any?(service_tools, &(&1["action"] == "direct_solver_rpc"))

    all_tools = ModelCollaboration.tools(session(true, false)["policy"])
    assert Enum.any?(all_tools, &(&1["action"] == "direct_solver_rpc"))
  end

  test "provider requests share catalog and redact context" do
    for provider <- [:openai, :openai_chat, :anthropic, :gemini, :canonical] do
      assert {:ok, request} =
               ModelCollaboration.build_request(provider, session(), %{
                 "authorization" => "Bearer secret-value",
                 "nested" => %{"api_key" => "secret"}
               })

      assert request["context"]["authorization"] == "[REDACTED]"
      assert request["context"]["nested"]["api_key"] == "[REDACTED]"
      assert request["tools"] != []
    end
  end

  test "normalizes provider calls" do
    responses = [
      openai: %{
        "output" => [
          %{
            "type" => "function_call",
            "call_id" => "o1",
            "name" => "service_health",
            "arguments" => "{}"
          }
        ]
      },
      openai_chat: %{
        "choices" => [
          %{
            "message" => %{
              "tool_calls" => [
                %{"id" => "c1", "function" => %{"name" => "service_health", "arguments" => "{}"}}
              ]
            }
          }
        ]
      },
      anthropic: %{
        "content" => [
          %{"type" => "tool_use", "id" => "a1", "name" => "service_health", "input" => %{}}
        ]
      },
      gemini: %{
        "candidates" => [
          %{
            "content" => %{
              "parts" => [
                %{"functionCall" => %{"id" => "g1", "name" => "service_health", "args" => %{}}}
              ]
            }
          }
        ]
      },
      gemini: %{
        "steps" => [
          %{
            "type" => "function_call",
            "id" => "g2",
            "name" => "service_health",
            "arguments" => %{}
          }
        ]
      },
      canonical: %{
        "schema_version" => ModelCollaboration.proposal_schema_version(),
        "session_id" => "session.elixir-model",
        "calls" => [%{"action" => "service_health", "payload" => %{}}]
      }
    ]

    for {provider, response} <- responses do
      assert {:ok, proposal} =
               ModelCollaboration.normalize_response(provider, "session.elixir-model", response)

      assert get_in(proposal, ["calls", Access.at(0), "action"]) == "service_health"
    end
  end

  test "oversized context fails closed" do
    bounded = put_in(session(), ["policy", "max_context_bytes"], 16)

    assert {:error, error} =
             ModelCollaboration.build_request(:openai, bounded, %{
               "token" => "secret",
               "payload" => "this remains too large"
             })

    assert error.message =~ "policy allows 16"
  end

  test "sensitive plans require confirmation" do
    proposal = %{
      "schema_version" => ModelCollaboration.proposal_schema_version(),
      "session_id" => "session.elixir-model",
      "calls" => [
        %{
          "action" => "fem_submit",
          "payload" => %{"solve_kind" => "thermal_frame_3d", "payload" => %{"model" => %{}}}
        }
      ]
    }

    assert {:ok, plan} = ModelCollaboration.build_plan(session(true), proposal)
    assert plan["ok"]
    refute plan["ready_without_confirmation"]
    assert get_in(plan, ["steps", Access.at(0), "requires_confirmation"])
  end

  test "hidden and malformed calls fail closed" do
    proposal = %{
      "schema_version" => ModelCollaboration.proposal_schema_version(),
      "session_id" => "session.elixir-model",
      "calls" => [%{"action" => "fem_submit", "payload" => %{"solve_kind" => "thermal_frame_3d"}}]
    }

    assert {:ok, hidden} = ModelCollaboration.build_plan(session(), proposal)
    refute hidden["ok"]
    assert {:ok, incomplete} = ModelCollaboration.build_plan(session(true), proposal)
    refute incomplete["ok"]

    assert {:error, error} =
             ModelCollaboration.normalize_response(:openai, "session.elixir-model", %{
               "output" => [
                 %{"type" => "function_call", "name" => "service_health", "arguments" => "[]"}
               ]
             })

    assert error.type == :validation

    assert {:error, policy_error} =
             ModelCollaboration.build_request(
               :openai,
               put_in(session(), ["policy", "allow_sensitive"], "yes"),
               %{}
             )

    assert policy_error.message =~ "allow_sensitive must be a boolean"

    assert {:error, envelope_error} =
             ModelCollaboration.normalize_response(:openai, "session.elixir-model", %{
               "output" => %{}
             })

    assert envelope_error.message =~ "no supported tool calls"
  end

  test "model research bootstrap reaches valid first plan" do
    root = Path.expand("../../..", __DIR__)

    bootstrap =
      root
      |> Path.join("docs/model-research-bootstrap.json")
      |> File.read!()
      |> Jason.decode!()

    for document <- bootstrap["required_documents"] do
      assert root |> Path.join(document["path"]) |> File.regular?()
    end

    first = bootstrap["first_research"]

    fixture_session =
      root
      |> Path.join(first["session_fixture"])
      |> File.read!()
      |> Jason.decode!()

    proposal =
      root
      |> Path.join(first["proposal_fixture"])
      |> File.read!()
      |> Jason.decode!()

    assert {:ok, request} =
             ModelCollaboration.build_request(:canonical, fixture_session, %{})

    assert request["output_contract"] == proposal["schema_version"]
    assert {:ok, plan} = ModelCollaboration.build_plan(fixture_session, proposal)
    assert plan["ok"]
    refute plan["ready_without_confirmation"]
  end
end
