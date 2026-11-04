use std::collections::BTreeMap;
use std::process::Command;

const CI: &str = include_str!("../../../../../.github/workflows/ci.yml");

#[derive(Debug)]
struct Selection {
    filter: String,
    skips: Vec<String>,
}

impl Selection {
    fn parse(args: &str) -> Self {
        let args: Vec<_> = args.split_whitespace().collect();
        let boundary = args
            .iter()
            .position(|arg| *arg == "--")
            .unwrap_or(args.len());
        let filter = match &args[..boundary] {
            [] => "",
            ["--lib", filter] if !filter.starts_with('-') => filter,
            other => panic!("unsupported solver CI selection: {other:?}"),
        };
        let mut skips = Vec::new();
        if boundary < args.len() {
            let mut rest = args[boundary + 1..].chunks_exact(2);
            for pair in &mut rest {
                assert_eq!(pair[0], "--skip", "unexpected harness flag in {args:?}");
                assert!(!pair[1].starts_with('-'));
                skips.push(pair[1].to_string());
            }
            assert!(rest.remainder().is_empty(), "incomplete skip in {args:?}");
        }
        Self {
            filter: filter.into(),
            skips,
        }
    }

    fn includes(&self, name: &str) -> bool {
        name.contains(&self.filter) && !self.skips.iter().any(|skip| name.contains(skip))
    }
}

fn selections() -> BTreeMap<String, Selection> {
    let solver = CI
        .split_once("  solver:\n")
        .unwrap()
        .1
        .split_once("  frontend:\n")
        .unwrap()
        .0;
    assert!(solver.contains("fail-fast: false"));
    assert!(
        solver.contains(
            "cargo test --locked -p kyuubiki-solver --no-fail-fast ${{ matrix.test_args }}"
        )
    );
    let mut selected = BTreeMap::new();
    let mut group = None;
    for line in solver.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("- group: ") {
            assert!(group.is_none(), "previous group has no test_args");
            group = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("test_args: ") {
            // Keep this small CI matrix in the quoted JSON-compatible YAML subset.
            let args: String = serde_json::from_str(value).expect("quoted CI test_args");
            let name = group.take().expect("test_args without group");
            assert!(selected.insert(name, Selection::parse(&args)).is_none());
        }
    }
    assert!(group.is_none());
    assert_eq!(selected.len(), 5);
    assert!(selected.contains_key("general"));
    selected
}

#[test]
fn solver_ci_groups_partition_every_compiled_unit_test_exactly_once() {
    // Listing the same harness discovers real tests, including future additions,
    // without executing them recursively or relying on hand-maintained names.
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--list", "--format", "terse"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let listing = String::from_utf8(output.stdout).unwrap();
    let selected = selections();
    let mut counts: BTreeMap<_, usize> = selected.keys().map(|key| (key, 0)).collect();
    let mut total = 0;
    for test in listing
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
    {
        let owners: Vec<_> = selected
            .iter()
            .filter(|(_, group)| group.includes(test))
            .collect();
        assert_eq!(
            owners.len(),
            1,
            "test must have exactly one CI owner: {test}; {owners:?}"
        );
        *counts.get_mut(owners[0].0).unwrap() += 1;
        total += 1;
    }
    assert!(total > 0);
    for (name, count) in counts {
        assert!(count > 0, "empty CI group: {name}");
        println!("solver CI group {name}: {count} unit tests");
    }
    println!("solver CI inventory: {total} tests, no gaps or duplicate owners");
}

#[test]
fn selection_matches_libtest_substring_and_skip_semantics() {
    let selected = Selection::parse("--lib anchor -- --skip ::rounded_beam::");
    assert!(selected.includes("parent::anchor::future_family::test"));
    assert!(!selected.includes("parent::anchor::rounded_beam::test"));
    assert!(!selected.includes("unrelated::test"));
    let general = Selection::parse("-- --skip anchor");
    assert!(general.includes("unrelated::test"));
    assert!(!general.includes("parent::anchor::test"));
}
