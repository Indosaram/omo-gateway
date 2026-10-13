use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::SqlitePool;

use super::Tool;
use crate::OmonError;

#[derive(Clone)]
pub struct SkillsTool {
    skills_dirs: Vec<PathBuf>,
    pool: Option<SqlitePool>,
}

impl Default for SkillsTool {
    fn default() -> Self {
        let mut dirs = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(&home).join(".hermes").join("skills"));
            dirs.push(PathBuf::from(&home).join(".omon").join("skills"));
        }
        Self {
            skills_dirs: dirs,
            pool: None,
        }
    }
}

impl SkillsTool {
    pub fn new(skills_dirs: Vec<PathBuf>) -> Self {
        Self {
            skills_dirs,
            pool: None,
        }
    }

    pub fn with_pool(mut self, pool: SqlitePool) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Resolve a single skill destination without creating anything, including at staging time.
    pub(crate) fn validated_write_path(root: &Path, name: &str) -> Result<PathBuf, OmonError> {
        if name.is_empty()
            || matches!(name, "." | "..")
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
        {
            return Err(OmonError::ToolExecution("invalid skill name".into()));
        }
        let io_error = |e| OmonError::ToolExecution(format!("invalid skill destination: {e}"));
        let absolute = std::path::absolute(root).map_err(io_error)?;
        let ancestor = absolute.ancestors().find(|p| p.exists()).ok_or_else(|| {
            OmonError::ToolExecution("skill root has no existing ancestor".into())
        })?;
        let base = ancestor.canonicalize().map_err(io_error)?.join(
            absolute
                .strip_prefix(ancestor)
                .map_err(|e| OmonError::ToolExecution(format!("invalid skill root: {e}")))?,
        );
        let mut target = base.clone();
        for component in [name, "SKILL.md"] {
            target.push(component);
            match std::fs::symlink_metadata(&target) {
                Ok(_) => {
                    target = target.canonicalize().map_err(io_error)?;
                    if !target.starts_with(&base) {
                        return Err(OmonError::ToolExecution(
                            "skill destination escapes root".into(),
                        ));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_error(e)),
            }
        }
        Ok(target)
    }

    fn find_all_skills(&self) -> Vec<(String, PathBuf)> {
        let mut results = Vec::new();
        for base in &self.skills_dirs {
            if !base.exists() {
                continue;
            }
            // read_dir order is arbitrary: within one root the shallowest, then
            // lexicographically first, path must win a name clash. Roots keep their
            // configured order, so an earlier root shadows a later one.
            let mut local = Vec::new();
            Self::scan_dir(base, &mut local);
            local.sort_by(|a, b| {
                a.1.components()
                    .count()
                    .cmp(&b.1.components().count())
                    .then_with(|| a.1.cmp(&b.1))
            });
            results.extend(local);
        }
        // Stable: equal names keep the precedence order established above.
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results.dedup_by(|a, b| a.0 == b.0);
        results
    }

    fn scan_dir(dir: &Path, acc: &mut Vec<(String, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_file = path.join("SKILL.md");
                if skill_file.exists() {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    acc.push((name, skill_file));
                } else {
                    Self::scan_dir(&path, acc);
                }
            }
        }
    }
}

#[async_trait]
impl Tool for SkillsTool {
    fn name(&self) -> &str {
        "skills"
    }

