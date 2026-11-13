import type { ProjectOperatorPin, ProjectDeploymentChunk } from "../../../../desktop-shared/src/store-download-bridge.ts";
import { validProjectOperatorPin } from "../../../../desktop-shared/src/store-download-bridge.ts";
import { requestJson } from "../api/core.ts";
export type OperatorDeploymentStatus = {
  schema_version: "kyuubiki.operator-deployment/v1"; package: ProjectOperatorPin; platform: string;
  state: "missing" | "uploading" | "installed_restart_required" | "ready" | "version_conflict";
  installed: ProjectOperatorPin | null; active: ProjectOperatorPin | null;
  received_bytes: number; total_bytes: number; process_instance_id: string; deployment_allowed: boolean;
};
const key = (p: ProjectOperatorPin) => JSON.stringify([p.package_id,p.package_version,p.manifest_sha256,p.entrypoint_sha256,p.operators.map(o=>[o.operator_id,o.kind])]);
const PATH = "/api/direct-mesh/operator-package";
function validStatus(s: OperatorDeploymentStatus, pin: ProjectOperatorPin): boolean {
  if (!validProjectOperatorPin(s.package) || key(s.package) !== key(pin)
    || typeof s.platform !== "string" || !/^[A-Za-z0-9_-]{1,128}$/.test(s.platform)) return false;
  for (const p of [s.installed, s.active]) {
    if (p !== null && (!validProjectOperatorPin(p) || p.package_id !== pin.package_id)) return false;
  }
  const installed = !!s.installed && key(s.installed) === key(pin);
  switch (s.state) {
    case "missing": return s.installed === null && s.total_bytes === 0;
    case "uploading": return s.installed === null && s.total_bytes > 0;
    case "installed_restart_required": return installed;
    case "ready": return installed && !!s.active && key(s.active) === key(pin);
    case "version_conflict": return !!s.installed && !installed;
    default: return false;
  }
}
export function createProjectOperatorDeployment({ request = requestJson, read }: {
  request?: (url:string,init:RequestInit)=>Promise<unknown>;
  read:(offset:number)=>Promise<ProjectDeploymentChunk>;
}) {
  async function exchange(pin:ProjectOperatorPin,endpoint:string,operation:Record<string,unknown>,current:()=>boolean,process:string|null=null) {
    if (!current()) throw new Error("deployment_context_changed");
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
    return s;
  }
  return {
    inspect:(pin:ProjectOperatorPin,endpoint:string,current:()=>boolean)=>exchange(pin,endpoint,{action:"inspect"},current),
    async abort(pin:ProjectOperatorPin,endpoint:string,current:()=>boolean){
      const p=structuredClone(pin),s=await exchange(p,endpoint,{action:"inspect"},current);
      return exchange(p,endpoint,{action:"abort"},current,s.process_instance_id);
    },
    async deploy(pin:ProjectOperatorPin,endpoint:string,current:()=>boolean,onProgress:(s:OperatorDeploymentStatus)=>void=()=>{}) {
      const p=structuredClone(pin);let s=await exchange(p,endpoint,{action:"inspect"},current);onProgress(s);
      if(["ready","installed_restart_required","version_conflict"].includes(s.state))return s;
      if(!s.deployment_allowed)throw new Error("deployment_disabled");
      const first=await read(0);if(!current())throw new Error("deployment_context_changed");
      if(key(first.spec.package)!==key(p) || first.spec.platform!==s.platform)throw new Error("deployment_platform_mismatch");
      const process=s.process_instance_id;
      s=await exchange(p,endpoint,{action:"begin",spec:first.spec},current,process);onProgress(s);
      for(let blocks=0;s.state==="uploading" && s.received_bytes<s.total_bytes;blocks++) {
        if(blocks>=513)throw new Error("deployment_limit");
        const offset=s.received_bytes,chunk=offset===0?first:await read(offset);
        if(!current())throw new Error("deployment_context_changed");
        if(chunk.offset!==offset || JSON.stringify(chunk.spec)!==JSON.stringify(first.spec) || !chunk.data.length)throw new Error("deployment_source_changed");
        s=await exchange(p,endpoint,{action:"chunk",offset,data:chunk.data},current,process);
        if(s.state==="uploading" && s.received_bytes!==offset+chunk.data.length)throw new Error("deployment_offset_conflict");
        onProgress(s);
      }
      if(s.state!=="uploading")return s;
      s=await exchange(p,endpoint,{action:"commit"},current,process);onProgress(s);return s;
    },
  };
}
