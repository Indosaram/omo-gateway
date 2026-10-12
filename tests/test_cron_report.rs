use std::sync::Arc;

use chrono::{TimeZone, Utc};
use clap::Parser;
use omon_gateway::cron::remediate::RunReceipt;
use omon_gateway::cron::report::{self, CronArgs, CronCommand};
use omon_gateway::{CronJobSpec, CronScheduler, Database, ShellAndPayloadTaskExecutor};
use serde_json::json;

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    args: CronArgs,
}

#[tokio::test]
async fn seeded_health_collapses_ok_jobs_and_lists_problem_lane() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(db.pool()).await.unwrap();
    let scheduler = CronScheduler::new(db.pool().clone(), Arc::new(ShellAndPayloadTaskExecutor));
    for id in ["ok", "broken"] {
        scheduler
            .register_with_id(id, CronJobSpec::new("interval:1h", json!({})))
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO cron_runs(run_id,job_id,claim_token,lease_expires_at,attempt,status,started_at,completed_at) VALUES('r-ok','ok','claim-ok','2026-10-11T00:05:00Z',1,'succeeded','2026-10-11T00:00:00Z','2026-10-11T00:01:00Z')").execute(db.pool()).await.unwrap();
    sqlx::query("INSERT INTO cron_problems(job_id,kind,signature,detail,status,lane_key,created_at,updated_at) VALUES('broken','failed','broken','error','remediating','cron-fix-broken-abcd',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)").execute(db.pool()).await.unwrap();
    let args = Cli::try_parse_from(["omo-gateway", "health", "--json"]).unwrap();
    assert!(matches!(
        args.args.command,
        CronCommand::Health { json: true }
    ));
    let health = report::health(db.pool()).await.unwrap();
    assert_eq!(health.ok_jobs, 1);
    let value = serde_json::to_value(&health).unwrap();
    assert_eq!(
        value["jobs"][0]["problems"][0]["lane_key"],
        "cron-fix-broken-abcd"
    );
    assert_eq!(
        value["jobs"][1]["last_success_kst"],
        "2026-10-11 09:01:00 KST"
    );
    assert!(report::render(&health).contains("cron-fix-broken-abcd"));
}

#[test]
fn outcome_contains_native_run_id_and_machine_evidence() {
    let receipt = RunReceipt {
        job_id: "job".into(),
        run_id: "r-verified".into(),
        status: "succeeded".into(),
        output: Some("delivered=3".into()),
        last_delivery_error: None,
        error: None,
    };
    let message = report::outcome_message("job", "cause", "fix", &receipt).unwrap();
    assert!(message.contains(&receipt.run_id));
    assert!(message.contains(receipt.output.as_ref().unwrap()));
    let failed = RunReceipt {
        last_delivery_error: Some("delivery failed".into()),
        ..receipt
    };
    assert!(report::outcome_message("job", "cause", "fix", &failed).is_err());
}

#[tokio::test]
async fn empty_cron_state_omits_digest_section() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(db.pool()).await.unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap();
    assert!(report::digest_section(db.pool(), now)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn digest_consolidates_outcomes_and_ignores_old_problems() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(db.pool()).await.unwrap();
    sqlx::query("INSERT INTO cron_problems(job_id,kind,signature,detail,status,created_at,updated_at) VALUES('old','failed','old','error','fixed','2026-10-09T00:00:00Z','2026-10-09T00:00:00Z')").execute(db.pool()).await.unwrap();
    sqlx::query("INSERT INTO cron_remediation_outcomes(problem_id,job_id,message,created_at) VALUES(1,'old','run_id=verified output_count=3','2026-10-11T23:00:00Z')").execute(db.pool()).await.unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap();
    let digest = report::digest_section(db.pool(), now)
        .await
        .unwrap()
        .unwrap();
    assert!(digest.contains("run_id=verified output_count=3"));
}

#[test]
fn persona_block_exposes_health_command_sentinel() {
    assert!(include_str!("../bot_profiles/blocks/lanes.md").contains(report::PERSONA_COMMAND));
}
