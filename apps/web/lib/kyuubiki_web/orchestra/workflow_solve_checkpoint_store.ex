defmodule KyuubikiWeb.Orchestra.WorkflowSolveCheckpointStore do
  @moduledoc false
  alias KyuubikiWeb.Orchestra.{WorkflowCheckpointStore, WorkflowSolveCheckpoint}

  def access(job_id, claim, action, lease),
    do: WorkflowCheckpointStore.access(job_id, claim, action, lease, WorkflowSolveCheckpoint)
end
