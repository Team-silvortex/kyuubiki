from __future__ import annotations


class KyuubikiSdkError(Exception):
    pass


class KyuubikiTransportError(KyuubikiSdkError):
    pass


class KyuubikiHttpError(KyuubikiSdkError):
    def __init__(self, status_code: int, body: str) -> None:
        self.status_code = status_code
        self.body = body
        super().__init__(f"http {status_code}: {body}")


class KyuubikiRpcError(KyuubikiSdkError):
    def __init__(self, message: str, code: str | None = None) -> None:
        self.code = code
        self.message = message
        super().__init__(message if code is None else f"{code}: {message}")


class KyuubikiTimeoutError(KyuubikiSdkError):
    pass


def classify_error(error: Exception) -> str:
    if isinstance(error, KyuubikiTimeoutError):
        return "timeout"
    if isinstance(error, KyuubikiTransportError):
        return "transport"
    if isinstance(error, KyuubikiRpcError):
        return "rpc"
    if isinstance(error, KyuubikiHttpError):
        if error.status_code in (401, 403):
            return "auth"
        if error.status_code == 404:
            return "not_found"
        if error.status_code >= 500:
            return "server"
        return "http"
    return "unknown"
