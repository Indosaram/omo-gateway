use sqlx::SqlitePool;
use std::str::FromStr;
use thiserror::Error;

use crate::storage::lanes::transition_lane;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneStatus {
    Queued,
    Working,
    Blocked,
    DoneClaimed,
    NotDone,
    VerifiedDone,
    Closed,
    Failed,
}

impl LaneStatus {
    pub const ALL: [LaneStatus; 8] = [
        LaneStatus::Queued,
        LaneStatus::Working,
        LaneStatus::Blocked,
        LaneStatus::DoneClaimed,
        LaneStatus::NotDone,
        LaneStatus::VerifiedDone,
        LaneStatus::Closed,
        LaneStatus::Failed,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            LaneStatus::Queued => "queued",
            LaneStatus::Working => "working",
            LaneStatus::Blocked => "blocked",
            LaneStatus::DoneClaimed => "done_claimed",
            LaneStatus::NotDone => "not_done",
            LaneStatus::VerifiedDone => "verified_done",
            LaneStatus::Closed => "closed",
            LaneStatus::Failed => "failed",
        }
    }

    pub fn allowed_from(&self, reason: Option<&str>) -> &'static [LaneStatus] {
        match self {
            LaneStatus::Queued => &[],
            LaneStatus::Working => &[
                LaneStatus::Queued,
                LaneStatus::Blocked,
                LaneStatus::NotDone,
                LaneStatus::Closed,
                LaneStatus::Failed,
            ],
            LaneStatus::Blocked => &[LaneStatus::Working],
            LaneStatus::DoneClaimed => &[
                LaneStatus::Working,
                LaneStatus::Blocked,
                LaneStatus::NotDone,
            ],
            LaneStatus::NotDone => &[LaneStatus::DoneClaimed],
            LaneStatus::VerifiedDone => &[LaneStatus::DoneClaimed],
            LaneStatus::Closed => {
                if reason.is_some_and(|r| r.starts_with("forced:")) {
                    &[LaneStatus::Working, LaneStatus::VerifiedDone]
                } else {
                    &[LaneStatus::VerifiedDone]
                }
            }
            LaneStatus::Failed => &[LaneStatus::Queued, LaneStatus::Working, LaneStatus::Blocked],
        }
    }

    pub fn can_transition_to(&self, to: LaneStatus, reason: Option<&str>) -> bool {
        to.allowed_from(reason).contains(self)
    }
}

impl std::fmt::Display for LaneStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown lane status: {0}")]
pub struct UnknownLaneStatus(pub String);

impl FromStr for LaneStatus {
    type Err = UnknownLaneStatus;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "queued" => Ok(LaneStatus::Queued),
            "working" => Ok(LaneStatus::Working),
            "blocked" => Ok(LaneStatus::Blocked),
            "done_claimed" => Ok(LaneStatus::DoneClaimed),
            "not_done" => Ok(LaneStatus::NotDone),
            "verified_done" => Ok(LaneStatus::VerifiedDone),
            "closed" => Ok(LaneStatus::Closed),
            "failed" => Ok(LaneStatus::Failed),
            other => Err(UnknownLaneStatus(other.to_string())),
        }
    }
}

