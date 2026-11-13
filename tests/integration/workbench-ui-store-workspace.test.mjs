import assert from "node:assert/strict";
import { before, after, test } from "node:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { chromium, startIsolatedWorkbenchUiRuntime, workbenchUrl } from "./workbench-ui-isolated.shared.mjs";

let browser;
let runtime;
before(async () => {
  runtime = await startIsolatedWorkbenchUiRuntime();
  browser = await launchIntegrationBrowser(chromium);
}, { timeout: 180_000 });
after(async () => { await browser?.close(); await runtime?.stop(); }, { timeout: 90_000 });

const entries = Array.from({ length: 18 }, (_, index) => ({
  id: `store-operator-${index}`, kind: "operator", title: `Mechanical research operator ${index}`,
  summary: "A catalog reference for a reproducible mechanical research workflow.",
  version: "1.0.0", source_id: index < 9 ? "builtin.local" : "lab",
  source_kind: index < 9 ? "builtin" : "catalog_file", domain: index % 2 ? "thermal" : "mechanical",
  tags: ["research", "headless"], install: { mode: "source_reference", requires_download: index >= 9 },
}));
const sources = ["builtin.local", "lab"].map((id) => ({
  id, label: id, type: id === "lab" ? "catalog_file" : "builtin", enabled: true, editable: false,
  status: "ready", supports: ["operator"],
}));

