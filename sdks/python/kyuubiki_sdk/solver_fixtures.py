from __future__ import annotations

from copy import deepcopy
from typing import Any


def minimal_solver_payloads() -> dict[str, dict[str, Any]]:
    """Return small stable solver inputs keyed by Python SDK solve kind."""
    return deepcopy(_PAYLOADS)


def solver_fixture_rpc_methods() -> dict[str, str]:
    """Return RPC method to solve-kind mappings for available fixtures."""
    return {
        "solve_stokes_flow_plane_quad_2d": "stokes_flow_quad_2d",
        "solve_stokes_flow_plane_triangle_2d": "stokes_flow_triangle_2d",
        **{
            f"solve_{kind}": kind
            for kind in _PAYLOADS
            if kind not in {"stokes_flow_quad_2d", "stokes_flow_triangle_2d"}
        },
    }


_BAR_1D = {
    "length": 1.0,
    "area": 0.01,
    "youngs_modulus": 210_000_000_000.0,
    "elements": 2,
    "tip_force": 1200.0,
}

_HEAT_BAR_1D = {
    "nodes": [
        {"id": "h0", "x": 0.0, "fix_temperature": True, "temperature": 100.0, "heat_load": 0.0},
        {"id": "h1", "x": 1.0, "fix_temperature": True, "temperature": 20.0, "heat_load": 0.0},
    ],
    "elements": [{"id": "he0", "node_i": 0, "node_j": 1, "area": 0.02, "conductivity": 45.0}],
}

_SPRING_1D = {
    "nodes": [
        {"id": "s0", "x": 0.0, "fix_x": True, "load_x": 0.0},
        {"id": "s1", "x": 1.2, "fix_x": False, "load_x": 0.0},
        {"id": "s2", "x": 2.4, "fix_x": False, "load_x": 1200.0},
    ],
    "elements": [
        {"id": "se0", "node_i": 0, "node_j": 1, "stiffness": 10_000.0},
        {"id": "se1", "node_i": 1, "node_j": 2, "stiffness": 10_000.0},
    ],
}

_SPRING_2D = {
    "nodes": [
        {"id": "s0", "x": 0.0, "y": 0.0, "fix_x": True, "fix_y": True, "load_x": 0.0, "load_y": 0.0},
        {"id": "s1", "x": 1.0, "y": 0.0, "fix_x": False, "fix_y": True, "load_x": 100.0, "load_y": 0.0},
    ],
    "elements": [{"id": "k0", "node_i": 0, "node_j": 1, "stiffness": 10_000.0}],
}

_SPRING_3D = {
    "nodes": [
        {"id": "s0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": True, "fix_y": True, "fix_z": True, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0},
        {"id": "s1", "x": 1.0, "y": 0.0, "z": 0.0, "fix_x": False, "fix_y": True, "fix_z": True, "load_x": 100.0, "load_y": 0.0, "load_z": 0.0},
    ],
    "elements": [{"id": "k0", "node_i": 0, "node_j": 1, "stiffness": 10_000.0}],
}

_DYNAMIC_SPRING_1D = {
    "nodes": [
        {"id": "fixed", "x": 0.0, "fix_x": True, "load_x": 0.0, "mass": 1.0},
        {"id": "tip", "x": 1.0, "fix_x": False, "load_x": 10.0, "mass": 2.0},
    ],
    "elements": [{"id": "s0", "node_i": 0, "node_j": 1, "stiffness": 100.0, "damping": 0.5}],
}

_NONLINEAR_SPRING_1D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "fix_x": True, "load_x": 0.0},
        {"id": "n1", "x": 1.0, "fix_x": False, "load_x": 1000.0},
    ],
    "elements": [{"id": "s0", "node_i": 0, "node_j": 1, "stiffness": 25_000.0, "cubic_stiffness": 10_000.0}],
    "load_steps": 4,
    "max_iterations": 16,
    "tolerance": 1.0e-10,
}

