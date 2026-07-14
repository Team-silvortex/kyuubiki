use super::{Options, fake_summary, option_issues, stage_health_issues};

#[test]
fn stage_health_accepts_good_summary() {
    let options = Options {
        input: "tmp/x.json".to_string(),
        max_stage_share_pct: 105.0,
        self_test: false,
    };
    assert!(
        stage_health_issues(&fake_summary(99.0, 10.0, "solve_spd_matvec"), &options).is_empty()
    );
    assert!(
        stage_health_issues(&fake_summary(106.0, 10.0, "solve_spd_matvec"), &options)[0]
            .contains("stage share")
    );
}

#[test]
fn stage_options_reject_non_positive_threshold() {
    let options = Options {
        input: "tmp/x.json".to_string(),
        max_stage_share_pct: 0.0,
        self_test: false,
    };
    assert!(option_issues(&options)[0].contains("positive"));
}
