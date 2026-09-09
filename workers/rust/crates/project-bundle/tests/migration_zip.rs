mod support;

use kyuubiki_project_bundle::{ProjectMigrationLimits, plan_project_migration};
use std::fs;
use support::{Fixture, manifest};

#[test]
fn exact_duplicate_entries_are_detected_before_zip_reader_collapses_them() {
    let fixture = Fixture::new();
    let body = serde_json::to_vec(&manifest()).unwrap();
    let path = fixture.zip(vec![("project.json", body.clone()), ("project.jsox", body)]);
    let mut bytes = fs::read(&path).unwrap();
    for index in 0..=bytes.len() - 12 {
        if bytes[index..index + 12] == *b"project.jsox" {
            bytes[index..index + 12].copy_from_slice(b"project.json");
        }
    }
    fs::write(&path, &bytes).unwrap();
    // Demonstrates why iterating ZipArchive::len() cannot detect this on its own.
    assert_eq!(
        zip::ZipArchive::new(fs::File::open(&path).unwrap())
            .unwrap()
            .len(),
        1
    );
    let plan =
        plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default()).unwrap();
    assert_eq!(plan.status, "blocked");
    assert!(plan.issues[0].contains("physical ZIP entry"));
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn zip64_directory_and_comment_are_supported_with_preallocation_limits() {
    let fixture = Fixture::new();
    let path = fixture.zip(vec![(
        "project.json",
        serde_json::to_vec(&manifest()).unwrap(),
    )]);
    let regular = fs::read(&path).unwrap();
    let (zip64, record_start) = zip64_footer(&regular);
    fs::write(&path, &zip64).unwrap();
    let plan =
        plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default()).unwrap();
    assert_eq!(plan.status, "upgrade_required", "{:?}", plan.issues);

    let mut oversized = zip64.clone();
    oversized[record_start + 24..record_start + 32].copy_from_slice(&100_001u64.to_le_bytes());
    oversized[record_start + 32..record_start + 40].copy_from_slice(&100_001u64.to_le_bytes());
    fs::write(&path, oversized).unwrap();
    let plan =
        plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default()).unwrap();
    assert_eq!(plan.status, "blocked");
    assert!(plan.issues[0].contains("entry limit"));

    let mut commented = zip64;
    let length = commented.len();
    commented[length - 2..].copy_from_slice(&4u16.to_le_bytes());
    commented.extend_from_slice(b"note");
    fs::write(&path, commented).unwrap();
    assert_eq!(
        plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
            .unwrap()
            .status,
        "upgrade_required"
    );
}

#[test]
fn malformed_directory_counts_offsets_and_truncations_fail_without_panicking() {
    let fixture = Fixture::new();
    let path = fixture.zip(vec![(
        "project.json",
        serde_json::to_vec(&manifest()).unwrap(),
    )]);
    let original = fs::read(&path).unwrap();
    let end = original.len() - 22;
    for offset in [8, 10, 12, 16] {
        let mut changed = original.clone();
        changed[end + offset] ^= 0x40;
        fs::write(&path, changed).unwrap();
        assert_eq!(
            plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
                .unwrap()
                .status,
            "blocked"
        );
    }
    for length in [0, 1, 4, 21, 22, original.len() - 1] {
        fs::write(&path, &original[..length]).unwrap();
        assert_eq!(
            plan_project_migration(path.to_str().unwrap(), &ProjectMigrationLimits::default())
                .unwrap()
                .status,
            "blocked"
        );
    }
}

fn zip64_footer(bytes: &[u8]) -> (Vec<u8>, usize) {
    let end = bytes.len() - 22;
    assert_eq!(&bytes[end..end + 4], b"PK\x05\x06");
    let count = u16::from_le_bytes(bytes[end + 10..end + 12].try_into().unwrap()) as u64;
    let size = u32::from_le_bytes(bytes[end + 12..end + 16].try_into().unwrap()) as u64;
    let offset = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as u64;
    let mut result = bytes[..end].to_vec();
    result.extend_from_slice(b"PK\x06\x06");
    result.extend_from_slice(&44u64.to_le_bytes());
    result.extend_from_slice(&45u16.to_le_bytes());
    result.extend_from_slice(&45u16.to_le_bytes());
    result.extend_from_slice(&0u32.to_le_bytes());
    result.extend_from_slice(&0u32.to_le_bytes());
    for value in [count, count, size, offset] {
        result.extend_from_slice(&value.to_le_bytes());
    }
    result.extend_from_slice(b"PK\x06\x07");
    result.extend_from_slice(&0u32.to_le_bytes());
    result.extend_from_slice(&(end as u64).to_le_bytes());
    result.extend_from_slice(&1u32.to_le_bytes());
    let new_end = result.len();
    result.extend_from_slice(&bytes[end..]);
    result[new_end + 8..new_end + 12].fill(0xff);
    (result, end)
}