for (const width of [1440, 390]) {
  test(`Store desktop session and authorized download recovery loop at ${width}px (mock native bridge)`, async () => {
    const context = await browser.newContext({ viewport: { width, height: 900 } });
    // Chromium needs permission for the mock shell's loopback iframe; Tauri is tested separately.
    await context.grantPermissions(["local-network-access"]);
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    const loadErrors = [];
    page.on("requestfailed", request => loadErrors.push(`${request.url()}: ${request.failure()?.errorText}`));
    page.on("console", message => { if (message.type() === "error") loadErrors.push(message.text()); });
    try {
      const url = new URL(workbenchUrl(runtime));
      url.searchParams.set("desktopLayout", "1");
      let remoteState="missing",received=0,remotePlatform="macos-aarch64",loseCommitReply=true;
      const deploymentActions=[];
      await context.route("**/api/direct-mesh/operator-package",route=>{
        const {request:r,endpoints}=route.request().postDataJSON();deploymentActions.push(r.operation.action);
        if(r.operation.action==="begin")remoteState="uploading";
        if(r.operation.action==="chunk")received+=r.operation.data.length;
        if(r.operation.action==="commit"){
          remoteState="installed_restart_required";
          if(loseCommitReply){loseCommitReply=false;return route.fulfill({status:503,json:{error:"test_commit_response_lost"}});}
        }
        return route.fulfill({json:{endpoint:endpoints[0],gateway_runtime:"rust-native",deployment:{
          schema_version:"kyuubiki.operator-deployment/v1",package:r.package,platform:remotePlatform,state:remoteState,
          installed:["ready","installed_restart_required","activation_failed"].includes(remoteState)?r.package:remoteState==="version_conflict"?{...r.package,package_version:"2.0.0"}:null,
          activation_error:remoteState==="activation_failed"?"operator_package_activation_failed":null,
          active:remoteState==="ready"?r.package:null,received_bytes:received,total_bytes:remoteState==="missing"?0:4,process_instance_id:"test-process",deployment_allowed:true,
        }}});
      });
      await context.route("http://tauri.localhost/**", route => route.fulfill({ contentType: "text/html", body: `
        <style>body{margin:0}iframe{border:0;width:100vw;height:100vh}</style><iframe src="${url.toString().replaceAll("&", "&amp;")}"></iframe>
        <script>
        const empty = {status:"signed_out",issuer:null,subject:null,display_name:null,expires_at:null,can_refresh:false,error:null};
        window.view = {...empty}; window.actions = []; window.next = "waiting";
        window.downloads = []; window.downloadActions = []; window.starts = 0;
        window.enabled = []; window.enabledPlatforms = {};
        addEventListener("message", event => {
          if (event.data?.type === "kyuubiki:store-download/v1") {
            if(event.data.action === "connect") {
              event.source.postMessage({type:"kyuubiki:store-download/v1:result",id:event.data.id,session:"12345678-1234-1234-1234-123456789abc",connected:true,views:null,error:null},event.origin);
              return;
            }
            const {id,request} = event.data;
            window.downloadActions.push(request);
            if(request.action === "project") {
              const packagePin={package_id:"demo",package_version:"1.0.0",manifest_sha256:"a".repeat(64),entrypoint_sha256:"b".repeat(64),operators:[{operator_id:"extract.demo",kind:"extract"}]};
              const op=request.operation;
              const platform=request.platform||"local";
              if(op.action==="enable")window.enabledPlatforms[platform]=[{package:packagePin,state:"ready"}];
              if(op.action==="disable")window.enabledPlatforms[platform]=[];
              window.enabled=window.enabledPlatforms[platform]||[];
              const views={schema_version:"kyuubiki.project-operators/v1",project_id:request.project_id,scope:request.scope,packages_root:"/test-only/operator-packages/packages",
                installed:[packagePin],installed_error:null,enabled:window.enabled,task:op.action==="prepare"?{schema_version:"kyuubiki.operator-task-ir/v1",task_id:op.task_id}:null};
              if(request.platform)views.platform=request.platform;
              if(op.action==="read_deployment")views.deployment={spec:{package:packagePin,platform:request.platform||"macos-aarch64",manifest_json:"{}",size_bytes:4},offset:op.offset,data:[1,2,3,4].slice(op.offset)};
              event.source.postMessage({type:"kyuubiki:store-download/v1:result",id,session:event.data.session,views,error:null},event.origin);return;
            }
            if(request.action === "catalog") {
              const views={issuer:"https://account.example.test",host_platform:"macos-aarch64",catalog_revision:"r1",next_cursor:null,
                items:[{kind:"operator",asset_id:"demo",title:"Verified demo",summary:"Catalog fixture",domain:"mechanical",languages:["en"],latest_version:"1.0.0"}]};
              event.source.postMessage({type:"kyuubiki:store-download/v1:result",id,session:event.data.session,views,error:null},event.origin);return;
            }
            if(request.action === "prepare" && !window.downloads.length) window.downloads = [{
              id:"b".repeat(64),issuer:"https://account.example.test",selection:request.selection,state:"prepared",
              file_count:2,total_bytes:300,received_bytes:0,metered:true,remaining:1,unlimited:false,charged:null,error:null,directory:null}];
            if(request.action === "start") {
              window.starts++;
              window.downloads[0] = {...window.downloads[0],state:window.starts === 1 ? "failed" : "downloaded",
                remaining:0,charged:true,error:window.starts === 1 ? "download_connection_failed" : null,
                received_bytes:window.starts === 1 ? 0 : 300,directory:window.starts === 1 ? null : "/test-only/store-downloads/verified/files"};
            }
            if(request.action === "remove") window.downloads = [];
            if(request.action === "install") {
              const attempts=window.downloadActions.filter(r=>r.action==="install").length;
              window.downloads[0] = {...window.downloads[0],installation:attempts===1 ? "install_failed" : "installed"};
            }
            if(request.action === "stage_variant") window.downloads[0]={...window.downloads[0],installation:"variant_staged"};
            event.source.postMessage({type:"kyuubiki:store-download/v1:result",id,session:event.data.session,views:window.downloads,error:null},event.origin);
            return;
          }
          if (event.data?.type !== "kyuubiki:store-identity/v1") return;
          const {id,request} = event.data; window.actions.push(request.action);
          if (request.action === "begin") window.view = {...empty,status:"awaiting_browser"};
          if (request.action === "status" && window.next === "verified") window.view = {
            ...empty,status:"signed_in",issuer:"https://account.example.test",subject:"test-researcher",display_name:"Researcher",expires_at:2000000000,can_refresh:true};
          if (request.action === "refresh") window.view = {...window.view,expires_at:2000000600};
          if (request.action === "logout" || request.action === "cancel") { window.view = {...empty}; window.next = "waiting"; }
          event.source.postMessage({type:"kyuubiki:store-identity/v1:result",id,ok:true,status:window.view}, event.origin);
        });
        </script>` }));
      await page.goto("http://tauri.localhost/", { waitUntil: "networkidle", timeout: 60_000 });
      const frame = page.frames().find(frame => frame.url().startsWith(url.origin));
      assert.ok(frame, JSON.stringify({ frames: page.frames().map(frame => frame.url()), loadErrors }));
      await frame.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await frame.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
      await frame.locator('[data-workbench-store-view-tab="account"]').click();
      await frame.locator('[data-workbench-store-workspace-action="expand"]').click();
      await frame.locator('[data-store-account-origin]').fill("https://account.example.test");
      await frame.locator('[data-store-account-save]').click();
      await frame.locator('[data-store-identity-client]').fill("registered-test-client");
      await frame.locator('[data-store-identity-save]').click();
      await frame.locator('[data-store-identity-connect]').click();
      const wait = state => frame.waitForFunction(state => document.querySelector('[data-store-identity]')?.dataset.storeIdentityStatus === state, state);
      await wait("awaiting_browser");
      assert.equal(await frame.locator('[data-store-identity-logout]').count(), 0, "opening browser is not signed-in");
      await frame.locator('[data-store-identity-cancel]').click(); await wait("signed_out");
      await frame.locator('[data-store-identity-connect]').click(); await wait("awaiting_browser");
      await page.evaluate(() => { window.next = "verified"; }); await wait("signed_in");
      await frame.locator('[data-store-identity-refresh]').click();
      await page.waitForFunction(() => window.actions.includes("refresh"));
      assert.equal(await frame.locator('[data-store-identity]').evaluate(el => el.scrollWidth <= el.clientWidth + 1), true);
      const stored = await frame.evaluate(() => JSON.parse(localStorage.getItem("kyuubiki-store-public-login-settings-v1")));
      assert.deepEqual(Object.keys(stored).sort(), ["issuer","client_id","callback_port","ca_pem","ca_sha256"].sort());
      assert.equal(stored.callback_port, 47831);
      if (process.env.KYUUBIKI_STORE_SCREENSHOT_DIR) {
        await mkdir(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, `session-${width}.png`) });
      }
      await frame.locator('[data-store-identity-downloads]').click();
      const downloads = frame.locator('[data-store-downloads]');
      await downloads.locator('[data-store-catalog-load]').click();
      await downloads.locator('[data-store-catalog-select]').click();
      assert.equal(await downloads.locator('[data-store-download-asset]').inputValue(),"demo");
      assert.equal(await downloads.locator('[data-store-download-version]').inputValue(),"1.0.0");
      assert.equal(await downloads.locator('[data-store-download-platform]').inputValue(),"macos-aarch64");
      await downloads.locator('[data-store-download-key]').fill("transport-test");
      await downloads.locator('[data-store-download-pin]').fill("a".repeat(64));
      assert.equal(await page.evaluate(() => window.starts), 0);
      await downloads.locator('[data-store-download-prepare]').click();
      await downloads.locator('[data-store-download-state="prepared"]').waitFor();
      assert.equal(await page.evaluate(() => window.starts), 0, "preflight never authorizes or charges");
      await downloads.locator('[data-store-download-start]').click();
      await downloads.locator('[data-store-download-state="failed"]').waitFor();
      await frame.locator('[data-workbench-store-view-tab="account"]').click();
      await frame.locator('[data-workbench-store-view-tab="downloads"]').click();
      await downloads.locator('[data-store-download-state="failed"]').waitFor();
      assert.equal(await page.evaluate(() => window.starts), 1, "remount only reads the original record");
      await downloads.locator('[data-store-download-start]').click();
      await downloads.locator('[data-store-download-state="downloaded"]').waitFor();
      const startIds = await page.evaluate(() => window.downloadActions.filter(r => r.action === "start").map(r => r.id));
      assert.deepEqual(startIds, ["b".repeat(64), "b".repeat(64)]);
      assert.match(await downloads.textContent(), /not installed|Not installed/);
      assert.equal(await downloads.evaluate(el => el.scrollWidth <= el.clientWidth + 1), true);
      assert.equal(await downloads.locator('[data-store-download-start]').count(), 0);
      page.once("dialog", dialog => dialog.dismiss());
      await downloads.locator('[data-store-download-install]').click();
      assert.equal(await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="install").length),0);
      page.once("dialog", dialog => dialog.accept());
      await downloads.locator('[data-store-download-install]').click();
      await downloads.locator('[data-store-installation="install_failed"]').waitFor();
      assert.doesNotMatch(await downloads.locator('[data-store-download-id]').textContent(), /not installed|Not installed/);
      page.once("dialog", dialog => dialog.accept());
      await downloads.locator('[data-store-download-install]').click();
      await downloads.locator('[data-store-installation="installed"]').waitFor();
      assert.equal(await page.evaluate(()=>window.starts),2,"installation never authorizes another download");
      await downloads.locator('[data-store-download-project]').click();
      await frame.locator('[data-project-operators]').waitFor();
      assert.equal(await page.evaluate(()=>window.enabled.length),0,"installation never enables a project automatically");
      await frame.locator('[data-workbench-store-view-tab="downloads"]').click();
      await downloads.locator('[data-store-download-id]').waitFor();
      if (process.env.KYUUBIKI_STORE_SCREENSHOT_DIR) {
        await page.screenshot({ path: join(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, `download-${width}.png`) });
      }
      page.once("dialog", dialog => dialog.dismiss());
      await downloads.locator('[data-store-download-remove]').click();
      assert.equal(await downloads.locator('[data-store-download-state="downloaded"]').count(), 1);
      page.once("dialog", dialog => dialog.accept());
      await downloads.locator('[data-store-download-remove]').click();
      await downloads.locator('[data-store-download-id]').waitFor({state:"detached"});
      // A Mac catalog can explicitly choose Linux, but never installs it into the Mac library.
      const linuxPlatform=width===1440?"linux-x86_64":"linux-aarch64";
      await downloads.locator('[data-store-catalog-load]').click();
      await downloads.locator('[data-store-catalog-platform]').selectOption(linuxPlatform);
      await downloads.locator('[data-store-catalog-select]').click();
      assert.equal(await downloads.locator('[data-store-download-platform]').inputValue(),linuxPlatform);
      await downloads.locator('[data-store-download-prepare]').click();
      await downloads.locator('[data-store-download-state="prepared"]').waitFor();
      await downloads.locator('[data-store-download-start]').click();
      await downloads.locator('[data-store-download-state="downloaded"]').waitFor();
      const installCount=await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="install").length);
      page.once("dialog",dialog=>dialog.accept());await downloads.locator('[data-store-download-stage-variant]').click();
      await downloads.locator('[data-store-installation="variant_staged"]').waitFor();
      await downloads.locator('[data-store-download-project]').click();
      assert.equal(await frame.locator('[data-project-operator-platform]').inputValue(),linuxPlatform);
      assert.equal(await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="install").length),installCount);
      await frame.locator('[data-workbench-store-view-tab="account"]').click();
      await wait("signed_in");
      await frame.locator('[data-store-identity-logout]').click(); await wait("signed_out");
      assert.equal(await frame.locator('[data-store-identity]').textContent().then(text => text.includes("test-researcher")), false);
      await frame.locator('[data-workbench-store-view-tab="operators"]').click();
      const project=frame.locator('[data-project-operators]');
      assert.equal(await project.locator('[data-project-operator-platform]').inputValue(),linuxPlatform);
      await project.locator('[data-project-operator-refresh]').click();
      await project.locator('[data-project-operator-installed] option[value="demo"]').waitFor({state:"attached"});
      await project.locator('[data-project-operator-installed]').selectOption("demo");
      page.once("dialog",dialog=>dialog.dismiss());await project.locator('[data-project-operator-enable]').click();
      assert.equal(await page.evaluate(()=>window.enabled.length),0);
      page.once("dialog",dialog=>dialog.accept());await project.locator('[data-project-operator-enable]').click();
      await project.locator('[data-project-operator-enabled] option[value="demo"]').waitFor({state:"attached"});
      await project.locator('[data-project-operator-enabled]').selectOption("demo");
      await project.locator('[data-project-operator-id]').selectOption("extract.demo");
      await project.locator('[data-project-operator-input]').fill('{"values":[2,4,8]}');
      await project.locator('[data-project-operator-prepare]').click();
      await project.locator('[role="status"]').filter({hasText:"Task prepared"}).waitFor();
      assert.equal(await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="project"&&r.operation.action==="prepare").length),1);
      await frame.evaluate(()=>window.__kyuubikiPwdt.invoke("settings/patch",{frontendRuntimeMode:"direct_mesh_gui"}));
      await project.locator('[data-project-operator-endpoint]').fill("127.0.0.1:5001");
      await project.locator('[data-project-operator-inspect-target]').click();
      await project.locator('[role="status"]').filter({hasText:"Agent platform does not match"}).waitFor();
      assert.deepEqual(deploymentActions,["inspect"],"wrong platform causes no source reads or uploads");
      deploymentActions.length=0;remotePlatform=linuxPlatform;
      await project.locator('[data-project-operator-inspect-target]').click();
      await project.locator('[data-project-operator-target-state="missing"]').waitFor();
      page.once("dialog",dialog=>dialog.accept());await project.locator('[data-project-operator-deploy]').click();
      await project.locator('[role="status"]').filter({hasText:"test_commit_response_lost"}).waitFor();
      assert.equal(await project.locator('[data-project-operator-target-state="ready"]').count(),0,"lost reply cannot assert readiness");
      assert.equal(await project.locator('[data-project-operator-target-state="installed_restart_required"]').count(),0,"lost reply cannot assert installation");
      assert.deepEqual(deploymentActions,["inspect","inspect","begin","chunk","commit"]);
      const sourceReads=await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="project"&&r.operation.action==="read_deployment").length);
      page.once("dialog",dialog=>dialog.accept());await project.locator('[data-project-operator-deploy]').click();
      await project.locator('[data-project-operator-target-state="installed_restart_required"]').waitFor();
      assert.deepEqual(deploymentActions,["inspect","inspect","begin","chunk","commit","inspect"],"resume inspects without replaying a committed upload");
      assert.equal(await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="project"&&r.operation.action==="read_deployment").length),sourceReads);
      remoteState="activation_failed";
      page.once("dialog",dialog=>dialog.accept());await project.locator('[data-project-operator-deploy]').click();
      await project.locator('[data-project-operator-target-state="activation_failed"]').waitFor();
      assert.match(await project.locator('[data-project-operator-target-state]').innerText(),/failed to load/);
      assert.equal(await project.locator('[data-project-operator-activation-error]').innerText(),"operator_package_activation_failed");
      assert.equal(deploymentActions.filter(a=>a==="begin").length,1,"failed activation never starts another upload");
      assert.equal(await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="project"&&r.operation.action==="read_deployment").length),sourceReads);
      const preparations=await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="project"&&r.operation.action==="prepare").length);
      page.once("dialog",dialog=>dialog.accept());await project.locator('[data-project-operator-run]').click();
      await project.locator('[role="status"]').filter({hasText:"failed to load"}).waitFor();
      assert.equal(await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="project"&&r.operation.action==="prepare").length),preparations,"execution is blocked before preparing or dispatching a task");
      assert.equal(await project.evaluate(el=>el.scrollWidth<=el.clientWidth+1),true,"activation diagnosis fits the viewport");
      remoteState="ready";await project.locator('[data-project-operator-inspect-target]').click();
      await project.locator('[data-project-operator-target-state="ready"]').waitFor();
      remotePlatform=linuxPlatform==="linux-x86_64"?"linux-aarch64":"linux-x86_64";
      await project.locator('[data-project-operator-inspect-target]').click();
      await project.locator('[role="status"]').filter({hasText:"Agent platform does not match"}).waitFor();
      assert.equal(await project.locator('[data-project-operator-target-state]').count(),0,"failed recheck clears stale readiness");
      remotePlatform=linuxPlatform;
      remoteState="version_conflict";
      page.once("dialog",dialog=>dialog.accept());await project.locator('[data-project-operator-deploy]').click();
      await project.locator('[data-project-operator-target-state="version_conflict"]').waitFor();
      assert.equal(deploymentActions.filter(a=>a==="begin").length,1,"conflicts never upload, overwrite or restart");
      assert.equal(await project.evaluate(el=>el.scrollWidth<=el.clientWidth+1),true,"project workflow fits the viewport");
      if(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR)await page.screenshot({path:join(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR,`project-${width}.png`)});
      page.once("dialog",dialog=>dialog.accept());await project.locator('[data-project-operator-disable]').click();
      await page.waitForFunction(()=>window.enabled.length===0);
      await project.locator('[data-project-operator-platform]').selectOption("");
      assert.equal(await project.locator('[data-project-operator-target-state]').count(),0);
      assert.equal(await project.locator('[data-project-operator-installed]').inputValue(),"");
      assert.equal(await page.evaluate(()=>window.starts),3,"signed-out project actions never acquire a store grant");
      const requests=await page.evaluate(()=>window.downloadActions.filter(r=>r.action==="project"));
      assert.ok(requests.every(r=>r.platform===linuxPlatform),"project selection never silently falls back to the host platform");
      assert.deepEqual(errors, []);
    } finally { await context.close(); }
  }, { timeout: 120_000 });
}

