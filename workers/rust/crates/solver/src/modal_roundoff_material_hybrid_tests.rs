use super::*;

#[path = "modal_roundoff_material_hybrid_control_tests.rs"]
mod controls;

fn run_hybrid(input: &SolveModalFrame2dRequest) -> Record {
    run_ablation(input, false, Tail::Hybrid)
}

#[test]
fn modal_material_hybrid_full_readback_retains_legacy_outputs() {
    let mut stages = [[0; 7]; 3];
    let mut boundaries = Vec::new();
    let mut preserved = 0;
    let mut gains = 0;
    let mut compare = |input: &SolveModalFrame2dRequest| {
        let old = run(input, Chart::Original);
        let beam = run_ablation(input, false, Tail::Beam);
        let hybrid = run_hybrid(input);
        let boundary = [old.stage, beam.stage, hybrid.stage];
        for (index, record) in [&old, &beam, &hybrid].into_iter().enumerate() {
            stages[index][record.stage as usize] += 1;
            assert_eq!(record.root, old.root);
            assert_eq!(record.seed, old.seed);
            assert_eq!(record.output.is_some(), record.stage == Stage::Accepted);
            assert_eq!(record.readback.is_some(), record.output.is_some());
        }
        assert!(hybrid.factors[0] <= 2 && hybrid.factors[1] <= 1);
        assert!(hybrid.certificates[0] <= 13 && hybrid.certificates[1] <= 25);
        if let Some(output) = &old.output {
            assert_eq!(hybrid.stage, Stage::Accepted, "{hybrid:?}");
            assert_eq!(
                serde_json::to_vec(&hybrid.output).unwrap(),
                serde_json::to_vec(&old.output).unwrap()
            );
            assert_eq!(hybrid.readback, old.readback);
            assert_eq!(hybrid.factors, old.factors);
            assert_eq!(hybrid.certificates, old.certificates);
            assert_eq!(output.input, hybrid.output.as_ref().unwrap().input);
            preserved += 1;
        } else if hybrid.stage == Stage::Accepted {
            gains += 1;
        }
        boundaries.push(boundary);
        println!(
            "material hybrid comparison stages={boundary:?} beam_readback={:?} hybrid_readback={:?} hybrid_factors={:?} hybrid_certificates={:?}",
            beam.readback, hybrid.readback, hybrid.factors, hybrid.certificates
        );
        boundary
    };
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let boundary = compare(&renumber(&recipe.request(members, scale), 7));
                println!(
                    "material hybrid boundary recipe={recipe:?} members={members} scale={scale:e} stages={boundary:?}"
                );
            }
        }
    }
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let boundary = compare(&renumber(&Fixture::request(profile, 128, scale), 7));
            println!(
                "material hybrid baseline profile={profile:?} scale={scale:e} stages={boundary:?}"
            );
        }
    }
    assert_eq!(boundaries.len(), 42);
    assert_eq!(preserved, 37);
    assert_eq!(gains, 1);
    assert_eq!(stages[0], [0, 0, 4, 1, 0, 0, 37]);
    assert_eq!(stages[1], [0, 0, 4, 1, 0, 0, 37]);
    assert_eq!(stages[2], [0, 0, 4, 0, 0, 0, 38]);
    use Stage::{Accepted as A, Internal as I, Physical as P};
    let mut expected = vec![[A; 3]; 42];
    expected[6] = [A, P, A];
    expected[8] = [P, A, A];
    expected[23..27].fill([I; 3]);
    assert_eq!(boundaries, expected);
    println!(
        "material hybrid full_inputs=42 cold_routes=126 stages_old_beam_hybrid={stages:?} exact_legacy_outputs={preserved} gains={gains} losses=0 boundaries={boundaries:?} production_admission=unchanged"
    );
}
