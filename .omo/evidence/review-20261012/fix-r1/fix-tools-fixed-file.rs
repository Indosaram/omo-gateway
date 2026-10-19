use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;

use super::Tool;
use crate::OmonError;

#[derive(Clone, Debug)]
pub struct FileTool {
    root: PathBuf,
    extra_roots: Vec<PathBuf>,
    max_read_bytes: usize,
    max_search_results: usize,
    require_write_approval: bool,
}

impl FileTool {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            extra_roots: Vec::new(),
            max_read_bytes: 200_000,
            max_search_results: 1000,
            require_write_approval: false,
        }
    }

    pub fn with_authorized_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.extra_roots = roots
            .into_iter()
            .filter_map(|path| match std::fs::canonicalize(&path) {
                Ok(canonical) if canonical.is_dir() => Some(canonical),
                Ok(_) => {
                    tracing::debug!(path = %path.display(), "authorized root is not a directory, skipping");
                    None
                }
                Err(err) => {
                    tracing::debug!(path = %path.display(), %err, "authorized root does not exist or failed to canonicalize, skipping");
                    None
                }
            })
            .collect();
        self
    }

    pub fn is_authorized(&self, canonical_path: &Path) -> bool {
        if let Ok(root) = self.canonical_root() {
            if canonical_path.starts_with(&root) {
                return true;
            }
        }
        self.extra_roots
            .iter()
            .any(|root| canonical_path.starts_with(root))
    }

    pub fn with_write_approval(mut self, enabled: bool) -> Self {
        self.require_write_approval = enabled;
        self
    }

    fn canonical_root(&self) -> Result<PathBuf, OmonError> {
        std::fs::canonicalize(&self.root).map_err(tool_error)
    }

    async fn read(&self, args: &Value) -> Result<Value, OmonError> {
        let path = self.checked_existing(required(args, "path")?)?;
        let metadata = std::fs::symlink_metadata(&path).map_err(tool_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(OmonError::ToolExecution(
                "read path must resolve to a regular file inside tool root".into(),
            ));
        }
        // Read bounded head/tail slices from a stat'd handle: loading the whole file first
        // would exhaust memory on a multi-gigabyte artifact long before the limit applied.
        let total_len = metadata.len();
        if total_len > self.max_read_bytes as u64 {
            use tokio::io::{AsyncReadExt, AsyncSeekExt};
            let head_len = (self.max_read_bytes * 2) / 5;
            let tail_len = self.max_read_bytes.saturating_sub(head_len);
            let mut file = fs::File::open(&path).await.map_err(tool_error)?;
            let mut head_bytes = vec![0u8; head_len];
            file.read_exact(&mut head_bytes).await.map_err(tool_error)?;
            let mut tail_bytes = vec![0u8; tail_len];
            file.seek(std::io::SeekFrom::Start(
                total_len.saturating_sub(tail_len as u64),
            ))
            .await
            .map_err(tool_error)?;
            file.read_exact(&mut tail_bytes).await.map_err(tool_error)?;
            let omitted = total_len.saturating_sub((head_bytes.len() + tail_bytes.len()) as u64);
            let text = format!(
                "{}\n\n... [file truncated: {} bytes omitted] ...\n\n{}",
                String::from_utf8_lossy(&head_bytes),
                omitted,
                String::from_utf8_lossy(&tail_bytes)
            );
            return Ok(json!({"content": text, "truncated": true}));
        }
        let bytes = fs::read(path).await.map_err(tool_error)?;
        let content = String::from_utf8(bytes)
            .map_err(|_| OmonError::ToolExecution("file is not valid UTF-8".into()))?;
        Ok(json!({"content": content}))
    }

    async fn write(&self, args: &Value) -> Result<Value, OmonError> {
        let path_arg = required(args, "path")?;
        let content = required(args, "content")?;
        let root = self.canonical_root()?;
        let path = if Path::new(path_arg).is_absolute() {
            PathBuf::from(path_arg)
        } else {
            root.join(path_arg)
        };
        let parent = path
            .parent()
            .ok_or_else(|| OmonError::ToolExecution("invalid write path".into()))?;

        let created_parent = self.create_checked_directories(parent).await?;
        let final_file_name = path
            .file_name()
            .ok_or_else(|| OmonError::ToolExecution("invalid write path".into()))?;
        let target = created_parent.join(final_file_name);

        match std::fs::symlink_metadata(&target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(OmonError::ToolExecution(
                    "refusing to write through a symbolic link".into(),
                ));
            }
            Ok(metadata) if metadata.is_dir() => {
                return Err(OmonError::ToolExecution(
                    "write path resolves to a directory".into(),
                ));
            }
            Ok(metadata) => {
                self.ensure_inside_root(&target)?;
                // A hard link makes the same inode reachable from outside the root, so a
                // write here also mutates that alias and bypasses confinement.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.nlink() > 1 {
                        return Err(OmonError::ToolExecution(
                            "refusing to write a hard-linked file".into(),
                        ));
                    }
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(tool_error(error)),
        }

        fs::write(&target, content).await.map_err(tool_error)?;
        let written = self.ensure_inside_root(&target)?;
        let metadata = std::fs::symlink_metadata(&written).map_err(tool_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(OmonError::ToolExecution(
                "written path is not a regular file inside tool root".into(),
            ));
        }
        Ok(json!({"path": relative_string(&root, &written), "bytes_written": content.len()}))
    }

    async fn create_checked_directories(&self, parent: &Path) -> Result<PathBuf, OmonError> {
        let mut existing_ancestor = parent.to_path_buf();
        let mut uncreated_components = Vec::new();
        while !existing_ancestor.exists() {
            if let Some(file_name) = existing_ancestor.file_name() {
                uncreated_components.push(file_name.to_os_string());
                if let Some(p) = existing_ancestor.parent() {
                    existing_ancestor = p.to_path_buf();
                } else {
                    return Err(OmonError::ToolExecution("path escapes tool root".into()));
                }
            } else {
                return Err(OmonError::ToolExecution("path escapes tool root".into()));
            }
        }
        uncreated_components.reverse();

        let canonical_ancestor = std::fs::canonicalize(&existing_ancestor).map_err(tool_error)?;
        if !self.is_authorized(&canonical_ancestor) {
            return Err(OmonError::ToolExecution("path escapes tool root".into()));
        }

        let mut current = canonical_ancestor;
        for component_name in uncreated_components {
            current.push(component_name);
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() || !metadata.is_dir() {
                        return Err(OmonError::ToolExecution(
                            "write path contains a non-directory or symbolic link".into(),
                        ));
                    }
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    fs::create_dir(&current).await.map_err(tool_error)?;
                    let metadata = std::fs::symlink_metadata(&current).map_err(tool_error)?;
                    if metadata.file_type().is_symlink() || !metadata.is_dir() {
                        return Err(OmonError::ToolExecution(
                            "created path is not a directory inside tool root".into(),
                        ));
                    }
                }
                Err(error) => return Err(tool_error(error)),
            }
            let canonical = std::fs::canonicalize(&current).map_err(tool_error)?;
            if !self.is_authorized(&canonical) {
                return Err(OmonError::ToolExecution("path escapes tool root".into()));
            }
        }
        Ok(current)
    }

    async fn list(&self, args: &Value) -> Result<Value, OmonError> {
        let path =
            self.checked_existing(args.get("path").and_then(Value::as_str).unwrap_or("."))?;
        let mut reader = fs::read_dir(&path).await.map_err(tool_error)?;
        let root = self.canonical_root()?;
        let mut entries = Vec::new();
        while let Some(entry) = reader.next_entry().await.map_err(tool_error)? {
            let metadata = entry.metadata().await.map_err(tool_error)?;
            entries.push(json!({
                "name": entry.file_name().to_string_lossy(),
                "path": relative_string(&root, &entry.path()),
                "is_dir": metadata.is_dir(),
                "size": metadata.len()
            }));
        }
        entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
        Ok(json!({"entries": entries}))
    }

    async fn search(&self, args: &Value) -> Result<Value, OmonError> {
        let query = required(args, "query")?.to_owned();
        let start =
            self.checked_existing(args.get("path").and_then(Value::as_str).unwrap_or("."))?;
        let root = self.canonical_root()?;
        let limit = self.max_search_results;
        let matches =
            tokio::task::spawn_blocking(move || search_files(&root, &start, &query, limit))
                .await
                .map_err(|error| OmonError::ToolExecution(error.to_string()))??;
        Ok(json!({"matches": matches}))
    }

    fn checked_existing(&self, value: &str) -> Result<PathBuf, OmonError> {
        let path = Path::new(value);
        let target = if path.is_absolute() {
            path.to_path_buf()
        } else {
            let root = self.canonical_root()?;
            root.join(path)
        };
        let canonical = std::fs::canonicalize(&target).map_err(tool_error)?;
        if !self.is_authorized(&canonical) {
            return Err(OmonError::ToolExecution("path escapes tool root".into()));
        }
        Ok(canonical)
    }

    fn ensure_inside_root(&self, path: &Path) -> Result<PathBuf, OmonError> {
        let canonical = std::fs::canonicalize(path).map_err(tool_error)?;
        if !self.is_authorized(&canonical) {
            return Err(OmonError::ToolExecution("path escapes tool root".into()));
        }
        Ok(canonical)
    }
}

