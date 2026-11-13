import test from "node:test";
import assert from "node:assert/strict";
import { createProjectOperatorDeployment, type OperatorDeploymentStatus } from "../../src/lib/workbench/project-operator-deployment.ts";

const pin = { package_id: "demo", package_version: "1.0.0", manifest_sha256: "a".repeat(64), entrypoint_sha256: "b".repeat(64), operators: [{ operator_id: "extract.demo", kind: "extract" }] };
const spec = { package: pin, platform: "linux-x86_64", manifest_json: "{}", size_bytes: 4 };
const status = (state: OperatorDeploymentStatus["state"] = "missing", bytes = 0): OperatorDeploymentStatus => ({
  schema_version: "kyuubiki.operator-deployment/v1", package: structuredClone(pin), platform: spec.platform, state,
  installed: state === "installed_restart_required" ? structuredClone(pin) : null, active: null,
  received_bytes: bytes, total_bytes: state === "missing" ? 0 : 4, process_instance_id: "process-1", deployment_allowed: true,
});
const envelope = (deployment: OperatorDeploymentStatus) => ({ endpoint: "linux:5001", gateway_runtime: "rust-native", deployment });
const read = async (offset: number) => ({ spec: structuredClone(spec), offset, data: [1, 2, 3, 4].slice(offset) });

test("activation failure is visible but never authorizes reupload or readiness", async () => {
  const failed = { ...status("installed_restart_required", 4), state: "activation_failed", activation_error: "operator_package_activation_failed" };
  const actions: string[] = [];
  const service = createProjectOperatorDeployment({ platform: spec.platform,
    read: async () => { throw new Error("must not read source"); },
    request: async (_url, init) => {
      actions.push(JSON.parse(init.body as string).request.operation.action);
      return { ...envelope(status()), deployment: structuredClone(failed) };
    },
  });
  assert.equal((await service.deploy(pin, "linux:5001", () => true)).state, "activation_failed");
  assert.deepEqual(actions, ["inspect"]);
  for (const invalid of [
    { ...failed, activation_error: undefined },
    { ...failed, activation_error: "untrusted path or message" },
    { ...failed, active: pin }, { ...failed, installed: null },
    { ...failed, received_bytes: 2 },
    { ...failed, state: "ready", active: pin },
  ]) {
    const invalidService = createProjectOperatorDeployment({ read, request: async () => ({ ...envelope(status()), deployment: invalid }) });
    await assert.rejects(invalidService.inspect(pin, "linux:5001", () => true), /deployment_invalid_receipt/);
  }
});

test("deployment validates source bounds before beginning a remote upload", async () => {
  for (const invalid of [
    { ...await read(0), offset: 1 },
    { ...await read(0), data: [] },
    { ...await read(0), data: [256] },
    { ...await read(0), data: [1.5] },
    { ...await read(0), data: new Array(4) },
    { ...await read(0), data: [1, 2, 3, 4, 5] },
    { ...await read(0), spec: { ...spec, size_bytes: 0 } },
    { ...await read(0), spec: { ...spec, size_bytes: 33554433 } },
    { ...await read(0), spec: { ...spec, manifest_json: "x".repeat(65537) } },
    { ...await read(0), spec: { ...spec, manifest_json: "\u00e9".repeat(32769) } },
  ]) {
    const actions: string[] = [];
    const service = createProjectOperatorDeployment({ read: async () => invalid, request: async (_url, init) => {
      const op = JSON.parse(init.body as string).request.operation; actions.push(op.action);
      return envelope(status(op.action === "inspect" ? "missing" : "installed_restart_required", op.action === "inspect" ? 0 : 4));
    } });
    await assert.rejects(service.deploy(pin, "linux:5001", () => true), /deployment_source_changed/);
    assert.deepEqual(actions, ["inspect"]);
  }
});

test("a later source change or a backwards resume cursor stops upload without cleanup", async () => {
  for (const mode of ["source", "cursor"]) {
    const actions: string[] = [];
    const service = createProjectOperatorDeployment({ read: async offset => ({
      ...await read(offset), data: [1, 2, 3, 4].slice(offset, offset + 2),
      spec: { ...spec, manifest_json: offset ? "changed" : spec.manifest_json },
    }), request: async (_url, init) => {
      const op = JSON.parse(init.body as string).request.operation; actions.push(op.action);
      return envelope(status("uploading", op.action === "inspect" && mode === "cursor" ? 2 : op.action === "chunk" ? 2 : 0));
    } });
    await assert.rejects(service.deploy(pin, "linux:5001", () => true), /deployment_(source_changed|offset_conflict)/);
    assert.deepEqual(actions, mode === "cursor" ? ["inspect", "begin"] : ["inspect", "begin", "chunk"]);
  }
});

