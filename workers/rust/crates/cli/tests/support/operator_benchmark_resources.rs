use serde::Serialize;
use std::{
    collections::BTreeSet,
    fs, io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Default, Serialize)]
pub struct Resources {
    pub processes: usize,
    pub rss_kib: u64,
    pub file_descriptors: usize,
    pub agent_file_descriptors: usize,
    pub unreadable_fd_processes: usize,
    pub unreadable_child_lists: usize,
    pub cpu_ticks_agent_and_reaped_children: u64,
}

fn exited(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound || error.raw_os_error() == Some(libc::ESRCH)
}

fn count_entries<T>(mut entries: impl Iterator<Item = io::Result<T>>) -> io::Result<usize> {
    entries.try_fold(0, |count, entry| match entry {
        Ok(_) => Ok(count + 1),
        Err(e) if exited(&e) => Ok(count),
        Err(e) => Err(e),
    })
}

fn cpu_ticks(stat: &str) -> io::Result<u64> {
    let fields: Vec<_> = stat
        .rsplit_once(") ")
        .ok_or_else(|| io::Error::other("invalid proc stat"))?
        .1
        .split_whitespace()
        .skip(11)
        .take(4)
        .collect();
    if fields.len() != 4 {
        return Err(io::Error::other("missing CPU accounting"));
    }
    fields.into_iter().try_fold(0_u64, |total, field| {
        let value = field.parse::<u64>().map_err(io::Error::other)?;
        total
            .checked_add(value)
            .ok_or_else(|| io::Error::other("CPU accounting overflow"))
    })
}

pub fn snapshot(root: u32) -> io::Result<Resources> {
    let mut result = Resources {
        cpu_ticks_agent_and_reaped_children: cpu_ticks(&fs::read_to_string(format!(
            "/proc/{root}/stat"
        ))?)?,
        ..Resources::default()
    };
    let mut queue = vec![root];
    let mut seen = BTreeSet::new();
    while let Some(pid) = queue.pop() {
        if !seen.insert(pid) {
            continue;
        }
        if seen.len() > 64 {
            return Err(io::Error::other("owned process budget exceeded"));
        }
        let status = match fs::read_to_string(format!("/proc/{pid}/status")) {
            Ok(status) => status,
            Err(e) if pid != root && exited(&e) => continue,
            Err(e) => return Err(e),
        };
        result.processes += 1;
        let rss = status
            .lines()
            .find_map(|line| line.strip_prefix("VmRSS:"))
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse::<u64>().ok());
        // Exiting children may already have no memory; the live Agent must remain readable.
        if pid == root && rss.is_none() {
            return Err(io::Error::other("Agent RSS unavailable"));
        }
        result.rss_kib += rss.unwrap_or(0);
        match fs::read_dir(format!("/proc/{pid}/fd")).and_then(count_entries) {
            Ok(count) => {
                result.file_descriptors += count;
                if pid == root {
                    result.agent_file_descriptors = count;
                }
            }
            Err(e) if pid != root && exited(&e) => {}
            Err(e) if pid != root && e.kind() == io::ErrorKind::PermissionDenied => {
                result.unreadable_fd_processes += 1;
            }
            Err(e) => return Err(e),
        }
        let tasks = match fs::read_dir(format!("/proc/{pid}/task")) {
            Ok(tasks) => tasks,
            Err(e) if pid != root && exited(&e) => continue,
            Err(e) if pid != root && e.kind() == io::ErrorKind::PermissionDenied => {
                result.unreadable_child_lists += 1;
                continue;
            }
            Err(e) => return Err(e),
        };
        // A worker is forked by an RPC thread, not necessarily the main Agent thread.
        for task in tasks {
            let task = match task {
                Ok(task) => task,
                Err(e) if exited(&e) => continue,
                Err(e) => return Err(e),
            };
            match fs::read_to_string(task.path().join("children")) {
                Ok(children) => queue.extend(
                    children
                        .split_whitespace()
                        .filter_map(|v| v.parse::<u32>().ok()),
                ),
                Err(e) if exited(&e) => {}
                Err(e) if pid != root && e.kind() == io::ErrorKind::PermissionDenied => {
                    result.unreadable_child_lists += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
    Ok(result)
}

pub struct Monitor {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<io::Result<(Resources, usize)>>>,
}
impl Monitor {
    pub fn start(pid: u32) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let join = thread::spawn(move || {
            let mut peak = Resources::default();
            let mut count = 0;
            while !flag.load(Ordering::Acquire) {
                let sample = snapshot(pid)?;
                peak.processes = peak.processes.max(sample.processes);
                peak.rss_kib = peak.rss_kib.max(sample.rss_kib);
                peak.file_descriptors = peak.file_descriptors.max(sample.file_descriptors);
                peak.agent_file_descriptors = peak
                    .agent_file_descriptors
                    .max(sample.agent_file_descriptors);
                peak.unreadable_fd_processes = peak
                    .unreadable_fd_processes
                    .max(sample.unreadable_fd_processes);
                peak.unreadable_child_lists = peak
                    .unreadable_child_lists
                    .max(sample.unreadable_child_lists);
                peak.cpu_ticks_agent_and_reaped_children =
                    sample.cpu_ticks_agent_and_reaped_children;
                count += 1;
                thread::sleep(Duration::from_millis(5));
            }
            Ok((peak, count))
        });
        Self {
            stop,
            join: Some(join),
        }
    }
    pub fn finish(mut self) -> (Resources, usize) {
        self.stop.store(true, Ordering::Release);
        self.join
            .take()
            .unwrap()
            .join()
            .expect("resource sampler panicked")
            .expect("resource sampler failed")
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_read_failure_is_not_reported_as_zero_usage() {
        assert!(snapshot(0).is_err());
        let own = snapshot(std::process::id()).unwrap();
        assert!(own.processes >= 1 && own.rss_kib > 0 && own.file_descriptors > 0);
    }

    #[test]
    fn vanished_processes_are_distinct_from_permission_failures() {
        assert!(exited(&io::Error::from_raw_os_error(libc::ESRCH)));
        assert!(exited(&io::Error::from_raw_os_error(libc::ENOENT)));
        assert!(!exited(&io::Error::from_raw_os_error(libc::EACCES)));
    }

    #[test]
    fn cpu_accounting_includes_waited_children_and_handles_spaces_in_names() {
        let stat = format!(
            "123 (worker ) odd name) {} 2 3 5 7 0 0",
            "S 0 0 0 0 0 0 0 0 0 0"
        );
        assert_eq!(cpu_ticks(&stat).unwrap(), 17);
        assert!(cpu_ticks("123 (incomplete) S 0").is_err());
    }

    #[test]
    fn directory_iterator_errors_are_not_counted_as_file_descriptors() {
        assert_eq!(
            count_entries(
                [
                    Ok(()),
                    Err(io::Error::from_raw_os_error(libc::ENOENT)),
                    Ok(())
                ]
                .into_iter()
            )
            .unwrap(),
            2
        );
        assert!(
            count_entries([Ok(()), Err(io::Error::from_raw_os_error(libc::EACCES))].into_iter())
                .is_err()
        );
    }
}
