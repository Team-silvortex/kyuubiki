#[path = "../examples/durable_modal_research/evidence.rs"]
mod evidence;

use std::{
    env, fs, io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> io::Result<Self> {
        // Keep the Unix socket case below macOS's short pathname limit.
        for _ in 0..64 {
            let root = env::temp_dir().join(format!(
                "kme-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&root) {
                Ok(()) => return Ok(Self(root)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "cannot allocate owned test directory",
        ))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn bounded_evidence_preserves_json_types_and_exact_bytes() -> io::Result<()> {
    let fixture = Fixture::new()?;
    let bytes = br#" {"types":[null,true,false,-9223372036854775808,18446744073709551615,-0.0,1.2345678901234567],"text":"\u4f60\u597d","nested":{"x":[{},[],1]}} "#;
    fs::write(fixture.0.join("input.json"), bytes)?;
    let document = evidence::read(&fixture.0, "input.json")?;
    assert_eq!(document.bytes, bytes);
    assert_eq!(
        document.value,
        serde_json::from_slice::<serde_json::Value>(bytes)?
    );
    Ok(())
}

#[test]
fn bounded_evidence_rejects_duplicate_keys_in_all_containers() -> io::Result<()> {
    let fixture = Fixture::new()?;
    for bytes in [
        r#"{"passed":false,"passed":true}"#,
        r#"{"execution":{"store_id":"wrong","store_id":"right"}}"#,
        r#"[{"mass":1,"mass":21}]"#,
        r#"{"passed":false,"\u0070assed":true}"#,
    ] {
        fs::write(fixture.0.join("verification.json"), bytes)?;
        let error = evidence::read(&fixture.0, "verification.json").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("verification.json"));
        assert!(error.to_string().contains("duplicate JSON object key"));
    }
    Ok(())
}

#[test]
fn bounded_evidence_enforces_size_before_parsing() -> io::Result<()> {
    let fixture = Fixture::new()?;
    let path = fixture.0.join("result.json");
    let mut exact = vec![b' '; evidence::MAX_BYTES as usize];
    exact[..2].copy_from_slice(b"{}");
    fs::write(&path, &exact)?;
    assert_eq!(evidence::read(&fixture.0, "result.json")?.bytes, exact);
    fs::OpenOptions::new()
        .write(true)
        .open(&path)?
        .set_len(evidence::MAX_BYTES + 1)?;
    let error = evidence::read(&fixture.0, "result.json").unwrap_err();
    assert!(error.to_string().contains("8 MiB file limit"));
    assert!(error.to_string().contains("result.json"));
    assert_eq!(fs::metadata(&path)?.len(), evidence::MAX_BYTES + 1);
    Ok(())
}

#[test]
fn bounded_evidence_rejects_trailing_malformed_and_deep_json() -> io::Result<()> {
    let fixture = Fixture::new()?;
    let deep = format!("{}0{}", "[".repeat(256), "]".repeat(256));
    for bytes in [
        b"".as_slice(),
        b"{",
        b"{} {}",
        b"1e999",
        b"\xff",
        deep.as_bytes(),
    ] {
        fs::write(fixture.0.join("receipt.json"), bytes)?;
        let error = evidence::read(&fixture.0, "receipt.json").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("receipt.json"));
    }
    Ok(())
}

#[test]
fn optional_evidence_requires_a_real_missing_leaf() -> io::Result<()> {
    let fixture = Fixture::new()?;
    assert!(evidence::read_optional(&fixture.0, "admission.json")?.is_none());
    assert_eq!(
        evidence::read(&fixture.0, "admission.json")
            .unwrap_err()
            .kind(),
        io::ErrorKind::NotFound
    );
    assert!(evidence::read_optional(&fixture.0.join("missing"), "admission.json").is_err());
    for name in [
        "",
        ".",
        "..",
        "../admission.json",
        "..\\admission.json",
        "C:admission.json",
    ] {
        assert!(evidence::read_optional(&fixture.0, name).is_err());
    }
    fs::create_dir(fixture.0.join("admission.json"))?;
    assert!(evidence::read_optional(&fixture.0, "admission.json").is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn evidence_links_and_special_files_are_rejected_without_following() -> io::Result<()> {
    use std::os::unix::{fs::symlink, net::UnixListener};
    let fixture = Fixture::new()?;
    fs::write(fixture.0.join("target.json"), b"{}")?;
    for (name, target) in [
        ("valid.json", "target.json"),
        ("broken.json", "absent.json"),
        ("cycle.json", "cycle.json"),
    ] {
        symlink(target, fixture.0.join(name))?;
        let error = evidence::read_optional(&fixture.0, name).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains(name));
    }
    symlink(&fixture.0, fixture.0.join("linked-root"))?;
    assert!(evidence::read_optional(&fixture.0.join("linked-root"), "absent.json").is_err());
    let _socket = UnixListener::bind(fixture.0.join("socket.json"))?;
    assert!(
        evidence::read(&fixture.0, "socket.json")
            .unwrap_err()
            .to_string()
            .contains("regular non-symlink")
    );
    assert_eq!(fs::read(fixture.0.join("target.json"))?, b"{}");
    Ok(())
}
