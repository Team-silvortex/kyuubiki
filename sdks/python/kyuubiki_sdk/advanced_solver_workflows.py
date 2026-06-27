from __future__ import annotations

from typing import Any

from .workflow_builders import (
    build_workflow_defaults,
    build_workflow_edge,
    build_workflow_graph,
    build_workflow_node,
    build_workflow_port,
)


def build_modal_frame_2d_workflow(
    graph_id: str = "workflow.modal-frame-2d",
    *,
    name: str = "Modal frame 2d",
    version: str = "1.0.0",
    orchestrated: bool = True,
) -> dict[str, Any]:
    return build_single_solver_workflow(
        graph_id,
        name=name,
        version=version,
        operator_id="solve.modal_frame_2d",
        input_artifact_type="study_model/modal_frame_2d",
        result_artifact_type="result/modal_frame_2d",
        orchestrated=orchestrated,
    )


def build_modal_frame_3d_workflow(
    graph_id: str = "workflow.modal-frame-3d",
    *,
    name: str = "Modal frame 3d",
    version: str = "1.0.0",
    orchestrated: bool = True,
) -> dict[str, Any]:
    return build_single_solver_workflow(
        graph_id,
        name=name,
        version=version,
        operator_id="solve.modal_frame_3d",
        input_artifact_type="study_model/modal_frame_3d",
        result_artifact_type="result/modal_frame_3d",
        orchestrated=orchestrated,
    )


def build_nonlinear_spring_1d_workflow(
    graph_id: str = "workflow.nonlinear-spring-1d",
    *,
    name: str = "Nonlinear spring 1d",
    version: str = "1.0.0",
    orchestrated: bool = True,
) -> dict[str, Any]:
    return build_single_solver_workflow(
        graph_id,
        name=name,
        version=version,
        operator_id="solve.nonlinear_spring_1d",
        input_artifact_type="study_model/nonlinear_spring_1d",
        result_artifact_type="result/nonlinear_spring_1d",
        orchestrated=orchestrated,
    )


def build_contact_gap_1d_workflow(
    graph_id: str = "workflow.contact-gap-1d",
    *,
    name: str = "Contact gap 1d",
    version: str = "1.0.0",
    orchestrated: bool = True,
) -> dict[str, Any]:
    return build_single_solver_workflow(
        graph_id,
        name=name,
        version=version,
        operator_id="solve.contact_gap_1d",
        input_artifact_type="study_model/contact_gap_1d",
        result_artifact_type="result/contact_gap_1d",
        orchestrated=orchestrated,
    )


def build_magnetostatic_plane_triangle_2d_workflow(
    graph_id: str = "workflow.magnetostatic-plane-triangle-2d",
    *,
    name: str = "Magnetostatic plane triangle 2d",
    version: str = "1.0.0",
    orchestrated: bool = True,
) -> dict[str, Any]:
    return build_single_solver_workflow(
        graph_id,
        name=name,
        version=version,
        operator_id="solve.magnetostatic_plane_triangle_2d",
        input_artifact_type="study_model/magnetostatic_plane_triangle_2d",
        result_artifact_type="result/magnetostatic_plane_triangle_2d",
        orchestrated=orchestrated,
    )


def build_magnetostatic_plane_quad_2d_workflow(
    graph_id: str = "workflow.magnetostatic-plane-quad-2d",
    *,
    name: str = "Magnetostatic plane quad 2d",
    version: str = "1.0.0",
    orchestrated: bool = True,
) -> dict[str, Any]:
    return build_single_solver_workflow(
        graph_id,
        name=name,
        version=version,
        operator_id="solve.magnetostatic_plane_quad_2d",
        input_artifact_type="study_model/magnetostatic_plane_quad_2d",
        result_artifact_type="result/magnetostatic_plane_quad_2d",
        orchestrated=orchestrated,
    )


def build_single_solver_workflow(
    graph_id: str,
    *,
    name: str,
    version: str,
    operator_id: str,
    input_artifact_type: str,
    result_artifact_type: str,
    orchestrated: bool = True,
) -> dict[str, Any]:
    return build_workflow_graph(
        graph_id,
        name=name,
        version=version,
        entry_nodes=["input"],
        output_nodes=["output"],
        defaults=build_workflow_defaults(cache_policy="cached", orchestrated=orchestrated),
        nodes=[
            build_workflow_node(
                "input",
                kind="input",
                inputs=[],
                outputs=[build_workflow_port("model", artifact_type=input_artifact_type)],
            ),
            build_workflow_node(
                "solve",
                kind="solve",
                operator_id=operator_id,
                inputs=[build_workflow_port("model", artifact_type=input_artifact_type)],
                outputs=[build_workflow_port("result", artifact_type=result_artifact_type)],
            ),
            build_workflow_node(
                "output",
                kind="output",
                inputs=[build_workflow_port("result", artifact_type=result_artifact_type)],
                outputs=[],
            ),
        ],
        edges=[
            build_workflow_edge(
                "input_to_solve",
                from_node="input",
                from_port="model",
                to_node="solve",
                to_port="model",
                artifact_type=input_artifact_type,
            ),
            build_workflow_edge(
                "solve_to_output",
                from_node="solve",
                from_port="result",
                to_node="output",
                to_port="result",
                artifact_type=result_artifact_type,
            ),
        ],
    )
