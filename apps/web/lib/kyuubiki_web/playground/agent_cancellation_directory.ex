defmodule KyuubikiWeb.Playground.AgentCancellationDirectory do
  @moduledoc "Current original-endpoint membership within a cancellation delivery-start budget."
  alias KyuubikiWeb.Playground.{AgentExecutionCancellation, AgentPool}

  def lookup(fingerprint, deadline, opts \\ []) do
    clock = Keyword.get(opts, :clock, fn -> System.monotonic_time(:millisecond) end)

    with {:ok, remaining} <- remaining_budget(deadline, clock) do
      configured =
        Keyword.get_lazy(opts, :endpoints, fn ->
          AgentPool.inspection_endpoints(min(remaining, 5_000))
        end)

      endpoint =
        Enum.find(
          configured,
          &(AgentExecutionCancellation.endpoint_fingerprint(&1) == fingerprint)
        )

      with {:ok, remaining} <- remaining_budget(deadline, clock) do
        case endpoint do
          nil -> {:error, "original_endpoint_not_configured"}
          endpoint -> {:ok, endpoint, remaining}
        end
      end
    end
  catch
    :exit, _ -> {:error, "endpoint_directory_unavailable"}
  end

  defp remaining_budget(deadline, clock) do
    case deadline - clock.() do
      remaining when remaining > 0 -> {:ok, remaining}
      _ -> {:error, "delivery_budget_exhausted"}
    end
  end
end
