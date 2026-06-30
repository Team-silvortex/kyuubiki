use crate::{Platform, UpdateArtifactRef, unified_update_plan};

const REMOTE_ARTIFACT_SCHEMA_VERSION: &str = "kyuubiki.remote-artifact-delivery/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteArtifactDeliveryManifest {
    pub schema_version: String,
    pub channel: String,
    pub target_version: String,
    pub platform: String,
    pub delivery_mode: String,
    pub artifacts: Vec<RemoteArtifactDeliveryRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteArtifactDeliveryRef {
    pub product: String,
    pub kind: String,
    pub source_path: String,
    pub remote_path: String,
    pub verify_policy: String,
}

impl RemoteArtifactDeliveryManifest {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote artifact delivery preview".to_string(),
            format!("schema: {}", self.schema_version),
            format!("channel: {}", self.channel),
            format!("target_version: {}", self.target_version),
            format!("platform: {}", self.platform),
            format!("delivery_mode: {}", self.delivery_mode),
            "artifacts:".to_string(),
        ];
        for artifact in &self.artifacts {
            lines.push(format!("  - {} {}", artifact.product, artifact.kind));
            lines.push(format!("    source_path: {}", artifact.source_path));
            lines.push(format!("    remote_path: {}", artifact.remote_path));
            lines.push(format!("    verify_policy: {}", artifact.verify_policy));
        }
        lines.join("\n")
    }
}

pub fn default_remote_artifact_delivery_manifest() -> Result<RemoteArtifactDeliveryManifest, String>
{
    remote_artifact_delivery_manifest(None, Platform::current())
}

pub fn remote_artifact_delivery_manifest(
    channel: Option<String>,
    platform: Platform,
) -> Result<RemoteArtifactDeliveryManifest, String> {
    let plan = unified_update_plan(channel)?;
    let platform_key = platform.as_str().to_string();
    let artifacts: Vec<RemoteArtifactDeliveryRef> = plan
        .artifacts
        .iter()
        .filter(|artifact| artifact.platform == platform_key)
        .map(remote_artifact_ref)
        .collect();

    if artifacts.is_empty() {
        return Err(format!(
            "no remote-deliverable artifacts declared for {} on channel {}",
            platform_key, plan.target_channel
        ));
    }

    Ok(RemoteArtifactDeliveryManifest {
        schema_version: REMOTE_ARTIFACT_SCHEMA_VERSION.to_string(),
        channel: plan.target_channel,
        target_version: plan.target_version,
        platform: platform_key,
        delivery_mode: "remote-pull-from-installer-source".to_string(),
        artifacts,
    })
}

fn remote_artifact_ref(artifact: &UpdateArtifactRef) -> RemoteArtifactDeliveryRef {
    let file_name = artifact
        .path
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("artifact.bin");
    RemoteArtifactDeliveryRef {
        product: artifact.product.clone(),
        kind: artifact.kind.clone(),
        source_path: artifact.path.clone(),
        remote_path: format!(
            ".kyuubiki/artifacts/{}/{}/{}",
            artifact.product, artifact.kind, file_name
        ),
        verify_policy: "checksum-and-component-integrity-before-start".to_string(),
    }
}
