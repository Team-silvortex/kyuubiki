// Test-only native library. Never package or install this as a real operator.
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{
    io::Read,
    process::{Command, Stdio},
};

static FREE_HANG: AtomicBool = AtomicBool::new(false);

fn text_field<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    input
        .split(&format!("\"{key}\":\""))
        .nth(1)?
        .split('"')
        .next()
}

#[repr(C)]
pub struct Buffer {
    ptr: *mut u8,
    len: usize,
    capacity: usize,
}

fn hang() -> ! {
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

extern "C" fn initialize() {
    #[cfg(constructor_abort)]
    std::process::abort();
    #[cfg(constructor_hang)]
    hang();
}

#[used]
#[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__mod_init_func"))]
#[cfg_attr(target_os = "linux", unsafe(link_section = ".init_array"))]
static INITIALIZE: extern "C" fn() = initialize;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_template_operator_json(
    input: *const u8,
    len: usize,
    output: *mut Buffer,
) -> i32 {
    let input = unsafe { std::slice::from_raw_parts(input, len) };
    let input = String::from_utf8_lossy(input);
    if let Some(path) = text_field(&input, "marker") {
        std::fs::write(path, std::process::id().to_string()).unwrap();
    }
    if let Some(path) = text_field(&input, "helper_path") {
        let mut child = Command::new(path)
            .arg(text_field(&input, "helper_address").unwrap())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        // Ensure the independent test connection exists before faulting or returning.
        child.stdout.take().unwrap().read_exact(&mut [0]).unwrap();
        // Intentionally leave the helper to the supervisor's process-group cleanup.
        drop(child);
    }
    if input.contains("\"fault\":\"abort\"") {
        std::process::abort();
    }
    if input.contains("\"fault\":\"hang\"") {
        hang();
    }
    if input.contains("\"fault\":\"invalid_pointer\"") {
        unsafe {
            *output = Buffer {
                ptr: std::ptr::dangling_mut(),
                len: 32,
                capacity: 32,
            }
        };
        return 0;
    }
    FREE_HANG.store(input.contains("\"fault\":\"free_hang\""), Ordering::Relaxed);
    // stdout/stderr are deliberately noisy: neither may carry the IPC protocol.
    println!("native operator output is not a protocol frame");
    eprintln!("native operator diagnostic");
    let inherited = std::env::var_os("KYUUBIKI_ISOLATION_SECRET_PROBE").is_some();
    let operator_id = match text_field(&input, "fault") {
        Some("wrong_result_id") => "extract.other",
        Some("empty_result_id") => "",
        _ => "extract.template_summary",
    };
    let error = if text_field(&input, "fault") == Some("ambiguous_success") {
        r#", "error":"fixture failure despite success""#
    } else {
        ""
    };
    let bytes = format!(
        r#"{{"schema_version":"kyuubiki.operator-json-c/v1","ok":true,"result":{{"operator_id":"{operator_id}","summary":{{"pid":{},"secret_inherited":{},"sum":14}},"artifacts":[]}}{error}}}"#,
        std::process::id(),
        inherited
    );
    let mut bytes = bytes.into_bytes();
    unsafe {
        *output = Buffer {
            ptr: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
        };
    }
    std::mem::forget(bytes);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn kyuubiki_operator_json_free(buffer: Buffer) {
    if FREE_HANG.load(Ordering::Relaxed) {
        hang();
    }
    unsafe { drop(Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.capacity)) };
}
