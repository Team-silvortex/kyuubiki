//! Authorized, bounded transport staging. Downloading never installs or executes a package.
mod catalog;
mod install;
mod storage;
pub use catalog::{CatalogPage, CatalogRequest, catalog};
#[cfg(test)]
mod tests;
mod transport;
mod trust;
mod types;
use crate::store_identity::{StoreIdentity, StoreSessionLease};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use types::*;
pub use types::{DownloadSelection, DownloadView};
pub type Result<T> = std::result::Result<T, &'static str>;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum DownloadRequest {
    List,
    Prepare { selection: DownloadSelection },
    Start { id: String },
    Cancel { id: String },
    Remove { id: String },
    Install { id: String },
    StageVariant { id: String },
}
struct Active {
    id: String,
    cancel: Arc<AtomicBool>,
    received: Arc<AtomicU64>,
}
#[derive(Clone, Default)]
pub struct StoreDownloads {
    active: Arc<Mutex<Option<Active>>>,
    root: Option<PathBuf>,
    #[cfg(test)]
    pub(crate) test_installer: Option<(PathBuf, PathBuf)>,
}
impl StoreDownloads {
    #[cfg(test)]
    pub(crate) fn with_root(root: PathBuf) -> Self {
        Self {
            root: Some(root),
            ..Default::default()
        }
    }
    fn root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.root {
            return Ok(root.clone());
        }
        let parent =
            crate::desktop_preferences_dir().map_err(|_| "download_storage_unavailable")?;
        std::fs::create_dir_all(&parent).map_err(|_| "download_storage_unavailable")?;
        Ok(parent.join("store-downloads"))
    }
    pub fn handle(
        &self,
        identity: &StoreIdentity,
        request: DownloadRequest,
    ) -> Result<Vec<DownloadView>> {
        match request {
            DownloadRequest::List => self.list(identity),
            DownloadRequest::Prepare { selection } => {
                self.prepare(identity, selection)?;
                self.list(identity)
            }
            DownloadRequest::Start { id } => {
                self.start(identity, &id)?;
                self.list(identity)
            }
            DownloadRequest::Install { id } => {
                self.install(identity, &id)?;
                self.list(identity)
            }
            DownloadRequest::StageVariant { id } => {
                self.stage_variant(identity, &id)?;
                self.list(identity)
            }
            DownloadRequest::Cancel { id } => {
                let session = identity.store_lease()?;
                owner(&storage::read(&self.root()?, &id)?, &session)?;
                self.cancel(&id)?;
                self.list(identity)
            }
            DownloadRequest::Remove { id } => {
                let session = identity.store_lease()?;
                let store = storage::Store::open(self.root()?)?;
                let record = store.read(&id)?;
                owner(&record, &session)?;
                store.remove(&record)?;
                self.list(identity)
            }
        }
    }
    pub fn list(&self, identity: &StoreIdentity) -> Result<Vec<DownloadView>> {
        let session = identity.store_lease()?;
        let root = self.root()?;
        let active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        let records = storage::list(&root)?;
        let mut views = Vec::new();
        for record in records
            .into_iter()
            .filter(|r| r.issuer == session.issuer && r.subject == session.subject)
        {
            views.push(download_view(&record, active.as_ref(), &root));
        }
        session.check()?;
        Ok(views)
    }
    pub fn prepare(
        &self,
        identity: &StoreIdentity,
        selection: DownloadSelection,
    ) -> Result<DownloadView> {
        selection.validate()?;
        let session = identity.store_lease()?;
        let store = storage::Store::open(self.root()?)?;
        let records = storage::list(&store.root)?;
        if let Some(existing) = records.iter().find(|r| {
            r.issuer == session.issuer
                && r.subject == session.subject
                && r.selection.kind == selection.kind
                && r.selection.asset_id == selection.asset_id
                && r.selection.version == selection.version
                && r.selection.platform == selection.platform
        }) {
            if existing.selection.key_id != selection.key_id
                || existing.selection.key_sha256 != selection.key_sha256
            {
                return Err("signing_key_changed");
            }
            return Ok(existing.view());
        }
        if records.len() >= 4 {
            return Err("download_capacity_reached");
        }
        let (files, quota, metered, _) = transport::preflight(&session, &selection, false)?;
        let mut bytes = [0; 32];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| "random_unavailable")?;
        let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let record = Record {
            format: "kyuubiki.store-download/v1".into(),
            id,
            issuer: session.issuer.clone(),
            subject: session.subject.clone(),
            selection,
            files,
            quota,
            metered,
            state: "prepared".into(),
            attempted: false,
            replay_until: None,
            grant_id: None,
            outcome: None,
            error: None,
            installation: None,
        };
        session.check()?;
        store.save(&record)?;
        Ok(record.view())
    }
    fn start(&self, identity: &StoreIdentity, id: &str) -> Result<()> {
        let session = identity.store_lease()?;
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        if active.is_some() {
            return Err("download_busy");
        }
        let store = storage::Store::open(self.root()?)?;
        let mut record = store.read(id)?;
        owner(&record, &session)?;
        if record.state == "downloaded" {
            return Ok(());
        }
        record.state = "authorizing".into();
        record.error = None;
        store.save(&record)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let received = Arc::new(AtomicU64::new(0));
        *active = Some(Active {
            id: id.into(),
            cancel: cancel.clone(),
            received: received.clone(),
        });
        let manager = self.clone();
        // Drop before spawn error handling; the worker may finish immediately.
        drop(active);
        std::thread::Builder::new()
            .name("store-download".into())
            .spawn(move || {
                let result = run(&store, &mut record, &session, &cancel, &received);
                if let Err(error) = result {
                    record.state = if error == "cancelled" {
                        "cancelled"
                    } else {
                        "failed"
                    }
                    .into();
                    record.error = Some(error.into());
                    if store.clear_partial(&record.id).is_err() {
                        record.error = Some("download_cleanup_failed".into());
                    }
                    let _ = store.save(&record); // A failed write leaves the prior recoverable, same-key journal.
                }
                // Release the cross-process writer lock before publishing completion.
                drop(store);
                manager
                    .active
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();
            })
            .map_err(|_| {
                self.active.lock().unwrap_or_else(|e| e.into_inner()).take();
                "download_worker_unavailable"
            })?;
        Ok(())
    }
    fn cancel(&self, id: &str) -> Result<()> {
        if !digest(id) {
            return Err("invalid_download_id");
        }
        let active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(job) = active.as_ref().filter(|job| job.id == id) {
            job.cancel.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
}
fn download_view(record: &Record, active: Option<&Active>, root: &std::path::Path) -> DownloadView {
    let mut view = record.view();
    if let Some(job) = active.filter(|a| a.id == record.id) {
        view.received_bytes = job.received.load(Ordering::Relaxed);
        if job.cancel.load(Ordering::Relaxed) {
            view.state = "cancelling".into();
        } else if ["downloaded", "failed", "cancelled"].contains(&record.state.as_str()) {
            // The journal can be terminal while its fsync/cleanup still owns the store.
            view.state = if view.received_bytes > 0 {
                "downloading"
            } else {
                "authorizing"
            }
            .into();
        }
        view.error = None;
    } else if ["authorizing", "downloading"].contains(&record.state.as_str()) {
        view.state = "interrupted".into();
    }
    if view.state == "downloaded" {
        view.received_bytes = view.total_bytes;
        view.directory = Some(
            root.join(&record.id)
                .join("files")
                .to_string_lossy()
                .into_owned(),
        );
    }
    view
}
fn owner(record: &Record, session: &StoreSessionLease) -> Result<()> {
    if record.issuer != session.issuer || record.subject != session.subject {
        return Err("download_owner_mismatch");
    }
    session.check()
}
fn run(
    store: &storage::Store,
    record: &mut Record,
    session: &StoreSessionLease,
    cancel: &AtomicBool,
    received: &AtomicU64,
) -> Result<()> {
    let check = || {
        if cancel.load(Ordering::Relaxed) {
            Err("cancelled")
        } else {
            session.check()
        }
    };
    check()?;
    // Once server idempotency retention expires, even replaying the original key can charge again.
    if record.attempted
        && record.grant_id.is_none()
        && record.replay_until.is_none_or(|until| until <= now())
    {
        return Err("authorization_recovery_expired");
    }
    let (files, _, _, retention) =
        transport::preflight(session, &record.selection, record.attempted)?;
    if files != record.files {
        return Err("artifact_mismatch");
    }
    check()?;
    if !record.attempted {
        record.replay_until = Some(now().saturating_add(retention));
    }
    record.attempted = true;
    store.save(record)?; // Durable intent and immutable key precede the first potentially charged POST.
    let grant = transport::grant(session, record)?;
    record.grant_id = Some(grant.grant_id.clone());
    record.outcome = Some(grant.quota.clone());
    record.state = "downloading".into();
    store.save(record)?;
    check()?;
    let base = store.reset_partial(&record.id)?;
    for file in &grant.files {
        check()?;
        let mut response = transport::bytes(session, file)?;
        let mut reader = response.body_mut().as_reader();
        let mut output = store.create_file(&base, &file.path)?;
        let mut hash = Sha256::new();
        let mut length = 0;
        let mut buffer = [0; 65536];
        loop {
            check()?;
            let size = reader
                .read(&mut buffer)
                .map_err(|_| "download_connection_failed")?;
            if size == 0 {
                break;
            }
            length += size as u64;
            if length > file.size_bytes {
                return Err("artifact_mismatch");
            }
            hash.update(&buffer[..size]);
            output
                .write_all(&buffer[..size])
                .map_err(|_| "download_storage_unavailable")?;
            received.fetch_add(size as u64, Ordering::Relaxed);
        }
        if length != file.size_bytes || format!("{:x}", hash.finalize()) != file.sha256 {
            return Err("artifact_mismatch");
        }
        output
            .sync_all()
            .map_err(|_| "download_storage_unavailable")?;
        storage::sync_dir(base.join(&file.path).parent().ok_or("artifact_mismatch")?)?;
    }
    check()?;
    session.with_current(|| {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled");
        }
        store.promote(record)?;
        record.state = "downloaded".into();
        record.error = None;
        store.save(record)
    })
}
