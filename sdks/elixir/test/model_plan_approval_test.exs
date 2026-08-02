defmodule KyuubikiSdk.ModelPlanApprovalTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.ModelCollaboration
  alias KyuubikiSdk.ModelPlanApproval

  test "shared plan builds digest-bound approval request" do
    schemas = Path.expand("../../../schemas", __DIR__)
    session = read_json!(schemas, "examples.model-collaboration-session.json")
    proposal = read_json!(schemas, "examples.model-workflow-proposal.json")

    expected =
      read_json!(schemas, "examples.model-plan-approval-request.json") |> Map.delete("$schema")

    {:ok, plan} = ModelCollaboration.build_plan(session, proposal)

    assert {:ok, request} = ModelPlanApproval.build_request(plan)
    assert request == expected

    assert {:ok, "sha256:22e040653a1fc2274201a86f3ffaff67e896cedb5754e6fee01fb0528704d18d"} =
             ModelPlanApproval.compute_digest(plan)
  end

  test "nested payload changes digest" do
    schemas = Path.expand("../../../schemas", __DIR__)
    session = read_json!(schemas, "examples.model-collaboration-session.json")
    proposal = read_json!(schemas, "examples.model-workflow-proposal.json")
    {:ok, plan} = ModelCollaboration.build_plan(session, proposal)

    changed =
      put_in(
        plan,
        [
          "steps",
          Access.at(1),
          "payload",
          "input_artifacts",
          "material_rows",
          "rows",
          Access.at(0),
          "case_id"
        ],
        "changed"
      )

    assert {:ok, before} = ModelPlanApproval.compute_digest(plan)
    assert {:ok, after_digest} = ModelPlanApproval.compute_digest(changed)
    refute before == after_digest
  end

  test "approval request rejects inconsistent gated risk" do
    schemas = Path.expand("../../../schemas", __DIR__)
    session = read_json!(schemas, "examples.model-collaboration-session.json")
    proposal = read_json!(schemas, "examples.model-workflow-proposal.json")
    {:ok, plan} = ModelCollaboration.build_plan(session, proposal)
    changed = put_in(plan, ["steps", Access.at(1), "risk"], "normal")

    assert {:error, error} = ModelPlanApproval.build_request(changed)
    assert error.message =~ "has invalid risk"
  end

  defp read_json!(schemas, name),
    do: schemas |> Path.join(name) |> File.read!() |> Jason.decode!()
end
