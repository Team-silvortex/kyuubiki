/// Build only the shared account page, never an arbitrary redirect or file URL.
pub fn store_account_portal_url(origin: &str, language: &str) -> Result<String, String> {
    let invalid = || "Account origin must be bare HTTPS without credentials or a path".to_string();
    let authority_text = origin.strip_prefix("https://").ok_or_else(invalid)?;
    let authority_text = authority_text.strip_suffix('/').unwrap_or(authority_text);
    if origin.len() > 2048
        || authority_text.is_empty()
        || !authority_text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".:-[]".contains(&b))
    {
        return Err(invalid());
    }
    let port = if authority_text.starts_with('[') {
        let (host, suffix) = authority_text.split_once(']').ok_or_else(invalid)?;
        host[1..]
            .parse::<std::net::Ipv6Addr>()
            .map_err(|_| invalid())?;
        if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix(':').ok_or_else(invalid)?)
        }
    } else {
        if authority_text.contains(['[', ']']) {
            return Err(invalid());
        }
        authority_text.split_once(':').map(|(_, port)| port)
    };
    if port.is_some_and(|port| port.parse::<u16>().map_or(true, |n| n == 0)) {
        return Err(invalid());
    }
    let uri: ureq::http::Uri = origin.parse().map_err(|_| invalid())?;
    let authority = uri.authority().ok_or_else(invalid)?;
    if uri.scheme_str() != Some("https")
        || uri.host().is_none_or(str::is_empty)
        || !matches!(uri.path(), "" | "/")
        || uri.query().is_some()
    {
        return Err(invalid());
    }
    let language = if language.starts_with("zh") {
        "zh"
    } else {
        "en"
    };
    Ok(format!("https://{authority}/account?lang={language}"))
}

/// Uses the OS browser association; no shell interpreter or account credentials.
pub fn open_store_account_portal(origin: &str, language: &str) -> Result<(), String> {
    let url = store_account_portal_url(origin, language)?;
    open_browser(&url)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn open_browser(url: &str) -> Result<(), String> {
    use std::{
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    #[cfg(target_os = "macos")]
    let mut command = Command::new("/usr/bin/open");
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("/usr/bin/gio");
        command.arg("open");
        command
    };
    let mut child = command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "System browser launcher is unavailable".to_string())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err("System browser could not be opened".into())
                };
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("System browser request did not complete".into());
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn open_browser(url: &str) -> Result<(), String> {
    use std::{
        ffi::{OsStr, c_void},
        os::windows::ffi::OsStrExt,
        ptr,
    };
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            window: *mut c_void,
            verb: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> *mut c_void;
    }
    let wide = |text: &str| {
        OsStr::new(text)
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let verb = wide("open");
    let url = wide(url);
    // All buffers live through this synchronous call; only a validated HTTPS URL is passed.
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            verb.as_ptr(),
            url.as_ptr(),
            ptr::null(),
            ptr::null(),
            1,
        )
    };
    if result as isize > 32 {
        Ok(())
    } else {
        Err("System browser could not be opened".into())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(crate) fn open_browser(_: &str) -> Result<(), String> {
    Err("System browser integration is unavailable on this platform".into())
}
