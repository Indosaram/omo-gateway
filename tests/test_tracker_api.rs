use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use omon_gateway::lanes::runner::lane_key;
use omon_gateway::storage::{insert_lane, Database, Lane};
use serde_json::{json, Value};
use tower::ServiceExt;

struct Harness {
    _dir: tempfile::TempDir,
    database: Database,
    app: Router,
}

impl Harness {
    async fn new() -> Self {
        // The harness must never mirror the developer machine's live omo sessions into its DB.
        std::env::set_var(omon_gateway::tracker::sessions::SYNC_ENV, "0");
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", dir.path().join("tracker.db").display());
        let database = Database::connect(&url).await.unwrap();
        let app = omon_gateway::tracker::router(database.pool().clone());
        Self {
            _dir: dir,
            database,
            app,
        }
    }

    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(path);
        let body = match body {
            Some(value) => {
                builder = builder.header("content-type", "application/json");
                Body::from(value.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .app
            .clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, value)
    }

    async fn get(&self, path: &str) -> Value {
        let (status, value) = self.call(Method::GET, path, None).await;
        assert_eq!(status, StatusCode::OK, "GET {path}: {value}");
        value
    }

    async fn send(&self, method: Method, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(method, path, Some(body)).await
    }

    async fn insert_lane(&self, key: &str, status: &str) {
        let mut lane = Lane::new(
            key,
            format!("Lane {key}"),
            lane_key("lead").storage_key(),
            "/tmp",
            "/nonexistent/brief.md",
            lane_key(key).storage_key(),
            "file /tmp/never-done",
        );
        lane.status = status.to_string();
        lane.model = Some("vendor/test-model".to_string());
        insert_lane(self.database.pool(), &lane).await.unwrap();
    }
}

fn team<'a>(bootstrap: &'a Value, key: &str) -> &'a Value {
    bootstrap["teams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|team| team["key"] == key)
        .unwrap()
}

