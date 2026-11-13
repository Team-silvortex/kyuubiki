//! Only failed preparations enter here; never rediscover or activate generations.
use super::*;
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, TryLockError, Weak};
use std::time::Duration;

pub(crate) const INTERVAL_MS: u64 = 1_000;
pub(crate) const BATCH_LIMIT: usize = 16;

#[derive(Debug)]
struct Pending {
    root: PathBuf,
    generations_root: PathBuf,
    marker: GenerationMarker,
}

#[derive(Debug, Default)]
struct State {
    pending: VecDeque<Pending>,
    known: BTreeSet<PathBuf>,
    worker_running: bool,
    removed: usize,
}

#[derive(Debug, Default)]
pub(crate) struct RetryQueue {
    state: Mutex<State>,
    sweep: Mutex<()>,
}

impl RetryQueue {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub(crate) fn counts(&self) -> (usize, usize) {
        let state = self.state();
        (state.known.len(), state.removed)
    }

    pub(super) fn enqueue(&self, generation: &OwnedOperatorPackageGeneration) {
        let mut state = self.state();
        if state.known.insert(generation.generation_root.clone()) {
            state.pending.push_back(Pending {
                root: generation.generation_root.clone(),
                generations_root: generation.generations_root.clone(),
                marker: generation.marker.clone(),
            });
        }
    }

    pub(crate) fn retry(&self, session: &Arc<OperatorPackageGenerationSession>) {
        let _sweep = match self.sweep.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return,
            Err(TryLockError::Poisoned(error)) => error.into_inner(),
        };
        let batch: Vec<_> = {
            let mut state = self.state();
            let count = state.pending.len().min(BATCH_LIMIT);
            state.pending.drain(..count).collect()
        };
        if batch.is_empty() {
            return;
        }
        let mut removed = BTreeSet::new();
        // Acquire the catalog once per batch, outside the queue mutex. Failure
        // preserves every entry; no second cleanup path bypasses admission.
        let _ = session.with_generation_cleanup(|| {
            for pending in &batch {
                let generation = OwnedOperatorPackageGeneration {
                    generation_root: pending.root.clone(),
                    generations_root: pending.generations_root.clone(),
                    marker: pending.marker.clone(),
                    session: session.clone(),
                };
                if cleanup::remove(&generation).is_ok() {
                    removed.insert(pending.root.clone());
                }
            }
            Ok(())
        });
        let mut state = self.state();
        for pending in batch {
            if removed.contains(&pending.root) {
                state.known.remove(&pending.root);
                state.removed = state.removed.saturating_add(1);
            } else {
                state.pending.push_back(pending);
            }
        }
    }

    pub(crate) fn ensure_worker(&self, session: &Arc<OperatorPackageGenerationSession>) {
        self.ensure_worker_with(session, |session| {
            std::thread::Builder::new()
                .name("kyuubiki-generation-cleanup".into())
                .spawn(move || run_worker(session))
                .map(|_| ())
        });
    }

    fn ensure_worker_with(
        &self,
        session: &Arc<OperatorPackageGenerationSession>,
        spawn: impl FnOnce(Weak<OperatorPackageGenerationSession>) -> std::io::Result<()>,
    ) {
        {
            let mut state = self.state();
            if state.worker_running || state.known.is_empty() {
                return;
            }
            state.worker_running = true;
        }
        let result = spawn(Arc::downgrade(session));
        if let Err(error) = result {
            self.state().worker_running = false;
            eprintln!(
                "generation cleanup worker unavailable: kind={:?}; retry before preparation",
                error.kind()
            );
        }
    }
}

fn run_worker(weak: Weak<OperatorPackageGenerationSession>) {
    loop {
        std::thread::sleep(Duration::from_millis(INTERVAL_MS));
        let Some(session) = weak.upgrade() else {
            return;
        };
        session.abandoned_generations.retry(&session);
        let mut state = session.abandoned_generations.state();
        if state.known.is_empty() {
            state.worker_running = false;
            return;
        }
        // Release locks and the strong session reference before sleeping.
    }
}

#[cfg(test)]
mod tests;
