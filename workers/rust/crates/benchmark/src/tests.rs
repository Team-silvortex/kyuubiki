#[cfg(test)]
mod tests {
    use super::{benchmark_cases, headless_sdk_cases, BenchmarkConfig, OutputFormat};
    use crate::catalog::{
        catalog_spec_path_candidates, default_catalog_spec, BenchmarkCatalogSpec,
    };
    use crate::config::BenchmarkProfile;
    use crate::models::{BenchmarkCase, BenchmarkWorkload};
    use crate::runner::run_case;
    use serde::Deserialize;
    use std::collections::HashSet;
    use std::fs;

    #[path = "tests_performance.rs"]
    mod performance;

    #[test]
    fn exposes_default_benchmark_config() {
        let config = BenchmarkConfig {
            repeat: 10,
            case_filter: None,
            matrix: "core".to_string(),
            format: OutputFormat::Table,
            profile: BenchmarkProfile::TenK,
            baseline_out: None,
            baseline_compare: None,
            compare_report_out: None,
            solver_preconditioner: "jacobi".to_string(),
            progress: false,
            dry_run_shapes: false,
            fail_on_median_regression_pct: None,
            fail_on_rss_regression_pct: None,
            min_baseline_median_ms: 5.0,
        };

        assert_eq!(config.repeat, 10);
        assert!(matches!(config.format, OutputFormat::Table));
        assert!(!config.dry_run_shapes);
    }

    #[test]
    fn runs_benchmark_cases() {
        let cases = benchmark_cases(BenchmarkProfile::TenK, "core");
        let result = run_case(&cases[0], 2);

        assert_eq!(result.repeat, 2);
        assert!(result.mean_ms >= 0.0);
        assert!(result.node_count > 0);
    }

    #[test]
    fn keeps_profile_specific_truss_generators() {
        let medium_cases = benchmark_cases(BenchmarkProfile::Medium, "core");
        let tenk_cases = benchmark_cases(BenchmarkProfile::TenK, "core");

        let medium_truss = &medium_cases[1];
        let tenk_truss = &tenk_cases[1];

        assert_eq!(medium_truss.id, "truss-roof-medium");
        assert_eq!(tenk_truss.id, "truss-roof-10k");

        match (&medium_truss.workload, &tenk_truss.workload) {
            (BenchmarkWorkload::Truss2d(medium), BenchmarkWorkload::Truss2d(tenk)) => {
                assert!(medium.nodes.len() < tenk.nodes.len());
                assert!(medium.elements.len() < tenk.elements.len());
            }
            _ => panic!("truss roof cases should stay on the 2d truss workload"),
        }
    }

    #[test]
    fn catalog_spec_round_trips_as_json() {
        let spec = default_catalog_spec();
        let json = serde_json::to_string_pretty(&spec).expect("catalog spec should serialize");
        let restored = serde_json::from_str::<crate::catalog::BenchmarkCatalogSpec>(&json)
            .expect("catalog spec should deserialize");

        assert_eq!(restored, spec);
    }

    #[test]
    fn default_catalog_spec_covers_all_profiles() {
        let spec = default_catalog_spec();

        assert_eq!(spec.templates.len(), 38);
        assert!(spec.matrices.len() >= 10);
        assert_eq!(spec.profiles.len(), 12);
        assert!(spec
            .profiles
            .iter()
            .any(|profile| profile.profile == BenchmarkProfile::HundredK));
        assert!(spec
            .profiles
            .iter()
            .any(|profile| profile.profile == BenchmarkProfile::TwoHundredK));
        assert!(spec
            .profiles
            .iter()
            .any(|profile| profile.profile == BenchmarkProfile::ThreeHundredK));
        assert!(spec
            .profiles
            .iter()
            .any(|profile| profile.profile == BenchmarkProfile::FourHundredK));
        assert!(spec
            .profiles
            .iter()
            .any(|profile| profile.profile == BenchmarkProfile::FiveHundredK));
        assert!(spec
            .profiles
            .iter()
            .any(|profile| profile.profile == BenchmarkProfile::OneMillion));
    }