fn state_id(bootstrap: &Value, team_id: &Value, pick: impl Fn(&Value) -> bool) -> String {
    bootstrap["states"]
        .as_array()
        .unwrap()
        .iter()
        .find(|state| &state["team_id"] == team_id && pick(state))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn bootstrap_seeds_workspace_and_agent_team() {
    let h = Harness::new().await;
    let boot = h.get("/bootstrap").await;
    let keys: Vec<&str> = boot["teams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|team| team["key"].as_str().unwrap())
        .collect();
    assert!(keys.contains(&"OMO") && keys.contains(&"AGT"), "{keys:?}");
    assert!(boot["states"].as_array().unwrap().len() >= 15);
    assert_eq!(boot["me"]["is_me"], true);
    assert_eq!(team(&boot, "AGT")["system"], true);
    let again = h.get("/bootstrap").await;
    assert_eq!(again["teams"].as_array().unwrap().len(), keys.len());
}

#[tokio::test]
async fn create_patch_and_detail_record_activity() {
    let h = Harness::new().await;
    let boot = h.get("/bootstrap").await;
    let omo = team(&boot, "OMO")["id"].clone();
    let (status, issue) = h
        .send(
            Method::POST,
            "/issues",
            json!({"team_id": omo, "title": "QA issue", "priority": 2}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{issue}");
    assert_eq!(issue["identifier"], "OMO-1");
    assert_eq!(issue["priority"], 2);

    let in_progress = state_id(&boot, &omo, |state| state["name"] == "In Progress");
    let (status, patched) = h
        .send(
            Method::PATCH,
            &format!("/issues/{}", issue["id"].as_str().unwrap()),
            json!({"state_id": in_progress}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert!(patched["started_at"].is_string());

    let detail = h.get("/issues/omo-1").await;
    let kinds: Vec<&str> = detail["activity"]
        .as_array()
        .unwrap()
        .iter()
        .map(|activity| activity["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["created", "state"]);
    assert_eq!(detail["subscriber_ids"], json!([boot["me"]["id"]]));

    let (status, error) = h
        .send(
            Method::POST,
            "/issues",
            json!({"team_id": omo, "title": "  "}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    let (status, _) = h.call(Method::GET, "/issues/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn lanes_mirror_into_agent_team_states() {
    let h = Harness::new().await;
    h.insert_lane("closed-one", "closed").await;
    h.insert_lane("working-one", "working").await;
    let boot = h.get("/bootstrap").await;
    let agt = team(&boot, "AGT")["id"].clone();
    let issues = h
        .get(&format!("/issues?team_id={}", agt.as_str().unwrap()))
        .await;
    let items = issues["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    for (key, lane_status) in [("closed-one", "closed"), ("working-one", "working")] {
        let issue = items.iter().find(|issue| issue["lane_key"] == key).unwrap();
        let expected = state_id(&boot, &agt, |state| state["lane_status"] == lane_status);
        assert_eq!(issue["state_id"], expected.as_str());
        assert_eq!(issue["lane"]["status"], lane_status);
    }
    let agent = boot["users"]
        .as_array()
        .unwrap()
        .iter()
        .find(|user| user["kind"] == "agent")
        .unwrap();
    assert_eq!(agent["name"], "test-model");

    let again = h
        .get(&format!("/issues?team_id={}", agt.as_str().unwrap()))
        .await;
    assert_eq!(again["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn lane_linked_state_changes_are_guarded() {
    let h = Harness::new().await;
    h.insert_lane("guarded", "working").await;
    let boot = h.get("/bootstrap").await;
    let agt = team(&boot, "AGT")["id"].clone();
    let issues = h
        .get(&format!("/issues?team_id={}", agt.as_str().unwrap()))
        .await;
    let issue_id = issues["items"][0]["id"].as_str().unwrap().to_string();

    let done = state_id(&boot, &agt, |state| state["lane_status"] == "closed");
    let (status, body) = h
        .send(
            Method::PATCH,
            &format!("/issues/{issue_id}"),
            json!({"state_id": done}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "lane_not_verified");

    let queued = state_id(&boot, &agt, |state| state["lane_status"] == "queued");
    let (status, body) = h
        .send(
            Method::PATCH,
            &format!("/issues/{issue_id}"),
            json!({"state_id": queued}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "illegal_lane_transition");

    let lane_status: String = sqlx::query_scalar("SELECT status FROM lanes WHERE key = 'guarded'")
        .fetch_one(h.database.pool())
        .await
        .unwrap();
    assert_eq!(lane_status, "working");

    let (status, body) = h
        .send(
            Method::PATCH,
            &format!("/issues/{issue_id}"),
            json!({"priority": 1}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["priority"], 1);

    let (status, body) = h
        .call(Method::DELETE, &format!("/issues/{issue_id}"), None)
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
}

#[tokio::test]
async fn reopen_request_holds_state_only_while_pending() {
    let h = Harness::new().await;
    h.insert_lane("reopen-me", "closed").await;
    let boot = h.get("/bootstrap").await;
    let agt = team(&boot, "AGT")["id"].clone();
    let path = format!("/issues?team_id={}", agt.as_str().unwrap());
    let issue_id = h.get(&path).await["items"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let working = state_id(&boot, &agt, |state| state["lane_status"] == "working");
    let done = state_id(&boot, &agt, |state| state["lane_status"] == "closed");

    let (status, body) = h
        .send(
            Method::PATCH,
            &format!("/issues/{issue_id}"),
            json!({"state_id": working}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(h.get(&path).await["items"][0]["state_id"], working.as_str());

    sqlx::query("UPDATE lane_owner_replies SET state = 'failed' WHERE lane_key = 'reopen-me'")
        .execute(h.database.pool())
        .await
        .unwrap();
    assert_eq!(h.get(&path).await["items"][0]["state_id"], done.as_str());
}

#[tokio::test]
async fn comments_relations_views_and_search_round_trip() {
    let h = Harness::new().await;
    let boot = h.get("/bootstrap").await;
    let omo = team(&boot, "OMO")["id"].clone();
    let (_, first) = h
        .send(
            Method::POST,
            "/issues",
            json!({"team_id": omo, "title": "Searchable alpha"}),
        )
        .await;
    let (_, second) = h
        .send(
            Method::POST,
            "/issues",
            json!({"team_id": omo, "title": "Blocker"}),
        )
        .await;
    let first_id = first["id"].as_str().unwrap();
    let second_id = second["id"].as_str().unwrap();

    let (status, comment) = h
        .send(
            Method::POST,
            &format!("/issues/{first_id}/comments"),
            json!({"body": "hello"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{comment}");
    let (_, reacted) = h
        .send(
            Method::POST,
            &format!("/comments/{}/reactions", comment["id"].as_str().unwrap()),
            json!({"emoji": "👍"}),
        )
        .await;
    assert_eq!(
        reacted["reactions"][0]["user_ids"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let (status, relation) = h
        .send(
            Method::POST,
            &format!("/issues/{first_id}/relations"),
            json!({"type": "blocks", "related_issue_id": second_id}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{relation}");
    assert_eq!(relation["direction"], "blocks");
    let blocked = h.get(&format!("/issues/{second_id}")).await;
    assert_eq!(blocked["relations"][0]["direction"], "blocked_by");

    let (status, view) = h
        .send(
            Method::POST,
            "/views",
            json!({"name": "High", "filters": {"priorities": [2]}, "display": {"layout": "board"}}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    assert_eq!(view["filters"]["priorities"], json!([2]));
    let boot = h.get("/bootstrap").await;
    assert_eq!(boot["views"][0]["display"]["layout"], "board");

    let found = h.get("/search?q=alpha").await;
    let ids: Vec<&str> = found["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| issue["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [first_id]);
}
