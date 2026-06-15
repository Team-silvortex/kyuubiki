from __future__ import annotations

import json
import urllib.parse
import urllib.request
from typing import Any
from urllib.error import HTTPError, URLError

from .auth import KyuubikiAuth
from .errors import KyuubikiHttpError, KyuubikiTransportError

_FEM_JOB_PATHS: dict[str, str] = {
    "bar_1d": "/api/v1/fem/axial-bar/jobs",
    "thermal_bar_1d": "/api/v1/fem/thermal-bar-1d/jobs",
    "heat_bar_1d": "/api/v1/fem/heat-bar-1d/jobs",
    "electrostatic_bar_1d": "/api/v1/fem/electrostatic-bar-1d/jobs",
    "beam_1d": "/api/v1/fem/beam-1d/jobs",
    "thermal_beam_1d": "/api/v1/fem/thermal-beam-1d/jobs",
    "torsion_1d": "/api/v1/fem/torsion-1d/jobs",
    "spring_1d": "/api/v1/fem/spring-1d/jobs",
    "spring_2d": "/api/v1/fem/spring-2d/jobs",
    "spring_3d": "/api/v1/fem/spring-3d/jobs",
    "truss_2d": "/api/v1/fem/truss-2d/jobs",
    "thermal_truss_2d": "/api/v1/fem/thermal-truss-2d/jobs",
    "frame_2d": "/api/v1/fem/frame-2d/jobs",
    "thermal_frame_2d": "/api/v1/fem/thermal-frame-2d/jobs",
    "plane_triangle_2d": "/api/v1/fem/plane-triangle-2d/jobs",
    "heat_plane_triangle_2d": "/api/v1/fem/heat-plane-triangle-2d/jobs",
    "thermal_plane_triangle_2d": "/api/v1/fem/thermal-plane-triangle-2d/jobs",
    "electrostatic_plane_triangle_2d": "/api/v1/fem/electrostatic-plane-triangle-2d/jobs",
    "plane_quad_2d": "/api/v1/fem/plane-quad-2d/jobs",
    "heat_plane_quad_2d": "/api/v1/fem/heat-plane-quad-2d/jobs",
    "thermal_plane_quad_2d": "/api/v1/fem/thermal-plane-quad-2d/jobs",
    "electrostatic_plane_quad_2d": "/api/v1/fem/electrostatic-plane-quad-2d/jobs",
    "truss_3d": "/api/v1/fem/truss-3d/jobs",
    "thermal_truss_3d": "/api/v1/fem/thermal-truss-3d/jobs",
    "frame_3d": "/api/v1/fem/frame-3d/jobs",
    "thermal_frame_3d": "/api/v1/fem/thermal-frame-3d/jobs",
}

_SOLVE_KIND_ALIASES: dict[str, str] = {
    "axial_bar_1d": "bar_1d",
}


