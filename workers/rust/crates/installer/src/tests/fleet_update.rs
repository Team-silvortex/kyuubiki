use crate::agent_update_payload::{install_agent_update_package_into, rollback_agent_update_in};
use crate::fleet_update::{
    FleetAgentUpdateTarget, FleetUpdatePlan, apply_fleet_update_transaction,
    apply_fleet_update_transaction_with_hook, fleet_store_is_clean, inspect_fleet_update_state,
    rollback_fleet_update_transaction,
};
use crate::runtime_payload::install_runtime_payload_into;
use crate::{Platform, seal_agent_update_package, seal_runtime_payload};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FIRST_VERSION: &str = "2.17.0";
const SECOND_VERSION: &str = "2.17.1";

#[test]
fn fleet_qualification_schema_matches_the_runtime_contract() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/fleet-update-qualification-report.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["properties"]["schema_version"]["const"],
        crate::FLEET_UPDATE_QUALIFICATION_SCHEMA_VERSION
    );
    assert_eq!(
        schema["$defs"]["transaction"]["properties"]["schema_version"]["const"],
        crate::FLEET_UPDATE_TRANSACTION_SCHEMA_VERSION
    );
}

#[test]
fn fleet_transaction_upgrades_and_rolls_back_all_components() {
    let fixture = FleetFixture::new("lifecycle");

    let upgraded = apply_fleet_update_transaction(&fixture.plan, fixture.platform).unwrap();
    assert_eq!(upgraded.active_version, SECOND_VERSION);
    assert_eq!(upgraded.components.len(), 3);

    let rolled_back = rollback_fleet_update_transaction(&fixture.plan, fixture.platform).unwrap();
    assert_eq!(rolled_back.active_version, FIRST_VERSION);
    fixture.assert_active(FIRST_VERSION);
    fixture.assert_clean();
}

