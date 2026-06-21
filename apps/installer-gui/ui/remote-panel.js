export function mountRemotePanel() {
  const root = document.getElementById("remote-panel-root");
  if (!root) return;
  root.innerHTML = `
    <div class="section-header">
      <div>
        <p class="section-eyebrow desktop-shell-eyebrow">Remote deployment</p>
        <h2>Operate remote solver nodes</h2>
        <p class="desktop-shell-note">
          Installer enforces bounded remote targets: absolute workspace paths only, plain host fields only, and optional host/workspace allowlists from the desktop environment.
        </p>
      </div>
    </div>

    <div class="form-shell">
      <div class="panel-header"><h2>SSH target</h2></div>
      <div class="field-grid">
        <label class="field"><span>SSH user</span><input id="remote-ssh-user" type="text" placeholder="ubuntu" /></label>
        <label class="field"><span>Target host</span><input id="remote-target-host" type="text" placeholder="10.20.0.11" /></label>
        <label class="field"><span>SSH port</span><input id="remote-ssh-port" type="number" placeholder="22" /></label>
        <label class="field field-span-2"><span>Remote workspace</span><input id="remote-workspace" type="text" placeholder="/opt/kyuubiki" /></label>
      </div>
      <div class="action-row">
        <button class="primary" data-action="remote-bootstrap">Remote bootstrap</button>
      </div>
    </div>

    <div class="form-shell">
      <div class="panel-header"><h2>Remote policy</h2></div>
      <div class="field-grid">
        <label class="field field-span-2"><span>Allowed hosts</span><input id="remote-policy-allowed-hosts" type="text" placeholder="192.168.1.12,solver-a" /></label>
        <label class="field field-span-2"><span>Allowed workspace roots</span><input id="remote-policy-allowed-workspaces" type="text" placeholder="/opt/kyuubiki,/srv/kyuubiki" /></label>
      </div>
      <div class="field-grid">
        <label class="field field-span-2"><span>Effective hosts</span><code id="remote-policy-effective-hosts">(unbounded)</code></label>
        <label class="field field-span-2"><span>Effective workspace roots</span><code id="remote-policy-effective-workspaces">(unbounded)</code></label>
        <label class="field field-span-2"><span>Config path</span><code id="remote-policy-config-path">config/installer-remote-policy.json</code></label>
      </div>
      <div class="action-row">
        <button data-action="refresh-remote-policy">Refresh policy</button>
        <button class="primary" data-action="save-remote-policy">Save policy</button>
      </div>
    </div>

    <div class="form-shell">
      <div class="panel-header">
        <h2>Certificate authority</h2>
        <p class="desktop-shell-note">
          Installer-managed PKI for orchestrated nodes and offline mesh agents. All storage paths and policy switches stay visible here.
        </p>
      </div>
      <div class="field-grid">
        <label class="field field-span-2"><span>Storage root</span><input id="certificate-policy-storage-root" type="text" placeholder="/abs/path/to/config/certificates" /></label>
        <label class="field"><span>Root common name</span><input id="certificate-policy-root-common-name" type="text" placeholder="kyuubiki-local-ca" /></label>
        <label class="field"><span>Default validity days</span><input id="certificate-policy-default-validity-days" type="number" min="1" max="3650" placeholder="365" /></label>
        <label class="field"><span>Require for orchestrated</span><select id="certificate-policy-require-orchestrated"><option value="false">Disabled</option><option value="true">Enabled</option></select></label>
        <label class="field"><span>Require for offline mesh</span><select id="certificate-policy-require-offline-mesh"><option value="true">Enabled</option><option value="false">Disabled</option></select></label>
        <label class="field field-span-2"><span>Allow SSH trust bootstrap</span><select id="certificate-policy-allow-ssh-bootstrap"><option value="true">Enabled</option><option value="false">Disabled</option></select></label>
      </div>
      <div class="field-grid">
        <label class="field field-span-2"><span>CA state</span><code id="certificate-ca-state">not initialized</code></label>
        <label class="field field-span-2"><span>CA fingerprint</span><code id="certificate-ca-fingerprint">(not initialized)</code></label>
        <label class="field field-span-2"><span>CA subject</span><code id="certificate-ca-subject">(not initialized)</code></label>
        <label class="field field-span-2"><span>CA certificate path</span><code id="certificate-ca-cert-path"></code></label>
        <label class="field field-span-2"><span>CA private key path</span><code id="certificate-ca-key-path"></code></label>
        <label class="field field-span-2"><span>Policy config path</span><code id="certificate-policy-config-path">config/installer-certificate-policy.json</code></label>
        <label class="field field-span-2"><span>Inventory path</span><code id="certificate-policy-inventory-path">config/installer-certificates.json</code></label>
      </div>
      <div class="action-row">
        <button data-action="refresh-certificate-policy">Refresh PKI</button>
        <button data-action="initialize-certificate-authority">Initialize CA</button>
        <button class="primary" data-action="save-certificate-policy">Save certificate policy</button>
      </div>
    </div>

    <div class="form-shell">
      <div class="panel-header"><h2>Node certificate issue</h2></div>
      <div class="field-grid">
        <label class="field"><span>Label</span><input id="certificate-issue-label" type="text" placeholder="lab-a" /></label>
        <label class="field"><span>Target host</span><input id="certificate-issue-target-host" type="text" placeholder="192.168.1.12" /></label>
        <label class="field"><span>Advertise host</span><input id="certificate-issue-advertise-host" type="text" placeholder="192.168.1.12" /></label>
        <label class="field"><span>Agent id</span><input id="certificate-issue-agent-id" type="text" placeholder="solver-lab-a" /></label>
        <label class="field"><span>Control mode</span><select id="certificate-issue-control-mode"><option value="orchestrated">Orchestrated</option><option value="offline_mesh">Offline mesh</option></select></label>
        <label class="field"><span>Override validity days</span><input id="certificate-issue-validity-days" type="number" min="1" max="3650" placeholder="use policy default" /></label>
        <label class="field field-span-2"><span>Extra subject alt names</span><input id="certificate-issue-sans" type="text" placeholder="solver-a.local,10.20.0.12" /></label>
      </div>
      <div class="action-row">
        <button class="primary" data-action="issue-node-certificate">Issue node certificate</button>
      </div>
    </div>

    <div class="form-shell">
      <div class="panel-header"><h2>Certificate inventory</h2></div>
      <pre id="certificate-inventory-summary">active 0 · revoked 0 · default 365 days</pre>
      <div class="field-grid">
        <label class="field field-span-2"><span>Revoke certificate</span><select id="certificate-revoke-id"><option value="">Select certificate id</option></select></label>
      </div>
      <div class="action-row">
        <button data-action="revoke-node-certificate">Revoke selected certificate</button>
      </div>
      <div id="certificate-inventory-list" class="certificate-record-grid"></div>
    </div>

    <div class="form-shell">
      <div class="panel-header"><h2>Remote nodes</h2></div>
      <label class="field"><span>Node registry JSON</span><textarea id="remote-node-registry" rows="8" placeholder='[{"label":"lab-a","target_host":"192.168.1.12","ssh_user":"kyuubiki-dev","remote_workspace":"/opt/kyuubiki","ssh_port":22,"control_mode":"orchestrated","orchestrator_url":"http://192.168.1.10:4000","agent_id":"solver-lab-a","advertise_host":"192.168.1.12","agent_port":5001,"certificate_id":"lab-a-1720000000000"},{"label":"mesh-b","target_host":"192.168.1.22","ssh_user":"kyuubiki-dev","remote_workspace":"/opt/kyuubiki","ssh_port":22,"control_mode":"offline_mesh","cluster_id":"lan-a","peer_endpoints":["192.168.1.23:5001"],"agent_id":"solver-mesh-b","advertise_host":"192.168.1.22","agent_port":5001,"certificate_id":"mesh-b-1720000000001"}]'></textarea></label>
      <pre id="remote-node-summary"></pre>
      <div class="field-grid">
        <label class="field"><span>Search nodes</span><input id="remote-node-search" type="text" placeholder="label, host, agent, cluster" /></label>
        <label class="field"><span>Filter status</span><select id="remote-node-filter"><option value="all">All</option><option value="ok">Healthy</option><option value="failed">Failed</option><option value="unknown">Unknown</option></select></label>
        <label class="field"><span>Filter mode</span><select id="remote-node-mode-filter"><option value="all">All</option><option value="orchestrated">Orchestrated</option><option value="offline_mesh">Offline mesh</option></select></label>
        <label class="field"><span>Certificate state</span><select id="remote-node-certificate-filter"><option value="all">All</option><option value="aligned">Aligned</option><option value="available">Available</option><option value="ambiguous">Ambiguous</option><option value="stale">Stale</option><option value="missing">Missing</option></select></label>
        <label class="field"><span>Group by</span><select id="remote-node-group"><option value="none">None</option><option value="status">Status</option><option value="workspace">Workspace</option><option value="control_mode">Control mode</option><option value="orchestrator">Orchestrator</option></select></label>
      </div>
      <div class="action-row">
        <button data-remote-bulk-action="probe">Probe visible</button>
        <button data-remote-bulk-action="mesh-preflight">Mesh preflight</button>
        <button data-remote-bulk-action="bootstrap">Bootstrap visible</button>
        <button data-remote-bulk-action="start">Start visible</button>
        <button data-remote-bulk-action="assign-certificates">Assign certificates</button>
        <button data-remote-bulk-action="clear-certificates">Clear certificates</button>
        <button id="remote-certificate-bulk-action" data-remote-bulk-action="certificate-focus-action">Resolve visible certificate state</button>
      </div>
      <div id="remote-mesh-health" class="remote-mesh-health"></div>
      <div id="remote-certificate-health" class="remote-certificate-health"></div>
      <div id="remote-mesh-issues" class="remote-mesh-issues"></div>
      <div id="remote-mesh-clusters" class="remote-mesh-clusters"></div>
      <div id="remote-node-cards" class="remote-node-grid"></div>
      <div class="action-row">
        <button data-action="refresh-remote-nodes">Refresh nodes</button>
        <button data-action="use-first-remote-node">Use first node</button>
        <button data-action="probe-remote-node">Probe node</button>
        <button class="primary" data-action="save-remote-nodes">Save nodes</button>
      </div>
    </div>

    <div class="form-shell">
      <div class="panel-header"><h2>Remote agent</h2></div>
      <div class="field-grid">
        <label class="field"><span>Control mode</span><select id="remote-control-mode"><option value="orchestrated">Orchestrated</option><option value="offline_mesh">Offline mesh</option></select></label>
        <label class="field"><span>Agent id</span><input id="remote-agent-id" type="text" placeholder="solver-shanghai-a" /></label>
        <label class="field"><span>Advertise host</span><input id="remote-advertise-host" type="text" placeholder="10.20.0.11" /></label>
        <label class="field"><span>Agent port</span><input id="remote-agent-port" type="number" placeholder="5001" /></label>
        <label class="field field-span-2">
          <span>Certificate id</span>
          <select id="remote-certificate-id">
            <option value="">Auto-match active certificate</option>
          </select>
        </label>
        <label class="field field-span-2"><span>Orchestrator URL</span><input id="remote-orchestrator-url" type="text" placeholder="http://control-plane.example.com:4000" /></label>
        <label class="field"><span>Mesh cluster id</span><input id="remote-cluster-id" type="text" placeholder="lan-a" /></label>
        <label class="field field-span-2"><span>Peer endpoints</span><input id="remote-peer-endpoints" type="text" placeholder="10.0.0.11:5001,10.0.0.12:5001" /></label>
      </div>
      <div class="action-row">
        <button class="primary" data-action="remote-start-agent">Start remote agent</button>
      </div>
    </div>
  `;
}
