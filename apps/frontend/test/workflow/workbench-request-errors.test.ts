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