class ControlPlaneClient:
    def __init__(
        self,
        base_url: str,
        token: str | None = None,
        auth: KyuubikiAuth | None = None,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.auth = auth if auth is not None else (KyuubikiAuth.access_token(token) if token else None)

    def _request(
        self,
        path: str,
        method: str = "GET",
        payload: dict[str, Any] | None = None,
        query: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        url = f"{self.base_url}{path}"
        if query:
            url = f"{url}?{urllib.parse.urlencode(query)}"
        body = None if payload is None else json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(url, data=body, method=method)
        headers = {"Content-Type": "application/json"}
        if self.auth is not None:
            self.auth.apply(headers)
        for header_name, header_value in headers.items():
            request.add_header(header_name, header_value)
        try:
            with urllib.request.urlopen(request) as response:
                return json.loads(response.read().decode("utf-8"))
        except HTTPError as error:
            body_text = error.read().decode("utf-8", errors="replace")
            raise KyuubikiHttpError(error.code, body_text) from error
        except URLError as error:
            raise KyuubikiTransportError(str(error.reason)) from error

    def health(self) -> dict[str, Any]:
        return self._request("/api/health")

    def protocol(self) -> dict[str, Any]:
        return self._request("/api/v1/protocol")

    def agents(self) -> dict[str, Any]:
        return self._request("/api/v1/protocol/agents")

    def list_workflow_catalog(self) -> dict[str, Any]:
        return self._request("/api/v1/workflows/catalog")

    def fetch_workflow_catalog_workflow(self, workflow_id: str) -> dict[str, Any]:
        quoted_id = urllib.parse.quote(workflow_id, safe="")
        return self._request(f"/api/v1/workflows/catalog/{quoted_id}")

    def list_workflow_operators(self, query: dict[str, Any] | None = None) -> dict[str, Any]:
        return self._request("/api/v1/operators", query=query)

    def fetch_workflow_operator(self, operator_id: str) -> dict[str, Any]:
        quoted_id = urllib.parse.quote(operator_id, safe="")
        return self._request(f"/api/v1/operators/{quoted_id}")

    def submit_fem_job(self, solve_kind: str, payload: dict[str, Any]) -> dict[str, Any]:
        normalized = normalize_solve_kind(solve_kind)
        path = _FEM_JOB_PATHS.get(normalized)
        if path is None:
            raise ValueError(f"unsupported solve kind: {solve_kind}")
        return self._request(path, "POST", payload)

    def create_axial_bar_job(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.submit_fem_job("bar_1d", payload)

    def create_truss_2d_job(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.submit_fem_job("truss_2d", payload)

    def create_truss_3d_job(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.submit_fem_job("truss_3d", payload)

    def create_plane_triangle_2d_job(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.submit_fem_job("plane_triangle_2d", payload)

    def submit_workflow_catalog_job(
        self,
        workflow_id: str,
        input_artifacts: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        return self._request(
            f"/api/v1/workflows/catalog/{workflow_id}/jobs",
            "POST",
            {"input_artifacts": input_artifacts or {}},
        )

    def submit_workflow_graph_job(
        self,
        graph: dict[str, Any],
        input_artifacts: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        return self._request(
            "/api/v1/workflows/graph/jobs",
            "POST",
            {"graph": graph, "input_artifacts": input_artifacts or {}},
        )

    def fetch_job(self, job_id: str) -> dict[str, Any]:
        return self._request(f"/api/v1/jobs/{job_id}")

    def list_jobs(self) -> dict[str, Any]:
        return self._request("/api/v1/jobs")

    def update_job(self, job_id: str, payload: dict[str, Any]) -> dict[str, Any]:
        return self._request(f"/api/v1/jobs/{job_id}", "PATCH", payload)

    def cancel_job(self, job_id: str) -> dict[str, Any]:
        return self._request(f"/api/v1/jobs/{job_id}/cancel", "POST")

    def delete_job(self, job_id: str) -> dict[str, Any]:
        return self._request(f"/api/v1/jobs/{job_id}", "DELETE")

    def list_results(self) -> dict[str, Any]:
        return self._request("/api/v1/results")

    def fetch_result(self, job_id: str) -> dict[str, Any]:
        return self._request(f"/api/v1/results/{job_id}")

    def fetch_result_chunk(
        self,
        job_id: str,
        kind: str,
        *,
        offset: int | None = None,
        limit: int | None = None,
    ) -> dict[str, Any]:
        query: dict[str, Any] = {}
        if offset is not None:
            query["offset"] = offset
        if limit is not None:
            query["limit"] = limit
        return self._request(f"/api/v1/results/{job_id}/chunks/{kind}", query=query or None)

    def update_result(self, job_id: str, result: dict[str, Any]) -> dict[str, Any]:
        return self._request(f"/api/v1/results/{job_id}", "PATCH", {"result": result})

    def delete_result(self, job_id: str) -> dict[str, Any]:
        return self._request(f"/api/v1/results/{job_id}", "DELETE")

    def export_database(self) -> dict[str, Any]:
        return self._request("/api/v1/export/database")

    def export_security_events(self, query: dict[str, Any] | None = None) -> dict[str, Any]:
        return self._request("/api/v1/export/security-events", query=query)

    def export_security_events_csv(self, query: dict[str, Any] | None = None) -> str:
        url = f"{self.base_url}/api/v1/export/security-events.csv"
        if query:
            url = f"{url}?{urllib.parse.urlencode(query)}"
        request = urllib.request.Request(url, method="GET")
        headers = {"Content-Type": "application/json"}
        if self.auth is not None:
            self.auth.apply(headers)
        for header_name, header_value in headers.items():
            request.add_header(header_name, header_value)
        try:
            with urllib.request.urlopen(request) as response:
                return response.read().decode("utf-8")
        except HTTPError as error:
            body_text = error.read().decode("utf-8", errors="replace")
            raise KyuubikiHttpError(error.code, body_text) from error
        except URLError as error:
            raise KyuubikiTransportError(str(error.reason)) from error


def normalize_solve_kind(solve_kind: str) -> str:
    normalized = solve_kind.strip().lower()
    return _SOLVE_KIND_ALIASES.get(normalized, normalized)
