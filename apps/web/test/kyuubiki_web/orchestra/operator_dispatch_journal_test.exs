defmodule KyuubikiWeb.Orchestra.OperatorDispatchJournalTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchInspection, as: Inspection

  setup do
    root =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-dispatch-test-#{Base.encode16(:crypto.strong_rand_bytes(16))}"
      )

    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    %{root: root, journal: journal}
  end

  test "caller death and journal restart retain ambiguous ownership without inputs or credentials",
       ctx do
    parent = self()

    pid =
      spawn(fn ->
        {:ok, attempt} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
        send(parent, {:recorded, attempt})
      end)

    monitor = Process.monitor(pid)
    assert_receive {:recorded, attempt}
    assert_receive {:DOWN, ^monitor, _, _, _}
    stop_supervised(Journal)
    restarted = start_supervised!({Journal, root: ctx.root, name: nil})
    assert {:ok, [record]} = Journal.lookup("task", digest(), restarted)
    assert record["attempt_id"] == attempt
    assert record["state"] == "dispatch_boundary_unconfirmed"
    bytes = File.read!(Path.join(ctx.root, attempt <> ".json"))
    refute bytes =~ "SECRET"
    refute bytes =~ "127.0.0.1"
    refute bytes =~ "model_payload"
    assert {:ok, [^record]} = Journal.lookup("task", digest(), restarted)
    assert {:ok, []} = Journal.lookup("task", String.duplicate("b", 64), restarted)
  end

  test "partial, oversized, symlinked and invalid generations fail closed without rollback",
       ctx do
    {:ok, attempt} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
    path = Path.join(ctx.root, attempt <> ".json")
    original = File.read!(path)
    stop_supervised(Journal)

    for corruption <- [:partial, :oversized, :invalid, :symlink] do
      File.write!(path, original)

      case corruption do
        :partial ->
          File.write!(path <> ".next", original)

        :oversized ->
          File.write!(path, String.duplicate("x", 4097))

        :invalid ->
          File.write!(path, "{}")

        :symlink ->
          File.rm!(path)
          File.ln_s!(path <> ".target", path)
      end

      restarted = start_supervised!({Journal, root: ctx.root, name: nil})
      assert {:error, _} = Journal.lookup("task", digest(), restarted)
      assert {:error, _} = Journal.begin_dispatch(request(), endpoint(), restarted)
      stop_supervised(Journal)
      File.rm(path <> ".next")
      File.rm(path)
    end
  end

  test "corruption during a live attempt is not overwritten and further dispatches stop", ctx do
    {:ok, id} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
    path = Path.join(ctx.root, id <> ".json")
    File.write!(path, "broken")
    assert {:error, _} = Journal.finish_dispatch(id, "observed_executed", ctx.journal)
    assert File.read!(path) == "broken"
    assert {:error, _} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
  end

  test "unresolved records fill a bounded journal without silent eviction", ctx do
    for _ <- 1..Files.limits().max_records do
      assert {:ok, _} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
    end

    assert {:error, :operator_task_dispatch_journal_full} =
             Journal.begin_dispatch(request(), endpoint(), ctx.journal)

    assert {:ok, records} = Journal.lookup("task", digest(), ctx.journal)
    assert length(records) == 512
    assert length(File.ls!(ctx.root)) == 512

    total_bytes =
      File.ls!(ctx.root) |> Enum.map(&File.stat!(Path.join(ctx.root, &1)).size) |> Enum.sum()

    assert total_bytes < 512 * Files.limits().max_file_bytes
  end

  test "terminal history is capped while unresolved attempts remain intact", ctx do
    {:ok, unknown} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)

    for _ <- 1..140 do
      {:ok, id} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
      assert :ok = Journal.finish_dispatch(id, "not_dispatched", ctx.journal)
    end

    assert {:ok, records} = Journal.lookup("task", digest(), ctx.journal)
    assert length(records) == 129
    assert Enum.any?(records, &(&1["attempt_id"] == unknown))
    assert {:ok, loaded} = Files.load(ctx.root)
    assert map_size(loaded) == 129
  end

  test "damaged confirmed history is not silently pruned", ctx do
    for _ <- 1..128 do
      {:ok, id} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
      :ok = Journal.finish_dispatch(id, "not_dispatched", ctx.journal)
    end

    {:ok, records} = Journal.lookup("task", digest(), ctx.journal)
    oldest = Enum.min_by(records, &{&1["updated_at_ms"], &1["attempt_id"]})
    path = Path.join(ctx.root, oldest["attempt_id"] <> ".json")
    File.write!(path, "damaged")
    assert {:error, _} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
    assert File.read!(path) == "damaged"
  end

  test "published sample and limits agree with the visible storage and inspection policies",
       ctx do
    schema_root = Path.expand("../../../../../schemas", __DIR__)

    sample =
      File.read!(Path.join(schema_root, "examples.operator-task-dispatch-inspection.json"))
      |> Jason.decode!()

    schema =
      File.read!(Path.join(schema_root, "operator-task-dispatch-inspection.schema.json"))
      |> Jason.decode!()

    {:ok, report} = Inspection.inspect_task(query(), journal: ctx.journal, endpoints: [])
    assert Jason.decode!(Jason.encode!(report["journal_policy"])) == sample["journal_policy"]

    assert Jason.decode!(Jason.encode!(report["inspection_policy"])) ==
             sample["inspection_policy"]

    assert schema["properties"]["attempts"]["maxItems"] ==
             report["inspection_policy"].max_attempts

    assert Enum.sort(Map.keys(report)) == Enum.sort(schema["required"])
    assert schema["additionalProperties"] == false
  end

  test "malformed nested identity fails without crashing the journal", ctx do
    for bad <- [
          nil,
          %{},
          %{"params" => %{"task_ir" => []}},
          put_in(request(), ["params", "task_ir", "integrity"], 1)
        ] do
      assert {:error, :operator_task_dispatch_journal_invalid} =
               Journal.begin_dispatch(bad, endpoint(), ctx.journal)
    end

    assert Process.alive?(ctx.journal)
    assert {:ok, _} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
  end

  test "oversized escaped identity fails before writing without poisoning other dispatches",
       ctx do
    oversized =
      request()
      |> put_in(["params", "task_ir", "operator", "id"], String.duplicate("\n", 1024))
      |> put_in(
        ["params", "task_ir", "execution_program", "program_id"],
        String.duplicate("\n", 1024)
      )

    assert {:error, :operator_task_dispatch_journal_invalid} =
             Journal.begin_dispatch(oversized, endpoint(), ctx.journal)

    refute File.exists?(ctx.root)
    assert {:ok, _} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)
  end

  test "normal test runs do not inherit production journal data roots" do
    previous = System.get_env("KYUUBIKI_DATA_DIR")
    System.put_env("KYUUBIKI_DATA_DIR", "/unowned/production/data")

    on_exit(fn ->
      if previous,
        do: System.put_env("KYUUBIKI_DATA_DIR", previous),
        else: System.delete_env("KYUUBIKI_DATA_DIR")
    end)

    expected =
      Application.get_env(:kyuubiki_web, :test_database_root) ||
        Application.fetch_env!(:kyuubiki_web, :operator_dispatch_journal_test_root)

    assert Journal.root() == Path.join(expected, "operator-task-dispatches")
    refute Journal.root() =~ "/unowned/production"
  end

  test "endpoint or session replacement never probes the new owner", ctx do
    {:ok, _} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)

    for replacement <- [
          Map.put(endpoint(), :port, 6000),
          Map.put(endpoint(), :agent_session_id, "new")
        ] do
      assert {:ok, report} =
               Inspection.inspect_task(query(),
                 journal: ctx.journal,
                 endpoints: [replacement],
                 probe: fn _ -> flunk("replacement queried") end
               )

      assert [attempt] = report["attempts"]
      assert attempt["observation"]["status"] == "original_endpoint_not_configured"
      assert attempt["observation"]["outcome"] == "unknown"
    end
  end

  test "only exact active request IDs are diagnostic, absence and malformed descriptors remain unknown",
       ctx do
    {:ok, _} = Journal.begin_dispatch(request(), endpoint(), ctx.journal)

    for {active, expected} <- [
          {[], "request_not_observed_active"},
          {[%{"request_id" => "other", "generation" => 1}], "request_not_observed_active"},
          {[%{"request_id" => "request", "generation" => 1, "cancel_requested" => false}],
           "original_endpoint_reports_active_request"},
          {[%{"request_id" => "request", "generation" => 0, "cancel_requested" => false}],
           "agent_observation_invalid"}
        ] do
      descriptor = %{
        "solver_control" => %{
          "schema_version" => "kyuubiki.agent-solver-control/v1",
          "available" => true,
          "active" => active
        },
        "lifecycle" => %{"process_instance_id" => "boot-1"},
        "SECRET" => "SECRET"
      }

      assert {:ok, report} =
               Inspection.inspect_task(query(),
                 journal: ctx.journal,
                 endpoints: [endpoint()],
                 probe: fn _ -> {:ok, descriptor} end
               )

      assert [attempt] = report["attempts"]
      assert attempt["observation"]["status"] == expected
      assert attempt["observation"]["outcome"] == "unknown"
      refute report["automatic_replay_authorized"]
      refute report["terminal_result_available"]
      refute Jason.encode!(report) =~ "SECRET"
    end
  end

  test "inspection caches targets and caps probes and visible attempts", ctx do
    endpoints = for port <- 5001..5006, do: Map.put(endpoint(), :port, port)

    for target <- endpoints do
      {:ok, _} = Journal.begin_dispatch(request(), target, ctx.journal)
    end

    parent = self()

    assert {:ok, report} =
             Inspection.inspect_task(query(),
               journal: ctx.journal,
               endpoints: endpoints,
               probe: fn target ->
                 send(parent, {:probed, target.port})
                 {:error, :offline}
               end
             )

    assert length(report["attempts"]) == 6

    assert Enum.count(report["attempts"], &(&1["observation"]["status"] == "probe_limit_reached")) ==
             2

    for _ <- 1..4, do: assert_receive({:probed, _})
    refute_receive {:probed, _}, 20

    for _ <- 1..130, do: Journal.begin_dispatch(request(), endpoint(), ctx.journal)
    assert {:ok, report} = Inspection.inspect_task(query(), journal: ctx.journal, endpoints: [])
    assert report["retained_attempt_count"] == 136
    assert length(report["attempts"]) == 128
    assert report["truncated"]
  end

  test "missing history and invalid queries cannot grant replay or select arbitrary hosts", ctx do
    assert {:ok, report} = Inspection.inspect_task(query(), journal: ctx.journal, endpoints: [])
    assert report["status"] == "no_retained_dispatch"
    refute report["automatic_replay_authorized"]

    for bad <- [
          %{},
          Map.put(query(), "host", "untrusted"),
          Map.put(query(), "task_digest", "bad")
        ] do
      assert {:error, :operator_task_dispatch_query_invalid} =
               Inspection.inspect_task(bad, journal: ctx.journal)
    end
  end

  defp digest, do: String.duplicate("a", 64)
  defp query, do: %{"task_id" => "task", "task_digest" => digest()}

  defp endpoint,
    do: %{
      id: "owner",
      host: "127.0.0.1",
      port: 5001,
      agent_session_id: "original",
      token: "SECRET"
    }

  defp request do
    %{
      "id" => "request",
      "params" => %{
        "task_ir" => %{
          "task_id" => "task",
          "integrity" => %{"task_digest" => digest()},
          "operator" => %{"id" => "solve.bar_1d"},
          "execution_program" => %{"program_id" => "native-bar"},
          "model_payload" => "SECRET"
        }
      }
    }
  end
end
