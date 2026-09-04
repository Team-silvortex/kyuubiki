from __future__ import annotations

import unittest

from kyuubiki_sdk.solver_fixtures import (
    minimal_solver_payloads,
    solver_fixture_rpc_methods,
)
from kyuubiki_sdk.solver_rpc import _SOLVER_METHODS


class SolverFixturesTest(unittest.TestCase):
    def test_returns_deep_copies(self) -> None:
        first = minimal_solver_payloads()
        second = minimal_solver_payloads()

        first["bar_1d"]["length"] = 99.0

        self.assertEqual(second["bar_1d"]["length"], 1.0)

    def test_covers_core_remote_agent_methods(self) -> None:
        methods = solver_fixture_rpc_methods()

        self.assertEqual(methods["solve_bar_1d"], "bar_1d")
        self.assertEqual(methods["solve_heat_bar_1d"], "heat_bar_1d")
        self.assertEqual(methods["solve_spring_2d"], "spring_2d")
        self.assertEqual(methods["solve_stokes_flow_plane_quad_2d"], "stokes_flow_quad_2d")
        self.assertEqual(
            methods["solve_stokes_flow_plane_triangle_2d"],
            "stokes_flow_triangle_2d",
        )

    def test_payloads_are_non_empty(self) -> None:
        payloads = minimal_solver_payloads()

        self.assertIn("bar_1d", payloads)
        self.assertIn("nodes", payloads["spring_3d"])
        self.assertEqual(payloads["transient_spring_1d"]["history_stride"], 2)
        self.assertEqual(payloads["transient_heat_bar_1d"]["history_stride"], 2)
        self.assertGreaterEqual(len(payloads), 40)

    def test_material_frame_fixture_exercises_the_fiber_section_contract(self) -> None:
        payload = minimal_solver_payloads()["frame_2d_material_p_delta"]

        self.assertEqual(payload["load_factor_schedule"], [1.2, 0.0, -1.2])
        self.assertEqual(len(payload["materials"]), 2)
        for material in payload["materials"]:
            self.assertEqual(material["longitudinal_integration_points"], 3)
            self.assertTrue(material["adaptive_longitudinal_integration"])
            self.assertEqual(material["longitudinal_integration_tolerance"], 1.0e-6)
            self.assertEqual(len(material["section_fibers"]), 2)
            self.assertAlmostEqual(
                sum(fiber["area"] for fiber in material["section_fibers"]),
                0.01,
            )

    def test_covers_python_solver_rpc_table(self) -> None:
        payloads = minimal_solver_payloads()
        fixture_methods = solver_fixture_rpc_methods()

        self.assertEqual(set(_SOLVER_METHODS), set(payloads))
        self.assertEqual(set(_SOLVER_METHODS.values()), set(fixture_methods))


if __name__ == "__main__":
    unittest.main()
