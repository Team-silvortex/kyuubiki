use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};
use std::thread;

use kyuubiki_protocol::AgentDescriptor;

pub(super) const MAX_CONCURRENT_PROBES: usize = 2;

pub(super) fn probe_round(
    peers: Vec<String>,
    running: &AtomicBool,
    observe: impl FnMut(String, Result<AgentDescriptor, String>),
) {
    run_round(
        peers,
        running,
        MAX_CONCURRENT_PROBES,
        super::peer_rpc::request_agent_descriptor,
        observe,
    );
}

fn run_round<T: Send>(
    peers: Vec<String>,
    running: &AtomicBool,
    worker_limit: usize,
    request: impl Fn(&str) -> Result<T, String> + Sync,
    mut observe: impl FnMut(String, Result<T, String>),
) {
    let peers = peers.into_iter().take(super::PROBES_PER_ROUND);
    let workers = worker_limit.min(MAX_CONCURRENT_PROBES).min(peers.len());
    let pending = Mutex::new(peers);
    // Enough space for the whole bounded round, including during scope unwinding.
    let (sender, receiver) = mpsc::sync_channel(super::PROBES_PER_ROUND);
    let probe = |peer: &str| {
        catch_unwind(AssertUnwindSafe(|| request(peer)))
            .unwrap_or_else(|_| Err("peer discovery probe panicked".into()))
    };
    let next_peer = || {
        let mut pending = pending.lock().unwrap_or_else(|error| error.into_inner());
        running
            .load(Ordering::SeqCst)
            .then(|| pending.next())
            .flatten()
    };

    thread::scope(|scope| {
        let mut started = 0;
        for index in 0..workers {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            let sender = sender.clone();
            let next_peer = &next_peer;
            let probe = &probe;
            match thread::Builder::new()
                .name(format!("agent-mesh-probe-{index}"))
                .spawn_scoped(scope, move || {
                    while let Some(peer) = next_peer() {
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }
                        let result = probe(&peer);
                        if sender.send((peer, result)).is_err() {
                            break;
                        }
                    }
                }) {
                Ok(_) => started += 1,
                Err(error) => {
                    eprintln!("peer discovery worker unavailable: kind={:?}", error.kind())
                }
            }
        }
        drop(sender);

        if started == 0 {
            // Do not rotate unprobed candidates away when thread resources are exhausted.
            while let Some(peer) = next_peer() {
                let result = probe(&peer);
                if running.load(Ordering::SeqCst) {
                    observe(peer, result);
                }
            }
        } else {
            for (peer, result) in receiver {
                if running.load(Ordering::SeqCst) {
                    observe(peer, result);
                }
            }
        }
        // The scope joins every worker before the next round or shutdown can finish.
    });
}

#[cfg(test)]
#[path = "tests/agent_mesh_probe.rs"]
mod tests;
