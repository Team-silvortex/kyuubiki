import type { ProjectOperatorPin, ProjectDeploymentChunk } from "../../../../desktop-shared/src/store-download-bridge.ts";
import { validProjectOperatorPin } from "../../../../desktop-shared/src/store-download-bridge.ts";
import { requestJson } from "../api/core.ts";
export type OperatorDeploymentStatus = {
  schema_version: "kyuubiki.operator-deployment/v1"; package: ProjectOperatorPin; platform: string;
  state: "missing" | "uploading" | "installed_restart_required" | "ready" | "version_conflict" | "activation_failed";
  activation_error?: "operator_package_activation_failed" | "operator_package_integrity_failed" | "operator_package_count_mismatch" | null;
  installed: ProjectOperatorPin | null; active: ProjectOperatorPin | null;
  received_bytes: number; total_bytes: number; process_instance_id: string; deployment_allowed: boolean;
};
const key = (p: ProjectOperatorPin) => JSON.stringify([p.package_id,p.package_version,p.manifest_sha256,p.entrypoint_sha256,p.operators.map(o=>[o.operator_id,o.kind])]);
const PATH = "/api/direct-mesh/operator-package";
const MAX_BYTES = 32 * 1024 * 1024;
const CHUNK_BYTES = 65536;
type Spec = ProjectDeploymentChunk["spec"];
const terminal = (s: OperatorDeploymentStatus) => ["ready", "installed_restart_required", "version_conflict", "activation_failed"].includes(s.state);
function validateSource(chunk: ProjectDeploymentChunk, pin: ProjectOperatorPin, platform: string, offset: number, expected?: Spec) {
  const spec = chunk?.spec;
  if (!spec || !validProjectOperatorPin(spec.package) || key(spec.package) !== key(pin)
    || !Number.isSafeInteger(spec.size_bytes) || spec.size_bytes < 1 || spec.size_bytes > MAX_BYTES
    || typeof spec.manifest_json !== "string" || !spec.manifest_json.length || spec.manifest_json.length > CHUNK_BYTES
    || new TextEncoder().encode(spec.manifest_json).length > CHUNK_BYTES
    || chunk.offset !== offset || !Array.isArray(chunk.data) || !chunk.data.length || chunk.data.length > CHUNK_BYTES
    || offset + chunk.data.length > spec.size_bytes || !Array.from(chunk.data).every(b => Number.isInteger(b) && b >= 0 && b <= 255)
    || (expected && (spec.manifest_json !== expected.manifest_json || spec.size_bytes !== expected.size_bytes))) {
    throw new Error("deployment_source_changed");
  }
  if (spec.platform !== platform) throw new Error("deployment_platform_mismatch");
}
function validateTransfer(s: OperatorDeploymentStatus, spec: Spec) {
  if ((s.state === "uploading" && s.total_bytes !== spec.size_bytes)
    || (s.total_bytes !== 0 && s.total_bytes !== spec.size_bytes)
    || s.state === "missing") throw new Error("deployment_invalid_receipt");
  if (s.state === "uploading" && !s.deployment_allowed) throw new Error("deployment_disabled");
}
function validStatus(s: OperatorDeploymentStatus, pin: ProjectOperatorPin): boolean {
  const failed = s.activation_error != null;
  if ((s.state === "activation_failed") !== failed || (failed && ![
    "operator_package_activation_failed", "operator_package_integrity_failed", "operator_package_count_mismatch",
  ].includes(s.activation_error!))) return false;
  if (!validProjectOperatorPin(s.package) || key(s.package) !== key(pin)
    || typeof s.platform !== "string" || !/^[A-Za-z0-9_-]{1,128}$/.test(s.platform)) return false;
  for (const p of [s.installed, s.active]) {
    if (p !== null && (!validProjectOperatorPin(p) || p.package_id !== pin.package_id)) return false;
  }
  const installed = !!s.installed && key(s.installed) === key(pin);
  switch (s.state) {
    case "missing": return s.installed === null && s.total_bytes === 0;
    case "uploading": return s.installed === null && s.total_bytes > 0;
    case "installed_restart_required": return installed && s.received_bytes === s.total_bytes;
    case "activation_failed": return installed && s.active === null && s.received_bytes === s.total_bytes;
    case "ready": return installed && !!s.active && key(s.active) === key(pin) && s.received_bytes === s.total_bytes;
    case "version_conflict": return !!s.installed && !installed;
    default: return false;
  }
}
export function createProjectOperatorDeployment({ request = requestJson, read, platform }: {
  platform?:string;
  request?: (url:string,init:RequestInit)=>Promise<unknown>;
  read:(offset:number)=>Promise<ProjectDeploymentChunk>;
}) {
  async function exchange(pin:ProjectOperatorPin,endpoint:string,operation:Record<string,unknown>,current:()=>boolean,process:string|null=null,target=platform) {
    if (!current()) throw new Error("deployment_context_changed");
    if (!validProjectOperatorPin(pin)) throw new Error("deployment_source_changed");
    const value=await request(PATH,{method:"POST",cache:"no-store",headers:{"Content-Type":"application/json"},
      body:JSON.stringify({endpoints:[endpoint],request:{package:pin,expected_process_instance_id:process,operation}})});
    if (!current()) throw new Error("deployment_context_changed");
    const v=value as {endpoint?:string;gateway_runtime?:string;deployment?:OperatorDeploymentStatus},s=v?.deployment;
    if(v?.endpoint!==endpoint || v.gateway_runtime!=="rust-native" || !s || s.schema_version!=="kyuubiki.operator-deployment/v1"
      || !validStatus(s,pin)
      || typeof s.process_instance_id!=="string" || !s.process_instance_id || s.process_instance_id.length>256
      || (process!==null && s.process_instance_id!==process) || typeof s.deployment_allowed!=="boolean"
      || !Number.isSafeInteger(s.total_bytes) || s.total_bytes<0 || s.total_bytes>33554432 || !Number.isSafeInteger(s.received_bytes) || s.received_bytes<0 || s.received_bytes>s.total_bytes
      || /[\x00-\x1f\x7f]/.test(s.process_instance_id))throw new Error("deployment_invalid_receipt");
    if (target!==undefined && s.platform!==target) throw new Error("deployment_platform_mismatch");
    return structuredClone(s);
  }
  return {
    inspect:(pin:ProjectOperatorPin,endpoint:string,current:()=>boolean)=>exchange(structuredClone(pin),endpoint,{action:"inspect"},current),
    async abort(pin:ProjectOperatorPin,endpoint:string,current:()=>boolean){
      const p=structuredClone(pin),s=await exchange(p,endpoint,{action:"inspect"},current);
      if (!s.deployment_allowed) throw new Error("deployment_disabled");
      const result = await exchange(p,endpoint,{action:"abort"},current,s.process_instance_id,s.platform);
      if (result.state === "uploading") throw new Error("deployment_invalid_receipt");
      return result;
    },
    async deploy(pin:ProjectOperatorPin,endpoint:string,current:()=>boolean,onProgress:(s:OperatorDeploymentStatus)=>void=()=>{}) {
      const p=structuredClone(pin);
      const progress = (s: OperatorDeploymentStatus) => {
        onProgress(structuredClone(s));
        if (!current()) throw new Error("deployment_context_changed");
      };
      let s=await exchange(p,endpoint,{action:"inspect"},current);progress(s);
      if(terminal(s))return s;
      if(!s.deployment_allowed)throw new Error("deployment_disabled");
      const target = s.platform, process = s.process_instance_id, previous = s.received_bytes;
      const first=structuredClone(await read(0));if(!current())throw new Error("deployment_context_changed");
      validateSource(first, p, target, 0);
      const spec = structuredClone(first.spec);
      if (s.state === "uploading") validateTransfer(s, spec);
      s=await exchange(p,endpoint,{action:"begin",spec},current,process,target);
      validateTransfer(s, spec);
      if (s.state === "uploading" && s.received_bytes < previous) throw new Error("deployment_offset_conflict");
      progress(s);
      for(let blocks=0;s.state==="uploading" && s.received_bytes<s.total_bytes;blocks++) {
        if(blocks>=513)throw new Error("deployment_limit");
        const offset=s.received_bytes,chunk=offset===0?first:structuredClone(await read(offset));
        if(!current())throw new Error("deployment_context_changed");
        validateSource(chunk, p, target, offset, spec);
        s=await exchange(p,endpoint,{action:"chunk",offset,data:chunk.data},current,process,target);
        validateTransfer(s, spec);
        if(s.state==="uploading" && s.received_bytes!==offset+chunk.data.length)throw new Error("deployment_offset_conflict");
        progress(s);
      }
      if(s.state!=="uploading")return s;
      s=await exchange(p,endpoint,{action:"commit"},current,process,target);
      validateTransfer(s, spec);
      if (!terminal(s)) throw new Error("deployment_invalid_receipt");
      progress(s);return s;
    },
  };
}
