use kyuubiki_protocol::{Job, JobStatus};
use kyuubiki_solver::MockSolver;

#[test]
fn progress_is_monotonic_and_finishes_completed() {
    let solver = MockSolver::new(4);
    let job = Job::new("job-1", "project-1", "case-1");

    let events = solver.solve(&job);

    assert_eq!(
        events.last().map(|event| event.stage),
        Some(JobStatus::Completed)
    );
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].progress <= pair[1].progress)
    );
}
