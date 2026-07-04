#[cfg(test)]
mod tests {
    use super::{BenchmarkConfig, OutputFormat, benchmark_cases, headless_sdk_cases};
    use crate::catalog::{
        BenchmarkCatalogSpec, catalog_spec_path_candidates, default_catalog_spec,
    };
    use crate::config::BenchmarkProfile;
    use crate::models::{BenchmarkCase, BenchmarkWorkload};
    use crate::runner::run_case;
    use std::fs;

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
            fail_on_median_regression_pct: None,
            fail_on_rss_regression_pct: None,
            min_baseline_median_ms: 5.0,
        };

        assert_eq!(config.repeat, 10);
        assert!(matches!(config.format, OutputFormat::Table));
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

        assert_eq!(spec.templates.len(), 36);
        assert!(spec.matrices.len() >= 10);
        assert_eq!(spec.profiles.len(), 10);
        assert!(
            spec.profiles
                .iter()
                .any(|profile| profile.profile == BenchmarkProfile::HundredK)
        );
        assert!(
            spec.profiles
                .iter()
                .any(|profile| profile.profile == BenchmarkProfile::TwoHundredK)
        );
        assert!(
            spec.profiles
                .iter()
                .any(|profile| profile.profile == BenchmarkProfile::ThreeHundredK)
        );
        assert!(
            spec.profiles
                .iter()
                .any(|profile| profile.profile == BenchmarkProfile::FourHundredK)
        );
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

        assert!(
            cases
                .iter()
                .any(|case| case.id == "compound-surface-panel-10k")
        );
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
    fn solver_preconditioner_all_expands_truss_cases() {
        let cases = benchmark_cases(BenchmarkProfile::Medium, "mechanical-core");
        let selected = cases
            .iter()
            .filter(|case| case.id == "truss-roof-medium")
            .collect::<Vec<_>>();
        let report = crate::runner::build_report(
            &selected,
            1,
            BenchmarkProfile::Medium,
            "mechanical-core",
            "all",
        );

        assert_eq!(report.cases.len(), 2);
        assert!(report.cases.iter().any(|case| case.id.ends_with("#jacobi")));
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.id.ends_with("#symmetric-gauss-seidel"))
        );
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

        assert_eq!(report.cases.len(), 12);
        assert!(report.cases.iter().all(|case| case.ok));
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "advection_diffusion_bar_1d")
        );
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "stokes_flow_plane_quad_2d")
        );
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "magnetostatic_plane_quad_2d")
        );
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

        assert_eq!(report.cases.len(), 9);
        assert!(report.cases.iter().all(|case| case.ok));
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "modal_frame_3d")
        );
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "contact_gap_1d")
        );
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
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "thermal_frame_3d")
        );
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
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "stokes_flow_plane_quad_2d")
        );
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "magnetostatic_plane_quad_2d")
        );
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.family == "thermal_frame_3d")
        );
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
            other => format!("solve.{other}"),
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
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.id == "headless-action-manifest" && case.node_count >= 40)
        );
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.id == "direct-fem-manifest" && case.node_count >= 26)
        );
    }

    fn benchmark_shape(case: &BenchmarkCase) -> (usize, usize, usize) {
        crate::runner_shape::workload_shape(&case.workload)
    }
}
