defmodule KyuubikiSdk.ModelResearchExecutionTest do
  use ExUnit.Case, async: false

  alias KyuubikiSdk.ControlPlaneClient
  alias KyuubikiSdk.ModelCollaboration
  alias KyuubikiSdk.ModelPlanApproval
  alias KyuubikiSdk.ModelResearchExecution
  alias KyuubikiSdk.Session

  test "rejects unapproved plan before any dispatch" do
    {:ok, plan} = ModelCollaboration.build_plan(session(), proposal())
    parent = self()
    dispatcher = fn action, _payload -> send(parent, {:dispatched, action}) end

    assert {:error, error} =
             ModelResearchExecution.execute(dispatcher, plan, nil, fn _, _ -> true end)

    assert error.message =~ "exact caller-issued approval"
    refute_received {:dispatched, _}
  end

  test "rejects unverified approval before any dispatch" do
    {:ok, plan} = ModelCollaboration.build_plan(session(), proposal())
    parent = self()
    dispatcher = fn action, _payload -> send(parent, {:dispatched, action}) end

    assert {:error, error} =
             ModelResearchExecution.execute(
               dispatcher,
               plan,
               approval(plan),
               fn _, _ -> false end
             )

    assert error.message =~ "verifier rejected approval"
    refute_received {:dispatched, _}
  end

  test "rejects payload changed after approval before any dispatch" do
    {:ok, plan} = ModelCollaboration.build_plan(session(), proposal())
    approval = approval(plan)

    changed =
      put_in(
        plan,
        ["steps", Access.at(1), "payload", "input_artifacts", "material_rows", "rows"],
        [
          %{"case_id" => "injected-after-approval"}
        ]
      )

    parent = self()
    dispatcher = fn action, _payload -> send(parent, {:dispatched, action}) end

    assert {:error, error} =
             ModelResearchExecution.execute(dispatcher, changed, approval, fn _, _ -> true end)

    assert error.message =~ "plan_digest does not match"
    refute_received {:dispatched, _}
  end

  test "executes approved plan and retains authority" do
    {:ok, plan} = ModelCollaboration.build_plan(session(), proposal())
    dispatcher = fake_dispatcher()

    assert {:ok, receipt} =
             ModelResearchExecution.execute(
               dispatcher,
               plan,
               approval(plan),
               fn _, _ -> true end
             )

    assert receipt["status"] == "completed"
    assert receipt["completed_steps"] == 2
    assert receipt["plan_digest"] == approval(plan)["plan_digest"]
    assert Enum.at(receipt["records"], 1)["authority"] == "test-dispatcher"
  end

  test "retains partial failure instead of claiming completion" do
    partial = %{
      proposal()
      | "calls" => [
          %{"action" => "service_health", "payload" => %{}},
          %{"action" => "protocol_describe", "payload" => %{}}
        ]
    }

    {:ok, plan} = ModelCollaboration.build_plan(session(), partial)

    dispatcher = fn
      "protocol_describe", _payload ->
        {:error, RuntimeError.exception("injected failure")}

      action, _payload ->
        {:ok, %{"authority" => "test-dispatcher", "output" => %{"action" => action}}}
    end

    assert {:ok, receipt} =
             ModelResearchExecution.execute(dispatcher, plan, nil, fn _, _ -> true end)

    assert receipt["status"] == "failed"
    assert receipt["failed_step"] == 2
    assert receipt["completed_steps"] == 1
    assert Enum.at(receipt["records"], 1)["error"] =~ "injected failure"
  end

  test "failure receipt preserves valid UTF-8 within byte bound" do
    single = %{proposal() | "calls" => [%{"action" => "service_health", "payload" => %{}}]}
    {:ok, plan} = ModelCollaboration.build_plan(session(), single)

    dispatcher = fn _action, _payload ->
      {:error, RuntimeError.exception(String.duplicate("错", 1_000))}
    end

    assert {:ok, receipt} =
             ModelResearchExecution.execute(dispatcher, plan, nil, fn _, _ -> true end)

    error = hd(receipt["records"])["error"]
    assert String.valid?(error)
    assert byte_size(error) <= 2_051
    assert String.ends_with?(error, "...")
  end

  test "plan rejects malformed payload types" do
    malformed =
      put_in(proposal(), ["calls", Access.at(1), "payload"], %{
        "workflow_id" => 42,
        "input_artifacts" => []
      })

    assert {:ok, plan} = ModelCollaboration.build_plan(session(), malformed)
    refute plan["ok"]
    assert Enum.any?(plan["issues"], &String.contains?(&1, "non-empty string"))
    assert Enum.any?(plan["issues"], &String.contains?(&1, "JSON object"))
  end

  test "session dispatcher reaches existing control plane routes" do
    {:ok, listener} =
      :gen_tcp.listen(0, [:binary, packet: 0, active: false, reuseaddr: true])

    {:ok, port} = :inet.port(listener)
    parent = self()

    server =
      spawn_link(fn ->
        Enum.each(1..2, fn _ ->
          {:ok, socket} = :gen_tcp.accept(listener)
          {:ok, request} = :gen_tcp.recv(socket, 0)
          [request_line | _] = String.split(request, "\r\n")
          send(parent, {:request_line, request_line})

          body =
            if String.starts_with?(request_line, "GET /api/health ") do
              Jason.encode!(%{"status" => "ok"})
            else
              Jason.encode!(%{
                "job" => %{"job_id" => "job-elixir-research", "status" => "queued"}
              })
            end

          :ok = :gen_tcp.send(socket, json_response(body))
          :gen_tcp.close(socket)
        end)

        :gen_tcp.close(listener)
      end)

    ref = Process.monitor(server)

    client = ControlPlaneClient.new("http://127.0.0.1:#{port}")
    headless = Session.new(control_plane: client)
    {:ok, plan} = ModelCollaboration.build_plan(session(), proposal())

    assert {:ok, receipt} =
             ModelResearchExecution.execute(
               ModelResearchExecution.session_dispatcher(headless),
               plan,
               approval(plan),
               fn _, _ -> true end
             )

    assert receipt["status"] == "completed"
    assert_receive {:request_line, "GET /api/health HTTP/1.1"}

    assert_receive {:request_line,
                    "POST /api/v1/workflows/catalog/workflow.material-study-envelope-ranking-json/jobs HTTP/1.1"}

    assert_receive {:DOWN, ^ref, :process, ^server, :normal}
  end

  test "repository bootstrap fixtures reach approved execution" do
    schemas = Path.expand("../../../schemas", __DIR__)
    session = schemas |> Path.join("examples.model-collaboration-session.json") |> read_json!()
    proposal = schemas |> Path.join("examples.model-workflow-proposal.json") |> read_json!()
    approval = schemas |> Path.join("examples.model-plan-approval.json") |> read_json!()
    {:ok, plan} = ModelCollaboration.build_plan(session, proposal)

    assert {:ok, receipt} =
             ModelResearchExecution.execute(
               fake_dispatcher(),
               plan,
               approval,
               fn _, _ -> true end
             )

    assert receipt["status"] == "completed"
    assert receipt["completed_steps"] == length(proposal["calls"])
  end

  test "execution receipt retains narrow job binding" do
    plan = %{
      "schema_version" => ModelCollaboration.plan_schema_version(),
      "session_id" => "elixir-research-session",
      "workflow_id" => "workflow.material",
      "ok" => true,
      "ready_without_confirmation" => true,
      "issues" => [],
      "steps" => [
        %{
          "index" => 1,
          "action" => "job_wait",
          "category" => "observation",
          "risk" => "normal",
          "payload" => %{"job_id" => "job-bound-001"},
          "requires_confirmation" => false,
          "confirmation_reason" => nil,
          "output_keys" => ["job"]
        }
      ]
    }

    assert {:ok, receipt} =
             ModelResearchExecution.execute(fake_dispatcher(), plan, nil, fn _, _ -> true end)

    assert get_in(receipt, ["records", Access.at(0), "job_id"]) == "job-bound-001"
  end

  defp session do
    %{
      "schema_version" => ModelCollaboration.session_schema_version(),
      "session_id" => "elixir-research-session",
      "workflow_id" => "workflow.material-study-envelope-ranking-json",
      "objective" => "Run one bounded material screening study.",
      "created_at" => "2026-08-01T00:00:00Z",
      "policy" => %{
        "allowed_actions" => [
          "service_health",
          "protocol_describe",
          "workflow_submit_catalog"
        ],
        "allow_sensitive" => true
      }
    }
  end

  defp proposal do
    %{
      "schema_version" => ModelCollaboration.proposal_schema_version(),
      "session_id" => "elixir-research-session",
      "calls" => [
        %{"action" => "service_health", "payload" => %{}},
        %{
          "action" => "workflow_submit_catalog",
          "payload" => %{
            "workflow_id" => "workflow.material-study-envelope-ranking-json",
            "input_artifacts" => %{"material_rows" => %{"rows" => []}}
          }
        }
      ]
    }
  end

  defp approval(plan) do
    {:ok, plan_digest} = ModelPlanApproval.compute_digest(plan)

    %{
      "schema_version" => ModelResearchExecution.approval_schema_version(),
      "approval_id" => "approval-elixir-test",
      "session_id" => plan["session_id"],
      "workflow_id" => plan["workflow_id"],
      "plan_digest" => plan_digest,
      "authority" => "elixir-integration-test",
      "issued_at" => "2026-08-01T00:01:00Z",
      "approved_steps" => [%{"index" => 2, "action" => "workflow_submit_catalog"}]
    }
  end

  defp fake_dispatcher do
    fn action, _payload ->
      {:ok,
       %{
         "authority" => "test-dispatcher",
         "output" => %{"action" => action, "ok" => true}
       }}
    end
  end

  defp json_response(body) do
    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: #{byte_size(body)}\r\nconnection: close\r\n\r\n#{body}"
  end

  defp read_json!(path), do: path |> File.read!() |> Jason.decode!()
end
