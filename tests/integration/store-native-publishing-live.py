"""Opt-in Silvortex interop using its disposable PostgreSQL/Rust fixture.

Requires a trusted backend checkout, its built store/migrate binaries, Docker's
cached postgres:16-alpine image, jsonschema, and a native builder output with an
installer-candidate directory. Never reads deployment credentials, deploys,
publishes, or loads uploads. This is test tooling, not a product dependency.
"""

import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile

import jsonschema


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class LostResponse(Exception):
    pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", type=Path, required=True)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    smoke = load("interop_publishing", args.backend / "scripts/store-publishing-smoke.py")
    submit = load("interop_submit", args.backend / "scripts/submit-store-package.py")
    contract = json.loads((args.backend / "contracts/kyuubiki/store-v1.openapi.json").read_text())

    def validate(name, value):
        jsonschema.Draft202012Validator({
            "$ref": "#/components/schemas/" + name,
            "components": contract["components"],
        }, format_checker=jsonschema.FormatChecker()).validate(value)

    original = json.loads((args.bundle / "submission.json").read_text())
    index = json.loads((args.bundle / "installer-candidate/kyuubiki-operator-distribution.json").read_text())
    declaration = copy.deepcopy(original)
    artifact = index["artifacts"][0]
    assert artifact["target"] == "macos-aarch64"
    declaration["platforms"][0]["artifacts"] = {
        role: {"path": artifact[role + "_path"], "sha256": artifact[role + "_sha256"],
               "size_bytes": artifact[role + "_size_bytes"]}
        for role in ("manifest", "entrypoint")
    }
    validate("SubmissionDeclaration", declaration)
    assert declaration["platforms"][0]["artifacts"]["entrypoint"]["sha256"] == original["platforms"][0]["artifacts"]["entrypoint"]["sha256"]

    with tempfile.TemporaryDirectory(prefix="kyuubiki-native-store-interop-") as directory:
        root = Path(directory)
        declaration_path = root / "submission.json"
        declaration_path.write_text(json.dumps(declaration))
        declaration, bindings = submit.load_submission(declaration_path, args.bundle / "installer-candidate")
        fixture = smoke.PublishingFixture(root, args.backend / "backend/target/debug")
        try:
            fixture.setup()
            fixture.env["STORE_SUBMISSIONS_ENABLED"] = "true"
            fixture.start()
            key = "native-macos-installer-interop"
            state = fixture.create(declaration, key)
            validate("Submission", state)
            path = "/store/v1/submissions/" + state["submission_id"]
            assert fixture.expect("POST", path + "/complete", 409, user="publisher")[0]["error"]["code"] == "upload_incomplete"
            fixture.check("real Installer declaration and live response conform to OpenAPI; incomplete set refused")

            class Client:
                lost = False
                uploads = []

                def request(self, method, route, payload=None, artifact=None, idempotency_key=None):
                    headers = {"Idempotency-Key": idempotency_key} if idempotency_key else {}
                    if artifact:
                        payload = artifact["local_path"].read_bytes()
                        self.uploads.append(artifact["path"])
                    status, result, _ = fixture.request(method, route, user="publisher", body=payload, headers=headers)
                    assert 200 <= status < 300, (status, result)
                    validate("Submission", result)
                    if method == "PUT" and not self.lost:
                        self.lost = True
                        raise LostResponse("deliberately dropped successful manifest response")
                    return result

            client = Client()
            try:
                submit.submit(client, declaration, bindings, key)
                raise AssertionError("Lost-response fault did not execute")
            except LostResponse:
                pass
            fixture.start()
            result = submit.submit(client, declaration, bindings, key)
            assert result["files_uploaded_this_run"] == 1
            assert len(client.uploads) == 2
            assert result["status"] == "pending_review" and result["review_status"] == "awaiting_review"
            sealed = fixture.expect("GET", path, user="publisher")[0]
            validate("Submission", sealed)
            fixture.check("real dylib uploads and seals; lost upload response plus process restart resumes only missing file")

            fixture.start()
            repeated = submit.submit(client, declaration, bindings, key)
            assert repeated["files_uploaded_this_run"] == 0
            recovered = fixture.expect("POST", path + "/complete", user="publisher")[0]
            assert recovered["seal_receipt"] == sealed["seal_receipt"]
            assert recovered["binding_sha256"] == sealed["binding_sha256"]
            fixture.check("sealed submission replay transfers no files and preserves exact binding and receipt")

            # Same native library, but a renamed entrypoint changes the fixed manifest.
            conflict = fixture.create(original, "original-layout-same-version", 409)
            assert conflict["error"]["code"] == "version_conflict"
            for (platform, role), binding in bindings.items():
                data = binding["local_path"].read_bytes()
                fixture.upload(state, platform, role, data)
                tampered = bytes([data[0] ^ 1]) + data[1:]
                fixture.upload(state, platform, role, tampered, status=409)
            fixture.check("same-version repackaging conflicts and sealed bytes cannot be overwritten")

            fixture.expect("GET", path, 404, user="free")
            assert fixture.sql("SELECT count(*) FROM store_releases;") == "0"
            assert fixture.sql("SELECT count(*) FROM store_quota_events;") == "0"
            assert fixture.sql("SELECT count(*) FROM store_submission_reviews;") == "0"
            fixture.check("native uploads remain owner-only, unreviewed, unpublished and outside download metering")

            args.output.mkdir(mode=0o700)
            for (platform, role), binding in bindings.items():
                stored = fixture.staging_root / state["submission_id"] / platform / role
                target = args.output / binding["path"]
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(stored, target)
                assert hashlib.sha256(target.read_bytes()).hexdigest() == binding["sha256"]
                assert target.stat().st_size == binding["size_bytes"]
            shutil.copyfile(args.bundle / "installer-candidate/kyuubiki-operator-distribution.json", args.output / "kyuubiki-operator-distribution.json")
            report = {"schema_version": "kyuubiki.store-native-interop-test/v1",
                      "platform": "macos-aarch64", "source_commit": original["source"]["revision"],
                      "transport": "SSH transfer plus server loopback HTTP; no external HTTPS ingress claim",
                      "identity": "ephemeral database-seeded test tokens; no interactive OIDC login claim",
                      "backend_execution": False, "publication_performed": False,
                      "review_status": recovered["review_status"], "status": recovered["status"],
                      "binding_sha256": recovered["binding_sha256"],
                      "files": [{k: v for k, v in value.items() if k != "local_path"} for value in bindings.values()],
                      "integration_groups": fixture.checks}
            (args.output / "interop-report.json").write_text(json.dumps(report, indent=2) + "\n")
            print(json.dumps(report, indent=2))
        finally:
            fixture.cleanup()


if __name__ == "__main__":
    main()
