defmodule KyuubikiWeb.Orchestra.OperatorDispatchInspectionSnapshotTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchInspection, as: Inspection
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal

  setup do
    root =
      Path.join(System.tmp_dir!(), "inspection-snapshot-#{System.unique_integer([:positive])}")

    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    endpoint = %{id: "owner", host: "127.0.0.1", port: 5001, agent_session_id: "session"}
    digest = String.duplicate("a", 64)

    request = %{
      "id" => "request",
      "params" => %{
        "job_id" => "parent",
        "task_ir" => %{
          "task_id" => "task",
          "integrity" => %{"task_digest" => digest},
          "operator" => %{"id" => "op"},
          "execution_program" => %{"program_id" => "op"}
        }
      }
    }

    {:ok, id} = Journal.begin_dispatch(request, endpoint, journal)
    {:ok, [record]} = Journal.lookup_task("task", journal)

    %{
      root: root,
      journal: journal,
      endpoint: endpoint,
      request: request,
      record: record,
      path: Path.join(root, id <> ".json"),
      query: %{"task_id" => "task", "task_digest" => digest}
    }
  end

  for fault <- [:deleted, :corrupt, :replacement, :partial, :missing_root],
      reply <- [:active, :absent, :invalid, :unreachable] do
    test "#{fault} while probing #{reply} cannot escape as a valid inspection", ctx do
      probe = fn _ ->
        send(self(), :probed)
        damage(ctx, unquote(fault))
        reply(unquote(reply))
      end

      assert {:error, error} = inspect_task(ctx, probe)

      assert error in ~w(operator_task_dispatch_journal_invalid operator_task_dispatch_journal_unavailable)a

      assert_receive :probed
      refute_receive :probed, 10
      bytes = File.read(ctx.path)
      assert {:error, ^error} = Journal.lookup_task("task", ctx.journal)
      assert File.read(ctx.path) == bytes
      assert Process.alive?(ctx.journal)
    end
  end

  for state <-
        ~w(observed_executed observed_failed observed_blocked not_dispatched outcome_unknown) do
    test "concurrent #{state} rejects only the stale inspection and permits a fresh read", ctx do
      probe = fn _ ->
        assert :ok =
                 Journal.finish_dispatch(ctx.record["attempt_id"], unquote(state), ctx.journal)

        reply(:active)
      end

      assert {:error, :operator_task_dispatch_observation_changed} = inspect_task(ctx, probe)
      bytes = File.read!(ctx.path)

      assert {:ok, fresh} = inspect_task(ctx, fn _ -> reply(:active) end)
      assert [attempt] = fresh["attempts"]
      assert attempt["state"] == unquote(state)
      refute fresh["automatic_replay_authorized"]
      refute fresh["terminal_result_available"]

      if unquote(state) != "outcome_unknown" do
        assert attempt["observation"] == %{
                 "status" => "retained_dispatch_observation",
                 "outcome" => unquote(state)
               }
      end

      assert File.read!(ctx.path) == bytes
    end
  end

  test "new matching dispatch invalidates the old count without retrying a probe", ctx do
    probe = fn _ ->
      send(self(), :probed)
      assert {:ok, _} = Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal)
      reply(:active)
    end

    assert {:error, :operator_task_dispatch_observation_changed} = inspect_task(ctx, probe)
    assert_receive :probed
    refute_receive :probed, 10
    assert {:ok, fresh} = inspect_task(ctx, fn _ -> reply(:active) end)
    assert fresh["retained_attempt_count"] == 2
  end

  test "later probe cannot return an earlier observation whose record has changed", ctx do
    peer = %{ctx.endpoint | id: "peer", port: 5002}
    assert {:ok, _} = Journal.begin_dispatch(ctx.request, peer, ctx.journal)
    assert {:ok, [first, _]} = Journal.lookup_task("task", ctx.journal)

    probe = fn endpoint ->
      send(self(), {:probed, endpoint.id})

      if Files.fingerprint(endpoint) != first["endpoint_fingerprint"] do
        assert :ok = Journal.finish_dispatch(first["attempt_id"], "not_dispatched", ctx.journal)
      end

      reply(:active)
    end

    assert {:error, :operator_task_dispatch_observation_changed} =
             Inspection.inspect_task(ctx.query,
               journal: ctx.journal,
               endpoints: [ctx.endpoint, peer],
               probe: probe
             )

    assert_receive {:probed, _}
    assert_receive {:probed, _}
    refute_receive {:probed, _}, 10
  end

  test "truncated inspection still revalidates all records behind its retained count", ctx do
    for _ <- 1..128,
        do: assert({:ok, _} = Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal))

    assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
    assert length(records) == 129
    hidden = List.last(records)

    probe = fn _ ->
      send(self(), :probed)
      assert :ok = Journal.finish_dispatch(hidden["attempt_id"], "not_dispatched", ctx.journal)
      reply(:active)
    end

    assert {:error, :operator_task_dispatch_observation_changed} = inspect_task(ctx, probe)
    assert_receive :probed
    refute_receive :probed, 10
    assert {:ok, fresh} = inspect_task(ctx, fn _ -> reply(:active) end)
    assert fresh["truncated"]
    assert fresh["retained_attempt_count"] == 129
    assert length(fresh["attempts"]) == 128
  end

  test "journal process loss cannot return a cached execution target", ctx do
    probe = fn _ ->
      assert :ok = stop_supervised(Journal)
      reply(:active)
    end

    assert {:error, :operator_task_dispatch_journal_unavailable} = inspect_task(ctx, probe)
  end

  for control <- [:unrelated_task, :other_digest, :same_observation, :unrelated_damage] do
    test "#{control} does not invalidate the selected inspection snapshot", ctx do
      assert :ok =
               Journal.finish_dispatch(ctx.record["attempt_id"], "outcome_unknown", ctx.journal)

      other = put_in(ctx.request, ~w(params task_ir task_id), "unrelated")
      {:ok, other_id} = Journal.begin_dispatch(other, ctx.endpoint, ctx.journal)
      bytes = File.read!(ctx.path)

      probe = fn _ ->
        case unquote(control) do
          :unrelated_task ->
            assert {:ok, _} = Journal.begin_dispatch(other, ctx.endpoint, ctx.journal)

          :other_digest ->
            changed =
              put_in(
                ctx.request,
                ~w(params task_ir integrity task_digest),
                String.duplicate("b", 64)
              )

            assert {:ok, _} = Journal.begin_dispatch(changed, ctx.endpoint, ctx.journal)

          :same_observation ->
            assert :ok =
                     Journal.finish_dispatch(
                       ctx.record["attempt_id"],
                       "outcome_unknown",
                       ctx.journal
                     )

          :unrelated_damage ->
            File.write!(
              Path.join(ctx.root, other_id <> ".json"),
              "SECRET damaged unrelated payload"
            )
        end

        reply(:active)
      end

      assert {:ok, result} = inspect_task(ctx, probe)
      assert result["retained_attempt_count"] == 1
      assert [attempt] = result["attempts"]
      assert attempt["observation"]["execution_target"]["job_id"] == "parent"
      refute result["automatic_replay_authorized"]
      refute result["terminal_result_available"]
      assert File.read!(ctx.path) == bytes
      refute inspect(result) =~ "SECRET"
    end
  end

  defp inspect_task(ctx, probe),
    do:
      Inspection.inspect_task(ctx.query,
        journal: ctx.journal,
        endpoints: [ctx.endpoint],
        probe: probe
      )

  defp damage(ctx, :deleted), do: File.rm!(ctx.path)
  defp damage(ctx, :corrupt), do: File.write!(ctx.path, "SECRET damaged observation")
  defp damage(ctx, :partial), do: File.write!(ctx.path <> ".next", "SECRET partial replacement")
  defp damage(ctx, :missing_root), do: File.rm_rf!(ctx.root)

  defp damage(ctx, :replacement),
    do:
      assert(
        :ok = Files.write(ctx.root, Map.put(ctx.record, "state", "outcome_unknown"), ctx.record)
      )

  defp reply(:unreachable), do: {:error, :timeout}
  defp reply(:invalid), do: {:ok, false}

  defp reply(status) do
    entry = %{
      "request_id" => "request",
      "job_id" => "parent",
      "generation" => 1,
      "cancel_requested" => false
    }

    {:ok,
     %{
       "lifecycle" => %{"process_instance_id" => "process"},
       "solver_control" => %{
         "schema_version" => "kyuubiki.agent-solver-control/v1",
         "available" => true,
         "active" => if(status == :active, do: [entry], else: [])
       }
     }}
  end
end
