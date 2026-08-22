//! Behavioural contract for Chroma's resident Emacs projection.

use chroma::{ProjectionReport, ProjectionStatus, ThemeMode, ThemeProjection, ThemeSnapshot};

fn snapshot(mode: ThemeMode, revision: u64) -> ThemeSnapshot {
    ThemeSnapshot::new(mode, revision)
}

#[test]
fn registration_rejects_a_second_live_sender_and_owner_loss_is_transient() {
    let mut projection = ThemeProjection::new(snapshot(ThemeMode::Dark, 7));
    assert_eq!(projection.register("emacs", ":1.10").expect("register"), snapshot(ThemeMode::Dark, 7));
    assert_eq!(projection.status(), ProjectionStatus::Pending);
    assert!(projection.register("emacs", ":1.11").is_err(), "a second sender cannot own the projection");

    projection.owner_disappeared(":1.10");
    assert_eq!(projection.status(), ProjectionStatus::Unavailable);
    assert_eq!(projection.register("emacs", ":1.11").expect("reconnect"), snapshot(ThemeMode::Dark, 7));
}

#[test]
fn current_acknowledgements_are_idempotent_and_stale_reports_never_regress_status() {
    let mut projection = ThemeProjection::new(snapshot(ThemeMode::Light, 3));
    projection.register("emacs", ":1.12").expect("register");
    projection.report(":1.12", ProjectionReport::applied(3)).expect("apply current");
    assert_eq!(projection.status(), ProjectionStatus::Applied { revision: 3 });

    projection.report(":1.12", ProjectionReport::failed(2, "load-failed", "old failure")).expect("ignore stale");
    projection.report(":1.12", ProjectionReport::applied(3)).expect("duplicate current");
    assert_eq!(projection.status(), ProjectionStatus::Applied { revision: 3 });
}

#[test]
fn stale_failure_reports_must_still_use_the_bounded_public_vocabulary() {
    let mut projection = ThemeProjection::new(snapshot(ThemeMode::Light, 3));
    projection.register("emacs", ":1.14").expect("register");
    projection.report(":1.14", ProjectionReport::applied(3)).expect("apply current");

    assert!(
        projection.report(":1.14", ProjectionReport::failed(2, "not-allowlisted", "old failure")).is_err(),
        "a stale report must not bypass failure-code validation"
    );
    assert!(
        projection.report(":1.14", ProjectionReport::failed(2, "load-failed", "x".repeat(241))).is_err(),
        "a stale report must not bypass the bounded-summary validation"
    );
    projection
        .report(":1.14", ProjectionReport::failed(2, "load-failed", "valid but stale"))
        .expect("a valid stale report is a typed no-op");
    assert_eq!(projection.status(), ProjectionStatus::Applied { revision: 3 });
}

#[test]
fn duplicate_current_reports_reconcile_status_in_both_directions() {
    let mut projection = ThemeProjection::new(snapshot(ThemeMode::Dark, 8));
    projection.register("emacs", ":1.15").expect("register");

    projection.report(":1.15", ProjectionReport::applied(8)).expect("apply current");
    projection
        .report(":1.15", ProjectionReport::failed(8, "application-failed", "postcondition later failed"))
        .expect("same-current failure reconciles applied state");
    assert_eq!(projection.status(), ProjectionStatus::Failed { revision: 8 });

    projection.report(":1.15", ProjectionReport::applied(8)).expect("same-current success reconciles failed state");
    assert_eq!(projection.status(), ProjectionStatus::Applied { revision: 8 });
}

#[test]
fn desired_change_enters_pending_and_accepts_only_the_plugin_failure_vocabulary() {
    let mut projection = ThemeProjection::new(snapshot(ThemeMode::Dark, 0));
    projection.register("emacs", ":1.13").expect("register");
    assert!(
        projection.replace_desired(snapshot(ThemeMode::Dark, 0)).is_none(),
        "same desired state has no new revision"
    );
    assert_eq!(projection.replace_desired(snapshot(ThemeMode::Light, 1)), Some(snapshot(ThemeMode::Light, 1)));
    assert_eq!(projection.status(), ProjectionStatus::Pending);
    assert!(projection.report(":1.13", ProjectionReport::failed(1, "unexpected", "bad")).is_err());
    projection
        .report(":1.13", ProjectionReport::failed(1, "verification-failed", "theme did not become active"))
        .expect("bounded plugin failure");
    assert_eq!(projection.status(), ProjectionStatus::Failed { revision: 1 });
}

#[test]
fn restart_has_no_live_owner_even_when_the_snapshot_was_persisted() {
    let projection = ThemeProjection::new(snapshot(ThemeMode::Light, 9));
    assert_eq!(projection.status(), ProjectionStatus::Unavailable);
}
