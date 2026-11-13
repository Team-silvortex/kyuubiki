import assert from "node:assert/strict";
import test from "node:test";
import { installStoreDownloadHost, requestStoreDownload, validDownloadSelection, publicDownloadViews, publicProjectOperators } from "../ui/store-download-bridge.js";
const protocol = "kyuubiki:store-download/v1";
const selection = {kind:"operator",asset_id:"demo",version:"1.0.0",platform:"macos-aarch64",key_id:"release",key_sha256:"a".repeat(64)};
const view = {id:"b".repeat(64),issuer:"https://id.example.test",selection,state:"prepared",file_count:1,total_bytes:300,
  received_bytes:0,metered:true,remaining:1,unlimited:false,charged:null,error:null,directory:null};
const settle = async () => { for(let i=0;i<12;i++) await Promise.resolve(); };
test("project enablement uses its own native command, not store authorization or arbitrary execution",async()=>{
  const packagePin={package_id:"demo",package_version:"1.0.0",manifest_sha256:"a".repeat(64),entrypoint_sha256:"b".repeat(64),operators:[{operator_id:"extract.demo",kind:"extract"}]};
  const status={schema_version:"kyuubiki.project-operators/v1",project_id:"p",scope:"c".repeat(64),packages_root:"/test/packages",installed:[packagePin],installed_error:null,enabled:[],task:null};
  assert.deepEqual(publicProjectOperators({...status,access_token:"never expose"}),status);
  assert.equal(publicProjectOperators({...status,installed:[{...packagePin,entrypoint_sha256:"bad"}]}),null);
  const deployment={spec:{package:packagePin,platform:"macos-aarch64",manifest_json:"{}",size_bytes:4},offset:0,data:[1,2,3,4]};
  assert.deepEqual(publicProjectOperators({...status,deployment:{...deployment,secret:"discard"}}).deployment,deployment);
  assert.equal(publicProjectOperators({...status,deployment:{...deployment,data:[256]}}),null);
  assert.equal(publicProjectOperators({...status,deployment:{...deployment,offset:5}}),null);
  const shell=host(),calls=[],replies=[],frame={...host(),contentWindow:{postMessage:data=>replies.push(data)}};
  const stop=installStoreDownloadHost(frame,async(...args)=>{calls.push(args);return status;},shell);
  shell.emit({origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id:"hello",action:"connect"}});
  const session=replies.shift().session;
  const send=(id,operation,extra={})=>shell.emit({origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id,session,request:{action:"project",project_id:"p",scope:status.scope,operation,...extra}}});
  send("bad",{action:"execute",package:packagePin});send("path",{action:"enable",package:packagePin},{store_root:"/tmp/evil"});
  send("extra",{action:"enable",package:packagePin,token:"secret"});await settle();assert.equal(calls.length,0);
  send("enable",{action:"enable",package:packagePin});await settle();
  assert.deepEqual(calls,[["project_operator",{request:{project_id:"p",scope:status.scope,operation:{action:"enable",package:packagePin}}}]]);
  assert.deepEqual(replies[0].views,status);stop();
});
function host() {
  const events=new Map();
  return {addEventListener:(k,f)=>{if(!events.has(k)) events.set(k,new Set());events.get(k).add(f);},
    removeEventListener:(k,f)=>events.get(k)?.delete(f),emit:(e,k="message")=>events.get(k)?.forEach(f=>f(e)),
    crypto:globalThis.crypto,setTimeout,clearTimeout};
}
test("download inputs cannot supply credentials, paths, URL or floating versions",()=>{
  assert.equal(validDownloadSelection(selection),true);
  for(const change of [{version:"latest"},{asset_id:"../demo"},{platform:"auto"},{key_sha256:""},{token:"secret"},{url:"https://evil.test"},{asset_id:"CON.json"}])
    assert.equal(validDownloadSelection({...selection,...change}),false);
  assert.deepEqual(publicDownloadViews([{...view,download_url:"secret",access_token:"secret"}]),[view]);
  assert.equal(publicDownloadViews([{...view,received_bytes:301}]),null);
  assert.equal(publicDownloadViews([view,view]),null);
});
test("native host pins source, deduplicates, redacts and never charges on page changes",async()=>{
  const shell=host(),calls=[],replies=[];
  const frame={...host(),contentWindow:{postMessage:(data,origin)=>replies.push({data,origin})}};
  const stop=installStoreDownloadHost(frame,async(...args)=>{calls.push(args);return [{...view,download_url:"secret-ticket"}];},shell);
  shell.emit({origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id:"hello",action:"connect"}});
  const session=replies.shift().data.session;
  const event={origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id:"one",session,request:{action:"prepare",selection}}};
  shell.emit({...event,source:{}});shell.emit({...event,origin:"https://evil.test"});
  shell.emit({...event,data:{...event.data,request:{action:"start",id:"../bad"}}});await settle();assert.equal(calls.length,0);
  shell.emit(event);shell.emit(event);await settle();assert.equal(calls.length,1);
  assert.deepEqual(replies[0].data.views,[view]);assert.equal(replies[0].origin,event.origin);
  frame.emit({},"load");shell.emit(event);await settle();assert.equal(calls.length,1,"an old document cannot authorize after reload");
  stop();await settle();assert.equal(calls.length,1);
});
test("native errors expose only bounded codes, never raw ticket-bearing errors",async()=>{
  const shell=host(),replies=[];
  const frame={...host(),contentWindow:{postMessage:data=>replies.push(data)}};
  const stop=installStoreDownloadHost(frame,async()=>{throw "https://id.test/download?ticket=secret";},shell);
  shell.emit({origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id:"hello",action:"connect"}});
  const session=replies.shift().session;
  shell.emit({origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id:"one",session,request:{action:"list"}}});
  await settle();assert.equal(replies[0].error,"download_operation_failed");assert.ok(!JSON.stringify(replies).includes("secret"));stop();
});
test("client requires native parent, fails closed, and timeouts instruct status checks",async()=>{
  const child=host(),sent=[];child.parent={postMessage:data=>sent.push(data)};child.location={search:"?desktopLayout=1"};
  const promise=requestStoreDownload({action:"list"},child,1000);
  assert.deepEqual(Object.keys(sent[0]).sort(),["action","id","type"]);
  const session=crypto.randomUUID();
  child.emit({source:child.parent,origin:"http://tauri.localhost",data:{type:`${protocol}:result`,id:sent[0].id,connected:true,session,views:null,error:null}});
  const event={source:child.parent,origin:"http://tauri.localhost",data:{type:`${protocol}:result`,id:sent[1].id,session,views:[view],error:null}};
  child.emit({...event,source:{}});child.emit({...event,origin:"https://evil.test"});child.emit(event);
  assert.deepEqual(await promise,[view]);
  await assert.rejects(requestStoreDownload({action:"list"},child,5),/download_timeout_check_status/);
  child.parent=child;await assert.rejects(requestStoreDownload({action:"list"},child),/desktop_required/);
});
test("catalog and installation stay on native commands with no arbitrary path handoff",async()=>{
  const shell=host(), calls=[], replies=[];
  const catalog={issuer:view.issuer,host_platform:"macos-aarch64",catalog_revision:"r1",next_cursor:null,
    items:[{kind:"operator",asset_id:"demo",latest_version:"1.0.0",title:"Demo",summary:"Fixture",domain:"mechanical",languages:["en"],secret:"never forward"}]};
  const frame={...host(),contentWindow:{postMessage:data=>replies.push(data)}};
  const stop=installStoreDownloadHost(frame,async(command,payload)=>{calls.push([command,payload]);return command==="store_catalog" ? catalog : [{...view,state:"downloaded",installation:"installed"}];},shell);
  shell.emit({origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id:"hello",action:"connect"}});
  const session=replies.shift().session;
  const send=(id,request)=>shell.emit({origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id,session,request}});
  send("catalog",{action:"catalog",query:"demo",cursor:null});await settle();
  assert.equal(calls[0]?.[0],"store_catalog");
  assert.equal(replies[0].views.items[0].latest_version,"1.0.0");
  assert.ok(!JSON.stringify(replies).includes("never forward"));
  send("bad",{action:"install",id:view.id,path:"/tmp/untrusted"});await settle();assert.equal(calls.length,1);
  send("install",{action:"install",id:view.id});await settle();
  assert.deepEqual(calls[1],["store_download",{request:{action:"install",id:view.id}}]);
  assert.equal(replies[1].views[0].installation,"installed");stop();
});