for (const width of [1440, 390]) {
  test(`Store account registration stays explicit and separate from local research at ${width}px`, async () => {
    const context = await browser.newContext({ viewport: { width, height: 900 } });
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    try {
      await context.route("https://account.example.test/**", route => route.fulfill({
        contentType: "text/html", body: "<h1>Test shared account portal</h1>",
      }));
      await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
      const accountTab = page.locator('[data-workbench-store-view-tab="account"]');
      await accountTab.click({ timeout: 10_000 });
      await page.locator('[data-workbench-store-workspace-action="expand"]').click();
      const panel = page.locator('[data-store-account]');
      const origin = panel.locator('[data-store-account-origin]');
      const save = panel.locator('[data-store-account-save]');
      const open = panel.locator('[data-store-account-open]');
      assert.equal(await open.count(), 0, "no guessed account destination");
      await origin.fill("https://account.example.test?token=secret");
      assert.equal(await save.isDisabled(), true);
      assert.equal(await open.count(), 0);
      await origin.fill("https://account.example.test");
      await save.click();
      assert.equal(await page.evaluate(() => localStorage.getItem("kyuubiki-store-account-origin")), "https://account.example.test");
      assert.equal(await open.getAttribute("target"), "_blank");
      assert.match(await open.getAttribute("rel"), /noopener/);
      assert.match(await open.getAttribute("href"), /^https:\/\/account.example.test\/account\?lang=(en|zh)$/);
      const currentUrl = page.url();
      const popupPromise = page.waitForEvent("popup");
      await open.click();
      const popup = await popupPromise;
      await popup.waitForLoadState();
      assert.equal(await popup.locator("h1").textContent(), "Test shared account portal");
      assert.equal(await popup.evaluate(() => window.opener === null), true);
      assert.equal(page.url(), currentUrl);
      await popup.close();
      assert.equal(await panel.locator('[data-store-identity]').getAttribute("data-store-identity-status"), "signed_out");
      assert.equal(await panel.locator(".store-account-session p").first().isVisible(), true, "the login boundary remains readable on narrow screens");
      assert.equal(await panel.locator(".store-account-steps p").first().isVisible(), true, "registration guidance must not disappear in compact mode");
      assert.equal(await panel.locator('[role="status"]').first().isVisible(), true, "opening a website is not a silent login");
      assert.equal(await panel.evaluate(el => el.scrollWidth <= el.clientWidth + 1), true);
      if (process.env.KYUUBIKI_STORE_SCREENSHOT_DIR) {
        await mkdir(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, `account-${width}.png`) });
      }
      await origin.fill("http://account.example.test");
      assert.equal(await open.count(), 0, "an invalid edit must not open the previous destination");
      await page.reload({ waitUntil: "networkidle" });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
      await accountTab.click();
      await page.waitForFunction(() => document.querySelector('[data-store-account]')?.getAttribute("aria-busy") === "false");
      assert.equal(await origin.inputValue(), "https://account.example.test");
      await panel.locator('[data-store-account-clear]').click();
      assert.equal(await open.count(), 0);
      assert.equal(await page.evaluate(() => localStorage.getItem("kyuubiki-store-account-origin")), null);
      await panel.locator('[data-store-account-back]').click();
      assert.equal(await page.locator('[data-workbench-store-panel]').getAttribute("data-workbench-store-view"), "catalog");
      assert.deepEqual(errors, []);
    } finally { await context.close(); }
  }, { timeout: 120_000 });
}

