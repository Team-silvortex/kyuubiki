defmodule KyuubikiWeb.WorkflowTaskDigestPolicy do
  @moduledoc "Explicit graph-bound encoding for native diagnostic and scoring TaskIR nodes."

  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.TaskDigestV2

  @legacy "kyuubiki.task-digest/fixed15-v1"
  def legacy, do: @legacy

  def resolve(graph) when is_map(graph) do
    case Map.get(graph, "defaults", %{}) do
      %{} = defaults -> validate(Map.get(defaults, "native_task_digest_encoding", @legacy))
      _ -> {:error, "workflow defaults must be an object"}
    end
  end

  def validate(@legacy), do: {:ok, @legacy}
  def validate("kyuubiki.task-digest/typed-ieee754-v2"), do: {:ok, TaskDigestV2.encoding()}
  def validate(_), do: {:error, "unsupported native_task_digest_encoding"}

  def seal(task, @legacy), do: {:ok, task}

  def seal(task, encoding) do
    with {:ok, encoding} <- validate(encoding),
         do: OperatorTaskIR.seal_task_digest(task, encoding)
  end
end
