# Ubuntu 24 Remote Pilot

Use this guide when you want to turn one local `Ubuntu 24` machine into the
first real remote target for `Kyuubiki Hub`.

The goal is not to prove every deployment shape at once. The goal is to make
one Linux box useful enough for:

- remote solver-node testing
- remote control-plane testing
- Hub runtime watch and provenance checks
- first practical operator runs outside the laptop-local stack

## Recommended order

Treat the Ubuntu host as three increasing levels of confidence:

1. `remote solver node`
   The fastest path. Keep the orchestrator local and let Ubuntu only run a
   headless Rust agent.
2. `remote control plane`
   The next step. Run frontend + orchestrator on Ubuntu and use Hub to point at
   it as a first-party remote target.
3. `remote catalog / workload source`
   The final pass. Let Ubuntu serve `/api/v1/workloads/catalog` and project
   bundles so Hub exercises remote workload intake too.

For a first real operator pass, level `1` is the best return.

## Level 1: Remote solver node

Use this when you want to test:

- remote agent registration
- registry/manifest/static discovery
- Hub runtime watch against a non-local agent path
- orchestrated solves that leave the laptop

Recommended shape:

- laptop:
  - Hub
  - Workbench
  - local orchestrator on `4000`
- Ubuntu host:
  - `kyuubiki-cli` Rust agent on `5001`

### Fastest setup

Use this when you want the smallest possible change from today's local flow.

On the Ubuntu host:

1. install the Rust toolchain and build dependencies
2. clone the same repository revision
3. run one Rust agent on `0.0.0.0:5001`

On the laptop:

1. point `KYUUBIKI_AGENT_ENDPOINTS` at the Ubuntu host
2. keep the orchestrator local
3. use Hub and Workbench exactly as before

### Ubuntu host commands

These commands assume:

- Ubuntu host IP: `192.168.1.50`
- repo checkout path: `~/kyuubiki`

Install the minimum build/runtime packages:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev screen curl git zsh
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
```

Clone and build:

```bash
git clone <your-repo-url> ~/kyuubiki
cd ~/kyuubiki
./scripts/kyuubiki build-agent
```

Start one remote agent in the foreground:

```bash
cd ~/kyuubiki
./scripts/kyuubiki agent -- --port 5001
```

Or keep it in the background with `screen`:

```bash
cd ~/kyuubiki
screen -S kyuubiki-agent-5001 -X quit >/dev/null 2>&1 || true
screen -dmS kyuubiki-agent-5001 zsh -lc './scripts/kyuubiki agent -- --port 5001'
```

If Ubuntu firewall is enabled, allow the agent port:

```bash
sudo ufw allow 5001/tcp
```

### Laptop changes

On the laptop, point the local orchestrator at the Ubuntu host:

```bash
export KYUUBIKI_AGENT_DISCOVERY=static
export KYUUBIKI_AGENT_ENDPOINTS=192.168.1.50:5001
./scripts/kyuubiki restart-local
```

If you prefer to keep this across sessions, put the same values in
`/Users/Shared/chroot/dev/kyuubiki/.env.local` on the laptop checkout.

### Validate the link before using Hub

Check that the control plane can see the remote agent:

```bash
curl http://127.0.0.1:4000/api/v1/protocol/agents
```

You should see a descriptor whose transport points at the Ubuntu host and port
`5001`.

Then verify that the local stack itself is healthy:

```bash
./scripts/kyuubiki status
curl http://127.0.0.1:4000/api/health
```

### First Hub pass

After the HTTP checks pass:

1. open Hub
2. go to `Observe`
3. confirm the runtime watch still feels understandable
4. open Workbench
5. run one study from each domain:
   - `axial_bar_1d`
   - `thermal_bar_1d`
   - one thermo-mechanical sample
6. confirm report and export still behave normally

### What to validate

- the Ubuntu agent is reachable
- `/api/v1/protocol/agents` shows the remote node
- Hub `Observe` can still guide first-line troubleshooting
- a representative job completes through the remote agent

### Best first studies

- `axial_bar_1d`
- `beam_1d`
- `thermal_bar_1d`

These are easier to reason about if something goes wrong.

## Level 2: Remote control plane

Use this when you want to test:

- Hub against a non-local orchestrator
- remote `/api/health`
- remote `/api/v1/protocol*`
- remote `/api/v1/jobs`
- remote `/api/v1/results`

Recommended shape:

- laptop:
  - Hub
- Ubuntu host:
  - frontend
  - orchestrator
  - one or more Rust agents

This is the first point where Hub becomes a genuine operator shell for another
machine rather than a launcher for the current one.

### What to validate

- Hub can treat Ubuntu as the active runtime target
- runtime watch still makes failures understandable
- Workbench can open the remote frontend and submit a study
- token-protected reads still behave correctly

## Level 3: Remote workload source

Use this when you want to test:

- first-party remote workload catalog behavior
- `analysis_domains`
- `analysis_families`
- `thermal_intents`
- bundle download + attach back into local Hub library

Recommended shape:

- Ubuntu serves:
  - `/api/v1/workloads/catalog`
  - `/api/v1/projects/:project_id/bundle`

### What to validate

- Hub classifies remote workloads correctly
- provenance reads as first-party remote control plane
- `Mechanical / Thermal / Thermo-mechanical` filters still work
- opening a remote workload lands in the expected Workbench context

## First real pilot checklist

If you only do one short pilot, use this:

1. Put one Rust agent on Ubuntu.
2. Point local orchestrator discovery at that node.
3. Confirm `/api/v1/protocol/agents` sees it.
4. Open Hub `Observe`.
5. Run:
   - one `Mechanical` study
   - one `Thermal` study
   - one `Thermo-mechanical` study
6. Confirm:
   - result completes
   - runtime watch is understandable
   - logs remain discoverable
   - exports still work

## Good enough criteria

Treat the Ubuntu pilot as successful when all of these are true:

- Hub can distinguish local vs remote runtime context without confusion
- one remote agent can solve representative studies reliably
- first-line troubleshooting is still possible from Hub alone
- remote control-plane reads feel no rougher than local reads
- bundle/workload provenance remains understandable to a non-author

## Suggested follow-up

After the first pilot, write down only these kinds of issues:

- operator confusion
- runtime-watch blind spots
- authentication friction
- remote log discoverability problems
- result or export surprises

Those are the most valuable `tamamono 1.x` issues, because they come from real
use rather than from another round of local self-test.