#[test]
fn fleet_transaction_compensates_an_interrupted_agent_activation() {
    let fixture = FleetFixture::new("upgrade-interruption");
    let failure =
        apply_fleet_update_transaction_with_hook(&fixture.plan, fixture.platform, |checkpoint| {
            if checkpoint == "agent-02" {
                Err("injected interruption".into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();

    assert_eq!(failure.failed_component_id, "agent-02");
    assert_eq!(failure.failure_class, "injected-fault");
    assert!(failure.compensated, "{failure}");
    fixture.assert_active(FIRST_VERSION);
    fixture.assert_clean();
}

#[test]
fn fleet_transaction_rejects_a_parallel_controller() {
    let fixture = FleetFixture::new("parallel-controller");
    let lock = fixture
        .plan
        .runtime_store_root
        .join(".fleet-transaction.lock");
    fs::write(&lock, "pid=fixture").unwrap();

    let failure = apply_fleet_update_transaction(&fixture.plan, fixture.platform).unwrap_err();
    assert_eq!(failure.failure_class, "preflight");
    assert!(failure.cause.contains("transaction lock is unavailable"));
    assert!(failure.compensated);
    fixture.assert_active(FIRST_VERSION);
    fs::remove_file(lock).unwrap();
    fixture.assert_clean();
}

#[test]
fn fleet_plan_rejects_aliased_component_stores() {
    let mut fixture = FleetFixture::new("aliased-stores");
    fixture.plan.agents[1].store_root = fixture.plan.agents[0].store_root.join(".");

    let error = inspect_fleet_update_state(&fixture.plan, fixture.platform).unwrap_err();
    assert!(
        error.contains("store overlaps component agent-01"),
        "{error}"
    );
}

#[test]
fn fleet_transaction_compensates_post_upgrade_version_drift_idempotently() {
    let fixture = FleetFixture::new("post-upgrade-drift");
    let drifting_store = fixture.plan.agents[0].store_root.clone();
    let failure =
        apply_fleet_update_transaction_with_hook(&fixture.plan, fixture.platform, |checkpoint| {
            if checkpoint == "fleet:verify" {
                rollback_agent_update_in(&drifting_store, fixture.platform)?;
            }
            Ok(())
        })
        .unwrap_err();

    assert_eq!(failure.failed_component_id, "fleet");
    assert_eq!(failure.failure_class, "post-upgrade-verification");
    assert!(failure.compensated, "{failure}");
    fixture.assert_active(FIRST_VERSION);
    fixture.assert_clean();
}

#[test]
fn fleet_rollback_rolls_agents_forward_when_runtime_rollback_is_blocked() {
    let fixture = FleetFixture::new("rollback-interruption");
    apply_fleet_update_transaction(&fixture.plan, fixture.platform).unwrap();
    let runtime_lock = fixture.plan.runtime_store_root.join(".update.lock");
    fs::write(&runtime_lock, "injected lock").unwrap();

    let failure = rollback_fleet_update_transaction(&fixture.plan, fixture.platform).unwrap_err();
    assert_eq!(failure.failed_component_id, "runtime");
    assert_eq!(failure.failure_class, "runtime-rollback");
    assert!(failure.compensated, "{failure}");
    fixture.assert_active(SECOND_VERSION);

    fs::remove_file(runtime_lock).unwrap();
    rollback_fleet_update_transaction(&fixture.plan, fixture.platform).unwrap();
    fixture.assert_active(FIRST_VERSION);
    fixture.assert_clean();
}

struct FleetFixture {
    root: PathBuf,
    plan: FleetUpdatePlan,
    platform: Platform,
}

impl FleetFixture {
    fn new(label: &str) -> Self {
        let root = fixture_root(label);
        let platform = Platform::current();
        let runtime_first = root.join("packages/runtime-first");
        let runtime_second = root.join("packages/runtime-second");
        let agent_first = root.join("packages/agent-first");
        let agent_second = root.join("packages/agent-second");
        write_runtime_payload(&runtime_first, FIRST_VERSION, platform);
        write_runtime_payload(&runtime_second, SECOND_VERSION, platform);
        write_agent_package(&agent_first, FIRST_VERSION, platform);
        write_agent_package(&agent_second, SECOND_VERSION, platform);

        let plan = FleetUpdatePlan {
            runtime_package_root: runtime_second,
            runtime_store_root: root.join("stores/runtime"),
            agents: (1..=2)
                .map(|index| FleetAgentUpdateTarget {
                    node_id: format!("agent-{index:02}"),
                    package_root: agent_second.clone(),
                    store_root: root.join(format!("stores/agent-{index:02}")),
                })
                .collect(),
        };
        install_runtime_payload_into(&runtime_first, &plan.runtime_store_root, platform).unwrap();
        for agent in &plan.agents {
            install_agent_update_package_into(&agent_first, &agent.store_root, platform).unwrap();
        }
        Self {
            root,
            plan,
            platform,
        }
    }

    fn assert_active(&self, expected: &str) {
        let snapshot = inspect_fleet_update_state(&self.plan, self.platform).unwrap();
        assert_eq!(snapshot.active_version, expected);
        assert!(
            snapshot
                .components
                .iter()
                .all(|component| component.active_version == expected)
        );
    }

    fn assert_clean(&self) {
        assert!(fleet_store_is_clean(&self.plan.runtime_store_root));
        assert!(
            self.plan
                .agents
                .iter()
                .all(|agent| fleet_store_is_clean(&agent.store_root))
        );
    }
}

impl Drop for FleetFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fixture_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kyuubiki-fleet-update-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn write_runtime_payload(root: &Path, version: &str, platform: Platform) {
    let files = [
        "bin/kyuubiki-cli",
        "bin/kyuubiki-runtime",
        "services/orchestrator/bin/kyuubiki_web",
        "services/frontend/index.html",
    ];
    for relative in files {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("{relative}:{version}")).unwrap();
    }
    fs::create_dir_all(root.join("manifests")).unwrap();
    fs::write(
        root.join("manifests/service-launch.json"),
        r#"{
          "schema_version":"kyuubiki.service-launch/v1",
          "services":[
            {"id":"agent","command":"bin/kyuubiki-cli","cwd":".","args":[]},
            {"id":"orchestrator","command":"services/orchestrator/bin/kyuubiki_web","cwd":"services/orchestrator","args":[]},
            {"id":"frontend","command":"bin/kyuubiki-runtime","cwd":".","args":[]}
          ]
        }"#,
    )
    .unwrap();
    seal_runtime_payload(root, version, platform).unwrap();
}

fn write_agent_package(root: &Path, version: &str, platform: Platform) {
    let relative = if platform == Platform::Windows {
        "bin/kyuubiki-agent.exe"
    } else {
        "bin/kyuubiki-agent"
    };
    let binary = root.join(relative);
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, format!("agent:{version}")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    }
    seal_agent_update_package(root, version, platform).unwrap();
}
