defmodule KyuubikiWeb.Api.SolverCompletionReceiptApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.TestSupport.ControlledSolverAgent
  alias KyuubikiWeb.TestSupport.AnalysisCommitFault

  @result %{"nodes" => [], "elements" => [], "max_displacement" => 0.25, "input" => %{}}

  test "completed progress cannot cause the final solver result to be discarded" do
    job_id = submit_frames([progress("completed"), %{"ok" => true, "result" => @result}])
    payload = await_terminal(job_id)
    assert payload["job"]["status"] == "completed"
    assert payload["job"]["has_result"]
    assert payload["result"] == @result
    assert {:ok, @result} = AnalysisResultStore.get(job_id)
  end

  test "a failed attempt notification does not override a successful final response" do
    job_id = submit_frames([progress("failed"), %{"ok" => true, "result" => @result}])
    payload = await_terminal(job_id)
    assert payload["job"]["status"] == "completed"
    assert payload["result"] == @result
  end

  test "completed progress followed by an RPC failure is a failure without a result" do
    job_id =
      submit_frames([
        progress("completed"),
        %{
          "ok" => false,
          "error" => %{"code" => "solve_failed", "message" => "finalization failed"}
        }
      ])

    payload = await_terminal(job_id)
    assert payload["job"]["status"] == "failed"
    refute payload["job"]["has_result"]
    assert :error = AnalysisResultStore.get(job_id)
  end

  test "completed progress followed by disconnect cannot leave a false completed job" do
    job_id = submit_frames([progress("completed")])
    payload = await_terminal(job_id)
    assert payload["job"]["status"] == "failed"
    refute payload["job"]["has_result"]
    assert :error = AnalysisResultStore.get(job_id)
  end

  test "the final cancellation response wins over a completed progress notification" do
    job_id =
      submit_frames([
        progress("completed"),
        %{"ok" => false, "error" => %{"code" => "cancelled", "message" => "cancel acknowledged"}}
      ])

    payload = await_terminal(job_id)
    assert payload["job"]["status"] == "cancelled"
    refute payload["job"]["has_result"]
    assert :error = AnalysisResultStore.get(job_id)
  end

  for {label, result} <- [
        {"null", nil},
        {"array", []},
        {"scalar", 7},
        {"text", "invalid"},
        {"boolean", false}
      ] do
    test "a #{label} success result fails promptly rather than stranding the job" do
      job_id =
        submit_frames([progress("solving"), %{"ok" => true, "result" => unquote(result)}])

      payload = await_terminal(job_id)
      assert payload["job"]["status"] == "failed"
      assert payload["job"]["message"] =~ "invalid solver result"
      refute payload["job"]["has_result"]
      assert :error = AnalysisResultStore.get(job_id)
    end
  end

  test "a delayed final response keeps the job active until its result is stored" do
    pid = start_supervised!({ControlledSolverAgent, {self(), progress("completed")["progress"]}})
    configure_fake_agent_pool(await_fake_agent_port())
    job_id = submit_job()
    assert_receive {:solver_request, ^pid, _request}
    payload = await_payload(job_id, &(&1["job"]["message"] == "agent attempt ended"))
    assert payload["job"]["status"] == "postprocessing"
    assert payload["job"]["progress"] < 1.0
    refute payload["job"]["has_result"]

    send(pid, {:reply, %{"ok" => true, "result" => @result}})
    final = await_terminal(job_id)
    assert final["job"]["status"] == "completed"
    assert final["result"] == @result
  end

  test "missing final result is diagnosed and a following request can complete" do
    job_id = submit_frames([progress("completed"), %{"ok" => true}])
    payload = await_terminal(job_id)
    assert payload["job"]["status"] == "failed"
    assert payload["job"]["message"] =~ "invalid solver result"
    assert :error = AnalysisResultStore.get(job_id)

    next_id = submit_frames([progress("completed"), %{"ok" => true, "result" => @result}])
    next = await_terminal(next_id)
    assert next["job"]["status"] == "completed"
    assert next["result"] == @result
    assert :error = AnalysisResultStore.get(job_id)
  end

  test "a cancelled job ignores a delayed solver result and the next job still completes" do
    existing_runners = Task.Supervisor.children(KyuubikiWeb.TaskSupervisor)
    pid = start_supervised!({ControlledSolverAgent, {self(), progress("solving")["progress"]}})
    configure_fake_agent_pool(await_fake_agent_port())
    job_id = submit_job()
    assert_receive {:solver_request, ^pid, _request}
    await_payload(job_id, &(&1["job"]["message"] == "agent attempt ended"))
    runners = Task.Supervisor.children(KyuubikiWeb.TaskSupervisor) -- existing_runners
    assert runners != []
    refs = Enum.map(runners, &Process.monitor/1)

    assert {:ok, _} =
             Store.apply_progress(%{job_id: job_id, stage: :cancelled, progress: 1.0})

    send(pid, {:reply, %{"ok" => true, "result" => @result}})
    for ref <- refs, do: assert_receive({:DOWN, ^ref, :process, _pid, _reason}, 2_000)

    payload = await_terminal(job_id)
    assert payload["job"]["status"] == "cancelled"
    assert :error = AnalysisResultStore.get(job_id)

    next_id = submit_frames([progress("completed"), %{"ok" => true, "result" => @result}])
    next = await_terminal(next_id)
    assert next["job"]["status"] == "completed"
    assert next["result"] == @result
  end

  @tag skip: not KyuubikiWeb.Storage.sqlite?()
  test "a rejected result commit produces a failed receipt without poisoning the next request" do
    pid = start_supervised!({ControlledSolverAgent, {self(), progress("completed")["progress"]}})
    configure_fake_agent_pool(await_fake_agent_port())
    job_id = submit_job()
    assert_receive {:solver_request, ^pid, _request}
    await_payload(job_id, &(&1["job"]["message"] == "agent attempt ended"))
    assert {:ok, before} = Store.get(job_id)
    drop = AnalysisCommitFault.reject_result(job_id)
    send(pid, {:reply, %{"ok" => true, "result" => @result}})

    final = await_terminal(job_id)
    assert final["job"]["status"] == "failed"
    assert final["job"]["message"] =~ "result persistence failed"
    refute final["job"]["has_result"]
    assert :error = AnalysisResultStore.get(job_id)
    assert {:ok, failed} = Store.get(job_id)
    assert failed.worker_id == before.worker_id

    drop.()
    next_id = submit_frames([progress("completed"), %{"ok" => true, "result" => @result}])
    next = await_terminal(next_id)
    assert next["job"]["status"] == "completed"
    assert next["result"] == @result
  end

  defp progress(stage) do
    %{
      "event" => "progress",
      "progress" => %{
        "stage" => stage,
        "progress" => 1.0,
        "iteration" => 9,
        "residual" => 0.01,
        "message" => "agent attempt ended"
      }
    }
  end

  defp submit_frames(frames) do
    {:ok, _pid} = FakePlaygroundAgent.start_link(frames)
    configure_fake_agent_pool(await_fake_agent_port())
    submit_job()
  end

  defp submit_job do
    response =
      conn(:post, "/api/v1/fem/truss-2d/jobs", Jason.encode!(%{"nodes" => [], "elements" => []}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert response.status == 202
    Jason.decode!(response.resp_body)["job"]["job_id"]
  end

  defp await_terminal(job_id) do
    await_payload(job_id, &(&1["job"]["status"] in ["completed", "failed", "cancelled"]))
  end

  defp await_payload(job_id, predicate, attempts \\ 200)
  defp await_payload(_job_id, _predicate, 0), do: flunk("solver receipt did not settle")

  defp await_payload(job_id, predicate, attempts) do
    response = conn(:get, "/api/v1/jobs/#{job_id}") |> Router.call(@opts)
    payload = Jason.decode!(response.resp_body)

    if predicate.(payload) do
      payload
    else
      Process.sleep(10)
      await_payload(job_id, predicate, attempts - 1)
    end
  end
end
