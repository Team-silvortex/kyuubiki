defmodule KyuubikiWeb.TestSupport.TaskDigestV2Authoring do
  @moduledoc false
  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.TaskDigestV2

  def main([path]) do
    inputs = path |> File.read!() |> Jason.decode!()

    tasks =
      Enum.map(inputs, fn input ->
        descriptor = %{
          "id" => input["operator_id"],
          "family" => input["family"],
          "kind" => "extract",
          "execution" => %{
            "package_ref" => "orchestra://operator-package/" <> input["operator_id"],
            "package_version" => "test-v2",
            "agent_fetchable" => true
          }
        }

        {:ok, task} =
          OperatorTaskIR.build_from_descriptor(descriptor, input["raw"], input["config"],
            task_id: input["task_id"]
          )

        {:ok, task} = OperatorTaskIR.seal_task_digest(task, TaskDigestV2.encoding())
        task
      end)

    IO.puts("TASK_DIGEST_V2_AUTHORED " <> Jason.encode!(tasks))
  end
end