test("Store account preference failure is visible and can recover without creating a session", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    await page.evaluate(() => {
      const original = Storage.prototype.setItem;
      Storage.prototype.setItem = function (key, value) {
        if (key === "kyuubiki-store-account-origin") throw new DOMException("Private test detail", "QuotaExceededError");
        return original.call(this, key, value);
      };
      window.restoreAccountStorage = () => { Storage.prototype.setItem = original; };
      window.__kyuubikiPwdt.openSidebar("store");
    });
    await page.locator('[data-workbench-store-view-tab="account"]').click();
    const panel = page.locator('[data-store-account]');
    await panel.locator('[data-store-account-origin]').fill("https://account.example.test");
    await panel.locator('[data-store-account-save]').click();
    await panel.locator(':scope > [role="status"]').waitFor();
    assert.match(await panel.locator(':scope > [role="status"]').textContent(), /could not be read or saved/);
    assert.doesNotMatch(await panel.textContent(), /Private test detail/);
    assert.equal(await panel.locator('[data-store-account-open]').count(), 0);
    await page.evaluate(() => window.restoreAccountStorage());
    await panel.locator('[data-store-account-save]').click();
    await panel.locator('[data-store-account-open]').waitFor();
    assert.equal(await panel.locator('[data-store-identity]').getAttribute("data-store-identity-status"), "signed_out");
  } finally { await context.close(); }
}, { timeout: 90_000 });