#[derive(Debug, Error)]
pub enum LaneTransitionError {
    #[error("illegal lane transition from {from:?} to {to:?} (reason: {reason:?})")]
    IllegalTransition {
        from: Option<LaneStatus>,
        to: LaneStatus,
        reason: Option<String>,
    },
    #[error("lane not found: {0}")]
    LaneNotFound(String),
    #[error("concurrent transition conflict for lane: {0}")]
    ConcurrentConflict(String),
    #[error("storage error: {0}")]
    Storage(#[from] crate::OmonError),
}

/// Free text must never equal a reason that generation fences or settlement
/// bookkeeping match on; a collision is prefixed so it stays distinguishable.
pub(super) fn safe_reason(text: &str) -> String {
    match text {
        "reopened" | "settlement_started" | "escalated" => format!("note: {text}"),
        _ => text.to_owned(),
    }
}

pub async fn transition(
    pool: &SqlitePool,
    key: &str,
    to: LaneStatus,
    reason: Option<&str>,
) -> std::result::Result<(), LaneTransitionError> {
    let allowed_from_set = to.allowed_from(reason);
    if allowed_from_set.is_empty() {
        let row: Option<(String,)> = sqlx::query_as("SELECT status FROM lanes WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await
            .map_err(crate::OmonError::from)?;
        let from = match row {
            Some((s,)) => s.parse().ok(),
            None => return Err(LaneTransitionError::LaneNotFound(key.to_string())),
        };
        return Err(LaneTransitionError::IllegalTransition {
            from,
            to,
            reason: reason.map(ToString::to_string),
        });
    }

    let allowed_strs: Vec<&'static str> = allowed_from_set.iter().map(|s| s.as_str()).collect();

    let applied = transition_lane(pool, key, &allowed_strs, to.as_str(), reason).await?;

    if !applied {
        let row: Option<(String,)> = sqlx::query_as("SELECT status FROM lanes WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await
            .map_err(crate::OmonError::from)?;

        match row {
            None => Err(LaneTransitionError::LaneNotFound(key.to_string())),
            Some((s,)) => {
                let from = s.parse().ok();
                Err(LaneTransitionError::IllegalTransition {
                    from,
                    to,
                    reason: reason.map(ToString::to_string),
                })
            }
        }
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::lanes::{get_lane, get_lane_history, insert_lane, Lane};
    use crate::storage::Database;

    #[test]
    fn test_status_display_and_parse_roundtrip() {
        for status in LaneStatus::ALL {
            let s = status.as_str();
            assert_eq!(status.to_string(), s);
            let parsed: LaneStatus = s.parse().expect("should parse correctly");
            assert_eq!(parsed, status);
        }
        assert!("invalid_status".parse::<LaneStatus>().is_err());
    }

    #[test]
    fn test_table_driven_all_status_pairs_allowed_rejected() {
        for from in LaneStatus::ALL {
            for to in LaneStatus::ALL {
                let ordinary_allowed = from.can_transition_to(to, None);
                let forced_allowed = from.can_transition_to(to, Some("forced: admin requested"));

                let expected_ordinary = matches!(
                    (from, to),
                    (LaneStatus::Queued, LaneStatus::Working)
                        | (LaneStatus::Queued, LaneStatus::Failed)
                        | (LaneStatus::Working, LaneStatus::Blocked)
                        | (LaneStatus::Working, LaneStatus::DoneClaimed)
                        | (LaneStatus::Working, LaneStatus::Failed)
                        | (LaneStatus::Blocked, LaneStatus::Working)
                        | (LaneStatus::Blocked, LaneStatus::DoneClaimed)
                        | (LaneStatus::Blocked, LaneStatus::Failed)
                        | (LaneStatus::DoneClaimed, LaneStatus::VerifiedDone)
                        | (LaneStatus::DoneClaimed, LaneStatus::NotDone)
                        | (LaneStatus::NotDone, LaneStatus::Working)
                        | (LaneStatus::NotDone, LaneStatus::DoneClaimed)
                        | (LaneStatus::VerifiedDone, LaneStatus::Closed)
                        | (LaneStatus::Closed, LaneStatus::Working)
                        | (LaneStatus::Failed, LaneStatus::Working)
                );

                assert_eq!(
                    ordinary_allowed, expected_ordinary,
                    "Mismatch for ordinary transition {:?} -> {:?}",
                    from, to
                );

                let expected_forced = match (from, to) {
                    (LaneStatus::Working, LaneStatus::Closed) => true,
                    _ => expected_ordinary,
                };

                assert_eq!(
                    forced_allowed, expected_forced,
                    "Mismatch for forced transition {:?} -> {:?}",
                    from, to
                );
            }
        }
    }

    #[test]
    fn test_force_reason_boundary() {
        assert!(!LaneStatus::Working.can_transition_to(LaneStatus::Closed, None));
        assert!(!LaneStatus::Working.can_transition_to(LaneStatus::Closed, Some("")));
        assert!(!LaneStatus::Working.can_transition_to(LaneStatus::Closed, Some("forced")));
        assert!(!LaneStatus::Working.can_transition_to(LaneStatus::Closed, Some("force: test")));
        assert!(
            !LaneStatus::Working.can_transition_to(LaneStatus::Closed, Some("not forced: test"))
        );
        assert!(LaneStatus::Working.can_transition_to(LaneStatus::Closed, Some("forced:")));
        assert!(LaneStatus::Working.can_transition_to(LaneStatus::Closed, Some("forced: test")));
    }

    #[tokio::test]
    async fn test_db_all_64_status_pairs_allowed_and_rejected_history_unchanged() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");
        let pool = db.pool();

        let mut allowed_count = 0;
        let mut rejected_count = 0;

        for from in LaneStatus::ALL {
            for to in LaneStatus::ALL {
                let from_slug = from.as_str().replace('_', "-");
                let to_slug = to.as_str().replace('_', "-");
                let key = format!("lane-{from_slug}-to-{to_slug}");
                let mut lane = Lane::new(
                    &key,
                    "Pair Test Lane",
                    "lead-1",
                    "/tmp",
                    "/tmp/brief.md",
                    "session-1",
                    "command [\"true\"]",
                );
                lane.status = from.as_str().to_string();
                insert_lane(pool, &lane).await.expect("insert test lane");

                let initial_history = get_lane_history(pool, &key).await.expect("history before");
                assert_eq!(initial_history.len(), 0);

                let is_allowed = from.can_transition_to(to, None);
                let res = transition(pool, &key, to, None).await;

                if is_allowed {
                    allowed_count += 1;
                    assert!(res.is_ok(), "expected Ok for {:?} -> {:?}", from, to);

                    let fetched = get_lane(pool, &key).await.expect("get lane").unwrap();
                    assert_eq!(fetched.status, to.as_str());

                    let history = get_lane_history(pool, &key).await.expect("history after");
                    assert_eq!(history.len(), 1);
                    assert_eq!(history[0].from_status.as_deref(), Some(from.as_str()));
                    assert_eq!(history[0].to_status, to.as_str());
                    assert_eq!(history[0].reason, None);
                } else {
                    rejected_count += 1;
                    match res {
                        Err(LaneTransitionError::IllegalTransition {
                            from: err_from,
                            to: err_to,
                            reason,
                        }) => {
                            assert_eq!(err_from, Some(from));
                            assert_eq!(err_to, to);
                            assert_eq!(reason, None);
                        }
                        other => panic!(
                            "expected IllegalTransition for rejected {:?} -> {:?}, got {:?}",
                            from, to, other
                        ),
                    }

                    let fetched = get_lane(pool, &key).await.expect("get lane").unwrap();
                    assert_eq!(fetched.status, from.as_str());

                    let history = get_lane_history(pool, &key).await.expect("history after");
                    assert_eq!(
                        history.len(),
                        0,
                        "history must remain 0 after rejected {:?} -> {:?}",
                        from,
                        to
                    );
                }
            }
        }

        assert_eq!(allowed_count, 15, "15 ordinary transitions allowed");
        assert_eq!(rejected_count, 49, "49 ordinary transitions rejected");
        assert_eq!(allowed_count + rejected_count, 64);
    }

    #[tokio::test]
    async fn test_db_forced_prefix_and_boundary_tests_real_sqlite() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");
        let pool = db.pool();

        let mut forced_allowed = 0;
        let mut forced_rejected = 0;
        for from in LaneStatus::ALL {
            let from_slug = from.as_str().replace('_', "-");
            let key = format!("forced-lane-{from_slug}");
            let mut lane = Lane::new(
                &key,
                "Forced Test Lane",
                "lead-1",
                "/tmp",
                "/tmp/brief.md",
                "session-1",
                "command [\"true\"]",
            );
            lane.status = from.as_str().to_string();
            insert_lane(pool, &lane).await.expect("insert lane");

            let res = transition(pool, &key, LaneStatus::Closed, Some("forced: admin close")).await;

            let expected_allowed = matches!(from, LaneStatus::Working | LaneStatus::VerifiedDone);
            if expected_allowed {
                forced_allowed += 1;
                assert!(res.is_ok(), "forced close from {:?} must succeed", from);
                let fetched = get_lane(pool, &key).await.expect("get lane").unwrap();
                assert_eq!(fetched.status, "closed");
                let history = get_lane_history(pool, &key).await.expect("history");
                assert_eq!(history.len(), 1);
                assert_eq!(history[0].to_status, "closed");
                assert_eq!(history[0].reason.as_deref(), Some("forced: admin close"));
            } else {
                forced_rejected += 1;
                assert!(
                    matches!(res, Err(LaneTransitionError::IllegalTransition { .. })),
                    "forced close from {:?} must fail with IllegalTransition",
                    from
                );
                let fetched = get_lane(pool, &key).await.expect("get lane").unwrap();
                assert_eq!(fetched.status, from.as_str());
                let history = get_lane_history(pool, &key).await.expect("history");
                assert_eq!(
                    history.len(),
                    0,
                    "history must remain unchanged for rejected forced close"
                );
            }
        }
        assert_eq!(forced_allowed, 2);
        assert_eq!(forced_rejected, 6);

        let boundary_lane_key = "boundary-working-lane";
        let mut lane = Lane::new(
            boundary_lane_key,
            "Boundary Lane",
            "lead-1",
            "/tmp",
            "/tmp/brief.md",
            "session-1",
            "command [\"true\"]",
        );
        lane.status = "working".to_string();
        insert_lane(pool, &lane)
            .await
            .expect("insert boundary lane");

        let rejected_reasons = [
            None,
            Some(""),
            Some("forced"),
            Some("force: admin close"),
            Some("not forced: admin close"),
            Some("FORCED: uppercase"),
        ];

        for reason in rejected_reasons {
            let res = transition(pool, boundary_lane_key, LaneStatus::Closed, reason).await;
            assert!(
                matches!(res, Err(LaneTransitionError::IllegalTransition { .. })),
                "reason {:?} must be rejected",
                reason
            );
            let fetched = get_lane(pool, boundary_lane_key)
                .await
                .expect("get lane")
                .unwrap();
            assert_eq!(fetched.status, "working");
            let history = get_lane_history(pool, boundary_lane_key)
                .await
                .expect("history");
            assert_eq!(history.len(), 0);
        }

        transition(pool, boundary_lane_key, LaneStatus::Closed, Some("forced:"))
            .await
            .expect("minimal 'forced:' must succeed");
        let fetched = get_lane(pool, boundary_lane_key)
            .await
            .expect("get lane")
            .unwrap();
        assert_eq!(fetched.status, "closed");
        let history = get_lane_history(pool, boundary_lane_key)
            .await
            .expect("history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].reason.as_deref(), Some("forced:"));

        transition(
            pool,
            boundary_lane_key,
            LaneStatus::Working,
            Some("reopened"),
        )
        .await
        .expect("reopen");

        transition(
            pool,
            boundary_lane_key,
            LaneStatus::Closed,
            Some("forced: with detailed note"),
        )
        .await
        .expect("forced with note must succeed");
        let history = get_lane_history(pool, boundary_lane_key)
            .await
            .expect("history");
        assert_eq!(history.len(), 3);
        assert_eq!(
            history[2].reason.as_deref(),
            Some("forced: with detailed note")
        );
    }

    #[tokio::test]
    async fn test_db_full_valid_cycle() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");
        let pool = db.pool();

        let lane = Lane::new(
            "cycle-lane",
            "Cycle Lane",
            "lead-1",
            "/tmp",
            "/tmp/brief.md",
            "session-1",
            "command [\"true\"]",
        );
        insert_lane(pool, &lane).await.expect("insert lane");

        transition(pool, "cycle-lane", LaneStatus::Working, Some("started"))
            .await
            .expect("queued -> working");

        transition(
            pool,
            "cycle-lane",
            LaneStatus::Blocked,
            Some("waiting for input"),
        )
        .await
        .expect("working -> blocked");

        transition(
            pool,
            "cycle-lane",
            LaneStatus::Working,
            Some("input provided"),
        )
        .await
        .expect("blocked -> working");

        transition(
            pool,
            "cycle-lane",
            LaneStatus::DoneClaimed,
            Some("claimed done"),
        )
        .await
        .expect("working -> done_claimed");

        transition(
            pool,
            "cycle-lane",
            LaneStatus::NotDone,
            Some("signal failed"),
        )
        .await
        .expect("done_claimed -> not_done");

        transition(
            pool,
            "cycle-lane",
            LaneStatus::Working,
            Some("resuming work"),
        )
        .await
        .expect("not_done -> working");

        transition(
            pool,
            "cycle-lane",
            LaneStatus::DoneClaimed,
            Some("claimed done again"),
        )
        .await
        .expect("working -> done_claimed");

        transition(
            pool,
            "cycle-lane",
            LaneStatus::VerifiedDone,
            Some("all signals verified"),
        )
        .await
        .expect("done_claimed -> verified_done");

        transition(pool, "cycle-lane", LaneStatus::Closed, Some("settled"))
            .await
            .expect("verified_done -> closed");

        transition(pool, "cycle-lane", LaneStatus::Working, Some("reopened"))
            .await
            .expect("closed -> working");

        transition(pool, "cycle-lane", LaneStatus::Failed, Some("crashed"))
            .await
            .expect("working -> failed");

        transition(pool, "cycle-lane", LaneStatus::Working, Some("restarted"))
            .await
            .expect("failed -> working");

        let history = get_lane_history(pool, "cycle-lane").await.expect("history");
        assert_eq!(history.len(), 12);
    }

    #[tokio::test]
    async fn test_concurrent_cas_preserves_engine() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");
        let pool = db.pool();

        let mut lane = Lane::new(
            "cas-lane",
            "CAS Lane",
            "lead-1",
            "/tmp",
            "/tmp/brief.md",
            "session-1",
            "command [\"true\"]",
        );
        lane.status = "working".to_string();
        insert_lane(pool, &lane).await.expect("insert lane");

        use std::sync::Arc;
        use tokio::sync::Barrier;

        let barrier = Arc::new(Barrier::new(2));
        let b1 = barrier.clone();
        let b2 = barrier.clone();

        let pool1 = pool.clone();
        let pool2 = pool.clone();

        let task1 = tokio::spawn(async move {
            b1.wait().await;
            transition(
                &pool1,
                "cas-lane",
                LaneStatus::Blocked,
                Some("task 1 blocked"),
            )
            .await
        });
        let task2 = tokio::spawn(async move {
            b2.wait().await;
            transition(
                &pool2,
                "cas-lane",
                LaneStatus::Blocked,
                Some("task 2 blocked"),
            )
            .await
        });

        let (res1, res2) = tokio::join!(task1, task2);
        let r1 = res1.unwrap();
        let r2 = res2.unwrap();

        let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
        let conflicts = [&r1, &r2]
            .iter()
            .filter(|r| {
                matches!(
                    r,
                    Err(LaneTransitionError::IllegalTransition { .. })
                        | Err(LaneTransitionError::ConcurrentConflict(_))
                )
            })
            .count();

        assert_eq!(
            successes, 1,
            "exactly one concurrent transition must succeed; r1={:?}, r2={:?}",
            r1, r2
        );
        assert_eq!(
            conflicts, 1,
            "exactly one concurrent transition must fail with conflict; r1={:?}, r2={:?}",
            r1, r2
        );

        let history = get_lane_history(pool, "cas-lane").await.expect("history");
        assert_eq!(
            history.len(),
            1,
            "exactly one history entry recorded across concurrent tasks"
        );
    }

    #[tokio::test]
    async fn test_concurrent_cas_distinct_targets_barrier() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");
        let pool = db.pool();

        let mut lane = Lane::new(
            "cas-distinct-lane",
            "CAS Distinct Lane",
            "lead-1",
            "/tmp",
            "/tmp/brief.md",
            "session-1",
            "command [\"true\"]",
        );
        lane.status = "working".to_string();
        insert_lane(pool, &lane).await.expect("insert lane");

        use std::sync::Arc;
        use tokio::sync::Barrier;

        let barrier = Arc::new(Barrier::new(2));
        let b1 = barrier.clone();
        let b2 = barrier.clone();

        let pool1 = pool.clone();
        let pool2 = pool.clone();

        let task1 = tokio::spawn(async move {
            b1.wait().await;
            transition(
                &pool1,
                "cas-distinct-lane",
                LaneStatus::Blocked,
                Some("task 1 blocked"),
            )
            .await
        });
        let task2 = tokio::spawn(async move {
            b2.wait().await;
            transition(
                &pool2,
                "cas-distinct-lane",
                LaneStatus::DoneClaimed,
                Some("task 2 claimed"),
            )
            .await
        });

        let (res1, res2) = tokio::join!(task1, task2);
        let r1 = res1.unwrap();
        let r2 = res2.unwrap();

        let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
        let conflicts = [&r1, &r2]
            .iter()
            .filter(|r| {
                matches!(
                    r,
                    Err(LaneTransitionError::IllegalTransition { .. })
                        | Err(LaneTransitionError::ConcurrentConflict(_))
                )
            })
            .count();

        assert_eq!(
            successes, 1,
            "exactly one concurrent transition must succeed; r1={:?}, r2={:?}",
            r1, r2
        );
        assert_eq!(
            conflicts, 1,
            "exactly one concurrent transition must fail with conflict; r1={:?}, r2={:?}",
            r1, r2
        );

        let history = get_lane_history(pool, "cas-distinct-lane")
            .await
            .expect("history");
        assert_eq!(
            history.len(),
            1,
            "exactly one history entry recorded across concurrent tasks with distinct targets"
        );
    }

