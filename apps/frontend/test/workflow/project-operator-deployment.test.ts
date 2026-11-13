import test from "node:test";
import assert from "node:assert/strict";
import { createProjectOperatorDeployment } from "../../src/lib/workbench/project-operator-deployment.ts";
const pin={package_id:"demo",package_version:"1.0.0",manifest_sha256:"a".repeat(64),entrypoint_sha256:"b".repeat(64),operators:[{operator_id:"extract.demo",kind:"extract"}]};
const spec={package:pin,platform:"macos-aarch64",manifest_json:"{}",size_bytes:4};
const report=(state="missing",received_bytes=0)=>({schema_version:"kyuubiki.operator-deployment/v1",package:pin,platform:spec.platform,state,installed:null as typeof pin|null,active:null,received_bytes,total_bytes:state==="missing"?0:4,process_instance_id:"process-1",deployment_allowed:true});
test("explicit Linux target rejects another architecture even when it claims ready",async()=>{
  for(const state of ["missing","ready"]){
    let reads=0,calls=0;
    const service=createProjectOperatorDeployment({platform:"linux-x86_64",read:async offset=>{reads++;return {spec,offset,data:[1,2,3,4]};},
      request:async()=>{calls++;return {endpoint:"linux:5001",gateway_runtime:"rust-native",deployment:{...report(state,state==="ready"?4:0),platform:"linux-aarch64",installed:state==="ready"?pin:null,active:state==="ready"?pin:null}};}});
    await assert.rejects(service.inspect(pin,"linux:5001",()=>true),/deployment_platform_mismatch/);
    await assert.rejects(service.deploy(pin,"linux:5001",()=>true),/deployment_platform_mismatch/);
    await assert.rejects(service.abort(pin,"linux:5001",()=>true),/deployment_platform_mismatch/);
    assert.equal(reads,0);assert.equal(calls,3);
  }
});
test("Mac control may deploy staged Linux bytes without store calls or platform fallback",async()=>{
  for(const platform of ["linux-x86_64","linux-aarch64"]){
    const actions:string[]=[];let state={...report(),platform};
    const service=createProjectOperatorDeployment({platform,read:async offset=>({spec:{...spec,platform},offset,data:[1,2,3,4].slice(offset)}),request:async(url,init)=>{
      assert.equal(url,"/api/direct-mesh/operator-package");const op=JSON.parse(init.body as string).request.operation;actions.push(op.action);
      if(op.action==="begin"){assert.equal(op.spec.platform,platform);state={...report("uploading"),platform};}
      if(op.action==="chunk")state={...report("uploading",4),platform};
      if(op.action==="commit")state={...report("installed_restart_required",4),platform,installed:pin};
      return {endpoint:"linux:5001",gateway_runtime:"rust-native",deployment:state};
    }});
    assert.equal((await service.deploy(pin,"linux:5001",()=>true)).state,"installed_restart_required");
    assert.deepEqual(actions,["inspect","begin","chunk","commit"]);
  }
});
test("project deployment resumes only matching bytes and never reports installed as ready",async()=>{
  const actions:string[]=[];let state=report("uploading",2);
  const service=createProjectOperatorDeployment({read:async(offset)=>({spec,offset,data:[1,2,3,4].slice(offset)}),request:async(_url,init)=>{
    const r=JSON.parse(init.body as string).request;actions.push(r.operation.action);
    if(r.operation.action!=="inspect")assert.equal(r.expected_process_instance_id,"process-1");
    if(r.operation.action==="chunk"){assert.equal(r.operation.offset,2);assert.deepEqual(r.operation.data,[3,4]);state=report("uploading",4);}
    if(r.operation.action==="commit")state={...report("installed_restart_required",4),installed:pin};
    return {endpoint:"127.0.0.1:5001",gateway_runtime:"rust-native",deployment:state};
  }});
  assert.equal((await service.deploy(pin,"127.0.0.1:5001",()=>true)).state,"installed_restart_required");
  assert.deepEqual(actions,["inspect","begin","chunk","commit"]);
});
test("project deployment never uploads after conflict, target change or authority change",async()=>{
  for(const mode of ["conflict","process","authority"]) {
    let calls=0,current=true;
    const service=createProjectOperatorDeployment({read:async(offset)=>({spec,offset,data:[1,2,3,4]}),request:async()=>{
      calls++;if(mode==="authority")current=false;
      return {endpoint:"127.0.0.1:5001",gateway_runtime:"rust-native",deployment:{...report(mode==="conflict"?"version_conflict":"uploading"),installed:mode==="conflict"?{...pin,package_version:"2.0.0"}:null,process_instance_id:calls>1?"replaced":"process-1"}};
    }});
    if(mode==="conflict")assert.equal((await service.deploy(pin,"127.0.0.1:5001",()=>current)).state,"version_conflict");
    else await assert.rejects(service.deploy(pin,"127.0.0.1:5001",()=>current));
    assert.ok(calls<=2);
  }
});
test("project deployment rejects inconsistent receipts before any source read",async()=>{
  for(const deployment of [
    {...report(),state:"version_conflict"},
    {...report(),state:"installed_restart_required"},
    {...report(),state:"ready",installed:pin,active:null},
    {...report(),state:"missing",installed:pin},
    {...report(),platform:""},
    {...report(),package:{...pin,operators:null}},
  ]) {
    const service=createProjectOperatorDeployment({read:async()=>{throw new Error("must_not_read");},
      request:async()=>({endpoint:"127.0.0.1:5001",gateway_runtime:"rust-native",deployment})});
    await assert.rejects(service.deploy(pin,"127.0.0.1:5001",()=>true),/deployment_invalid_receipt/);
  }
});
test("project deployment rejects platform mismatch and disabled upload without mutations",async()=>{
  for(const allowed of [true,false]){
    let reads=0,calls=0;
    const service=createProjectOperatorDeployment({read:async offset=>{reads++;return {spec:{...spec,platform:"linux-x86_64"},offset,data:[1,2,3,4]};},
      request:async()=>{calls++;return {endpoint:"127.0.0.1:5001",gateway_runtime:"rust-native",deployment:{...report(),deployment_allowed:allowed}};}});
    await assert.rejects(service.deploy(pin,"127.0.0.1:5001",()=>true),allowed?/deployment_platform_mismatch/:/deployment_disabled/);
    assert.equal(calls,1);assert.equal(reads,allowed?1:0);
  }
});