test("Store PWDT selects an exact source/version and agrees with the GUI reference", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const errors = [];
  let legacyCalls = 0;
  const candidates = [
    { ...entries[0], id: "shared.operator", title: "Shared operator" },
    { ...entries[0], id: "shared.operator", title: "Shared operator", source_id: "lab" },
    { ...entries[0], id: "shared.operator", title: "Shared operator", source_id: "lab", version: "2.0.0",
      package_ref: "operator://shared/2", install: { mode: "source_reference", requires_download: true, target: "macos-aarch64" } },
  ];
  page.on("pageerror", (error) => errors.push(error.message));
  const stage = (selection = {}) => page.evaluate((selection) =>
    window.__kyuubikiPwdt.stageStoreEntry("operator", "shared.operator", selection), selection);
  const manifestEntries = () => page.evaluate(() => JSON.parse(
    localStorage.getItem("kyuubiki-workbench-store-manifests") || "{}",
  )["qualification-project"]?.entries || []);
  try {
    await page.route("**/api/v1/store**", async (route) => {
      if (new URL(route.request().url()).pathname !== "/api/v1/store") {
        legacyCalls++;
        await route.fulfill({ json: { entry: candidates[0] } });
        return;
      }
      // Deliberately ignore server-side filters: the client must still check identity.
      await route.fulfill({ json: { entries: candidates, sources,
        summary: { entry_count: candidates.length, kinds: { operator: candidates.length }, sources: {} } } });
    });
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedProjectId: "qualification-project" }));
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
    await page.locator('[data-workbench-store-entry-id="shared.operator"]').first().waitFor();
    await assert.rejects(stage(), /store_entry_ambiguous/);
    await assert.rejects(stage({ sourceId: "lab" }), /store_entry_ambiguous/);
    await assert.rejects(stage({ sourceId: "lab", version: "missing" }), /store_entry_not_found/);
    assert.deepEqual(await manifestEntries(), []);
    const selection = { sourceId: "lab", version: "2.0.0", packageRef: "operator://shared/2", target: "macos-aarch64" };
    const result = await stage(selection);
    for (const [key, value] of Object.entries(selection)) assert.equal(result[key], value);
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ storeManifestEntryCount: 1 }));
    const staged = await manifestEntries();
    assert.equal(staged[0].source_id, "lab");
    assert.equal(staged[0].version, "2.0.0");
    await assert.rejects(stage({ sourceId: "builtin.local" }), /Another source or version/);
    assert.deepEqual(await manifestEntries(), staged, "a rejected replacement cannot rewrite the existing reference");
    let confirmed = false;
    page.once("dialog", async (dialog) => {
      assert.equal(dialog.type(), "confirm");
      confirmed = true;
      await dialog.accept();
    });
    await page.evaluate(() => window.__kyuubikiPwdt.removeStoreEntry("operator", "shared.operator"));
    assert.equal(confirmed, true, "explicit removal retains the product confirmation boundary");
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ storeManifestEntryCount: 0 }));
    const chosenCard = page.locator('[data-workbench-store-entry-id="shared.operator"]').nth(2);
    await chosenCard.locator('[data-workbench-store-entry-action="details"]').click();
    await page.locator('[data-workbench-store-detail-action="stage"]').click();
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ storeManifestEntryCount: 1 }));
    const fromGui = await manifestEntries();
    assert.deepEqual({ ...fromGui[0], installed_at: null }, { ...staged[0], installed_at: null });
    assert.equal(legacyCalls, 0, "no unqualified kind/id detail fallback");
    assert.deepEqual(errors, []);
  } finally { await context.close(); }
}, { timeout: 120_000 });