    #[tokio::test]
    async fn test_manual_runtime_working_to_verified_done_error_with_history_unchanged() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");
        let pool = db.pool();

        let mut lane = Lane::new(
            "manual-qa-lane",
            "Manual QA Lane",
            "lead-1",
            "/tmp",
            "/tmp/brief.md",
            "session-1",
            "command [\"true\"]",
        );
        lane.status = "working".to_string();
        insert_lane(pool, &lane).await.expect("insert lane");

        let fetched_before = get_lane(pool, "manual-qa-lane")
            .await
            .expect("get lane")
            .unwrap();
        assert_eq!(fetched_before.status, "working");
        let history_before = get_lane_history(pool, "manual-qa-lane")
            .await
            .expect("get history");
        assert_eq!(history_before.len(), 0);

        let res = transition(pool, "manual-qa-lane", LaneStatus::VerifiedDone, None).await;
        match res {
            Err(LaneTransitionError::IllegalTransition { from, to, reason }) => {
                assert_eq!(from, Some(LaneStatus::Working));
                assert_eq!(to, LaneStatus::VerifiedDone);
                assert_eq!(reason, None);
            }
            other => panic!("expected IllegalTransition, got {:?}", other),
        }

        let fetched_after = get_lane(pool, "manual-qa-lane")
            .await
            .expect("get lane")
            .unwrap();
        assert_eq!(fetched_after.status, "working");

