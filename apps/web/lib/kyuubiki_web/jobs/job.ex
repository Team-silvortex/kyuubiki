defmodule KyuubikiWeb.Jobs.Job do
  @moduledoc """
  In-memory representation of the durable job contract from `schemas/job.schema.json`.
  """

  @statuses ~w(
    queued
    preprocessing
    partitioning
    solving
    postprocessing
    completed
    failed
    cancelled
  )a

  @enforce_keys [:job_id, :project_id, :simulation_case_id]
  defstruct [
    :job_id,
    :project_id,
    :simulation_case_id,
    :worker_id,
    status: :queued,
    progress: 0.0,
    residual: nil,
    iteration: nil,
    created_at: nil,
    updated_at: nil
  ]

  @type status :: unquote(Enum.reduce(@statuses, &{:|, [], [&1, &2]}))

  @type t :: %__MODULE__{
          job_id: String.t(),
          project_id: String.t(),
          simulation_case_id: String.t(),
          worker_id: String.t() | nil,
          status: status(),
          progress: float(),
          residual: float() | nil,
          iteration: non_neg_integer() | nil,
          created_at: DateTime.t(),
          updated_at: DateTime.t()
        }

  @spec statuses() :: [status()]
  def statuses, do: @statuses

  @spec new(map()) :: {:ok, t()} | {:error, term()}
  def new(attrs) when is_map(attrs) do
    now = DateTime.utc_now(:second)

    with {:ok, job_id} <- fetch_required_string(attrs, :job_id),
         {:ok, project_id} <- fetch_required_string(attrs, :project_id),
         {:ok, simulation_case_id} <- fetch_required_string(attrs, :simulation_case_id),
         {:ok, status} <- fetch_status(attrs, :status, :queued),
         {:ok, progress} <- fetch_progress(attrs, :progress, 0.0) do
      {:ok,
       %__MODULE__{
         job_id: job_id,
         project_id: project_id,
         simulation_case_id: simulation_case_id,
         worker_id: fetch_optional_string(attrs, :worker_id),
         status: status,
         progress: progress,
         residual: fetch_optional_number(attrs, :residual),
         iteration: fetch_optional_integer(attrs, :iteration),
         created_at: Map.get(attrs, :created_at, now),
         updated_at: Map.get(attrs, :updated_at, now)
       }}
    end
  end

  @spec apply_progress(t(), KyuubikiWeb.Jobs.ProgressEvent.t()) :: t()
  def apply_progress(%__MODULE__{} = job, %KyuubikiWeb.Jobs.ProgressEvent{} = event) do
    %__MODULE__{
      job
      | status: event.stage,
        progress: event.progress,
        residual: event.residual,
        iteration: event.iteration,
        updated_at: event.emitted_at || DateTime.utc_now(:second)
    }
  end

  defp fetch_required_string(attrs, key) do
    case Map.get(attrs, key) do
      value when is_binary(value) and byte_size(value) > 0 -> {:ok, value}
      _ -> {:error, {:invalid_or_missing, key}}
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

  defp fetch_status(attrs, key, default) do
    status =
      attrs
      |> Map.get(key, default)
      |> normalize_status()

    if status in @statuses do
      {:ok, status}
    else
      {:error, {:invalid_status, status}}
    end
  end

  defp normalize_status(value) when value in @statuses, do: value

  defp normalize_status(value) when is_binary(value) do
    Enum.find(@statuses, fn status -> Atom.to_string(status) == value end) || value
  end

  defp normalize_status(value), do: value

  defp fetch_progress(attrs, key, default) do
    case Map.get(attrs, key, default) do
      value when is_integer(value) and value >= 0 and value <= 1 -> {:ok, value * 1.0}
      value when is_float(value) and value >= 0.0 and value <= 1.0 -> {:ok, value}
      _ -> {:error, {:invalid_progress, key}}
    end
  rescue
    ArgumentError -> {:error, {:invalid_progress, key}}
  end
end