    fn description(&self) -> &str {
        "Discover, list, read, and write specialized capability skills. Actions: 'list' (shows all available skills), 'search' (find skill by keyword), 'read' (reads the complete SKILL.md guide for a specific skill), 'write' (creates or updates a skill)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "search", "read", "write"],
                    "description": "Action to perform (list, search, read, write)."
                },
                "name": {
                    "type": "string",
                    "description": "Skill name to read or write."
                },
                "content": {
                    "type": "string",
                    "description": "Skill content in Markdown format (required for write action)."
                },
                "query": {
                    "type": "string",
                    "description": "Keyword to search (required for search action)."
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, OmonError> {
        let action = args
            .get("action")
            .and_then(Value::as_str)
            .ok_or_else(|| OmonError::ToolExecution("missing 'action'".into()))?;

        let all_skills = self.find_all_skills();

        match action {
            "list" => {
                let skill_names: Vec<String> = all_skills.iter().map(|(n, _)| n.clone()).collect();
                Ok(json!({
                    "total_skills": skill_names.len(),
                    "skills": skill_names
                }))
            }
            "search" => {
                let query = args
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_lowercase();
                let matches: Vec<String> = all_skills
                    .iter()
                    .filter(|(n, _)| n.to_lowercase().contains(&query))
                    .map(|(n, _)| n.clone())
                    .collect();
                Ok(json!({
                    "query": query,
                    "count": matches.len(),
                    "matches": matches
                }))
            }
            "read" => {
                let name = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| OmonError::ToolExecution("missing 'name'".into()))?;

                let found = all_skills.iter().find(|(n, _)| n == name);
                match found {
                    Some((_, path)) => {
                        let content = std::fs::read_to_string(path).map_err(|e| {
                            OmonError::ToolExecution(format!("failed to read skill {name}: {e}"))
                        })?;
                        Ok(json!({
                            "name": name,
                            "path": path.display().to_string(),
                            "content": content
                        }))
                    }
                    None => Err(OmonError::ToolExecution(format!(
                        "skill '{name}' not found. Use action='list' to see all skills."
                    ))),
                }
            }
            "write" => {
                let name = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| OmonError::ToolExecution("missing 'name'".into()))?;
                let content = args
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| OmonError::ToolExecution("missing 'content'".into()))?;

                let approval_pool = if crate::storage::write_approval_enabled() {
                    Some(self.pool.as_ref().ok_or_else(|| {
                        OmonError::ToolExecution(
                            "write approval requires a pending-write store".into(),
                        )
                    })?)
                } else {
                    None
                };
                let target_dir = self.skills_dirs.first().cloned().unwrap_or_else(|| {
                    if let Ok(home) = std::env::var("HOME") {
                        PathBuf::from(&home).join(".omon").join("skills")
                    } else {
                        PathBuf::from(".omon").join("skills")
                    }
                });
                let skill_path = Self::validated_write_path(&target_dir, name)?;
                if let Some(pool) = approval_pool {
                    let payload = json!({
                        "name": name,
                        "content": content,
                        "skills_root": std::path::absolute(&target_dir).map_err(|error| {
                            OmonError::ToolExecution(format!("invalid skill root: {error}"))
                        })?,
                        "destination": skill_path
                    })
                    .to_string();
                    let id = crate::storage::stage_pending_write(pool, "skill", &payload).await?;
                    return Ok(json!({
                        "name": name,
                        "status": "staged",
                        "id": id,
                        "message": format!("Skill '{name}' write staged for approval with id {id}. Run /skills approve {id} to apply.")
                    }));
                }

                let skill_dir = skill_path.with_file_name("");
                std::fs::create_dir_all(&skill_dir).map_err(|e| {
                    OmonError::ToolExecution(format!("failed to create skill directory: {e}"))
                })?;
                std::fs::write(&skill_path, content).map_err(|e| {
                    OmonError::ToolExecution(format!("failed to write SKILL.md: {e}"))
                })?;
                Ok(json!({
                    "name": name,
                    "status": "saved",
                    "path": skill_path.display().to_string()
                }))
            }
            _ => Err(OmonError::ToolExecution(format!(
                "unknown skills action: {action}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadowed_skill_resolves_to_shallowest_path_in_first_root() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let deep = first.path().join("a/b/dup");
        let shallow = first.path().join("dup");
        let other_root = second.path().join("dup");
        for dir in [&deep, &shallow, &other_root] {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join("SKILL.md"), "x").unwrap();
        }
        let tool = SkillsTool::new(vec![
            first.path().to_path_buf(),
            second.path().to_path_buf(),
        ]);

        let skills = tool.find_all_skills();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].1, shallow.join("SKILL.md"));
    }

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[cfg(windows)]
    fn symlink<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
        src: P,
        dst: Q,
    ) -> std::io::Result<()> {
        if src.as_ref().is_dir() {
            match std::os::windows::fs::symlink_dir(&src, &dst) {
                Ok(()) => Ok(()),
                Err(e) if e.raw_os_error() == Some(1314) => {
                    let output = std::process::Command::new("cmd.exe")
                        .args(["/c", "mklink", "/J"])
                        .arg(dst.as_ref().as_os_str())
                        .arg(src.as_ref().as_os_str())
                        .output()?;
                    if output.status.success() {
                        Ok(())
                    } else {
                        Err(e)
                    }
                }
                Err(e) => Err(e),
            }
        } else {
            std::os::windows::fs::symlink_file(src, dst)
        }
    }

    #[test]
    fn staged_write_replays_once_to_original_destination() {
        const TEST: &str =
            "tools::skills::tests::staged_write_replays_once_to_original_destination";
        if std::env::var_os("U07_CHILD").is_none() {
            let temp = tempfile::tempdir().unwrap();
            let mut failures = Vec::new();
            for case in ["destination", "metadata"] {
                let output = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([TEST, "--exact", "--nocapture"])
                    .env("U07_CHILD", case)
                    .env("HOME", temp.path())
                    .env("HERMES_HOME", temp.path().join("custom-hermes"))
                    .env("WRITE_APPROVAL", "true")
                    .output()
                    .unwrap();
                println!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                if !output.status.success() {
                    failures.push(case);
                }
            }
            temp.close().unwrap();
            assert!(failures.is_empty(), "failed cases: {failures:?}");
            return;
        }
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            let home = PathBuf::from(std::env::var_os("HOME").unwrap());
            let root = PathBuf::from(std::env::var_os("HERMES_HOME").unwrap()).join("skills");
            let fallback = home.join(".omon/skills");
            let database = crate::storage::Database::connect(&format!("sqlite://{}", home.join("qa.db").display())).await.unwrap();
            let pool = database.pool();
            let tool = SkillsTool::new(vec![root.clone(), fallback.clone()]).with_pool(pool.clone());
            if std::env::var("U07_CHILD").unwrap() == "metadata" {
                let session = crate::SessionKey::new("discord", Some("guild6"), "channel7", Some("thread9"), "user8").with_bot_id("bot10");
                let store = crate::memory::MemoryStore::new(pool.clone());
                let staged = store.remember(&session, "metadata-sentinel", json!({"source":"qa"})).await.unwrap();
                let row: Option<(String, String, String, String)> = sqlx::query_as("SELECT guild_id, channel_id, thread_id, user_id FROM sessions WHERE session_key = ?").bind(session.storage_key()).fetch_optional(pool).await.unwrap();
                println!("surface staged memory={} session={row:?}", staged.id);
                assert_eq!(row, Some(("guild6".into(), "channel7".into(), "thread9".into(), "".into())));
                assert!(crate::storage::approve_pending_write(pool, &staged.id, None).await.unwrap().is_some());
                assert_eq!(store.search(&session, "metadata-sentinel", 10).await.unwrap().len(), 1);
                pool.close().await;
                return;
            }
            let staged = tool.execute(json!({"action":"write","name":"x","content":"u07-sentinel"})).await.unwrap();
            let id = staged["id"].as_str().unwrap();
            assert!(!root.join("x/SKILL.md").exists());
            let result = crate::storage::approve_pending_write(pool, id, None).await.unwrap();
            println!("surface result={result:?} A={} B={}", root.join("x/SKILL.md").exists(), fallback.join("x/SKILL.md").exists());
            assert!(root.join("x/SKILL.md").exists() && !fallback.join("x/SKILL.md").exists());
            assert_eq!(tool.execute(json!({"action":"read","name":"x"})).await.unwrap()["content"], "u07-sentinel");
            assert!(crate::storage::approve_pending_write(pool, id, None).await.unwrap().is_none());
            pool.close().await;
        });
    }

    #[tokio::test]
    async fn skill_write_destination_adjacent_controls() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("new-root");
        for name in ["", ".", "..", "nested/skill", "nested\\skill", "safe/"] {
            assert!(SkillsTool::validated_write_path(&root, name).is_err());
        }
        assert!(!root.exists());
        let database = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let pool = database.pool();
        let payload = json!({"name":"safe-v2.0", "content":"first"});
        let id = crate::storage::stage_pending_write(pool, "skill", &payload.to_string())
            .await
            .unwrap();
        crate::storage::approve_pending_write(pool, &id, Some(&root))
            .await
            .unwrap();
        let file = root.join("safe-v2.0/SKILL.md");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "first");
        let outside = temp.path().join("existing.md");
        std::fs::write(&outside, "unchanged").unwrap();
        std::fs::remove_file(&file).unwrap();
        if let Err(e) = symlink(&outside, &file) {
            if e.raw_os_error() == Some(1314) {
                pool.close().await;
                temp.close().unwrap();
                return;
            }
            panic!("symlink failed: {e:?}");
        }
        let id = crate::storage::stage_pending_write(pool, "skill", &payload.to_string())
            .await
            .unwrap();
        assert!(
            crate::storage::approve_pending_write(pool, &id, Some(&root))
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "unchanged");
        pool.close().await;
        temp.close().unwrap();
    }

    #[tokio::test]
    async fn staged_skill_rejects_changed_root_and_keeps_pending() {
        // Given a recorded destination beneath an existing configured root.
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&outside).unwrap();
        let destination = SkillsTool::validated_write_path(&root, "safe").unwrap();
        let database = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let pool = database.pool();
        let payload = json!({
            "name": "safe",
            "content": "root-sentinel",
            "skills_root": root,
            "destination": destination
        });
        let id = crate::storage::stage_pending_write(pool, "skill", &payload.to_string())
            .await
            .unwrap();
        std::fs::rename(&root, temp.path().join("original-root")).unwrap();
        symlink(&outside, &root).unwrap();

        // When the configured path now resolves to a different destination.
        let result = crate::storage::approve_pending_write(pool, &id, None).await;

        // Then refusal preserves intent and does not write through the new root.
        assert!(result.is_err());
        assert!(!outside.join("safe").exists());
        assert!(crate::storage::get_pending_write(pool, &id)
            .await
            .unwrap()
            .is_some());
        pool.close().await;
        temp.close().unwrap();
    }

    #[test]
    fn skill_writes_cannot_escape_or_bypass_staging() {
        const TEST: &str = "tools::skills::tests::skill_writes_cannot_escape_or_bypass_staging";
        let Ok(case) = std::env::var("U06_CASE") else {
            let mut failures = Vec::new();
            for case in [
                "direct-parent",
                "direct-absolute",
                "direct-link",
                "direct-file-link",
                "replay-parent",
                "replay-absolute",
                "replay-link",
                "replay-file-link",
                "missing-store",
                "happy",
                "stage-link",
            ] {
                let output = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([TEST, "--exact", "--nocapture"])
                    .env("U06_CASE", case)
                    .env(
                        "WRITE_APPROVAL",
                        if matches!(case, "missing-store" | "happy" | "stage-link") {
                            "true"
                        } else {
                            "false"
                        },
                    )
                    .output()
                    .unwrap();
                println!(
                    "CASE {case} exit={}\n{}{}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                if !output.status.success() {
                    failures.push(case);
                }
            }
            assert!(failures.is_empty(), "failed scenarios: {failures:?}");
            return;
        };
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            // Given private filesystem roots, environment and real SQLite.
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("skills");
            let outside = temp.path().join("outside");
            std::fs::create_dir_all(&root).unwrap();
            std::fs::create_dir_all(&outside).unwrap();
            let database = crate::storage::Database::connect("sqlite::memory:").await.unwrap();
            let pool = database.pool();
            let mut name = "safe-skill".to_string();
            if case.ends_with("parent") { name = "../outside".into(); }
            else if case.ends_with("absolute") { name = outside.display().to_string(); }
            else if case.ends_with("file-link") {
                std::fs::create_dir(root.join(&name)).unwrap();
                if let Err(e) = symlink(outside.join("SKILL.md"), root.join(&name).join("SKILL.md")) {
                    if e.raw_os_error() == Some(1314) {
                        return;
                    }
                    panic!("symlink failed: {e:?}");
                }
            } else if case.ends_with("link") {
                if let Err(e) = symlink(&outside, root.join(&name)) {
                    if e.raw_os_error() == Some(1314) {
                        return;
                    }
                    panic!("symlink failed: {e:?}");
                }
            }
            let payload = json!({"action":"write", "name":name, "content":"u06-sentinel"});
            let tool = SkillsTool::new(vec![root.clone()]);
            // When the real Tool/apply entry point receives the payload.
            if case == "happy" {
                let tool = tool.with_pool(pool.clone());
                let staged = tool.execute(payload).await.unwrap();
                assert_eq!(staged["status"], "staged");
                assert!(!root.join(&name).join("SKILL.md").exists());
                let id = staged["id"].as_str().unwrap();
                assert!(crate::storage::approve_pending_write(pool, id, Some(&root)).await.unwrap().is_some());
                assert!(crate::storage::get_pending_write(pool, id).await.unwrap().is_none());
                assert_eq!(tool.execute(json!({"action":"read", "name":name})).await.unwrap()["content"], "u06-sentinel");
                assert_eq!(tool.execute(json!({"action":"search", "query":"safe"})).await.unwrap()["count"], 1);
                assert_eq!(tool.execute(json!({"action":"list"})).await.unwrap()["total_skills"], 1);
            } else {
                let result = if case.starts_with("replay") {
                    let id = crate::storage::stage_pending_write(pool, "skill", &payload.to_string()).await.unwrap();
                    let result = crate::storage::approve_pending_write(pool, &id, Some(&root)).await.map(|_| json!(null));
                    if result.is_err() { assert!(crate::storage::get_pending_write(pool, &id).await.unwrap().is_some()); }
                    result
                } else if case == "stage-link" { tool.with_pool(pool.clone()).execute(payload).await }
                else { tool.execute(payload).await };
                // Then refusal precedes filesystem writes or approval bypass.
                let outside_exists = outside.join("SKILL.md").exists();
                println!("observable result={result:?} outside/SKILL.md={outside_exists} inside/SKILL.md={}", root.join(&name).join("SKILL.md").exists());
                assert!(result.is_err() && !outside_exists, "write must fail closed");
                if case == "missing-store" { assert!(!root.join(&name).exists()); }
            }
            pool.close().await;
            temp.close().unwrap();
        });
    }
}
