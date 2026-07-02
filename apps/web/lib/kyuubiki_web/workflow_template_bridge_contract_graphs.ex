defmodule KyuubikiWeb.WorkflowTemplateBridgeContractGraphs do
  @moduledoc false

  alias KyuubikiWeb.WorkflowTemplateBridgeContractQuadGraphs
  alias KyuubikiWeb.WorkflowTemplateBridgeContractThermoGraphs
  alias KyuubikiWeb.WorkflowTemplateBridgeContractTriangleGraphs

  defdelegate electrostatic_to_heat_quad_graph(), to: WorkflowTemplateBridgeContractQuadGraphs

  defdelegate electrostatic_to_heat_triangle_graph(),
    to: WorkflowTemplateBridgeContractTriangleGraphs

  defdelegate heat_to_thermo_quad_graph(), to: WorkflowTemplateBridgeContractThermoGraphs
end
