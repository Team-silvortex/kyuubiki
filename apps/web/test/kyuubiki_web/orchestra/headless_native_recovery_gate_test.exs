defmodule KyuubikiWeb.Orchestra.HeadlessNativeRecoveryGateTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.TestSupport.HeadlessNativeRecoveryGate, as: Gate

  test "live readers only observe complete atomically published recovery captures" do
    root =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-native-gate-#{Base.encode16(:crypto.strong_rand_bytes(12))}"
      )

    File.mkdir!(root)
    on_exit(fn -> File.rm_rf!(root) end)
    name = "native-recovery-held.json"
    payload = %{"padding" => String.duplicate("x", 32_000), "values" => [1, 1.0, -0.0, 1.0e-30]}
    Gate.publish_snapshot(root, name, %{"iteration" => 0, "task_ir" => payload})

    writer =
      Task.async(fn ->
        for iteration <- 1..32 do
          Gate.publish_snapshot(root, name, %{"iteration" => iteration, "task_ir" => payload})
          Process.sleep(1)
        end
      end)

    observe(Path.join(root, name), payload, writer.pid)
    Task.await(writer)
    assert Jason.decode!(File.read!(Path.join(root, name)))["iteration"] == 32
    assert File.ls!(root) == [name]
  end

  defp observe(path, payload, writer) do
    value = Jason.decode!(File.read!(path))
    assert value["task_ir"] === payload
    assert <<Enum.at(value["task_ir"]["values"], 2)::float-64>> == <<128, 0, 0, 0, 0, 0, 0, 0>>

    if Process.alive?(writer) do
      Process.sleep(1)
      observe(path, payload, writer)
    end
  end
end
