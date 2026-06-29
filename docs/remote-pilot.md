# Remote Pilot Guide

Use this page when you want one `Ubuntu 24` machine to become the first real
remote target for `Kyuubiki Hub`.

This is an operator rollout sketch.

It is not the primary source for:

- Installer-owned remote-control surface rules
- authority-mode transitions
- headless runtime transport contracts

Those remain anchored in:

- [installer-remote-control.md](installer-remote-control.md)
- [remote-deployment-roadmap.html](remote-deployment-roadmap.html)
- [agent-control-authority.md](agent-control-authority.md)
- [headless-agent-contract.md](headless-agent-contract.md)

## Confidence ladder

Treat the rollout as three confidence levels:

1. `remote solver node`
   Keep the orchestrator local and run only a headless Rust agent on Ubuntu.
2. `remote control plane`
   Run frontend + orchestrator on Ubuntu and point Hub at it as a remote
   runtime target.
3. `remote catalog / workload source`
   Let Ubuntu also serve workload-catalog and project-bundle endpoints.

For the first practical operator pass, level `1` is the best return.

## Level 1: Remote solver node

Recommended shape:

- laptop:
  - Hub
  - Workbench
  - local orchestrator on `4000`
- Ubuntu host:
  - `kyuubiki-cli` Rust agent on `5001`

Minimum Ubuntu setup:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev screen curl git
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
git clone <your-repo-url> ~/kyuubiki
cd ~/kyuubiki
./scripts/kyuubiki build-agent
./scripts/kyuubiki agent -- --port 5001
```

If you want the agent to survive your shell session:

```bash
cd ~/kyuubiki
screen -S kyuubiki-agent-5001 -X quit >/dev/null 2>&1 || true
screen -dmS kyuubiki-agent-5001 sh -lc './scripts/kyuubiki agent -- --port 5001'
```

Laptop-side local orchestrator changes:

```bash
export KYUUBIKI_AGENT_DISCOVERY=static
export KYUUBIKI_AGENT_ENDPOINTS=192.168.1.50:5001
./scripts/kyuubiki restart-local
```

If you want to persist those values, put them in `.env.local` at the root of
the laptop checkout.

Validate before using Hub:

```bash
curl http://127.0.0.1:4000/api/v1/protocol/agents
./scripts/kyuubiki status
curl http://127.0.0.1:4000/api/health
```

Best first studies:

- `axial_bar_1d`
- `beam_1d`
- `thermal_bar_1d`

## Level 2: Remote control plane

Recommended shape:

- laptop:
  - Hub
- Ubuntu host:
  - frontend
  - orchestrator
  - one or more Rust agents

Validate:

- Hub can treat Ubuntu as the active runtime target
- runtime watch still makes failures understandable
- Workbench can open the remote frontend and submit a study
- token-protected reads still behave correctly

## Level 3: Remote workload source

Recommended shape:

- Ubuntu serves:
  - `/api/v1/workloads/catalog`
  - `/api/v1/projects/:project_id/bundle`

Validate:

- Hub classifies remote workloads correctly
- provenance still reads as first-party remote control plane
- `Mechanical / Thermal / Thermo-mechanical` filters still work
- opening a remote workload lands in the expected Workbench context

## Good-enough criteria

Treat the remote pilot as successful when:

- Hub can distinguish local vs remote runtime context without confusion
- one remote agent can solve representative studies reliably
- first-line troubleshooting is still possible from Hub alone
- remote control-plane reads feel no rougher than local reads
- bundle/workload provenance remains understandable to a non-author
