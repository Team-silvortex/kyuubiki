use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn current_peak_rss_kib() -> u64 {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
        if status == 0 {
            let usage = unsafe { usage.assume_init() };
            #[cfg(target_os = "macos")]
            {
                return (usage.ru_maxrss as u64) / 1024;
            }
            #[cfg(target_os = "linux")]
            {
                return usage.ru_maxrss as u64;
            }
        }
    }

    0
}

pub(crate) fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

pub(crate) fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
