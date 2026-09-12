use std::io;

/// Identifies a process incarnation, not just a reusable PID. Errors must not authorize cleanup.
pub fn process_instance_token(pid: u32) -> io::Result<Option<String>> {
    if pid == 0 || pid > i32::MAX as u32 {
        return Ok(None);
    }
    platform_token(pid)
}

#[cfg(target_os = "linux")]
fn platform_token(pid: u32) -> io::Result<Option<String>> {
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(start) = linux_start_ticks(&stat)? else {
        return Ok(None);
    };
    let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
    if boot.trim().is_empty() {
        return Err(io::Error::other("missing Linux boot identity"));
    }
    Ok(Some(format!("linux:{}:{start}", boot.trim())))
}

#[cfg(any(target_os = "linux", test))]
fn linux_start_ticks(stat: &str) -> io::Result<Option<u64>> {
    // comm may contain spaces and parentheses; fields after its final ')' start at field 3.
    let (_, fields) = stat
        .rsplit_once(") ")
        .ok_or_else(|| io::Error::other("invalid process stat record"))?;
    let fields: Vec<_> = fields.split_whitespace().collect();
    if matches!(fields.first().copied(), Some("Z" | "X" | "x")) {
        return Ok(None);
    }
    let start = fields
        .get(19)
        .and_then(|field| field.parse().ok())
        .ok_or_else(|| io::Error::other("missing process start ticks"))?;
    Ok(Some(start))
}

#[cfg(target_os = "macos")]
fn platform_token(pid: u32) -> io::Result<Option<String>> {
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info) as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            std::ptr::addr_of_mut!(info).cast(),
            size,
        )
    };
    if read != size {
        let error = io::Error::last_os_error();
        if matches!(error.raw_os_error(), Some(libc::ESRCH | libc::ENOENT)) {
            return Ok(None);
        }
        return Err(error);
    }
    if info.pbi_status == libc::SZOMB {
        return Ok(None);
    }
    if info.pbi_pid != pid {
        return Err(io::Error::other(
            "process identity changed during observation",
        ));
    }
    Ok(Some(format!(
        "macos:{}:{}",
        info.pbi_start_tvsec, info.pbi_start_tvusec
    )))
}

#[cfg(windows)]
fn platform_token(pid: u32) -> io::Result<Option<String>> {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_INVALID_PARAMETER, FILETIME, STILL_ACTIVE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            Ok(None)
        } else {
            Err(error)
        };
    }
    let result = (|| {
        let mut exit_code = 0;
        if unsafe { GetExitCodeProcess(handle, &mut exit_code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if exit_code != STILL_ACTIVE as u32 {
            return Ok(None);
        }
        let mut created: FILETIME = unsafe { std::mem::zeroed() };
        let mut exited = created;
        let mut kernel = created;
        let mut user = created;
        if unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) }
            == 0
        {
            return Err(io::Error::last_os_error());
        }
        let ticks = ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64;
        Ok(Some(format!("windows:{ticks}")))
    })();
    unsafe { CloseHandle(handle) };
    result
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_token(_pid: u32) -> io::Result<Option<String>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native process identity is unavailable",
    ))
}

#[cfg(test)]
mod tests {
    use super::{linux_start_ticks, process_instance_token};

    #[test]
    fn current_incarnation_is_stable_and_invalid_pids_are_not_identities() {
        let token = process_instance_token(std::process::id()).unwrap();
        assert!(token.is_some());
        assert_eq!(token, process_instance_token(std::process::id()).unwrap());
        assert_eq!(process_instance_token(0).unwrap(), None);
        assert_eq!(process_instance_token(u32::MAX).unwrap(), None);
    }

    #[test]
    fn linux_stat_handles_parentheses_and_rejects_truncated_or_dead_processes() {
        let mut fields = vec!["0"; 20];
        fields[0] = "S";
        fields[19] = "123456";
        assert_eq!(
            linux_start_ticks(&format!("99 (worker (test)) {}", fields.join(" "))).unwrap(),
            Some(123456)
        );
        assert_eq!(linux_start_ticks("99 (worker) Z").unwrap(), None);
        assert!(linux_start_ticks("99 (worker) S 0").is_err());
    }
}
