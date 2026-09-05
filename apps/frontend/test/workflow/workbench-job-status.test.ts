import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import {
  isJobStatusDetailConsistent,
  isWorkflowJobStatusContractValid,
  normalizeWorkflowRunProgress,
  resolveJobStatusDetailTone,
  resolveWorkflowRunPollDisposition,
  resolveWorkflowRunStatusTone,
  type JobStatusDetail,
} from "../../src/lib/api/job-status.ts";

const ACTIVE_DETAIL: JobStatusDetail = {
  lifecycle: "active",
  active: true,
  terminal: false,
  failure_class: null,
  recoverable: false,
  timing: {
    phase: "execution",
    queue_wait_ms: 12,
    execution_elapsed_ms: 8,
    total_elapsed_ms: 20,
    queue_timeout_ms: 1_000,
    execution_timeout_ms: 5_000,
    effective_timeout_ms: 5_000,
    job_submission_deadline: "2026-08-27T00:00:01.000Z",
    execution_started_at: "2026-08-27T00:00:00.012Z",
    effective_deadline: "2026-08-27T00:00:05.012Z",
  },
};

test("unknown workflow statuses fail closed in the GUI", () => {
  assert.equal(resolveWorkflowRunStatusTone("mystery"), "risk");
});

test("status detail lifecycle contradictions are treated as contract failures", () => {
  const contradictory: JobStatusDetail = {
    ...ACTIVE_DETAIL,
    lifecycle: "terminal",
    active: false,
    terminal: true,
  };

  assert.equal(isJobStatusDetailConsistent("solving", contradictory), false);
  assert.equal(resolveWorkflowRunStatusTone("solving", "attached", contradictory), "risk");
  assert.equal(resolveJobStatusDetailTone(contradictory, "solving"), "risk");
});

test("valid active and completed status details remain healthy", () => {
  assert.equal(isJobStatusDetailConsistent("solving", ACTIVE_DETAIL), true);
  assert.equal(resolveWorkflowRunStatusTone("solving", "attached", ACTIVE_DETAIL), "watch");

  const completed: JobStatusDetail = {
    ...ACTIVE_DETAIL,
    lifecycle: "terminal",
    active: false,
    terminal: true,
  };
  assert.equal(isJobStatusDetailConsistent("completed", completed), true);
  assert.equal(resolveWorkflowRunStatusTone("completed", "attached", completed), "good");
});

test("missing timing and invalid progress fail the runtime job contract", () => {
  const withoutTiming = {
    lifecycle: "active",
    active: true,
    terminal: false,
    failure_class: null,
    recoverable: false,
  } as unknown as JobStatusDetail;

  assert.equal(isJobStatusDetailConsistent("solving", withoutTiming), false);
  assert.equal(
    isWorkflowJobStatusContractValid({
      status: "solving",
      progress: 1.25,
      statusDetail: ACTIVE_DETAIL,
    }),
    false,
  );
  assert.equal(normalizeWorkflowRunProgress(Number.NaN), 0);
  assert.equal(normalizeWorkflowRunProgress(1.25), 1);
});

test("completed workflow polling requires a valid workflow result", () => {
  const completed: JobStatusDetail = {
    ...ACTIVE_DETAIL,
    lifecycle: "terminal",
    active: false,
    terminal: true,
  };

  assert.equal(resolveWorkflowRunPollDisposition("completed", completed, false), "invalid");
  assert.equal(resolveWorkflowRunPollDisposition("completed", completed, true), "completed");
  assert.equal(resolveWorkflowRunPollDisposition("solving", ACTIVE_DETAIL, false), "continue");
});

test("job schema includes the timing detail emitted by the runtime", async () => {
  const schemaUrl = new URL("../../../../schemas/job.schema.json", import.meta.url);
  const schema = JSON.parse(await readFile(schemaUrl, "utf8")) as {
    properties: {
      status_detail: {
        required: string[];
        properties: Record<string, unknown>;
      };
    };
  };
  const statusDetail = schema.properties.status_detail;

  assert.ok(statusDetail.required.includes("timing"));
  assert.ok("timing" in statusDetail.properties);
});
