from __future__ import annotations

import json
import urllib.parse
import urllib.request
from typing import Any
from urllib.error import HTTPError, URLError

from .auth import KyuubikiAuth
from .errors import KyuubikiHttpError, KyuubikiTransportError


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
