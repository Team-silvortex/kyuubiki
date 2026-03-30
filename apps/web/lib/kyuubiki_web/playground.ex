defmodule KyuubikiWeb.Playground do
  @moduledoc """
  Synchronous playground orchestration flow for small browser FEM runs.
  """

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Playground.AgentClient

  @worker_id "rust-agent-rpc"

  @spec run(map()) :: {:ok, map()} | {:error, term()}
  def run(params) when is_map(params) do
    with {:ok, normalized} <- normalize_params(params),
         {:ok, job} <- create_job(),
         {:ok, _job} <- Store.assign_worker(job.job_id, @worker_id),
         {:ok, _job} <-
           Store.apply_progress(%{job_id: job.job_id, stage: "solving", progress: 0.35}),
         {:ok, result} <- AgentClient.solve(normalized),
         {:ok, final_job} <-
           Store.apply_progress(%{job_id: job.job_id, stage: "completed", progress: 1.0}) do
      {:ok, %{"job" => serialize_job(final_job), "result" => result}}
    end
  end

  defp normalize_params(params) do
    with {:ok, length} <- fetch_number(params, ["length", :length]),
         {:ok, area} <- fetch_number(params, ["area", :area]),
         {:ok, elements} <- fetch_number(params, ["elements", :elements]),
         {:ok, tip_force} <- fetch_number(params, ["tip_force", :tip_force]),
         {:ok, youngs_modulus_gpa} <-
           fetch_number(params, ["youngs_modulus_gpa", :youngs_modulus_gpa]) do
      {:ok,
       %{
         length: length,
         area: area,
         elements: round(elements),
         tip_force: tip_force,
         youngs_modulus: youngs_modulus_gpa * 1.0e9
       }}
    end
  end

  defp create_job do
    Store.create(%{
      job_id: random_id(),
      project_id: random_id(),
      simulation_case_id: random_id()
    })
  end

  defp serialize_job(job) do
    %{
      "job_id" => job.job_id,
      "project_id" => job.project_id,
      "simulation_case_id" => job.simulation_case_id,
      "worker_id" => job.worker_id,
      "status" => Atom.to_string(job.status),
      "progress" => job.progress,
      "residual" => job.residual,
      "iteration" => job.iteration
    }
  end

  defp random_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end

  defp fetch_number(params, [key | rest]) do
    case Map.fetch(params, key) do
      {:ok, value} -> cast_number(value)
      :error -> fetch_number(params, rest)
    end
  end

  defp fetch_number(_params, []), do: {:error, :missing_parameter}

  defp cast_number(value) when is_integer(value), do: {:ok, value * 1.0}
  defp cast_number(value) when is_float(value), do: {:ok, value}

  defp cast_number(value) when is_binary(value) do
    case Float.parse(value) do
      {number, ""} -> {:ok, number}
      _ -> {:error, :invalid_parameter}
    end
  end

  defp cast_number(_value), do: {:error, :invalid_parameter}
end
