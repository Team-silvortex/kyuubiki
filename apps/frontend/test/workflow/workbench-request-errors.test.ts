import test from "node:test";
import assert from "node:assert/strict";

import {
  createHttpWorkbenchRequestError,
  normalizeWorkbenchRequestError,
} from "../../src/lib/api/request-errors.ts";

test("WebView and browser network failures normalize as retryable offline errors", () => {
  for (const message of ["Load failed", "Failed to fetch", "Network request failed", "NetworkError"]) {
    const normalized = normalizeWorkbenchRequestError(new TypeError(message), "/api/health");

    assert.equal(normalized.kind, "offline", message);
    assert.equal(normalized.retryable, true, message);
  }
});

test("uncertain direct mutations never offer blind resubmission after network or HTTP failure", () => {
  for (const path of ["/api/direct-mesh/tasks", "/api/direct-mesh/tasks/cancel", "/api/direct-mesh/solve"]) {
    for (const url of [path, `https://runtime.example${path}?view=recovery`, `https://runtime.example/research${path}/`]) {
      const failures = [
        normalizeWorkbenchRequestError(new TypeError("Failed to fetch"), url),
        normalizeWorkbenchRequestError(new Error("request timed out"), url),
        ...[408, 429, 500, 502, 503].map(statusCode => createHttpWorkbenchRequestError({ message: "failure", statusCode, url })),
      ];
      for (const error of failures) {
        assert.equal(error.retryable, false, url);
        assert.match(error.recoveryHint, /original Agent/);
        assert.match(error.recoveryHint, /[Nn]o automatic replay|[Dd]o not resubmit/);
      }
    }
  }
});

test("read-only recovery can be retried without implying task failure or owner transfer", () => {
  for (const suffix of ["inspect", "list"]) {
    const error = createHttpWorkbenchRequestError({ message: "busy", statusCode: 502,
      agentErrorCode: "durable_execution_busy", url: `/api/direct-mesh/tasks/${suffix}` });
    assert.equal(error.agentErrorCode, "durable_execution_busy");
    assert.equal(error.retryable, true);
    assert.match(error.recoveryHint, /original Agent/);
    assert.match(error.recoveryHint, /[Rr]ecovery/);
    assert.match(error.recoveryHint, /[Nn]o automatic replay|[Dd]o not resubmit/);
  }
});

test("transient HTTP responses remain retryable", () => {
  for (const statusCode of [408, 425, 429, 500, 503]) {
    const error = createHttpWorkbenchRequestError({
      message: `request failed: ${statusCode}`,
      statusCode,
      url: "/api/health",
    });

    assert.equal(error.retryable, true, String(statusCode));
  }
});

test("authorization and missing-resource failures are not blindly retried", () => {
  for (const statusCode of [400, 401, 403, 404]) {
    const error = createHttpWorkbenchRequestError({
      message: `request failed: ${statusCode}`,
      statusCode,
      url: "/api/health",
    });

    assert.equal(error.retryable, false, String(statusCode));
  }
});
