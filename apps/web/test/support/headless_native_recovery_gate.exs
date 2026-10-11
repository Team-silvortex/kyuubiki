defmodule KyuubikiWeb.TestSupport.HeadlessNativeRecoveryGate do
  @moduledoc false
  alias KyuubikiWeb.Playground.AgentClient

  def run_operator_task_ir(task, opts, progress) do
    root = System.fetch_env!("KYUUBIKI_HEADLESS_LIVE_NATIVE_GATE_ROOT")
    marker = Path.join(root, "native-recovery-gate")

    if get_in(task, ["operator", "kind"]) == "extract" do
      publish_snapshot(root, "native-recovery-diagnostic.json", task)
    end

    if get_in(task, ["operator", "kind"]) == "transform" and File.regular?(marker) do
      # A deterministic test-only pause before journal/RPC admission, never a fake receipt.
      publish_snapshot(
        root,
        "native-recovery-held.json",
        %{
          "job_id" => opts[:job_id],
          "task_id" => task["task_id"],
          "task_digest" => task["integrity"]["task_digest"],
          "task_ir" => task
        }
      )

      wait(marker, System.monotonic_time(:millisecond) + 25_000)
    end

    AgentClient.run_operator_task_ir(task, opts, progress)
  end

  def publish_snapshot(root, name, value)
      when name in ["native-recovery-diagnostic.json", "native-recovery-held.json"] do
    path = Path.join(root, name)
    temporary = path <> ".next-" <> Integer.to_string(System.unique_integer([:positive]))

    try do
      # A reader must see the complete capture, not File.write!'s initial empty file.
      File.write!(temporary, Jason.encode!(value))
      File.rename!(temporary, path)
    after
      File.rm(temporary)
    end
  end

  defp wait(marker, deadline) do
    if File.regular?(marker) do
      if System.monotonic_time(:millisecond) >= deadline,
        do: raise("native recovery gate timed out")

      Process.sleep(10)
      wait(marker, deadline)
    end
  end
end
