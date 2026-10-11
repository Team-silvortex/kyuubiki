defmodule KyuubikiWeb.Storage.MemoryReadSnapshotTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.{Persistence, Storage}
  alias KyuubikiWeb.Library.MemoryBackend, as: LibraryMemory
  alias KyuubikiWeb.Storage.{AnalysisMemoryState, MemorySnapshot, ReadSnapshot}
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @moduletag skip: not Storage.memory?()

  test "model and version reads cannot span a concurrent publication" do
    {:ok, project} = Library.create_project(%{"name" => "Memory snapshot"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Concurrent model",
        "kind" => "truss_2d",
        "payload" => %{"revision" => 1}
      })

    {current, [latest | _]} =
      ReadSnapshot.run(fn ->
        {:ok, current} = Library.get_model(model["model_id"])

        once(fn ->
          Library.create_version(model["model_id"], %{"payload" => %{"revision" => 2}})
        end)

        {:ok, versions} = Library.list_versions(model["model_id"])
        {current, versions}
      end)

    assert current["latest_version_id"] == latest["version_id"]
    assert current["payload"] == latest["payload"]
  end

  test "job and result reads cannot span a joint deletion" do
    fixture = Fixture.terminal_job()

    {job, result} =
      ReadSnapshot.run(fn ->
        job = Store.get(fixture.id)
        once(fn -> Store.delete_with_result(fixture.id) end)
        {job, AnalysisResultStore.get(fixture.id)}
      end)

    assert {job, result} in [{:error, :error}, {{:ok, fixture.job}, {:ok, fixture.runtime}}]
  end

  test "separate stores cannot combine an old project with a later result" do
    {:ok, project} = Library.create_project(%{"name" => "Generation one"})
    fixture = Fixture.terminal_job()

    {current, {:ok, result}} =
      ReadSnapshot.run(fn ->
        {:ok, current} = Library.get_project(project["project_id"])

        once(fn ->
          {:ok, _} = Library.update_project(project["project_id"], %{"name" => "Generation two"})
          AnalysisResultStore.put(fixture.id, Map.put(fixture.runtime, "generation", 2))
        end)

        {current, AnalysisResultStore.get(fixture.id)}
      end)

    assert current["name"] == "Generation two"
    assert result["generation"] == 2
  end

  test "ABA content restoration still invalidates the captured generation" do
    {:ok, _} = Library.create_project(%{"name" => "Same visible state"})
    original = Agent.get(LibraryMemory, & &1)

    assert {:ok, [_]} =
             ReadSnapshot.run(fn ->
               count_attempt()
               reply = Library.list_projects()

               once(fn ->
                 :ok =
                   MemorySnapshot.update(LibraryMemory, fn _ -> %{original | projects: %{}} end)

                 :ok = MemorySnapshot.update(LibraryMemory, fn _ -> original end)
               end)

               reply
             end)

    assert Agent.get(LibraryMemory, & &1) === original
    assert Process.get(:memory_read_attempts) == 2
  end

  test "a restarted analysis store cannot reuse an old process generation" do
    fixture = Fixture.terminal_job()

    assert {:ok, fixture.runtime} ==
             ReadSnapshot.run(fn ->
               count_attempt()
               reply = AnalysisResultStore.get(fixture.id)
               once(fn -> Outage.without(AnalysisMemoryState, fn -> :ok end) end)
               reply
             end)

    assert Process.get(:memory_read_attempts) == 2
    Fixture.unchanged(fixture)

    saved = Persistence.read_json(Persistence.analysis_state_path(), %{})
    assert Map.keys(saved) |> Enum.sort() == ["jobs", "results", "schema_version"]
  end

  test "continuous writes stop after three attempts and release the read context" do
    {:ok, project} = Library.create_project(%{"name" => "Busy"})

    assert {:error, :analysis_store_unavailable} =
             ReadSnapshot.run(fn ->
               number = count_attempt()
               reply = Library.get_project(project["project_id"])

               Task.async(fn ->
                 Library.update_project(project["project_id"], %{"name" => "Busy #{number}"})
               end)
               |> Task.await(10_000)

               reply
             end)

    assert Process.get(:memory_read_attempts) == 3
    assert {:ok, _} = Library.create_project(%{"name" => "Writable after failed export"})
    assert {:ok, [_, _]} = ReadSnapshot.run(&Library.list_projects/0)
  end

  test "in-context mutations are refused without executing or retrying writes" do
    assert {:error, :analysis_store_unavailable} =
             ReadSnapshot.run(fn ->
               count_attempt()
               Library.create_project(%{"name" => "Must not publish"})
             end)

    assert Process.get(:memory_read_attempts) == 1
    assert {:ok, []} = Library.list_projects()
    assert {:ok, _} = Library.create_project(%{"name" => "Writable again"})
  end

  test "nested read boundaries reuse one captured context" do
    {:ok, project} = Library.create_project(%{"name" => "Nested before"})

    assert {:ok, %{"name" => "Nested after"}} =
             ReadSnapshot.run(fn ->
               count_attempt()
               {:ok, _} = Library.get_project(project["project_id"])

               once(fn ->
                 Library.update_project(project["project_id"], %{"name" => "Nested after"})
               end)

               ReadSnapshot.run(fn -> Library.get_project(project["project_id"]) end)
             end)

    assert Process.get(:memory_read_attempts) == 2
  end

  test "each accessed store is captured once and telemetry never carries stored payloads" do
    {:ok, _} = Library.create_project(%{"name" => "One capture"})
    fixture = Fixture.terminal_job()
    before = File.read!(Persistence.analysis_state_path())
    handler = {__MODULE__, make_ref()}

    :ok =
      :telemetry.attach(
        handler,
        [:kyuubiki, :memory_snapshot, :capture],
        &__MODULE__.capture/4,
        self()
      )

    on_exit(fn -> :telemetry.detach(handler) end)

    assert :checked =
             ReadSnapshot.run(fn ->
               for _ <- 1..20 do
                 assert {:ok, [_]} = Library.list_projects()
                 assert {:ok, fixture.job} == Store.get(fixture.id)
                 assert {:ok, fixture.runtime} == AnalysisResultStore.get(fixture.id)
               end

               :checked
             end)

    assert_received {:memory_capture, %{store: LibraryMemory, attempt: 1} = first}
    assert_received {:memory_capture, %{store: AnalysisMemoryState, attempt: 1} = second}
    assert Enum.sort(Map.keys(first)) == [:attempt, :store]
    assert Enum.sort(Map.keys(second)) == [:attempt, :store]
    refute_received {:memory_capture, _}
    assert File.read!(Persistence.analysis_state_path()) == before
  end

  def capture(_event, _measurements, metadata, owner) do
    if self() == owner, do: send(owner, {:memory_capture, metadata})
  end

  defp count_attempt do
    number = Process.get(:memory_read_attempts, 0) + 1
    Process.put(:memory_read_attempts, number)
    number
  end

  defp once(writer) do
    unless Process.get(:memory_snapshot_writer_done) do
      Process.put(:memory_snapshot_writer_done, true)
      writer |> Task.async() |> Task.await(10_000)
    end
  end
end
