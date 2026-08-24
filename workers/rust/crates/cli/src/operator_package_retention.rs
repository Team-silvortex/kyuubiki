use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

const MAX_JOB_ID_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RetentionError {
    MissingJobId,
    InvalidJobId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JobReleasePlan {
    pub job_id: String,
    pub was_bound: bool,
    pub released_package_ids: BTreeSet<String>,
    pub evicted_package_ids: BTreeSet<String>,
    pub retained_package_ids: BTreeSet<String>,
}

#[derive(Default)]
struct RetentionState {
    job_packages: BTreeMap<String, BTreeSet<String>>,
    package_jobs: BTreeMap<String, BTreeSet<String>>,
    durable_packages: BTreeSet<String>,
}

pub(crate) fn reset() {
    *retention_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = RetentionState::default();
}

pub(crate) fn validate_registration(
    cache_scope: Option<&str>,
    job_id: Option<&str>,
) -> Result<(), RetentionError> {
    if cache_scope != Some("job") {
        return Ok(());
    }
    validate_job_id(job_id)
}

pub(crate) fn register_package(
    cache_scope: Option<&str>,
    job_id: Option<&str>,
    package_id: &str,
) -> Result<(), RetentionError> {
    validate_registration(cache_scope, job_id)?;
    let mut state = retention_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match cache_scope {
        Some("job") => {
            let job_id = job_id.expect("validated job cache scope must have a job id");
            state
                .job_packages
                .entry(job_id.to_string())
                .or_default()
                .insert(package_id.to_string());
            state
                .package_jobs
                .entry(package_id.to_string())
                .or_default()
                .insert(job_id.to_string());
        }
        Some("session" | "agent") => {
            state.durable_packages.insert(package_id.to_string());
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn package_is_retained(package_id: &str) -> bool {
    let state = retention_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.durable_packages.contains(package_id)
        || state
            .package_jobs
            .get(package_id)
            .is_some_and(|owners| !owners.is_empty())
}

pub(crate) fn plan_job_release(job_id: &str) -> Result<JobReleasePlan, RetentionError> {
    validate_job_id(Some(job_id))?;
    let state = retention_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(packages) = state.job_packages.get(job_id) else {
        return Ok(JobReleasePlan {
            job_id: job_id.to_string(),
            was_bound: false,
            released_package_ids: BTreeSet::new(),
            evicted_package_ids: BTreeSet::new(),
            retained_package_ids: BTreeSet::new(),
        });
    };
    let mut evicted_package_ids = BTreeSet::new();
    let mut retained_package_ids = BTreeSet::new();
    for package_id in packages {
        let shared_job_owner = state
            .package_jobs
            .get(package_id)
            .is_some_and(|owners| owners.iter().any(|owner| owner != job_id));
        if shared_job_owner || state.durable_packages.contains(package_id) {
            retained_package_ids.insert(package_id.clone());
        } else {
            evicted_package_ids.insert(package_id.clone());
        }
    }
    Ok(JobReleasePlan {
        job_id: job_id.to_string(),
        was_bound: true,
        released_package_ids: packages.clone(),
        evicted_package_ids,
        retained_package_ids,
    })
}

pub(crate) fn commit_job_release(plan: &JobReleasePlan) {
    if !plan.was_bound {
        return;
    }
    let mut state = retention_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.job_packages.remove(&plan.job_id);
    for package_id in &plan.released_package_ids {
        if let Some(owners) = state.package_jobs.get_mut(package_id) {
            owners.remove(&plan.job_id);
            if owners.is_empty() {
                state.package_jobs.remove(package_id);
            }
        }
    }
}

fn validate_job_id(job_id: Option<&str>) -> Result<(), RetentionError> {
    let Some(job_id) = job_id else {
        return Err(RetentionError::MissingJobId);
    };
    if job_id.is_empty() || job_id.len() > MAX_JOB_ID_BYTES || job_id.chars().any(char::is_control)
    {
        return Err(RetentionError::InvalidJobId);
    }
    Ok(())
}

fn retention_state() -> &'static Mutex<RetentionState> {
    static STATE: OnceLock<Mutex<RetentionState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(RetentionState::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::MutexGuard;

    #[test]
    fn shared_job_package_is_evicted_only_after_last_owner_releases() {
        let _guard = test_guard();
        reset();
        register_package(Some("job"), Some("job-a"), "package-a").expect("register job a");
        register_package(Some("job"), Some("job-b"), "package-a").expect("register job b");

        let first = plan_job_release("job-a").expect("plan first release");
        assert!(first.evicted_package_ids.is_empty());
        assert_eq!(
            first.retained_package_ids,
            BTreeSet::from(["package-a".to_string()])
        );
        commit_job_release(&first);

        let second = plan_job_release("job-b").expect("plan second release");
        assert_eq!(
            second.evicted_package_ids,
            BTreeSet::from(["package-a".to_string()])
        );
    }

    #[test]
    fn durable_scope_outlives_job_release() {
        let _guard = test_guard();
        reset();
        register_package(Some("agent"), None, "package-a").expect("register durable package");
        register_package(Some("job"), Some("job-a"), "package-a").expect("register job package");

        let plan = plan_job_release("job-a").expect("plan release");
        assert!(plan.evicted_package_ids.is_empty());
        assert!(plan.retained_package_ids.contains("package-a"));
    }

    #[test]
    fn one_job_releases_all_of_its_exclusive_packages_together() {
        let _guard = test_guard();
        reset();
        register_package(Some("job"), Some("job-a"), "package-a").expect("register package a");
        register_package(Some("job"), Some("job-a"), "package-b").expect("register package b");

        let plan = plan_job_release("job-a").expect("plan release");
        assert_eq!(
            plan.evicted_package_ids,
            BTreeSet::from(["package-a".to_string(), "package-b".to_string()])
        );
        assert!(plan.retained_package_ids.is_empty());
        commit_job_release(&plan);
        assert!(!plan_job_release("job-a").expect("repeat release").was_bound);
    }

    #[test]
    fn job_scope_requires_a_bounded_visible_identity() {
        let _guard = test_guard();
        assert_eq!(
            validate_registration(Some("job"), None),
            Err(RetentionError::MissingJobId)
        );
        assert_eq!(
            validate_registration(Some("job"), Some("")),
            Err(RetentionError::InvalidJobId)
        );
    }

    fn test_guard() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
