use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use kyuubiki_protocol::ClusterPeerDescriptor;

use super::{MAX_PEERS, PROBES_PER_ROUND, build_peer_descriptors, peer_rpc};

pub(super) const FAILURE_THRESHOLD: u32 = 3;
pub(super) const COOLDOWN_MS: u64 = 60_000;
pub(super) const MAX_RETIRED: usize = 128;

pub(super) struct CandidatePool {
    known: Vec<String>,
    seeds: HashSet<String>,
    own: Vec<String>,
    failures: HashMap<String, u32>,
    last_seen: HashMap<String, u64>,
    retired: HashMap<String, Instant>,
    probe_queue: VecDeque<String>,
}

impl CandidatePool {
    pub(super) fn new(seeds: Vec<String>, own: Vec<String>) -> Self {
        Self {
            seeds: seeds.iter().cloned().collect(),
            probe_queue: seeds.iter().cloned().collect(),
            known: seeds,
            own,
            failures: HashMap::new(),
            last_seen: HashMap::new(),
            retired: HashMap::new(),
        }
    }

    pub(super) fn next_round(&mut self) -> Vec<String> {
        let count = self.probe_queue.len().min(PROBES_PER_ROUND);
        let mut round = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(peer) = self.probe_queue.pop_front() {
                self.probe_queue.push_back(peer.clone());
                round.push(peer);
            }
        }
        round
    }

    pub(super) fn record_success(
        &mut self,
        peer: &str,
        hints: Vec<ClusterPeerDescriptor>,
        now: Instant,
        seen_unix_s: u64,
    ) {
        if self.known.iter().any(|address| address == peer) {
            self.failures.insert(peer.into(), 0);
            self.last_seen.insert(peer.into(), seen_unix_s);
            self.merge_hints(hints, now);
        }
    }

    pub(super) fn record_failure(&mut self, peer: &str) {
        if self.known.iter().any(|address| address == peer) {
            let count = self.failures.entry(peer.into()).or_default();
            *count = count.saturating_add(1);
        }
    }

    pub(super) fn merge_hints(&mut self, hints: Vec<ClusterPeerDescriptor>, now: Instant) {
        for hint in hints {
            if self.known.len() >= MAX_PEERS {
                break;
            }
            if let Ok(address) = peer_rpc::normalize_endpoint(&hint.address)
                && !self.known.contains(&address)
                && !self.own.contains(&address)
                && self.retired.get(&address).is_none_or(|until| now >= *until)
            {
                // Hints never restore old health, failure counts or timestamps.
                self.probe_queue.push_back(address.clone());
                self.known.push(address);
            }
        }
    }

    pub(super) fn finish_round(&mut self, now: Instant) -> Vec<ClusterPeerDescriptor> {
        self.retired.retain(|_, until| now < *until);
        self.known.retain(|peer| {
            if self.seeds.contains(peer)
                || self.failures.get(peer).copied().unwrap_or(0) < FAILURE_THRESHOLD
                || self.retired.len() >= MAX_RETIRED
            {
                return true;
            }
            // Keep the entire cooldown when saturated rather than evicting a newer tombstone.
            self.retired
                .insert(peer.clone(), now + Duration::from_millis(COOLDOWN_MS));
            self.failures.remove(peer);
            self.last_seen.remove(peer);
            false
        });
        // Removing or sorting the display list must not move waiting peers behind fresh hints.
        self.probe_queue.retain(|peer| self.known.contains(peer));
        self.known.sort();
        build_peer_descriptors(&self.known, &self.failures, &self.last_seen)
    }
}

#[cfg(test)]
#[path = "tests/agent_mesh_candidates.rs"]
mod tests;
