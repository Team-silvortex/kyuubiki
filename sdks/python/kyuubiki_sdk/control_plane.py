from __future__ import annotations

import json
import urllib.request
from typing import Any


class ControlPlaneClient:
    def __init__(self, base_url: str, token: str | None = None) -> None:
        self.base_url = base_url.rstrip("/")
        self.token = token

    def _request(self, path: str, method: str = "GET", payload: dict[str, Any] | None = None) -> dict[str, Any]:
        url = f"{self.base_url}{path}"
        body = None if payload is None else json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(url, data=body, method=method)
        request.add_header("Content-Type", "application/json")
        if self.token:
            request.add_header("x-kyuubiki-token", self.token)
        with urllib.request.urlopen(request) as response:
            return json.loads(response.read().decode("utf-8"))

    def health(self) -> dict[str, Any]:
        return self._request("/api/health")

    def protocol(self) -> dict[str, Any]:
        return self._request("/api/v1/protocol")

    def agents(self) -> dict[str, Any]:
        return self._request("/api/v1/protocol/agents")

    def create_axial_bar_job(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self._request("/api/v1/fem/axial-bar/jobs", "POST", payload)

    def create_truss_2d_job(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self._request("/api/v1/fem/truss-2d/jobs", "POST", payload)

    def create_truss_3d_job(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self._request("/api/v1/fem/truss-3d/jobs", "POST", payload)

    def create_plane_triangle_2d_job(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self._request("/api/v1/fem/plane-triangle-2d/jobs", "POST", payload)

    def fetch_job(self, job_id: str) -> dict[str, Any]:
        return self._request(f"/api/v1/jobs/{job_id}")

    def cancel_job(self, job_id: str) -> dict[str, Any]:
        return self._request(f"/api/v1/jobs/{job_id}/cancel", "POST")
