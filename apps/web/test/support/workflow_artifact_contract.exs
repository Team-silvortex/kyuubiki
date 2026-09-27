defmodule KyuubikiWeb.TestSupport.WorkflowArtifactContract do
  @moduledoc false
  @fixture Path.expand("../../../../tests/fixtures/workflow-artifact-contract.json", __DIR__)
           |> File.read!()
           |> Jason.decode!()

  def request, do: @fixture["request"]
  def cases, do: @fixture["cases"]

  def value(%{"kind" => "literal", "value" => value}), do: value
  def value(%{"kind" => "array", "size" => size}), do: List.duplicate(0, size)

  def value(%{"kind" => "nested", "size" => size}),
    do: Enum.reduce(1..size, 0, fn _, inner -> [inner] end)

  def value(%{"kind" => "string", "size" => size} = recipe),
    do: String.duplicate(Map.get(recipe, "text", "x"), size)

  def value(%{"kind" => "key", "size" => size} = recipe),
    do: %{String.duplicate(Map.get(recipe, "text", "x"), size) => 0}

  def value(%{"kind" => "late_nul", "size" => size}),
    do: List.duplicate(0, size) ++ [%{"bad" => <<0>>}]

  def node(request, id, fun) do
    update_in(request, ["graph", "nodes"], fn nodes ->
      Enum.map(nodes, &if(&1["id"] == id, do: fun.(&1), else: &1))
    end)
  end

  def exported(policy, large?) do
    request()
    |> node("producer", fn node ->
      Map.merge(node, %{
        "kind" => "export",
        "operator_id" => "export.summary_json",
        "config" => %{"on_error" => policy}
      })
    end)
    |> put_in(
      ["input_artifacts", "input"],
      %{"text" => if(large?, do: String.duplicate("\n", 250_000), else: "healthy")}
    )
  end
end
