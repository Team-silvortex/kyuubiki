defmodule KyuubikiWeb.Orchestra.WorkflowAdmission do
  @moduledoc false

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope

  # The coordinator calls both operations inside its existing ownership boundary.
  def admit(attrs, graph, inputs, context, options) do
    with {:ok, runtime} <- runtime(attrs[:job_id], graph, inputs, context, options),
         do: Store.create_with_result(attrs, runtime)
  end

  def initialize(job_id, graph, inputs, context, options) do
    with {:ok, runtime} <- runtime(job_id, graph, inputs, context, options),
         {:ok, _job} <- Store.initialize_result(job_id, runtime),
         do: :ok
  end

  defp runtime(job_id, graph, inputs, context, options)
       when is_binary(job_id) and byte_size(job_id) > 0 do
    with {:ok, recovery} <-
           WorkflowRecoveryEnvelope.new(
             graph,
             inputs,
             Map.put(context, "job_id", job_id),
             options
           ) do
      {:ok,
       %{
         "workflow_id" => Map.get(graph, "id"),
         "current_node" => nil,
         "progress_events" => [],
         "completed_nodes" => [],
         "artifacts" => %{},
         "response_options" => options,
         "orchestration_context" => context,
         WorkflowRecoveryEnvelope.internal_key() => recovery
       }}
    end
  end

  defp runtime(_job_id, _graph, _inputs, _context, _options),
    do: {:error, :invalid_analysis_admission}
end
