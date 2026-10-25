#[path = "../examples/durable_modal_research/evidence.rs"]
mod evidence;
#[path = "../examples/durable_modal_research/records.rs"]
mod records;

use serde_json::json;
use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Result<Self> {
        for _ in 0..64 {
            let path = env::temp_dir().join(format!(
                "kmr-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Err("cannot allocate test directory".into())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn interrupted_record_write_never_exposes_a_partial_final_file() -> Result<()> {
    let fixture = Fixture::new()?;
    let value = json!({"status":"running", "request_id":"original"});
    let result = records::retain_with(&fixture.0, "admission.json", &value, |file, bytes| {
        file.write_all(&bytes[..bytes.len() / 2])?;
        file.sync_all()?;
        Err("injected partial write failure".into())
    });
    assert!(result.is_err());
    assert!(
        !fixture.0.join("admission.json").exists(),
        "partial record became final evidence"
    );
    assert_eq!(fs::read_dir(&fixture.0)?.count(), 0);
    records::retain(&fixture.0, "admission.json", &value)?;
    assert_eq!(evidence::read(&fixture.0, "admission.json")?.value, value);
    Ok(())
}

#[test]
fn complete_record_preserves_exact_bytes_without_pending_alias() -> Result<()> {
    let fixture = Fixture::new()?;
    for name in [
        "input.json",
        "submission-intent.json",
        "admission.json",
        "receipt.json",
    ] {
        let value = json!({"name":name,"nested":[-0.0,1.2345678901234567,u64::MAX]});
        records::retain(&fixture.0, name, &value)?;
        assert_eq!(
            fs::read(fixture.0.join(name))?,
            serde_json::to_vec_pretty(&value)?
        );
        assert!(!fixture.0.join(format!(".{name}.pending")).exists());
        assert!(records::retain(&fixture.0, name, &json!({"replacement":true})).is_err());
        assert_eq!(evidence::read(&fixture.0, name)?.value, value);
    }
    assert_eq!(fs::read_dir(&fixture.0)?.count(), 4);
    Ok(())
}

#[test]
fn staged_record_corruption_is_rejected_before_publication() -> Result<()> {
    let fixture = Fixture::new()?;
    let error = records::retain_with(
        &fixture.0,
        "receipt.json",
        &json!({"valid":"original"}),
        |file, _| {
            file.write_all(b"{}")?;
            Ok(())
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("staged record changed"));
    assert_eq!(fs::read_dir(&fixture.0)?.count(), 0);
    Ok(())
}

#[test]
fn record_publication_collision_preserves_the_existing_winner() -> Result<()> {
    let fixture = Fixture::new()?;
    let result = records::retain_with(
        &fixture.0,
        "admission.json",
        &json!({"new":true}),
        |file, bytes| {
            file.write_all(bytes)?;
            fs::write(fixture.0.join("admission.json"), b"existing winner")?;
            Ok(())
        },
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read(fixture.0.join("admission.json"))?,
        b"existing winner"
    );
    assert_eq!(fs::read_dir(&fixture.0)?.count(), 1);
    Ok(())
}

#[test]
fn stale_pending_records_are_preserved_not_promoted_or_reused() -> Result<()> {
    let fixture = Fixture::new()?;
    let pending = fixture.0.join(".admission.json.pending");
    fs::write(&pending, b"partial old record")?;
    assert!(records::retain(&fixture.0, "admission.json", &json!({})).is_err());
    assert_eq!(fs::read(&pending)?, b"partial old record");
    assert!(evidence::read_optional(&fixture.0, "admission.json")?.is_none());
    let complete = json!({"committed":true});
    fs::write(&pending, serde_json::to_vec(&complete)?)?;
    fs::hard_link(&pending, fixture.0.join("admission.json"))?;
    assert_eq!(
        evidence::read(&fixture.0, "admission.json")?.value,
        complete
    );
    assert!(records::retain(&fixture.0, "admission.json", &json!({})).is_err());
    assert!(pending.is_file());
    assert_eq!(
        evidence::read(&fixture.0, "admission.json")?.value,
        complete
    );
    Ok(())
}

#[test]
fn invalid_or_oversized_records_do_not_create_files() -> Result<()> {
    let fixture = Fixture::new()?;
    for name in [
        "",
        "../admission.json",
        "..\\receipt.json",
        "arbitrary.json",
    ] {
        assert!(records::retain(&fixture.0, name, &json!({})).is_err());
    }
    let oversized = json!({"payload":"x".repeat(evidence::MAX_BYTES as usize)});
    let error = records::retain(&fixture.0, "receipt.json", &oversized).unwrap_err();
    assert!(error.to_string().contains("8 MiB"));
    assert_eq!(fs::read_dir(&fixture.0)?.count(), 0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn record_writer_does_not_replace_dangling_links_or_follow_link_containers() -> Result<()> {
    let fixture = Fixture::new()?;
    let link = fixture.0.join("admission.json");
    std::os::unix::fs::symlink("absent-target", &link)?;
    assert!(records::retain(&fixture.0, "admission.json", &json!({})).is_err());
    assert!(fs::symlink_metadata(link)?.file_type().is_symlink());
    assert!(!fixture.0.join("absent-target").exists());
    let container = fixture.0.join("linked-root");
    std::os::unix::fs::symlink(&fixture.0, &container)?;
    assert!(records::retain(&container, "input.json", &json!({})).is_err());
    assert!(!fixture.0.join("input.json").exists());
    Ok(())
}
