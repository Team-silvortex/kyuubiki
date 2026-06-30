# Remote SSH Fixture

This fixture is the next step after `remote-ssh-fixture` command-shape checks.
It provides a local-only Docker sshd target for future Installer remote
deployment integration tests.

It is intentionally not run by default.

## Safety Rules

- The fixture binds only `127.0.0.1:2222`.
- Runtime keys, known-host files, copied artifacts, and workspace state live
  under `tests/integration/remote-ssh-fixture/runtime/`.
- `runtime/` is ignored by git and must not contain production credentials.
- The fixture user is `kyuubiki-fixture`.
- The fixture workspace is `/tmp/kyuubiki-fixture`.

## Manual Bring-Up

Preferred explicit one-shot runner:

```bash
make test-integration-remote-ssh-fixture
```

The target generates a throwaway key under ignored `runtime/`, starts the
fixture, probes it, and tears the container down.

Manual equivalent:

Generate a throwaway key:

```bash
mkdir -p tests/integration/remote-ssh-fixture/runtime/workspace
ssh-keygen -t ed25519 -N "" \
  -f tests/integration/remote-ssh-fixture/runtime/client_key \
  -C kyuubiki-remote-ssh-fixture
```

Start the fixture:

```bash
docker compose -f tests/integration/remote-ssh-fixture/compose.yaml up --build
```

In another shell, probe it with explicit local-only options:

```bash
ssh \
  -i tests/integration/remote-ssh-fixture/runtime/client_key \
  -o UserKnownHostsFile=tests/integration/remote-ssh-fixture/runtime/known_hosts \
  -o StrictHostKeyChecking=accept-new \
  -p 2222 \
  kyuubiki-fixture@127.0.0.1 \
  "cd /tmp/kyuubiki-fixture && printf '%s' 'kyuubiki-remote-ok'"
```

Stop and remove the container:

```bash
docker compose -f tests/integration/remote-ssh-fixture/compose.yaml down
```

## Installer Commands

Use these before adding executable remote deployment tests:

```bash
cargo run -p kyuubiki-installer -- remote-ssh-fixture
cargo run -p kyuubiki-installer -- remote-ssh-fixture-plan
cargo run -p kyuubiki-installer -- remote-host-trust
```

`remote-ssh-fixture` validates command shape without opening sockets.
`remote-ssh-fixture-plan` points to this Docker fixture and documents the
runtime files that remain local-only.
`remote-host-trust` documents why fixture mode can use `accept-new`, while
managed deployment must move to pinned known-host verification.