for (const viewport of [{ width: 1440, height: 900 }, { width: 390, height: 844 }]) {
  test(`Store workspace browse, filter, detail and manifest loop at ${viewport.width}px`, async () => {
    const context = await browser.newContext({ viewport });
    const page = await context.newPage();
    const errors = [];
    let failNext = false;
    page.on("pageerror", (error) => errors.push(error.message));
    try {
      await page.route("**/api/v1/store**", async (route) => {
        if (failNext) {
          failNext = false;
          await route.fulfill({ status: 503, json: { error: "Store temporarily unavailable" } });
          return;
        }
        const query = new URL(route.request().url()).searchParams.get("q") || "";
        const filtered = entries.filter((entry) => entry.title.includes(query));
        await route.fulfill({ json: { entries: filtered, sources, summary: { entry_count: filtered.length, kinds: { operator: filtered.length }, sources: {} } } });
      });
      await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await page.evaluate(() => window.__kyuubikiPwdt.waitUntil(
        (state) => state.selectedProjectId === "qualification-project", { timeoutMs: 15_000 },
      ));
      await page.locator('[aria-label="workbench-rail:store"]').click();
      await page.locator('[data-workbench-store-entry-id]').first().waitFor();
      assert.equal(await page.locator('[data-workbench-store-entry-id]').count(), 4);
      await page.locator('[data-workbench-store-workspace-action="expand"]').click();
      const dialog = page.locator('[data-workbench-store-workspace="expanded"]');
      await dialog.waitFor();
      assert.equal(await dialog.evaluate((element) => element.matches(":modal")), true);
      assert.equal(await dialog.locator('[data-workbench-store-entry-id]').count(), viewport.width <= 600 ? 4 : 12);
      assert.equal(await dialog.locator('[data-workbench-panel-content]').evaluate((element) => element.scrollWidth <= element.clientWidth + 1), true);
      const box = await dialog.boundingBox();
      assert.ok(box.x >= 0 && box.y >= 0 && box.x + box.width <= viewport.width + 1 && box.y + box.height <= viewport.height + 1);
      if (process.env.KYUUBIKI_STORE_SCREENSHOT_DIR) {
        await mkdir(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, `store-${viewport.width}.png`) });
      }
      await dialog.locator("details > summary").click();
      await dialog.locator('[data-workbench-store-filter="source"]').selectOption("lab");
      await dialog.locator('[data-workbench-store-filter="domain"]').selectOption("mechanical");
      assert.equal(await dialog.locator('[data-workbench-store-entry-id]').count(), 4);
      await dialog.locator('[data-workbench-store-entry-action="details"]').first().click();
      const detail = dialog.locator('[data-workbench-store-detail]');
      await detail.waitFor();
      assert.match(await detail.textContent(), /not connected yet|尚未接通|未接続/);
      await detail.locator('[data-workbench-store-detail-action="stage"]').click();
      await page.waitForFunction(() => document.querySelector('[data-workbench-store-panel]')?.getAttribute("data-workbench-store-manifest-count") === "1");
      assert.equal(await detail.locator('[data-workbench-store-detail-action="stage"]').isDisabled(), true);
      await dialog.locator('[data-workbench-store-view-tab="project"]').click();
      await dialog.locator('[data-workbench-store-manifest-action="remove"]').click();
      assert.equal(await dialog.locator('[data-workbench-store-manifest-entry-id]').count(), 0);
      await dialog.locator('[data-workbench-store-view-tab="catalog"]').click();
      assert.equal(await dialog.locator('[data-workbench-store-filter="source"]').inputValue(), "lab");
      const search = dialog.locator('[data-workbench-store-search="query"]');
      await search.fill("not-found");
      await search.press("Enter");
      await page.waitForFunction(() => document.querySelector('[data-workbench-store-panel]')?.getAttribute("data-workbench-store-visible-count") === "0");
      await search.fill("");
      await search.press("Enter");
      await dialog.locator('[data-workbench-store-entry-id]').first().waitFor();
      failNext = true;
      await dialog.locator('[data-workbench-store-action="refresh"]').click();
      await dialog.locator('[data-workbench-store-status="error"]').waitFor();
      assert.equal(await dialog.locator('[data-workbench-store-entry-action="stage"]:enabled').count(), 0);
      await dialog.locator('[data-workbench-store-action="refresh"]').click();
      await dialog.locator('[data-workbench-store-status="ready"]').waitFor();
      assert.equal(await dialog.locator('[data-workbench-store-entry-action="stage"]:enabled').count(), 4);
      await page.keyboard.press("Escape");
      await dialog.waitFor({ state: "detached" });
      assert.equal(await page.locator('[data-workbench-store-workspace-action="expand"]').evaluate((element) => element === document.activeElement), true);
      assert.deepEqual(errors, []);
    } finally { await context.close(); }
  }, { timeout: 120_000 });
}
