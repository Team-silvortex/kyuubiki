use super::*;
use crate::agent_lifecycle;

fn begin(owner: &DurableTransportOwner, id: &str) -> ExecutionGuard {
    owner
        .begin(
            id.into(),
            Some("durable-owner-job".into()),
            "solve_bar_1d".into(),
        )
        .unwrap()
}

fn assert_failure(guard: ExecutionGuard, expected: &str) {
    let failure = agent_lifecycle::fail_execution(
        guard,
        "unexpected_late_failure",
        "must not replace the original cause",
    );
    assert_eq!(failure.reason_code, expected);
    assert_eq!(failure.method, "solve_bar_1d");
    assert_eq!(failure.job_id.as_deref(), Some("durable-owner-job"));
}

#[test]
fn synchronous_owner_without_transport_defers_completion_until_commit() {
    let owner = DurableTransportOwner::new(None);
    let retained = DeferredTaskReceipt::default();
    let guard = begin(&owner, "sync-owner-success");
    let probe = guard.clone();
    ExecutionContext::DurableTransport(&owner, &retained).complete(guard);
    assert!(probe.result_publication_allowed());
    owner.complete();
    assert!(!probe.result_publication_allowed());
}

#[test]
fn synchronous_commit_failure_retires_the_owner_before_any_transport_reply() {
    let owner = DurableTransportOwner::new(None);
    let guard = begin(&owner, "sync-owner-commit-failed");
    owner.fail_commit();
    assert!(!guard.result_publication_allowed());
    assert_failure(guard, "durable_receipt_commit_failed");
}

#[test]
fn synchronous_commit_failure_preserves_an_existing_solver_failure() {
    let owner = DurableTransportOwner::new(None);
    let guard = begin(&owner, "sync-owner-solver-failed");
    agent_lifecycle::fail_execution(guard.clone(), "cancelled", "cancelled during computation");
    owner.fail_commit();
    assert_failure(guard, "cancelled");
}

#[test]
fn committed_response_rejection_is_failure_for_both_durable_owner_types() {
    for synchronous in [false, true] {
        let owner = DurableTransportOwner::new(None);
        let guard = begin(
            &owner,
            &format!("committed-response-rejection-{synchronous}"),
        );
        let response = serde_json::json!({"ok":false, "error":{"code":"durable_response_unrecordable", "message":"response discarded"}});
        if synchronous {
            owner.observe_receipt(&response);
        } else {
            observe_committed_response(&guard, &response);
        }
        owner.complete();
        assert!(!guard.result_publication_allowed());
        assert_failure(guard, "durable_response_unrecordable");
    }
}

#[test]
fn synchronous_unwind_retires_its_owner_without_poisoning_a_reused_request() {
    let saved = RefCell::new(None);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let owner = DurableTransportOwner::new(None);
        saved.replace(Some(begin(&owner, "sync-owner-unwind")));
        panic!("test synchronous dispatch unwind");
    }));
    assert!(outcome.is_err());
    let old = saved.take().unwrap();
    assert_failure(old.clone(), "durable_execution_aborted");
    let next = DurableTransportOwner::new(None);
    let current = begin(&next, "sync-owner-unwind");
    old.request_cancellation();
    drop(old);
    assert!(current.result_publication_allowed());
    assert!(!current.cancellation_requested());
    next.complete();
}

#[test]
fn network_owner_keeps_delivery_boundary_but_error_delivery_cannot_become_success() {
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;
    use std::time::Duration;

    for fail_commit in [false, true] {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let _client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (stream, _) = listener.accept().unwrap();
        let writer = Arc::new(agent_reply_writer::ReplyWriter::new(
            stream,
            Duration::from_secs(1),
        ));
        let owner = DurableTransportOwner::new(Some(writer.clone()));
        let guard = begin(&owner, &format!("sync-owner-network-{fail_commit}"));
        if fail_commit {
            owner.fail_commit();
        } else {
            owner.complete();
        }
        assert_eq!(guard.result_publication_allowed(), !fail_commit);
        writer.finish(Ok(())).unwrap();
        assert!(!guard.result_publication_allowed());
        if fail_commit {
            assert_failure(guard, "durable_receipt_commit_failed");
        }
    }
}