_ELECTROSTATIC_BAR_1D = {
    "nodes": [
        {"id": "e0", "x": 0.0, "fix_potential": True, "potential": 12.0, "charge_density": 0.0},
        {"id": "e1", "x": 1.0, "fix_potential": True, "potential": 0.0, "charge_density": 0.0},
    ],
    "elements": [{"id": "ee0", "node_i": 0, "node_j": 1, "area": 0.01, "permittivity": 8.8541878128e-12}],
}

_ELECTROSTATIC_PLANE_TRIANGLE_2D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "y": 0.0, "fix_potential": True, "potential": 10.0, "charge_density": 0.0},
        {"id": "n1", "x": 1.0, "y": 0.0, "fix_potential": True, "potential": 0.0, "charge_density": 0.0},
        {"id": "n2", "x": 0.0, "y": 1.0, "fix_potential": True, "potential": 10.0, "charge_density": 0.0},
    ],
    "elements": [{"id": "ep0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.05, "permittivity": 2.0}],
}

_ELECTROSTATIC_PLANE_QUAD_2D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "y": 0.0, "fix_potential": True, "potential": 10.0, "charge_density": 0.0},
        {"id": "n1", "x": 1.0, "y": 0.0, "fix_potential": True, "potential": 0.0, "charge_density": 0.0},
        {"id": "n2", "x": 1.0, "y": 1.0, "fix_potential": True, "potential": 0.0, "charge_density": 0.0},
        {"id": "n3", "x": 0.0, "y": 1.0, "fix_potential": True, "potential": 10.0, "charge_density": 0.0},
    ],
    "elements": [{"id": "epq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.05, "permittivity": 2.0}],
}

_MAGNETOSTATIC_PLANE_TRIANGLE_2D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "y": 0.0, "fix_vector_potential": True, "vector_potential": 10.0, "current_density": 0.0},
        {"id": "n1", "x": 1.0, "y": 0.0, "fix_vector_potential": True, "vector_potential": 0.0, "current_density": 0.0},
        {"id": "n2", "x": 0.0, "y": 1.0, "fix_vector_potential": True, "vector_potential": 10.0, "current_density": 0.0},
    ],
    "elements": [{"id": "mp0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.05, "permeability": 2.0}],
}

_MAGNETOSTATIC_PLANE_QUAD_2D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "y": 0.0, "fix_vector_potential": True, "vector_potential": 10.0, "current_density": 0.0},
        {"id": "n1", "x": 1.0, "y": 0.0, "fix_vector_potential": True, "vector_potential": 0.0, "current_density": 0.0},
        {"id": "n2", "x": 1.0, "y": 1.0, "fix_vector_potential": True, "vector_potential": 0.0, "current_density": 0.0},
        {"id": "n3", "x": 0.0, "y": 1.0, "fix_vector_potential": True, "vector_potential": 10.0, "current_density": 0.0},
    ],
    "elements": [{"id": "mpq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.05, "permeability": 2.0}],
}

_HEAT_PLANE_TRIANGLE_2D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "y": 0.0, "fix_temperature": True, "temperature": 100.0, "heat_load": 0.0},
        {"id": "n1", "x": 1.0, "y": 0.0, "fix_temperature": True, "temperature": 0.0, "heat_load": 0.0},
        {"id": "n2", "x": 0.0, "y": 1.0, "fix_temperature": False, "temperature": 0.0, "heat_load": 0.0},
    ],
    "elements": [{"id": "hp0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "conductivity": 10.0}],
}

_HEAT_PLANE_QUAD_2D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "y": 0.0, "fix_temperature": True, "temperature": 100.0, "heat_load": 0.0},
        {"id": "n1", "x": 1.0, "y": 0.0, "fix_temperature": True, "temperature": 0.0, "heat_load": 0.0},
        {"id": "n2", "x": 1.0, "y": 1.0, "fix_temperature": False, "temperature": 0.0, "heat_load": 0.0},
        {"id": "n3", "x": 0.0, "y": 1.0, "fix_temperature": False, "temperature": 0.0, "heat_load": 0.0},
    ],
    "elements": [{"id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 10.0}],
}

_TRUSS_2D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "y": 0.0, "fix_x": True, "fix_y": True, "load_x": 0.0, "load_y": 0.0},
        {"id": "n1", "x": 1.0, "y": 0.0, "fix_x": True, "fix_y": True, "load_x": 0.0, "load_y": 0.0},
        {"id": "n2", "x": 0.5, "y": 0.8, "fix_x": False, "fix_y": False, "load_x": 0.0, "load_y": -1000.0},
    ],
    "elements": [
        {"id": "e0", "node_i": 0, "node_j": 2, "area": 0.01, "youngs_modulus": 210_000_000_000.0},
        {"id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 210_000_000_000.0},
    ],
}