#[async_trait]
impl Tool for FileTool {
    fn name(&self) -> &str {
        "file"
    }

    fn description(&self) -> &str {
        "Read, write, list, and search UTF-8 files inside the configured workspace"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {"enum": ["read", "write", "list", "search"]},
                "path": {"type": "string"},
                "content": {"type": "string"},
                "query": {"type": "string"}
            },
            "required": ["operation"]
        })
    }

    fn requires_approval(&self, args: &Value) -> Option<String> {
        if self.require_write_approval {
            if let Some(op) = args.get("operation").and_then(Value::as_str) {
                if op.eq_ignore_ascii_case("write") {
                    let path = args.get("path").and_then(Value::as_str).unwrap_or("file");
                    return Some(format!("destructive file write to '{path}'"));
                }
            }
        }
        None
    }

    async fn execute(&self, args: Value) -> Result<Value, OmonError> {
        match required(&args, "operation")? {
            "read" => self.read(&args).await,
            "write" => self.write(&args).await,
            "list" => self.list(&args).await,
            "search" => self.search(&args).await,
            operation => Err(OmonError::ToolExecution(format!(
                "unsupported file operation: {operation}"
            ))),
        }
    }
}

fn search_files(
    root: &Path,
    start: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<Value>, OmonError> {
    let mut pending = vec![start.to_owned()];
    let mut matches = Vec::new();
    while let Some(path) = pending.pop() {
        // Only the start path may fail the search; an unreadable or vanished entry
        // deeper in the walk is skipped.
        let is_start = path.as_path() == start;
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if is_start => return Err(tool_error(error)),
            Err(_) => continue,
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            let entries = match std::fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(error) if is_start => return Err(tool_error(error)),
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.starts_with(start) {
                    pending.push(entry_path);
                }
            }
            continue;
        }
        if metadata.len() > 1024 * 1024 {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in content.lines().enumerate() {
            if line.contains(query) {
                matches.push(json!({
                    "path": relative_string(root, &path),
                    "line": index + 1,
                    "content": line
                }));
                if matches.len() >= limit {
                    return Ok(matches);
                }
            }
        }
    }
    matches.sort_by(|left, right| {
        left["path"]
            .as_str()
            .cmp(&right["path"].as_str())
            .then_with(|| left["line"].as_u64().cmp(&right["line"].as_u64()))
    });
    Ok(matches)
}

fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str, OmonError> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| OmonError::ToolExecution(format!("missing string argument: {key}")))
}

fn relative_string(root: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(root) {
        rel.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().into_owned()
    }
}

fn tool_error(error: impl std::fmt::Display) -> OmonError {
    OmonError::ToolExecution(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_authorized_temp_layout() {
        let primary = tempfile::tempdir().unwrap();
        let extra = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();

        let primary_sub = primary.path().join("sub");
        std::fs::create_dir(&primary_sub).unwrap();
        let extra_sub = extra.path().join("sub");
        std::fs::create_dir(&extra_sub).unwrap();

        let tool =
            FileTool::new(primary.path()).with_authorized_roots(vec![extra.path().to_path_buf()]);

        assert!(tool.is_authorized(&std::fs::canonicalize(primary.path()).unwrap()));
        assert!(tool.is_authorized(&std::fs::canonicalize(&primary_sub).unwrap()));
        assert!(tool.is_authorized(&std::fs::canonicalize(extra.path()).unwrap()));
        assert!(tool.is_authorized(&std::fs::canonicalize(&extra_sub).unwrap()));
        assert!(!tool.is_authorized(&std::fs::canonicalize(outside.path()).unwrap()));
    }

    #[test]
    fn test_with_authorized_roots_skips_invalid() {
        let primary = tempfile::tempdir().unwrap();
        let valid_extra = tempfile::tempdir().unwrap();
        let file_path = primary.path().join("some_file.txt");
        std::fs::write(&file_path, "test").unwrap();
        let nonexistent = primary.path().join("does_not_exist");

        let tool = FileTool::new(primary.path()).with_authorized_roots(vec![
            valid_extra.path().to_path_buf(),
            file_path,
            nonexistent,
        ]);

        assert_eq!(tool.extra_roots.len(), 1);
        assert_eq!(
            tool.extra_roots[0],
            std::fs::canonicalize(valid_extra.path()).unwrap()
        );
    }

    #[tokio::test]
    async fn test_read_oversized_file_is_bounded() {
        use std::io::{Seek, SeekFrom, Write};

        let dir = tempfile::tempdir().unwrap();
        let mut tool = FileTool::new(dir.path());
        tool.max_read_bytes = 40;

        // A sparse file far larger than any sane allocation: reading it whole is
        // impossible, reading bounded head/tail slices is trivial.
        const SIZE: u64 = 1 << 50;
        let head = b"HEAD-0123456789X";
        let tail = b"TAIL-0123456789ABCDEFGHI";
        let path = dir.path().join("huge.bin");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(head).unwrap();
        file.set_len(SIZE).unwrap();
        file.seek(SeekFrom::Start(SIZE - tail.len() as u64))
            .unwrap();
        file.write_all(tail).unwrap();
        drop(file);

        let result = tool
            .execute(json!({"operation": "read", "path": "huge.bin"}))
            .await
            .unwrap();

        assert_eq!(result["truncated"], true);
        let content = result["content"].as_str().unwrap();
        assert_eq!(
            content,
            format!(
                "{}\n\n... [file truncated: {} bytes omitted] ...\n\n{}",
                String::from_utf8_lossy(head),
                SIZE - 40,
                String::from_utf8_lossy(tail)
            )
        );
        assert!(
            content.len() < 200,
            "content should stay bounded, got {} bytes",
            content.len()
        );
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn search_skips_unreadable_entries_instead_of_aborting() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hit.txt"), "findme\n").unwrap();
        let locked = dir.path().join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("inner.txt"), "findme\n").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let tool = FileTool::new(dir.path());
        let result = tool
            .execute(json!({"operation": "search", "query": "findme"}))
            .await;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        let result = result.expect("one unreadable directory must not abort the search");
        let paths: Vec<&str> = result["matches"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|m| m["path"].as_str())
            .collect();
        assert_eq!(paths, ["hit.txt"]);

        // A start path that cannot be walked still reports an error.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let err = tool
            .execute(json!({"operation": "search", "path": "locked", "query": "findme"}))
            .await;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(err.is_err(), "unreadable start path must error: {err:?}");
    }

    #[tokio::test]
    async fn test_write_refuses_hard_linked_file() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let tool = FileTool::new(root.path());

        let inside = root.path().join("note.txt");
        std::fs::write(&inside, "original").unwrap();
        let alias = outside.path().join("alias.txt");
        std::fs::hard_link(&inside, &alias).unwrap();

        let err = tool
            .execute(json!({
                "operation": "write",
                "path": "note.txt",
                "content": "attacker controlled"
            }))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, OmonError::ToolExecution(msg) if msg.contains("hard-linked")),
            "expected hard link refusal, got {:?}",
            err
        );
        assert_eq!(std::fs::read_to_string(&alias).unwrap(), "original");
        assert_eq!(std::fs::read_to_string(&inside).unwrap(), "original");
    }

    #[tokio::test]
    async fn test_file_relative_and_extra_root_paths() {
        let primary = tempfile::tempdir().unwrap();
        let extra = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();

        let tool =
            FileTool::new(primary.path()).with_authorized_roots(vec![extra.path().to_path_buf()]);

        // 1. Relative write/read under primary allowed
        let write_res = tool
            .execute(json!({
                "operation": "write",
                "path": "primary_dir/note.txt",
                "content": "hello primary\nfindme here\n"
            }))
            .await
            .unwrap();
        assert_eq!(write_res["path"], "primary_dir/note.txt");

        let read_res = tool
            .execute(json!({
                "operation": "read",
                "path": "primary_dir/note.txt"
            }))
            .await
            .unwrap();
        assert_eq!(read_res["content"], "hello primary\nfindme here\n");

        let list_res = tool
            .execute(json!({
                "operation": "list",
                "path": "primary_dir"
            }))
            .await
            .unwrap();
        assert_eq!(list_res["entries"][0]["name"], "note.txt");
        assert_eq!(list_res["entries"][0]["path"], "primary_dir/note.txt");

        let search_res = tool
            .execute(json!({
                "operation": "search",
                "path": "primary_dir",
                "query": "findme"
            }))
            .await
            .unwrap();
        assert_eq!(search_res["matches"][0]["path"], "primary_dir/note.txt");

        // 2. Absolute write/read under extra root allowed
        let extra_file = extra.path().join("extra_dir").join("doc.txt");
        let write_extra = tool
            .execute(json!({
                "operation": "write",
                "path": extra_file.to_str().unwrap(),
                "content": "hello extra\nfindme in extra\n"
            }))
            .await
            .unwrap();
        // Path returned for extra root should be absolute
        let canonical_extra_file = std::fs::canonicalize(&extra_file).unwrap();
        assert_eq!(write_extra["path"], canonical_extra_file.to_str().unwrap());

        let read_extra = tool
            .execute(json!({
                "operation": "read",
                "path": extra_file.to_str().unwrap()
            }))
            .await
            .unwrap();
        assert_eq!(read_extra["content"], "hello extra\nfindme in extra\n");

        let list_extra = tool
            .execute(json!({
                "operation": "list",
                "path": extra.path().join("extra_dir").to_str().unwrap()
            }))
            .await
            .unwrap();
        assert_eq!(list_extra["entries"][0]["name"], "doc.txt");
        assert_eq!(
            list_extra["entries"][0]["path"],
            canonical_extra_file.to_str().unwrap()
        );

        let search_extra = tool
            .execute(json!({
                "operation": "search",
                "path": extra.path().to_str().unwrap(),
                "query": "findme"
            }))
            .await
            .unwrap();
        assert_eq!(
            search_extra["matches"][0]["path"],
            canonical_extra_file.to_str().unwrap()
        );

        // 3. Absolute outside all roots rejected ("escapes tool root")
        let outside_file = outside.path().join("secret.txt");
        std::fs::write(&outside_file, "secret").unwrap();

        let err = tool
            .execute(json!({
                "operation": "read",
                "path": outside_file.to_str().unwrap()
            }))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, OmonError::ToolExecution(msg) if msg.contains("path escapes tool root")),
            "expected 'path escapes tool root', got {:?}",
            err
        );

        let err = tool
            .execute(json!({
                "operation": "write",
                "path": outside_file.to_str().unwrap(),
                "content": "overwrite"
            }))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, OmonError::ToolExecution(msg) if msg.contains("path escapes tool root")),
            "expected 'path escapes tool root', got {:?}",
            err
        );

        // 4. `..` landing outside all roots rejected
        let err = tool
            .execute(json!({
                "operation": "read",
                "path": "../../outside"
            }))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, OmonError::ToolExecution(msg) if msg.contains("path escapes tool root") || msg.contains("No such file") || msg.contains("os error 2")),
            "expected error for traversal, got {:?}",
            err
        );

        let err = tool
            .execute(json!({
                "operation": "write",
                "path": "../../outside.txt",
                "content": "fail"
            }))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, OmonError::ToolExecution(msg) if msg.contains("path escapes tool root")),
            "expected 'path escapes tool root', got {:?}",
            err
        );
    }
}
