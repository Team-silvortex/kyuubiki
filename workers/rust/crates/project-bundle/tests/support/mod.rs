use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub struct Fixture(pub PathBuf);

#[allow(dead_code)]
impl Fixture {
    pub fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("kyuubiki-migration-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    pub fn json(&self, value: &Value) -> PathBuf {
        let path = self.0.join("source.json");
        fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
        path
    }

    pub fn zip(&self, entries: Vec<(&str, Vec<u8>)>) -> PathBuf {
        let path = self.0.join("source.kyuubiki");
        let mut writer = ZipWriter::new(File::create(&path).unwrap());
        for (name, bytes) in entries {
            writer
                .start_file(
                    name,
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
        path
    }

    pub fn no_staging(&self) {
        assert!(fs::read_dir(&self.0).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".kyuubiki-migrate-")
        }));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn manifest() -> Value {
    json!({
        "project_schema_version": "kyuubiki.project/v1",
        "project": { "project_id": "research", "name": "Thermal study", "inserted_at": "2026-09-09T00:00:00Z", "updated_at": "2026-09-09T00:00:00Z" },
        "models": [{ "model_id": "model-1", "payload": {"nodes": [[0.0, -0.0], [1.2, 3.4]], "young_modulus": 210000000000.0} }],
        "model_versions": [{"model_id": "model-1", "version_id": "version-1", "payload": {"conductivity": 0.0000001234567890123456}}],
        "jobs": [{"job_id": "job-1", "model_version_id": "version-1"}],
        "results": [{"job_id": "job-1", "result": {"temperatures": [293.15, 393.15], "u": [-0.0, 1.0e-20], "counter": 18446744073709551615_u64}}],
        "active_model_id": "model-1",
        "active_version_id": "version-1",
        "vendor_metadata": {"do_not_drop": true}
    })
}

#[allow(dead_code)]
pub fn read_entry(path: &std::path::Path, name: &str) -> Vec<u8> {
    let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}
