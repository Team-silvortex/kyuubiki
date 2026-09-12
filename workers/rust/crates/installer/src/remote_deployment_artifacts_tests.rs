use super::{RemoteArtifactDeliveryManifest, remote_artifact_delivery_manifest_from_plan};
use crate::{Platform, UnifiedUpdatePlan, UpdateArtifactRef};

const PLATFORMS: [Platform; 3] = [Platform::Macos, Platform::Linux, Platform::Windows];

fn plan_fixture() -> UnifiedUpdatePlan {
    UnifiedUpdatePlan {
        schema_version: "kyuubiki.update-catalog/v1".to_string(),
        workspace: "fixture-workspace".to_string(),
        current_version: "3.1.0".to_string(),
        target_channel: "preview".to_string(),
        target_tag: "fixture-release".to_string(),
        target_version: "3.2.0".to_string(),
        update_state: "update_available".to_string(),
        summary: "Remote artifact contract fixture".to_string(),
        contract_rules: Vec::new(),
        artifacts: PLATFORMS
            .iter()
            .flat_map(|platform| {
                ["hub", "installer", "workbench"].map(|product| UpdateArtifactRef {
                    product: product.to_string(),
                    platform: platform.as_str().to_string(),
                    kind: "archive".to_string(),
                    path: format!("dist/{}/{product}.tar.gz", platform.as_str()),
                    exists: false,
                })
            })
            .collect(),
    }
}

fn assert_remote_pull_contract(manifest: &RemoteArtifactDeliveryManifest, platform: Platform) {
    assert_eq!(
        manifest.schema_version,
        "kyuubiki.remote-artifact-delivery/v1"
    );
    assert_eq!(manifest.delivery_mode, "remote-pull-from-installer-source");
    assert_eq!(manifest.platform, platform.as_str());
    assert_eq!(manifest.channel, "preview");
    assert_eq!(manifest.target_version, "3.2.0");
    assert_eq!(manifest.artifacts.len(), 3);
    for (artifact, product) in manifest
        .artifacts
        .iter()
        .zip(["hub", "installer", "workbench"])
    {
        assert_eq!(artifact.product, product);
        assert_eq!(artifact.kind, "archive");
        assert_eq!(
            artifact.source_path,
            format!("dist/{}/{product}.tar.gz", platform.as_str())
        );
        assert_eq!(
            artifact.remote_path,
            format!(".kyuubiki/artifacts/{product}/archive/{product}.tar.gz")
        );
        assert_eq!(
            artifact.verify_policy,
            "checksum-and-component-integrity-before-start"
        );
    }
    assert!(
        manifest
            .render()
            .contains("remote artifact delivery preview")
    );
}

#[test]
fn remote_artifact_delivery_manifest_uses_remote_pull_contract_on_every_platform() {
    // Contract coverage must not depend on which installers have been published or built locally.
    for platform in PLATFORMS {
        let manifest =
            remote_artifact_delivery_manifest_from_plan(plan_fixture(), platform).unwrap();
        assert_remote_pull_contract(&manifest, platform);
    }
}

#[test]
fn remote_artifact_delivery_rejects_an_unpublished_platform_without_cross_platform_fallback() {
    for platform in PLATFORMS {
        let mut plan = plan_fixture();
        plan.artifacts
            .retain(|artifact| artifact.platform != platform.as_str());
        let error = remote_artifact_delivery_manifest_from_plan(plan, platform).unwrap_err();
        assert_eq!(
            error,
            format!(
                "no remote-deliverable artifacts declared for {} on channel preview",
                platform.as_str()
            )
        );
    }
}

#[test]
fn remote_artifact_delivery_rejects_an_empty_catalog_on_every_platform() {
    for platform in PLATFORMS {
        let mut plan = plan_fixture();
        plan.artifacts.clear();
        assert_eq!(
            remote_artifact_delivery_manifest_from_plan(plan, platform).unwrap_err(),
            format!(
                "no remote-deliverable artifacts declared for {} on channel preview",
                platform.as_str()
            )
        );
    }
}
