pub mod discord;
pub mod github;
pub mod imessage;
pub mod kakao;
pub mod mail;
pub mod slack;

use std::ffi::OsStr;
use std::process::Stdio;

use tokio::process::Command;

use crate::{OmonError, Result};

pub fn decode_structured_output(bytes: &[u8], program: &str) -> Result<serde_json::Value> {
    if bytes.is_empty() || bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(serde_json::Value::Array(Vec::new()));
    }
    if let Ok(value) = serde_json::from_slice(bytes) {
        return Ok(value);
    }
    let text = String::from_utf8_lossy(bytes);
    let mut ndjson_items = Vec::new();
    let mut is_ndjson = true;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Ok(item) = serde_json::from_str::<serde_json::Value>(line) {
            ndjson_items.push(item);
        } else {
            is_ndjson = false;
            break;
        }
    }
    if is_ndjson && !ndjson_items.is_empty() {
        return Ok(serde_json::Value::Array(ndjson_items));
    }
    serde_yaml::from_slice::<serde_json::Value>(bytes).map_err(|error| {
        OmonError::ToolExecution(format!(
            "{program} returned invalid structured output: {error}"
        ))
    })
}

pub(super) async fn run_json<I, S>(program: &str, args: I) -> Result<serde_json::Value>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|error| OmonError::ToolExecution(format!("{program} could not start: {error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.to_lowercase().contains("token expired") {
            return Err(OmonError::ToolExecution(
                "auth_expired: mail credentials expired".into(),
            ));
        }
        return Err(OmonError::ToolExecution(format!(
            "{program} exited {}: {}",
            output.status,
            stderr.trim()
        )));
    }
    decode_structured_output(&output.stdout, program)
}

pub(super) fn array<'a>(
    value: &'a serde_json::Value,
    context: &str,
) -> Result<&'a [serde_json::Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| OmonError::ToolExecution(format!("{context} response must be a JSON array")))
}

pub(super) fn required_string<'a>(
    value: &'a serde_json::Value,
    key: &str,
    context: &str,
) -> Result<&'a str> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            OmonError::ToolExecution(format!("{context} response missing string field {key}"))
        })
}

pub(super) fn optional_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    })
}

#[cfg(test)]
mod run_json_tests {
    use super::*;
    use std::time::Duration;

    /// Spawns a child that reports its pid through a FIFO and then sleeps for
    /// 300s, drops the `run_json` future (as the 60s timeout in `collect_all`
    /// does), and asserts the child exits promptly instead of running on.
    #[tokio::test]
    async fn dropped_run_json_future_kills_hung_child() {
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("pid.fifo");
        let c_path = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
        let script = format!("echo $$ > '{}'; exec sleep 300", fifo.display());
        let mut fut = Box::pin(run_json("sh", ["-c", script.as_str()]));
        let fifo_read = fifo.clone();
        let pid_task = tokio::task::spawn_blocking(move || {
            std::fs::read_to_string(fifo_read)
                .unwrap()
                .trim()
                .parse::<i32>()
                .unwrap()
        });
        let pid = tokio::select! {
            _ = &mut fut => panic!("run_json finished before the child reported its pid"),
            pid = pid_task => pid.unwrap(),
        };
        drop(fut);
        let reaped = tokio::time::timeout(
            Duration::from_secs(10),
            tokio::task::spawn_blocking(move || {
                let mut status = 0;
                unsafe { libc::waitpid(pid, &mut status, 0) }
            }),
        )
        .await;
        if reaped.is_err() {
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        assert!(
            reaped.is_ok(),
            "hung child {pid} survived dropping run_json"
        );
    }
}