        let history_after = get_lane_history(pool, "manual-qa-lane")
            .await
            .expect("get history");
        assert_eq!(history_after.len(), 0);
    }

    #[tokio::test]
    async fn test_disk_sqlite_manual_runtime_working_to_verified_done_and_cycle_cleanup() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("task7_qa.db");
        let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

        let db = Database::connect(&db_url).await.expect("connect sqlite db");
        let pool = db.pool();

        let mut lane = Lane::new(
            "disk-qa-lane",
            "Disk QA Lane",
            "lead-1",
            "/tmp",
            "/tmp/brief.md",
            "session-1",
            "command [\"true\"]",
        );
        lane.status = "working".to_string();
        insert_lane(pool, &lane).await.expect("insert lane");

        let fetched_before = get_lane(pool, "disk-qa-lane")
            .await
            .expect("get lane")
            .unwrap();
        assert_eq!(fetched_before.status, "working");
        let history_before = get_lane_history(pool, "disk-qa-lane")
            .await
            .expect("get history");
        assert_eq!(history_before.len(), 0);

        let res = transition(pool, "disk-qa-lane", LaneStatus::VerifiedDone, None).await;
        match res {
            Err(LaneTransitionError::IllegalTransition { from, to, reason }) => {
                assert_eq!(from, Some(LaneStatus::Working));
                assert_eq!(to, LaneStatus::VerifiedDone);
                assert_eq!(reason, None);
            }
            other => panic!("expected IllegalTransition, got {:?}", other),
        }

        let fetched_after = get_lane(pool, "disk-qa-lane")
            .await
            .expect("get lane")
            .unwrap();
        assert_eq!(fetched_after.status, "working");
        let history_after = get_lane_history(pool, "disk-qa-lane")
            .await
            .expect("get history");
        assert_eq!(history_after.len(), 0);

        let cycle_lane = Lane::new(
            "disk-cycle-lane",
            "Disk Cycle Lane",
            "lead-1",
            "/tmp",
            "/tmp/brief.md",
            "session-1",
            "command [\"true\"]",
        );
        insert_lane(pool, &cycle_lane)
            .await
            .expect("insert cycle lane");

        let steps = [
            (LaneStatus::Working, Some("started")),
            (LaneStatus::Blocked, Some("waiting for input")),
            (LaneStatus::Working, Some("input provided")),
            (LaneStatus::DoneClaimed, Some("claimed done")),
            (LaneStatus::NotDone, Some("signal failed")),
            (LaneStatus::Working, Some("resuming work")),
            (LaneStatus::DoneClaimed, Some("claimed done again")),
            (LaneStatus::VerifiedDone, Some("all signals verified")),
            (LaneStatus::Closed, Some("settled")),
            (LaneStatus::Working, Some("reopened")),
            (LaneStatus::Failed, Some("crashed")),
            (LaneStatus::Working, Some("restarted")),
        ];

        for (target, reason) in steps {
            transition(pool, "disk-cycle-lane", target, reason)
                .await
                .unwrap_or_else(|e| panic!("failed transition to {:?}: {:?}", target, e));
        }

        let history = get_lane_history(pool, "disk-cycle-lane")
            .await
            .expect("get history");
        assert_eq!(history.len(), 12);
        let final_lane = get_lane(pool, "disk-cycle-lane")
            .await
            .expect("get lane")
            .unwrap();
        assert_eq!(final_lane.status, "working");

        drop(db);
        let path_to_clean = temp_dir.path().to_path_buf();
        temp_dir.close().expect("close tempdir");
        assert!(!path_to_clean.exists(), "temp dir successfully cleaned up");
    }

    #[tokio::test]
    async fn test_malformed_unknown_status_in_db() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");
        let pool = db.pool();

        let res = transition(pool, "missing-lane", LaneStatus::Working, None).await;
        assert!(matches!(res, Err(LaneTransitionError::LaneNotFound(_))));
    }
}