    #[test]
    fn checked_in_catalog_file_matches_fallback_spec() {
        let json = catalog_spec_path_candidates()
            .into_iter()
            .find_map(|path| fs::read_to_string(path).ok())
            .expect("catalog json should exist");
        let file_spec =
            serde_json::from_str::<BenchmarkCatalogSpec>(&json).expect("catalog json should parse");

        assert_eq!(file_spec, default_catalog_spec());
    }

    #[test]
    fn thermal_matrix_only_emits_thermal_cases() {
        let cases = benchmark_cases(BenchmarkProfile::TenK, "thermal");

        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, "heat-plane-quad-10k");
    }

    #[test]
    fn compound_matrix_can_define_owned_templates() {
        let cases = benchmark_cases(BenchmarkProfile::TenK, "compound");

        assert!(cases
            .iter()
            .any(|case| case.id == "compound-surface-panel-10k"));
        assert!(cases.iter().any(|case| case.id == "heat-plane-quad-10k"));
    }

    #[test]
    fn report_captures_matrix_identity() {
        let cases = benchmark_cases(BenchmarkProfile::TenK, "thermal");
        let selected = cases.iter().collect::<Vec<_>>();
        let report =
            crate::runner::build_report(&selected, 1, BenchmarkProfile::TenK, "thermal", "jacobi");

        assert_eq!(report.matrix, "thermal");
        assert_eq!(report.profile, BenchmarkProfile::TenK);
    }

    #[test]
    fn standard_matrices_exist_for_v19_baselines() {
        let spec = default_catalog_spec();
        let names = spec
            .matrices
            .iter()
            .map(|matrix| matrix.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"mechanical-core"));
        assert!(names.contains(&"thermal-core"));
        assert!(names.contains(&"compound-core"));
        assert!(names.contains(&"extended-physics"));
        assert!(names.contains(&"structural-extended"));
        assert!(names.contains(&"thermal-structural"));
        assert!(names.contains(&"physics-coverage"));
    }

    #[test]
    fn benchmark_cases_follow_matrix_template_order() {
        let cases = benchmark_cases(BenchmarkProfile::Medium, "thermal-structural");
        let ids = cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "thermal-bar-medium",
                "thermal-truss-2d-medium",
                "thermal-truss-3d-medium",
                "thermal-plane-triangle-medium",
                "thermal-plane-quad-medium",
                "frame-2d-medium",
                "frame-3d-medium",
                "thermal-frame-2d-medium",
                "thermal-frame-3d-medium",
            ]
        );
    }

    #[test]
    fn extended_physics_matrix_runs_uncovered_solver_families() {
        let cases = benchmark_cases(BenchmarkProfile::Medium, "extended-physics");
        let selected = cases.iter().collect::<Vec<_>>();
        let report = crate::runner::build_report(
            &selected,
            1,
            BenchmarkProfile::Medium,
            "extended-physics",
            "jacobi",
        );

        assert_eq!(report.cases.len(), 13);
        assert!(report.cases.iter().all(|case| case.ok));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "advection_diffusion_bar_1d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "stokes_flow_plane_quad_2d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "stokes_flow_plane_triangle_2d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "magnetostatic_plane_quad_2d"));
    }

    #[test]
    fn structural_extended_matrix_runs_structural_solver_families() {
        let cases = benchmark_cases(BenchmarkProfile::Medium, "structural-extended");
        let selected = cases.iter().collect::<Vec<_>>();
        let report = crate::runner::build_report(
            &selected,
            1,
            BenchmarkProfile::Medium,
            "structural-extended",
            "jacobi",
        );

        assert_eq!(report.cases.len(), 10);
        assert!(report.cases.iter().all(|case| case.ok));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "modal_frame_3d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "solid_tetra_3d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "contact_gap_1d"));
    }

    #[test]
    fn thermal_structural_matrix_runs_coupled_solver_families() {
        let cases = benchmark_cases(BenchmarkProfile::Medium, "thermal-structural");
        let selected = cases.iter().collect::<Vec<_>>();
        let report = crate::runner::build_report(
            &selected,
            1,
            BenchmarkProfile::Medium,
            "thermal-structural",
            "jacobi",
        );

        assert_eq!(report.cases.len(), 9);
        assert!(report.cases.iter().all(|case| case.ok));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "thermal_frame_3d"));
        assert!(report.cases.iter().any(|case| case.family == "frame_3d"));
    }

    #[test]
    fn physics_coverage_matrix_runs_all_declared_builtin_templates() {
        let spec = default_catalog_spec();
        let cases = benchmark_cases(BenchmarkProfile::Medium, "physics-coverage");
        let selected = cases.iter().collect::<Vec<_>>();
        let report = crate::runner::build_report(
            &selected,
            1,
            BenchmarkProfile::Medium,
            "physics-coverage",
            "jacobi",
        );

        assert_eq!(cases.len(), spec.templates.len());
        assert_eq!(report.cases.len(), spec.templates.len());
        assert!(report.cases.iter().all(|case| case.ok));
        assert!(report.cases.iter().any(|case| case.family == "frame_3d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "stokes_flow_plane_quad_2d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "stokes_flow_plane_triangle_2d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "magnetostatic_plane_quad_2d"));
        assert!(report
            .cases
            .iter()
            .any(|case| case.family == "thermal_frame_3d"));
    }

    #[test]
    fn physics_coverage_families_have_headless_workflow_solve_operators() {
        let cases = benchmark_cases(BenchmarkProfile::Medium, "physics-coverage");
        let supported = kyuubiki_engine::supported_workflow_operator_ids()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();

        let missing = cases
            .iter()
            .map(|case| {
                let operator_id = workflow_operator_id_for_family(case.family);
                (case.family, operator_id)
            })
            .filter(|(_, operator_id)| !supported.contains(operator_id.as_str()))
            .map(|(family, operator_id)| format!("{family} -> {operator_id}"))
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "physics-coverage families missing workflow solve operators: {}",
            missing.join(", ")
        );
    }

    #[test]
    fn physics_coverage_cases_run_through_headless_workflow_solve_executor() {
        let cases = benchmark_cases(BenchmarkProfile::Medium, "physics-coverage");
        let failures = cases
            .iter()
            .filter_map(|case| {
                let (operator_id, payload) =
                    crate::workflow_payloads::workflow_payload_for_case(case);
                kyuubiki_engine::run_solve_operator(operator_id, payload)
                    .err()
                    .map(|err| format!("{} via {operator_id}: {err}", case.id))
            })
            .collect::<Vec<_>>();

        assert!(
            failures.is_empty(),
            "physics-coverage cases failed through workflow solve executor: {}",
            failures.join(", ")
        );
    }

    #[test]
    fn two_hundred_k_profile_covers_standard_matrix_shapes_without_solving() {
        let matrix_cases = [
            ("mechanical-core", 5, 200_000),
            ("thermal-core", 1, 200_000),
            ("compound-core", 4, 200_000),
            ("thermal-structural", 9, 200_000),
        ];

        for (matrix, expected_count, minimum_nodes) in matrix_cases {
            let cases = benchmark_cases(BenchmarkProfile::TwoHundredK, matrix);

            assert_eq!(cases.len(), expected_count, "{matrix} case count changed");
            assert!(
                cases
                    .iter()
                    .any(|case| benchmark_shape(case).0 >= minimum_nodes),
                "{matrix} no longer includes a 200k-scale case"
            );
            assert!(
                cases.iter().all(|case| case.id.ends_with("-200k")),
                "{matrix} should keep the 200k case suffix"
            );
        }
    }

    fn workflow_operator_id_for_family(family: &str) -> String {
        match family {
            "axial_bar_1d" => "solve.bar_1d".to_string(),
            "stokes_flow_plane_quad_2d" => "solve.stokes_flow_quad_2d".to_string(),
            "stokes_flow_plane_triangle_2d" => "solve.stokes_flow_triangle_2d".to_string(),
            other => format!("solve.{other}"),
        }
    }

    fn benchmark_profile_from_manifest_name(name: &str) -> BenchmarkProfile {
        match name {
            "medium" => BenchmarkProfile::Medium,
            "large" => BenchmarkProfile::Large,
            "v2" => BenchmarkProfile::V2,
            "ten_k" | "10k" => BenchmarkProfile::TenK,
            "fifteen_k" | "15k" => BenchmarkProfile::FifteenK,
            "twenty_k" | "20k" => BenchmarkProfile::TwentyK,
            "hundred_k" | "100k" => BenchmarkProfile::HundredK,
            "two_hundred_k" | "200k" => BenchmarkProfile::TwoHundredK,
            "three_hundred_k" | "300k" => BenchmarkProfile::ThreeHundredK,
            "four_hundred_k" | "400k" => BenchmarkProfile::FourHundredK,
            "five_hundred_k" | "500k" => BenchmarkProfile::FiveHundredK,
            "one_million" | "1m" | "1000k" => BenchmarkProfile::OneMillion,
            other => panic!("unsupported benchmark profile coverage target: {other}"),
        }
    }

    #[test]
    fn one_million_profile_exposes_simple_probe_shapes_without_solving() {
        let cases = benchmark_cases(BenchmarkProfile::OneMillion, "thermal-structural");
        let case = cases
            .iter()
            .find(|case| case.id == "thermal-bar-1m")
            .expect("thermal-bar 1m case should exist");
        let (nodes, elements, dofs) = crate::runner_shape::workload_shape(&case.workload);

        assert_eq!(nodes, 1_000_001);
        assert_eq!(elements, 1_000_000);
        assert_eq!(dofs, 1_000_001);
    }

    #[test]
    fn one_million_frame_cases_use_the_profile_node_target() {
        let cases = benchmark_cases(BenchmarkProfile::OneMillion, "thermal-structural");
        for case_id in [
            "frame-2d-1m",
            "frame-3d-1m",
            "thermal-frame-2d-1m",
            "thermal-frame-3d-1m",
        ] {
            let case = cases
                .iter()
                .find(|case| case.id == case_id)
                .unwrap_or_else(|| panic!("{case_id} should exist"));
            let (nodes, elements, _) = crate::runner_shape::workload_shape(&case.workload);
            assert_eq!(nodes, 1_000_001, "{case_id} node count");
            assert_eq!(elements, 1_000_000, "{case_id} element count");
        }
    }

    #[test]
    fn one_million_chain_nonlinear_cases_use_the_profile_node_target() {
        let cases = benchmark_cases(BenchmarkProfile::OneMillion, "structural-extended");
        for case_id in ["nonlinear-spring-chain-1m", "contact-gap-chain-1m"] {
            let case = cases
                .iter()
                .find(|case| case.id == case_id)
                .unwrap_or_else(|| panic!("{case_id} should exist"));
            let (nodes, elements, dofs) = crate::runner_shape::workload_shape(&case.workload);
            assert_eq!(nodes, 1_000_001, "{case_id} node count");
            assert_eq!(elements, 1_000_000, "{case_id} element count");
            assert_eq!(dofs, 1_000_001, "{case_id} dof count");
        }
    }

    #[test]
    fn one_million_modal_frame_cases_use_sparse_single_mode_shapes() {
        let cases = benchmark_cases(BenchmarkProfile::OneMillion, "structural-extended");
        for case_id in ["modal-frame-2d-1m", "modal-frame-3d-1m"] {
            let case = cases
                .iter()
                .find(|case| case.id == case_id)
                .unwrap_or_else(|| panic!("{case_id} should exist"));
            let (nodes, elements, _) = crate::runner_shape::workload_shape(&case.workload);
            assert_eq!(nodes, 1_000_001, "{case_id} node count");
            assert_eq!(elements, 1_000_000, "{case_id} element count");
        }
    }

    #[test]
    fn three_hundred_k_profile_covers_standard_matrix_shapes_without_solving() {
        let matrix_cases = [
            ("mechanical-core", 5, 300_000),
            ("thermal-core", 1, 300_000),
            ("compound-core", 4, 300_000),
            ("thermal-structural", 9, 300_000),
        ];

        for (matrix, expected_count, minimum_nodes) in matrix_cases {
            let cases = benchmark_cases(BenchmarkProfile::ThreeHundredK, matrix);

            assert_eq!(cases.len(), expected_count, "{matrix} case count changed");
            assert!(
                cases
                    .iter()
                    .any(|case| benchmark_shape(case).0 >= minimum_nodes),
                "{matrix} no longer includes a 300k-scale case"
            );
            assert!(
                cases.iter().all(|case| case.id.ends_with("-300k")),
                "{matrix} should keep the 300k case suffix"
            );
        }
    }

    #[test]
    fn four_hundred_k_profile_covers_standard_matrix_shapes_without_solving() {
        let matrix_cases = [
            ("mechanical-core", 5, 400_000),
            ("thermal-core", 1, 400_000),
            ("compound-core", 4, 400_000),
            ("thermal-structural", 9, 400_000),
        ];

        for (matrix, expected_count, minimum_nodes) in matrix_cases {
            let cases = benchmark_cases(BenchmarkProfile::FourHundredK, matrix);

            assert_eq!(cases.len(), expected_count, "{matrix} case count changed");
            assert!(
                cases
                    .iter()
                    .any(|case| benchmark_shape(case).0 >= minimum_nodes),
                "{matrix} no longer includes a 400k-scale case"
            );
            assert!(
                cases.iter().all(|case| case.id.ends_with("-400k")),
                "{matrix} should keep the 400k case suffix"
            );
        }
    }

    #[test]
    fn five_hundred_k_profile_covers_standard_matrix_shapes_without_solving() {
        let matrix_cases = [
            ("mechanical-core", 5, 500_000),
            ("thermal-core", 1, 500_000),
            ("compound-core", 4, 500_000),
            ("thermal-structural", 9, 500_000),
        ];

        for (matrix, expected_count, minimum_nodes) in matrix_cases {
            let cases = benchmark_cases(BenchmarkProfile::FiveHundredK, matrix);

            assert_eq!(cases.len(), expected_count, "{matrix} case count changed");
            assert!(
                cases
                    .iter()
                    .any(|case| benchmark_shape(case).0 >= minimum_nodes),
                "{matrix} no longer includes a 500k-scale case"
            );
            assert!(
                cases.iter().all(|case| case.id.ends_with("-500k")),
                "{matrix} should keep the 500k case suffix"
            );
        }
    }

    #[derive(Debug, Deserialize)]
    struct ProfileCoverageManifest {
        targets: Vec<ProfileCoverageTarget>,
    }

    #[derive(Debug, Deserialize)]
    struct ProfileCoverageTarget {
        matrix: String,
        profile: String,
        expected_cases: Vec<String>,
    }

    #[test]
    fn profile_coverage_manifest_matches_generated_cases() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../config/benchmark-profile-coverage.json");
        let manifest_json = fs::read_to_string(&manifest_path)
            .expect("benchmark profile coverage manifest should exist");
        let manifest = serde_json::from_str::<ProfileCoverageManifest>(&manifest_json)
            .expect("benchmark profile coverage manifest should parse");

        assert!(
            !manifest.targets.is_empty(),
            "benchmark profile coverage manifest must keep at least one target"
        );

        for target in manifest.targets {
            let profile = benchmark_profile_from_manifest_name(&target.profile);
            let generated = benchmark_cases(profile, &target.matrix)
                .into_iter()
                .map(|case| case.id)
                .collect::<HashSet<_>>();
            let missing = target
                .expected_cases
                .iter()
                .filter(|case_id| !generated.contains(*case_id))
                .cloned()
                .collect::<Vec<_>>();

            assert!(
                missing.is_empty(),
                "{} / {} declares cases that catalog generation cannot produce: {}",
                target.matrix,
                target.profile,
                missing.join(", ")
            );
        }
    }

    #[test]
    fn headless_sdk_matrix_runs_manifest_benchmarks() {
        let cases = headless_sdk_cases();
        let selected = cases.iter().collect::<Vec<_>>();
        let report = crate::runner::build_report(
            &selected,
            2,
            BenchmarkProfile::Medium,
            "headless-sdk",
            "jacobi",
        );

        assert_eq!(report.matrix, "headless-sdk");
        assert_eq!(report.cases.len(), 2);
        assert!(report.cases.iter().all(|case| case.ok));
        assert!(report
            .cases
            .iter()
            .any(|case| case.id == "headless-action-manifest" && case.node_count >= 40));
        assert!(report
            .cases
            .iter()
            .any(|case| case.id == "direct-fem-manifest" && case.node_count >= 26));
    }

    fn benchmark_shape(case: &BenchmarkCase) -> (usize, usize, usize) {
        crate::runner_shape::workload_shape(&case.workload)
    }
}
