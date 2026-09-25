defmodule KyuubikiWeb.Orchestra.WorkflowNodeProgressTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.WorkflowNodeProgress, as: Progress

  test "legacy progress remains completed-only and terminal counters are validated" do
    legacy = %{"node_id" => "n", "completed_nodes" => 2, "total_nodes" => 4}
    assert {:ok, normalized} = Progress.normalize(legacy)
    assert normalized["resolved_nodes"] == 2
    assert normalized["status"] == "completed"
    assert normalized["failed_nodes"] == 0

    event = Map.merge(legacy, %{"status" => "failed", "failed_nodes" => 1, "skipped_nodes" => 1})
    assert {:ok, normalized} = Progress.normalize(event)
    assert normalized["resolved_nodes"] == 4
    assert normalized["completed_nodes"] == 2

    for patch <- [
          %{"node_id" => ""},
          %{"completed_nodes" => -1},
          %{"completed_nodes" => false},
          %{"failed_nodes" => "1"},
          %{"skipped_nodes" => -1},
          %{"total_nodes" => 0},
          %{"total_nodes" => 3},
          %{"resolved_nodes" => 3},
          %{"resolved_nodes" => nil},
          %{"status" => "bogus"}
        ] do
      assert {:error, :invalid_workflow_progress} = Progress.normalize(Map.merge(event, patch))
    end
  end

  test "throttling counts resolved nodes and always retains a new failure" do
    previous = %{resolved: 10, persisted_at_ms: System.monotonic_time(:millisecond)}
    event = %{"node_id" => "n", "completed_nodes" => 10, "total_nodes" => 1000}
    refute Progress.persist?(event, previous)
    assert Progress.persist?(Map.put(event, "skipped_nodes", 10), previous)

    assert Progress.persist?(
             Map.merge(event, %{"status" => "failed", "failed_nodes" => 1}),
             previous
           )

    assert Progress.persist?(%{}, previous)
    state = Progress.remember(%{progress: %{}}, "job", Map.put(event, "skipped_nodes", 5))
    assert state.progress["job"].resolved == 15
    assert Progress.remember(state, "job", %{}) == state
  end
end