test("deployment pins receipt platform, byte budget and completion across every operation", async () => {
  for (const mode of ["inspect-size", "begin-size", "chunk-size", "platform", "commit-incomplete", "commit-missing", "permission"]) {
    const actions: string[] = [];
    const service = createProjectOperatorDeployment({ read, request: async (_url, init) => {
      const op = JSON.parse(init.body as string).request.operation; actions.push(op.action);
      let reply = status(op.action === "inspect" ? "missing" : op.action === "commit" ? "installed_restart_required" : "uploading", op.action === "chunk" || op.action === "commit" ? 4 : 0);
      if (mode === "inspect-size" && op.action === "inspect") reply = { ...status("uploading"), total_bytes: 8 };
      if (mode === "begin-size" && op.action === "begin") reply.total_bytes = 8;
      if (mode === "chunk-size" && op.action === "chunk") reply.total_bytes = 8;
      if (mode === "platform" && op.action === "begin") reply.platform = "linux-aarch64";
      if (mode === "commit-incomplete" && op.action === "commit") reply = status("uploading", 4);
      if (mode === "commit-missing" && op.action === "commit") reply = status();
      if (mode === "permission" && op.action === "begin") reply.deployment_allowed = false;
      return envelope(reply);
    } });
    await assert.rejects(service.deploy(pin, "linux:5001", () => true), /deployment_(invalid_receipt|platform_mismatch|disabled)/, mode);
    if (mode === "inspect-size") assert.deepEqual(actions, ["inspect"]);
    if (["begin-size", "platform", "permission"].includes(mode)) assert.deepEqual(actions, ["inspect", "begin"]);
    if (mode === "chunk-size") assert.deepEqual(actions, ["inspect", "begin", "chunk"]);
  }
});

test("progress subscribers cannot mutate the transfer identity or byte cursor", async () => {
  const actions: string[] = [];
  const service = createProjectOperatorDeployment({ read, request: async (_url, init) => {
    const req = JSON.parse(init.body as string).request; actions.push(req.operation.action);
    assert.deepEqual(req.package, pin);
    if (req.operation.action !== "inspect") assert.equal(req.expected_process_instance_id, "process-1");
    return envelope(status(req.operation.action === "inspect" ? "missing" : req.operation.action === "commit" ? "installed_restart_required" : "uploading", ["chunk", "commit"].includes(req.operation.action) ? 4 : 0));
  } });
  const result = await service.deploy(pin, "linux:5001", () => true, progress => {
    progress.package.package_version = "99";
    progress.process_instance_id = "forged";
    progress.received_bytes = 4;
    progress.state = "ready";
  });
  assert.equal(result.state, "installed_restart_required");
  assert.deepEqual(actions, ["inspect", "begin", "chunk", "commit"]);
});

test("inspection snapshots the requested pin before awaiting the transport", async () => {
  const requested = structuredClone(pin);
  const service = createProjectOperatorDeployment({ read, request: async () => {
    requested.package_version = "99";
    return envelope({ ...status(), package: structuredClone(requested) });
  } });
  await assert.rejects(service.inspect(requested, "linux:5001", () => true), /deployment_invalid_receipt/);
});

test("a lost chunk or commit response resumes through inspection without replay or automatic restart", async () => {
  for (const lost of ["chunk", "commit"]) {
    const actions: string[] = []; let remote = status(), dropped = false, reads = 0;
    const service = createProjectOperatorDeployment({ platform: spec.platform, read: async offset => { reads++; return read(offset); }, request: async (_url, init) => {
      const op = JSON.parse(init.body as string).request.operation; actions.push(op.action);
      if (op.action === "begin") remote = status("uploading", remote.received_bytes);
      if (op.action === "chunk") remote = status("uploading", 4);
      if (op.action === "commit") remote = status("installed_restart_required", 4);
      if (op.action === lost && !dropped) { dropped = true; throw new Error("response_lost"); }
      return envelope(structuredClone(remote));
    } });
    await assert.rejects(service.deploy(pin, "linux:5001", () => true), /response_lost/);
    const readCount = reads;
    assert.equal((await service.deploy(pin, "linux:5001", () => true)).state, "installed_restart_required");
    assert.equal(actions.filter(a => a === "chunk").length, 1);
    assert.equal(actions.filter(a => a === "commit").length, 1);
    assert.equal(actions.filter(a => a === "inspect").length, 2);
    if (lost === "commit") assert.equal(reads, readCount);
    assert.ok(!actions.includes("abort"));
  }
});