_THERMAL_TRUSS_2D = {
    "nodes": [{**node, "temperature_delta": 40.0 if node["id"] == "n2" else 20.0} for node in _TRUSS_2D["nodes"]],
    "elements": [{**element, "thermal_expansion": 0.000012} for element in _TRUSS_2D["elements"]],
}

_PLANE_TRIANGLE_2D = {
    "nodes": [
        {"id": "p0", "x": 0.0, "y": 0.0, "fix_x": True, "fix_y": True, "load_x": 0.0, "load_y": 0.0},
        {"id": "p1", "x": 1.0, "y": 0.0, "fix_x": False, "fix_y": True, "load_x": 0.0, "load_y": 0.0},
        {"id": "p2", "x": 0.0, "y": 1.0, "fix_x": False, "fix_y": False, "load_x": 0.0, "load_y": -1000.0},
    ],
    "elements": [{"id": "pt0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70_000_000_000.0, "poisson_ratio": 0.33}],
}

_PLANE_QUAD_2D = {
    "nodes": [
        {"id": "q0", "x": 0.0, "y": 0.0, "fix_x": True, "fix_y": True, "load_x": 0.0, "load_y": 0.0},
        {"id": "q1", "x": 1.0, "y": 0.0, "fix_x": False, "fix_y": True, "load_x": 0.0, "load_y": 0.0},
        {"id": "q2", "x": 1.0, "y": 1.0, "fix_x": False, "fix_y": False, "load_x": 0.0, "load_y": -1000.0},
        {"id": "q3", "x": 0.0, "y": 1.0, "fix_x": True, "fix_y": False, "load_x": 0.0, "load_y": 0.0},
    ],
    "elements": [{"id": "pq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70_000_000_000.0, "poisson_ratio": 0.33}],
}

_BEAM_1D = {
    "nodes": [
        {"id": "b0", "x": 0.0, "fix_y": True, "fix_rz": True, "load_y": 0.0, "moment_z": 0.0},
        {"id": "b1", "x": 2.0, "fix_y": False, "fix_rz": False, "load_y": -1000.0, "moment_z": 0.0},
    ],
    "elements": [{"id": "be0", "node_i": 0, "node_j": 1, "youngs_modulus": 210_000_000_000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.00016}],
}

_FRAME_2D = {
    "nodes": [
        {"id": "f0", "x": 0.0, "y": 0.0, "fix_x": True, "fix_y": True, "fix_rz": True, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
        {"id": "f1", "x": 2.0, "y": 0.0, "fix_x": False, "fix_y": False, "fix_rz": False, "load_x": 0.0, "load_y": -1000.0, "moment_z": 0.0},
    ],
    "elements": [{"id": "fe0", "node_i": 0, "node_j": 1, "area": 0.02, "youngs_modulus": 210_000_000_000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.00016}],
}

_TRUSS_3D = {
    "nodes": [
        {"id": "n0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": True, "fix_y": True, "fix_z": True, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0},
        {"id": "n1", "x": 1.0, "y": 0.0, "z": 0.0, "fix_x": True, "fix_y": True, "fix_z": True, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0},
        {"id": "n2", "x": 0.0, "y": 1.0, "z": 0.0, "fix_x": True, "fix_y": True, "fix_z": True, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0},
        {"id": "n3", "x": 0.3, "y": 0.3, "z": 1.0, "fix_x": False, "fix_y": False, "fix_z": False, "load_x": 0.0, "load_y": 0.0, "load_z": -1000.0},
    ],
    "elements": [
        {"id": "e0", "node_i": 0, "node_j": 3, "area": 0.01, "youngs_modulus": 210_000_000_000.0},
        {"id": "e1", "node_i": 1, "node_j": 3, "area": 0.01, "youngs_modulus": 210_000_000_000.0},
        {"id": "e2", "node_i": 2, "node_j": 3, "area": 0.01, "youngs_modulus": 210_000_000_000.0},
    ],
}

_FRAME_3D = {
    "nodes": [
        {"id": "f0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": True, "fix_y": True, "fix_z": True, "fix_rx": True, "fix_ry": True, "fix_rz": True, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "moment_x": 0.0, "moment_y": 0.0, "moment_z": 0.0},
        {"id": "f1", "x": 2.0, "y": 0.0, "z": 0.0, "fix_x": False, "fix_y": False, "fix_z": False, "fix_rx": False, "fix_ry": False, "fix_rz": False, "load_x": 0.0, "load_y": -1000.0, "load_z": 0.0, "moment_x": 0.0, "moment_y": 0.0, "moment_z": 0.0},
    ],
    "elements": [{"id": "f30", "node_i": 0, "node_j": 1, "area": 0.02, "youngs_modulus": 210_000_000_000.0, "shear_modulus": 80_000_000_000.0, "torsion_constant": 0.000005, "moment_of_inertia_y": 0.000008, "moment_of_inertia_z": 0.000008, "section_modulus_y": 0.00016, "section_modulus_z": 0.00016}],
}

_PAYLOADS = {
    "bar_1d": _BAR_1D,
    "thermal_bar_1d": {
        "nodes": [{"id": "n0", "x": 0.0, "fix_x": True, "load_x": 0.0, "temperature_delta": 0.0}, {"id": "n1", "x": 1.0, "fix_x": True, "load_x": 0.0, "temperature_delta": 35.0}],
        "elements": [{"id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210_000_000_000.0, "thermal_expansion": 0.000012}],
    },
    "heat_bar_1d": _HEAT_BAR_1D,
    "electrostatic_bar_1d": _ELECTROSTATIC_BAR_1D,
    "magnetostatic_bar_1d": {
        "nodes": [
            {"id": "n0", "x": 0.0, "fix_magnetic_potential": True, "magnetic_potential": 10.0, "magnetomotive_source": 0.0},
            {"id": "n1", "x": 1.0, "fix_magnetic_potential": True, "magnetic_potential": 0.0, "magnetomotive_source": 0.0},
        ],
        "elements": [{"id": "mb0", "node_i": 0, "node_j": 1, "area": 0.02, "permeability": 2.0}],
    },
    "advection_diffusion_bar_1d": {
        "nodes": [
            {"id": "c0", "x": 0.0, "fix_concentration": True, "concentration": 1.0, "source": 0.0},
            {"id": "c1", "x": 1.0, "fix_concentration": True, "concentration": 0.1, "source": 0.0},
        ],
        "elements": [{"id": "cd0", "node_i": 0, "node_j": 1, "area": 0.01, "diffusivity": 0.05, "velocity": 0.1}],
    },
    "magnetostatic_plane_triangle_2d": _MAGNETOSTATIC_PLANE_TRIANGLE_2D,
    "magnetostatic_plane_quad_2d": _MAGNETOSTATIC_PLANE_QUAD_2D,
    "electrostatic_plane_triangle_2d": _ELECTROSTATIC_PLANE_TRIANGLE_2D,
    "electrostatic_plane_quad_2d": _ELECTROSTATIC_PLANE_QUAD_2D,
    "heat_plane_triangle_2d": _HEAT_PLANE_TRIANGLE_2D,
    "heat_plane_quad_2d": _HEAT_PLANE_QUAD_2D,
    "thermal_truss_2d": _THERMAL_TRUSS_2D,
    "spring_1d": _SPRING_1D,
    "transient_spring_1d": {
        **_DYNAMIC_SPRING_1D,
        "time_step": 0.01,
        "steps": 10,
    },
    "harmonic_spring_1d": {
        **_DYNAMIC_SPRING_1D,
        "frequencies_hz": [0.0, 0.5, 1.0],
    },
    "nonlinear_spring_1d": _NONLINEAR_SPRING_1D,
    "contact_gap_1d": {
        **_NONLINEAR_SPRING_1D,
        "contacts": [{"id": "c0", "node": 1, "gap": 0.01, "normal_stiffness": 100_000.0}],
    },
    "spring_2d": _SPRING_2D,
    "spring_3d": _SPRING_3D,
    "beam_1d": _BEAM_1D,
    "thermal_beam_1d": {
        "nodes": _BEAM_1D["nodes"],
        "elements": [{**_BEAM_1D["elements"][0], "thermal_expansion": 0.000012, "section_depth": 0.2, "temperature_gradient_y": 15.0}],
    },
    "torsion_1d": {
        "nodes": [{"id": "t0", "x": 0.0, "fix_rz": True, "torque_z": 0.0}, {"id": "t1", "x": 1.0, "fix_rz": False, "torque_z": 500.0}],
        "elements": [{"id": "te0", "node_i": 0, "node_j": 1, "shear_modulus": 80_000_000_000.0, "polar_moment": 0.000005, "section_modulus": 0.00016}],
    },
    "truss_2d": _TRUSS_2D,
    "truss_3d": _TRUSS_3D,
    "frame_2d": _FRAME_2D,
    "modal_frame_2d": {
        "nodes": [{**node, "load_y": 0.0} for node in _FRAME_2D["nodes"]],
        "elements": [{**_FRAME_2D["elements"][0], "density": 7850.0}],
        "mode_count": 2,
    },
    "frame_3d": _FRAME_3D,
    "modal_frame_3d": {
        "nodes": [{**node, "load_y": 0.0} for node in _FRAME_3D["nodes"]],
        "elements": [{**_FRAME_3D["elements"][0], "density": 7850.0}],
        "mode_count": 2,
    },
    "plane_triangle_2d": _PLANE_TRIANGLE_2D,
    "thermal_plane_triangle_2d": {
        "nodes": [{**node, "temperature_delta": 30.0} for node in _PLANE_TRIANGLE_2D["nodes"]],
        "elements": [{**_PLANE_TRIANGLE_2D["elements"][0], "thermal_expansion": 0.000011}],
    },
    "plane_quad_2d": _PLANE_QUAD_2D,
    "thermal_plane_quad_2d": {
        "nodes": [{**node, "temperature_delta": 30.0} for node in _PLANE_QUAD_2D["nodes"]],
        "elements": [{**_PLANE_QUAD_2D["elements"][0], "thermal_expansion": 0.000011}],
    },
    "thermal_truss_3d": {
        "nodes": [{**node, "temperature_delta": 40.0 if node["id"] == "n3" else 20.0} for node in _TRUSS_3D["nodes"]],
        "elements": [{**element, "thermal_expansion": 0.000012} for element in _TRUSS_3D["elements"]],
    },
    "thermal_frame_2d": {
        "nodes": [{**node, "temperature_delta": 20.0} for node in _FRAME_2D["nodes"]],
        "elements": [{**_FRAME_2D["elements"][0], "thermal_expansion": 0.000012, "section_depth": 0.2, "temperature_gradient_y": 5.0}],
    },
    "thermal_frame_3d": {
        "nodes": [{**node, "temperature_delta": 20.0} for node in _FRAME_3D["nodes"]],
        "elements": [{**_FRAME_3D["elements"][0], "thermal_expansion": 0.000012, "section_depth_y": 0.2, "section_depth_z": 0.2, "temperature_gradient_y": 5.0, "temperature_gradient_z": 0.0}],
    },
    "transient_heat_bar_1d": {
        "nodes": [
            {"id": "hot", "x": 0.0, "fix_temperature": True, "temperature": 100.0, "heat_load": 0.0},
            {"id": "mid", "x": 0.5, "fix_temperature": False, "temperature": 20.0, "heat_load": 0.0},
            {"id": "cold", "x": 1.0, "fix_temperature": True, "temperature": 0.0, "heat_load": 0.0},
        ],
        "elements": [
            {"id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "conductivity": 10.0, "density": 7800.0, "specific_heat": 450.0},
            {"id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "conductivity": 10.0, "density": 7800.0, "specific_heat": 450.0},
        ],
        "time_step": 0.1,
        "steps": 4,
    },
    "acoustic_bar_1d": {
        "frequency_hz": 100.0,
        "nodes": [
            {"id": "a0", "x": 0.0, "fix_pressure": True, "pressure": 1.0, "volume_velocity_source": 0.0},
            {"id": "a1", "x": 1.0, "fix_pressure": False, "pressure": 0.0, "volume_velocity_source": 0.01},
        ],
        "elements": [{"id": "ae0", "node_i": 0, "node_j": 1, "area": 0.1, "density": 1.2, "bulk_modulus": 142000.0, "damping_ratio": 0.02}],
    },
    "stokes_flow_quad_2d": {
        "nodes": [
            {"id": "n0", "x": 0.0, "y": 0.0, "fix_velocity_x": True, "velocity_x": 0.0, "fix_velocity_y": True, "velocity_y": 0.0, "fix_pressure": True, "pressure": 1.0, "body_force_x": 0.0, "body_force_y": 0.0},
            {"id": "n1", "x": 1.0, "y": 0.0, "fix_velocity_x": False, "velocity_x": 0.0, "fix_velocity_y": True, "velocity_y": 0.0, "fix_pressure": False, "pressure": 0.0, "body_force_x": 2.0, "body_force_y": 0.0},
            {"id": "n2", "x": 1.0, "y": 1.0, "fix_velocity_x": False, "velocity_x": 0.0, "fix_velocity_y": False, "velocity_y": 0.0, "fix_pressure": False, "pressure": 0.0, "body_force_x": 2.0, "body_force_y": 0.5},
            {"id": "n3", "x": 0.0, "y": 1.0, "fix_velocity_x": True, "velocity_x": 0.0, "fix_velocity_y": True, "velocity_y": 0.0, "fix_pressure": False, "pressure": 0.0, "body_force_x": 0.0, "body_force_y": 0.0},
        ],
        "elements": [{"id": "sf0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.1, "viscosity": 2.0, "density": 1.0}],
    },
    "stokes_flow_triangle_2d": {
        "nodes": [
            {"id": "n0", "x": 0.0, "y": 0.0, "fix_velocity_x": True, "velocity_x": 0.0, "fix_velocity_y": True, "velocity_y": 0.0, "fix_pressure": True, "pressure": 1.0, "body_force_x": 0.0, "body_force_y": 0.0},
            {"id": "n1", "x": 1.0, "y": 0.0, "fix_velocity_x": False, "velocity_x": 0.0, "fix_velocity_y": True, "velocity_y": 0.0, "fix_pressure": False, "pressure": 0.0, "body_force_x": 2.0, "body_force_y": 0.0},
            {"id": "n2", "x": 0.0, "y": 1.0, "fix_velocity_x": True, "velocity_x": 0.0, "fix_velocity_y": False, "velocity_y": 0.0, "fix_pressure": False, "pressure": 0.0, "body_force_x": 0.0, "body_force_y": 0.5},
        ],
        "elements": [{"id": "sf_tri0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.1, "viscosity": 2.0, "density": 1.0}],
    },
    "solid_tetra_3d": {
        "nodes": [
            {"id": "n0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": True, "fix_y": True, "fix_z": True, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0},
            {"id": "n1", "x": 1.0, "y": 0.0, "z": 0.0, "fix_x": True, "fix_y": True, "fix_z": True, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0},
            {"id": "n2", "x": 0.0, "y": 1.0, "z": 0.0, "fix_x": True, "fix_y": True, "fix_z": True, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0},
            {"id": "n3", "x": 0.0, "y": 0.0, "z": 1.0, "fix_x": False, "fix_y": False, "fix_z": False, "load_x": 0.0, "load_y": 0.0, "load_z": -1000.0},
        ],
        "elements": [{"id": "t0", "node_a": 0, "node_b": 1, "node_c": 2, "node_d": 3, "youngs_modulus": 70_000_000_000.0, "poisson_ratio": 0.33}],
    },
}
