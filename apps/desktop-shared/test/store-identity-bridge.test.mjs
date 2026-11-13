import assert from "node:assert/strict";
import test from "node:test";
import { installStoreIdentityHost, requestStoreIdentity, validIdentityConfig } from "../ui/store-identity-bridge.js";
const protocol = "kyuubiki:store-identity/v1";
const settle = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
const status = { status: "signed_out", issuer: null, subject: null, display_name: null, expires_at: null, can_refresh: false, error: null };
const config = { issuer: "https://id.example.test", client_id: "registered", callback_port: 47831, ca_pem: "", ca_sha256: "" };
function host() {
  const events = new Map();
  return { addEventListener: (k,f) => { if (!events.has(k)) events.set(k,new Set()); events.get(k).add(f); },
    removeEventListener: (k,f) => events.get(k)?.delete(f), emit: (e,k="message") => events.get(k)?.forEach(f=>f(e)),
    crypto: globalThis.crypto, setTimeout, clearTimeout };
}
test("only explicit public client settings cross the bridge", () => {
  assert.equal(validIdentityConfig(config), true);
  for (const change of [{issuer:"http://id.example.test"},{issuer:"https://id.example.test/path"},
    {client_id:""},{callback_port:0},{callback_port:1.5},{client_secret:"secret"},
    {ca_pem:"PUBLIC"},{ca_sha256:"0".repeat(64)},{ca_pem:"PRIVATE KEY",ca_sha256:"0".repeat(64)}]) {
    assert.equal(validIdentityConfig({...config,...change}), false);
  }
});
test("host pins iframe, narrows actions, deduplicates and projects credential-free replies", async () => {
  const shell = host(), calls = [], replies = [];
  const frame = {...host(), contentWindow:{postMessage:(data,origin)=>replies.push({data,origin})}};
  const stop = installStoreIdentityHost(frame, async (...args) => { calls.push(args); return {...status, access_token:"never-expose"}; }, shell);
  const event = {origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id:"one",request:{action:"begin",config}}};
  shell.emit({...event,origin:"https://evil.test"}); shell.emit({...event,source:{}});
  shell.emit({...event,data:{...event.data,request:{action:"read_token"}}});
  shell.emit({...event,data:{...event.data,request:{action:"status",token:"secret"}}});
  await settle(); assert.equal(calls.length,0);
  shell.emit(event); shell.emit(event); await settle();
  assert.deepEqual(calls,[ ["store_identity",{request:{action:"begin",config}}] ]);
  assert.deepEqual(replies[0].data.status,status);
  assert.equal(replies[0].origin,event.origin);
  assert.ok(!JSON.stringify(replies).includes("never-expose"));
  stop(); await settle();
  assert.equal(calls.at(-1)[1].request.action,"cancel");
});
test("reload fences old results and cancels pending login without signing out an established session", async () => {
  const shell = host(), replies = [], calls = [];
  let finish;
  const frame = {...host(),contentWindow:{postMessage:data=>replies.push(data)}};
  const stop = installStoreIdentityHost(frame, async (_,payload) => {
    calls.push(payload.request.action);
    if (payload.request.action === "begin") return new Promise(resolve=>{finish=resolve;});
    return status;
  },shell);
  shell.emit({origin:"http://127.0.0.1:3000",source:frame.contentWindow,data:{type:protocol,id:"one",request:{action:"begin",config}}});
  await settle(); frame.emit({},"load"); finish(status); await settle();
  assert.deepEqual(replies,[]); assert.deepEqual(calls,["begin","cancel"]);
  stop();
});
test("client accepts only its native parent and cleans up failed or timed-out requests", async () => {
  const child = host(), sent=[];
  child.parent={postMessage:data=>sent.push(data)}; child.location={search:"?desktopLayout=1"};
  const promise=requestStoreIdentity({action:"status"},child,1000);
  const event={source:child.parent,origin:"https://tauri.localhost",data:{type:`${protocol}:result`,id:sent[0].id,ok:true,status}};
  child.emit({...event,source:{}}); child.emit({...event,origin:"https://evil.test"});
  child.emit(event); assert.deepEqual(await promise,status);
  await assert.rejects(requestStoreIdentity({action:"status"},child,5),/timed out/);
  child.parent=child; await assert.rejects(requestStoreIdentity({action:"status"},child),/unavailable/);
});
