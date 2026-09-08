use kyuubiki_protocol::canonical_json_sha256;
use serde_json::{Value, json};
use std::{env, fs, path::PathBuf};

pub struct Evidence {
    root: Option<PathBuf>,
    suite: &'static str,
    expected: usize,
    rows: Vec<Value>,
}

impl Evidence {
    pub fn new(suite: &'static str, expected: usize) -> Self {
        let root = env::var_os("KYUUBIKI_RESEARCH_EVIDENCE_ROOT")
            .map(|root| PathBuf::from(root).join(suite));
        if let Some(root) = &root {
            fs::create_dir(root).expect("a new research evidence directory");
        }
        let evidence = Self {
            root,
            suite,
            expected,
            rows: vec![],
        };
        evidence.write_report(false);
        evidence
    }

    pub fn record(&mut self, request: &Value, result: &Value, validation: &Value) {
        let Some(root) = &self.root else {
            return;
        };
        let index = self.rows.len();
        let request_file = format!("{index:03}-request.json");
        let result_file = format!("{index:03}-result.json");
        for (name, value) in [(&request_file, request), (&result_file, result)] {
            use std::io::Write;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(root.join(name))
                .unwrap()
                .write_all(&serde_json::to_vec(value).unwrap())
                .unwrap();
        }
        self.rows.push(json!({"validation":validation,"request_file":request_file,"result_file":result_file,
            "request_canonical_sha256":canonical_json_sha256(request),"result_canonical_sha256":canonical_json_sha256(result)}));
        self.write_report(false);
    }

    pub fn finish(self) {
        self.write_report(true);
    }

    fn write_report(&self, complete: bool) {
        let Some(root) = &self.root else {
            return;
        };
        let report = json!({"schema_version":"kyuubiki.source-research-evidence/v1",
            "execution":"current_source_rust_engine_integration", "installed_service_verified":false,
            "suite":self.suite,"complete":complete,"expected_case_count":self.expected,"case_count":self.rows.len(),
            "passed_count":self.rows.iter().filter(|row|row["validation"]["passed"]==true).count(),
            "source_revision":env::var("KYUUBIKI_SOURCE_REVISION").unwrap_or_else(|_|"unrecorded".into()),
            "qualification":"synthetic_reference_not_material_certification","cases":self.rows});
        fs::write(
            root.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}
