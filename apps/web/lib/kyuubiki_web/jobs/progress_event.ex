defmodule KyuubikiWeb.Jobs.ProgressEvent do
  @moduledoc """
  Streamed runtime event consumed by the orchestration layer.
  """

  alias KyuubikiWeb.Jobs.Job

  @enforce_keys [:job_id, :stage, :progress]
  defstruct [
    :job_id,
    :stage,
    :progress,
    :residual,
    :iteration,
    :peak_memory,
    :message,
    :emitted_at
  ]

  @type t :: %__MODULE__{
          job_id: String.t(),
          stage: Job.status(),
          progress: float(),
          residual: float() | nil,
          iteration: non_neg_integer() | nil,
          peak_memory: non_neg_integer() | nil,
          message: String.t() | nil,
          emitted_at: DateTime.t() | nil
        }

  @spec new(map()) :: {:ok, t()} | {:error, term()}
  def new(attrs) when is_map(attrs) do
    with {:ok, job_id} <- fetch_required_string(attrs, :job_id),
         {:ok, stage} <- fetch_stage(attrs),
         {:ok, progress} <- fetch_progress(attrs) do
      {:ok,
       %__MODULE__{
         job_id: job_id,
         stage: stage,
         progress: progress,
         residual: fetch_optional_number(attrs, :residual),
         iteration: fetch_optional_integer(attrs, :iteration),
         peak_memory: fetch_optional_integer(attrs, :peak_memory),
         message: fetch_optional_string(attrs, :message),
         emitted_at: Map.get(attrs, :emitted_at, DateTime.utc_now(:second))
       }}
    end
  end

  defp fetch_required_string(attrs, key) do
    case Map.get(attrs, key) do
      value when is_binary(value) and byte_size(value) > 0 -> {:ok, value}
      _ -> {:error, {:invalid_or_missing, key}}
    end
  end

  defp fetch_stage(attrs) do
    case Map.get(attrs, :stage) do
      value when is_atom(value) ->
        if value in Job.statuses() do
          {:ok, value}
        else
          {:error, {:invalid_stage, value}}
        end

      value when is_binary(value) ->
        atom = Enum.find(Job.statuses(), fn status -> Atom.to_string(status) == value end)

        if atom do
          {:ok, atom}
        else
          {:error, {:invalid_stage, value}}
        end

      value ->
        {:error, {:invalid_stage, value}}
    end
  end

  defp fetch_progress(attrs) do
    case Map.get(attrs, :progress) do
      value when is_integer(value) and value >= 0 and value <= 1 -> {:ok, value * 1.0}
      value when is_float(value) and value >= 0.0 and value <= 1.0 -> {:ok, value}
      _ -> {:error, :invalid_progress}
    end
  end

  defp fetch_optional_string(attrs, key) do
    case Map.get(attrs, key) do
      value when is_binary(value) and byte_size(value) > 0 -> value
      _ -> nil
    end
  end

  defp fetch_optional_number(attrs, key) do
    case Map.get(attrs, key) do
      value when is_integer(value) -> value * 1.0
      value when is_float(value) -> value
      _ -> nil
    end
  end

  defp fetch_optional_integer(attrs, key) do
    case Map.get(attrs, key) do
      value when is_integer(value) and value >= 0 -> value
      _ -> nil
    end
  end
end
