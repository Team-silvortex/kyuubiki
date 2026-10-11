defmodule KyuubikiWeb.Storage.MemorySnapshot do
  @moduledoc false

  @generation {__MODULE__, :generation}
  @context {__MODULE__, :context}
  @max_attempts 3

  def start_link(initializer, options) do
    Agent.start_link(
      fn ->
        Process.put(@generation, make_ref())
        initializer.()
      end,
      options
    )
  end

  def get(store, reader) do
    case Process.get(@context) do
      nil -> Agent.get(store, reader)
      context -> reader.(captured_state(store, context))
    end
  end

  def update(store, change),
    do: get_and_update(store, fn state -> {:ok, change.(state)} end)

  def get_and_update(store, change) do
    if Process.get(@context), do: throw({__MODULE__, :write_in_snapshot})

    Agent.get_and_update(store, fn state ->
      {reply, updated} = change.(state)
      # Every completed write attempt advances the token, including no-ops.
      # Fresh references also distinguish ABA changes and process incarnations.
      Process.put(@generation, make_ref())
      {reply, updated}
    end)
  end

  def run(callback) when is_function(callback, 0) do
    if Process.get(@context), do: callback.(), else: attempt(callback, 1)
  catch
    {__MODULE__, :write_in_snapshot} -> {:error, :analysis_store_unavailable}
  end

  defp attempt(callback, number) do
    case read_once(callback, number) do
      {:stable, reply} -> reply
      :changed when number < @max_attempts -> attempt(callback, number + 1)
      :changed -> {:error, :analysis_store_unavailable}
    end
  end

  defp read_once(callback, number) do
    Process.put(@context, %{stores: %{}, attempt: number})

    try do
      reply = callback.()

      # All captures precede validation. Unchanged unique tokens prove the
      # captured states coexisted at the last capture, even across stores.
      stable? =
        Enum.all?(Process.get(@context).stores, fn {store, {generation, _state}} ->
          generation == Agent.get(store, fn _state -> Process.get(@generation) end)
        end)

      if stable?, do: {:stable, reply}, else: :changed
    after
      Process.delete(@context)
    end
  end

  defp captured_state(store, context) do
    case Map.fetch(context.stores, store) do
      {:ok, {_generation, state}} ->
        state

      :error ->
        {generation, state} = Agent.get(store, fn state -> {Process.get(@generation), state} end)
        stores = Map.put(context.stores, store, {generation, state})
        Process.put(@context, %{context | stores: stores})

        :telemetry.execute(
          [:kyuubiki, :memory_snapshot, :capture],
          %{},
          %{store: store, attempt: context.attempt}
        )

        state
    end
  end
end
