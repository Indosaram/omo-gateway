use crate::migrate::sys::MigrationEnv;
use crate::{OmonError, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

const SCALAR_ENV_KEYS: [&str; 11] = [
    "DISCORD_ALLOWED_USERS",
    "DISCORD_ALLOWED_CHANNELS",
    "DISCORD_IGNORED_CHANNELS",
    "DISCORD_ALLOWED_ROLES",
    "DISCORD_ALLOW_ALL_USERS",
    "DISCORD_THREAD_SESSIONS_PER_USER",
    "DISCORD_THREAD_REQUIRE_MENTION",
    "DISCORD_FREE_RESPONSE_CHANNELS",
    "DISCORD_HOME_CHANNEL",
    "APPROVAL_MODE",
    "APPROVALS_DESTRUCTIVE_SLASH_CONFIRM",
];

const ACCESS_CONTROL_KEYS: [&str; 9] = [
    "DISCORD_ALLOWED_USERS",
    "DISCORD_ALLOWED_CHANNELS",
    "DISCORD_ALLOWED_ROLES",
    "DISCORD_ALLOW_ALL_USERS",
    "DISCORD_IGNORED_CHANNELS",
    "DISCORD_FREE_RESPONSE_CHANNELS",
    "DISCORD_HOME_CHANNEL",
    // The approval policy is gateway-wide; one profile's yolo mode must not disable it.
    "APPROVAL_MODE",
    "APPROVALS_DESTRUCTIVE_SLASH_CONFIRM",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigImportResult {
    pub values: BTreeMap<String, String>,
    pub diff: String,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct HermesConfig {
    #[serde(default)]
    model: HermesModel,
    #[serde(default)]
    approvals: HermesApprovals,
    #[serde(default)]
    discord: HermesDiscord,
}

#[derive(Debug, Default, Deserialize)]
struct HermesModel {
    default: Option<String>,
    name: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    #[serde(rename = "api_mode")]
    _api_mode: Option<String>,
}

impl HermesModel {
    fn effective_default(&self) -> Option<&str> {
        self.default
            .as_deref()
            .or(self.name.as_deref())
            .or(self.model.as_deref())
    }
}

#[derive(Debug, Default, Deserialize)]
struct HermesApprovals {
    mode: Option<String>,
    destructive_slash_confirm: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct HermesDiscord {
    #[serde(default = "default_true")]
    enabled: bool,
    bot_token: Option<String>,
    token: Option<String>,
    #[serde(default)]
    allowed_users: Option<Vec<String>>,
    #[serde(default)]
    allowed_channels: Option<Vec<String>>,
    #[serde(default)]
    ignored_channels: Option<Vec<String>>,
    #[serde(default)]
    allowed_roles: Option<Vec<String>>,
    #[serde(default)]
    free_response_channels: Option<Vec<String>>,
    #[serde(default)]
    home_channel: Option<String>,
}

impl Default for HermesDiscord {
    fn default() -> Self {
        Self {
            enabled: true,
            bot_token: None,
            token: None,
            allowed_users: None,
            allowed_channels: None,
            ignored_channels: None,
            allowed_roles: None,
            free_response_channels: None,
            home_channel: None,
        }
    }
}

fn default_true() -> bool {
    true
}

pub fn import_config(
    env: &dyn MigrationEnv,
    hermes_root: &Path,
    target_env: &Path,
    dry_run: bool,
) -> Result<ConfigImportResult> {
    validate_root(env, hermes_root)?;

    let root_config_path = hermes_root.join("config.yaml");
    let root_env_path = hermes_root.join(".env");
    let root_config = read_yaml(env, &root_config_path)?;
    let root_env = read_optional_env(env, &root_env_path)?;
    let profiles = read_profiles(env, hermes_root)?;

    // Parse and validate every source before touching the target. This keeps malformed input from
    // producing a partially migrated file.
    let mut profile_sources = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let profile_env = read_optional_env(env, &profile.join(".env"))?;
        let profile_config_path = profile.join("config.yaml");
        let profile_config = if env.exists(&profile_config_path) {
            Some(read_yaml(env, &profile_config_path)?)
        } else {
            None
        };
        profile_sources.push((profile_env, profile_config));
    }

    let values = map_values(&root_config, &root_env, &profile_sources);
    validate_output_values(&values)?;

    let existing = if env.exists(target_env) {
        Some(parse_env_document(
            target_env,
            &env.read_to_string(target_env)?,
        )?)
    } else {
        None
    };
    let merged_values = merged_values(existing.as_ref(), &values);
    let diff = masked_diff(
        existing.as_ref().map(|document| &document.values),
        &merged_values,
    );

    if dry_run {
        return Ok(ConfigImportResult {
            values,
            diff,
            backup_path: None,
        });
    }

    let backup_path = if env.exists(target_env) {
        let backup_path = backup_path(target_env, env.now());
        let current = env.read(target_env)?;
        Some(env.write_unique(&backup_path, &current)?)
    } else {
        None
    };

    env.write_atomic(
        target_env,
        render_merged_env(existing.as_ref(), &values).as_bytes(),
    )?;

    Ok(ConfigImportResult {
        values,
        diff,
        backup_path,
    })
}

fn validate_root(env: &dyn MigrationEnv, hermes_root: &Path) -> Result<()> {
    if !env.exists(hermes_root) {
        return Err(OmonError::Config(format!(
            "Hermes home does not exist: {}",
            hermes_root.display()
        )));
    }
    if !env.is_dir(hermes_root) {
        return Err(OmonError::Config(format!(
            "Hermes home is not a directory: {}",
            hermes_root.display()
        )));
    }
    Ok(())
}

fn read_yaml(env: &dyn MigrationEnv, path: &Path) -> Result<HermesConfig> {
    if !env.is_file(path) {
        return Err(OmonError::Config(format!(
            "Hermes config is not a file: {}",
            path.display()
        )));
    }
    let contents = env.read_to_string(path)?;
    serde_yaml::from_str(&contents).map_err(|error| {
        OmonError::Config(format!(
            "failed to parse Hermes config {}: {error}",
            path.display()
        ))
    })
}

fn read_optional_env(env: &dyn MigrationEnv, path: &Path) -> Result<BTreeMap<String, String>> {
    if !env.exists(path) {
        return Ok(BTreeMap::new());
    }
    if !env.is_file(path) {
        return Err(OmonError::Config(format!(
            "Hermes environment path is not a file: {}",
            path.display()
        )));
    }
    parse_env(path, &env.read_to_string(path)?)
}

fn read_profiles(env: &dyn MigrationEnv, hermes_root: &Path) -> Result<Vec<PathBuf>> {
    let profiles_root = hermes_root.join("profiles");
    if !env.exists(&profiles_root) {
        return Ok(Vec::new());
    }
    if !env.is_dir(&profiles_root) {
        return Err(OmonError::Config(format!(
            "Hermes profiles path is not a directory: {}",
            profiles_root.display()
        )));
    }
    let mut profiles = env
        .read_dir(&profiles_root)?
        .into_iter()
        .filter(|path| env.is_dir(path))
        .collect::<Vec<_>>();
    profiles.sort();
    Ok(profiles)
}

#[derive(Debug)]
struct EnvDocument {
    lines: Vec<EnvLine>,
    values: BTreeMap<String, String>,
}

#[derive(Debug)]
enum EnvLine {
    Raw(String),
    Assignment { key: String, raw: String },
}

fn parse_env(path: &Path, contents: &str) -> Result<BTreeMap<String, String>> {
    Ok(parse_env_document(path, contents)?.values)
}

fn parse_env_document(path: &Path, contents: &str) -> Result<EnvDocument> {
    // Limit read-ahead to one physical line to retain each logical assignment's bytes.
    struct Lines<'a> {
        contents: &'a [u8],
        offset: &'a std::cell::Cell<usize>,
    }
    impl std::io::Read for Lines<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let remaining = &self.contents[self.offset.get()..];
            let end = remaining
                .iter()
                .position(|&b| b == b'\n')
                .map_or(remaining.len(), |i| i + 1);
            let count = end.min(buf.len());
            buf[..count].copy_from_slice(&remaining[..count]);
            self.offset.set(self.offset.get() + count);
            Ok(count)
        }
    }
    let offset = std::cell::Cell::new(0);
    let mut parser = dotenvy::from_read_iter(Lines {
        contents: contents.as_bytes(),
        offset: &offset,
    });
    let mut lines = Vec::new();
    let mut values = BTreeMap::new();
    loop {
        let start = offset.get();
        let assignment = parser.next().transpose().map_err(|_| {
            // dotenvy errors contain source values; do not disclose secrets.
            OmonError::Config(format!("failed to parse environment {}", path.display()))
        })?;
        let raw = &contents[start..offset.get()];
        let Some((key, value)) = assignment else {
            lines.push(EnvLine::Raw(raw.to_string()));
            break;
        };
        let prefix = raw
            .split_inclusive('\n')
            .take_while(|line| {
                let line = line.trim();
                line.is_empty() || line.starts_with('#')
            })
            .map(str::len)
            .sum::<usize>();
        if prefix > 0 {
            lines.push(EnvLine::Raw(raw[..prefix].to_string()));
        }
        values.insert(key.clone(), value);
        lines.push(EnvLine::Assignment {
            key,
            raw: raw[prefix..].to_string(),
        });
    }
    Ok(EnvDocument { lines, values })
}

fn map_values(
    root_config: &HermesConfig,
    root_env: &BTreeMap<String, String>,
    profiles: &[(BTreeMap<String, String>, Option<HermesConfig>)],
) -> BTreeMap<String, String> {
    let mut output = BTreeMap::new();

    let default_model = root_config.model.effective_default();
    insert_nonempty(&mut output, "DEFAULT_MODEL", default_model);
    insert_nonempty(
        &mut output,
        "LLM_PROVIDER",
        root_config.model.provider.as_deref(),
    );
    let claude =
        default_model.is_some_and(|model| model.to_ascii_lowercase().starts_with("claude"));
    if claude {
        insert_nonempty(
            &mut output,
            "ANTHROPIC_BASE_URL",
            root_config.model.base_url.as_deref(),
        );
        insert_nonempty(
            &mut output,
            "ANTHROPIC_API_KEY",
            root_config.model.api_key.as_deref(),
        );
    } else {
        insert_nonempty(
            &mut output,
            "OPENAI_API_BASE",
            root_config.model.base_url.as_deref(),
        );
        insert_nonempty(
            &mut output,
            "OPENAI_API_KEY",
            root_config.model.api_key.as_deref(),
        );
    }

    let root_discord_token = if root_config.discord.enabled {
        root_env
            .get("DISCORD_BOT_TOKEN")
            .map(String::as_str)
            .or(root_config.discord.bot_token.as_deref())
            .or(root_config.discord.token.as_deref())
            .filter(|value| !value.is_empty())
    } else {
        None
    };

    let mut seen_tokens = HashSet::new();
    let mut primary_token = root_discord_token.map(ToString::to_string);
    if let Some(ref primary) = primary_token {
        seen_tokens.insert(primary.clone());
    }
    let mut extra_tokens = Vec::new();
    for (profile_env, profile_config) in profiles {
        if let Some(config) = profile_config {
            if !config.discord.enabled {
                continue;
            }
        }
        let token = profile_env
            .get("DISCORD_BOT_TOKEN")
            .map(String::as_str)
            .or_else(|| {
                profile_config.as_ref().and_then(|config| {
                    config
                        .discord
                        .bot_token
                        .as_deref()
                        .or(config.discord.token.as_deref())
                })
            });
        if let Some(token) = token.filter(|value| !value.is_empty()) {
            if primary_token.is_none() {
                primary_token = Some(token.to_string());
                seen_tokens.insert(token.to_string());
            } else if seen_tokens.insert(token.to_string()) {
                extra_tokens.push(token.to_string());
            }
        }
    }
    if let Some(ref primary) = primary_token {
        output.insert("DISCORD_BOT_TOKEN".into(), primary.clone());
    }
    if !extra_tokens.is_empty() {
        output.insert("DISCORD_BOT_TOKENS".into(), extra_tokens.join(","));
    }

    if root_config.discord.enabled {
        if let Some(ref users) = root_config.discord.allowed_users {
            if !users.is_empty() {
                output
                    .entry("DISCORD_ALLOWED_USERS".into())
                    .or_insert_with(|| users.join(","));
            }
        }
        if let Some(ref channels) = root_config.discord.allowed_channels {
            if !channels.is_empty() {
                output
                    .entry("DISCORD_ALLOWED_CHANNELS".into())
                    .or_insert_with(|| channels.join(","));
            }
        }
        if let Some(ref channels) = root_config.discord.ignored_channels {
            if !channels.is_empty() {
                output
                    .entry("DISCORD_IGNORED_CHANNELS".into())
                    .or_insert_with(|| channels.join(","));
            }
        }
        if let Some(ref roles) = root_config.discord.allowed_roles {
            if !roles.is_empty() {
                output
                    .entry("DISCORD_ALLOWED_ROLES".into())
                    .or_insert_with(|| roles.join(","));
            }
        }
        if let Some(ref free) = root_config.discord.free_response_channels {
            if !free.is_empty() {
                output
                    .entry("DISCORD_FREE_RESPONSE_CHANNELS".into())
                    .or_insert_with(|| free.join(","));
            }
        }
        if let Some(ref home) = root_config.discord.home_channel {
            if !home.trim().is_empty() {
                output
                    .entry("DISCORD_HOME_CHANNEL".into())
                    .or_insert_with(|| home.trim().to_string());
            }
        }
    }

    for key in SCALAR_ENV_KEYS {
        let root_value = if root_config.discord.enabled || !key.starts_with("DISCORD_") {
            root_env.get(key).map(String::as_str)
        } else {
            None
        };
        let profile_value = profiles
            .iter()
            .filter(|(_, cfg)| cfg.as_ref().is_none_or(|c| c.discord.enabled))
            .find_map(|(profile_env, _)| profile_env.get(key).map(String::as_str));
        if ACCESS_CONTROL_KEYS.contains(&key) {
            // Access control is never inherited from a profile, and root
            // config.yaml values already set above are never overwritten.
            if !output.contains_key(key) {
                insert_nonempty(&mut output, key, root_value);
            }
            continue;
        }
        insert_nonempty(&mut output, key, root_value.or(profile_value));
    }
    if !output.contains_key("APPROVAL_MODE") {
        insert_nonempty(
            &mut output,
            "APPROVAL_MODE",
            root_config.approvals.mode.as_deref(),
        );
    }
    if let Some(confirm) = root_config.approvals.destructive_slash_confirm {
        output
            .entry("APPROVALS_DESTRUCTIVE_SLASH_CONFIRM".into())
            .or_insert_with(|| confirm.to_string());
    }

    output
}

fn insert_nonempty(output: &mut BTreeMap<String, String>, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        output.insert(key.to_string(), value.to_string());
    }
}

fn validate_output_values(values: &BTreeMap<String, String>) -> Result<()> {
    if let Some((key, _)) = values
        .iter()
        .find(|(_, value)| value.contains(['\n', '\r']))
    {
        return Err(OmonError::Config(format!(
            "cannot write migrated environment key {key}: value contains a newline"
        )));
    }
    Ok(())
}

fn overlay_owns_tokens(overlay: &BTreeMap<String, String>) -> bool {
    overlay.contains_key("DISCORD_BOT_TOKEN") || overlay.contains_key("DISCORD_BOT_TOKENS")
}

fn merged_values(
    existing: Option<&EnvDocument>,
    overlay: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut merged = existing
        .map(|document| document.values.clone())
        .unwrap_or_default();
    // Hermes owns the bot token set only when it supplies one; otherwise the
    // target's existing credentials are left alone.
    if overlay_owns_tokens(overlay) {
        if !overlay.contains_key("DISCORD_BOT_TOKEN") {
            merged.remove("DISCORD_BOT_TOKEN");
        }
        if !overlay.contains_key("DISCORD_BOT_TOKENS") {
            merged.remove("DISCORD_BOT_TOKENS");
        }
    }
    merged.extend(overlay.clone());
    merged
}

fn render_merged_env(existing: Option<&EnvDocument>, overlay: &BTreeMap<String, String>) -> String {
    let Some(existing) = existing else {
        return render_env(overlay);
    };

    let mut rendered = String::new();
    let mut overlaid = HashSet::new();
    for line in &existing.lines {
        match line {
            EnvLine::Assignment { key, raw } => {
                if let Some(value) = overlay.get(key) {
                    rendered.push_str(key);
                    rendered.push('=');
                    render_env_value(&mut rendered, value);
                    rendered.push('\n');
                    overlaid.insert(key.as_str());
                } else if overlay_owns_tokens(overlay)
                    && (key == "DISCORD_BOT_TOKEN" || key == "DISCORD_BOT_TOKENS")
                {
                    continue;
                } else {
                    rendered.push_str(raw);
                }
            }
            EnvLine::Raw(raw) => rendered.push_str(raw),
        }
    }
    for (key, value) in overlay {
        if overlaid.contains(key.as_str()) {
            continue;
        }
        if !rendered.is_empty() && !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        rendered.push_str(key);
        rendered.push('=');
        render_env_value(&mut rendered, value);
        rendered.push('\n');
    }
    rendered
}

fn render_env(values: &BTreeMap<String, String>) -> String {
    let mut rendered = String::new();
    for (key, value) in values {
        rendered.push_str(key);
        rendered.push('=');
        render_env_value(&mut rendered, value);
        rendered.push('\n');
    }
    rendered
}

fn render_env_value(rendered: &mut String, value: &str) {
    if !value.contains(|c: char| c.is_whitespace() || matches!(c, '#' | '$' | '\'' | '"' | '\\')) {
        rendered.push_str(value);
        return;
    }
    // Double quoting preserves whitespace/comments; escape dotenvy substitution and escapes.
    rendered.push('"');
    for c in value.chars() {
        if matches!(c, '$' | '"' | '\\') {
            rendered.push('\\');
        }
        rendered.push(c);
    }
    rendered.push('"');
}

fn masked_diff(
    old_values: Option<&BTreeMap<String, String>>,
    new_values: &BTreeMap<String, String>,
) -> String {
    let empty = BTreeMap::new();
    let old_values = old_values.unwrap_or(&empty);
    let keys = old_values
        .keys()
        .chain(new_values.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut lines = Vec::new();
    for key in keys {
        match (old_values.get(&key), new_values.get(&key)) {
            (None, Some(value)) => lines.push(format!("+ {key}=*** ({} chars)", value.len())),
            (Some(value), None) => lines.push(format!("- {key}=*** ({} chars)", value.len())),
            (Some(old), Some(new)) if old != new => lines.push(format!(
                "~ {key}=*** ({} -> {} chars)",
                old.len(),
                new.len()
            )),
            (Some(value), Some(_)) => {
                lines.push(format!("= {key}=*** ({} chars, unchanged)", value.len()))
            }
            (None, None) => {}
        }
    }
    lines.join("\n")
}

fn backup_path(target_env: &Path, now: chrono::DateTime<chrono::Utc>) -> PathBuf {
    let mut path = target_env.as_os_str().to_os_string();
    path.push(format!(".bak-{}", now.format("%Y%m%dT%H%M%SZ")));
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::import_config;
    use crate::migrate::sys::{FakeMigrationEnv, MigrationEnv};
    use crate::OmonError;
    use chrono::{TimeZone, Utc};
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    struct RuntimeConfigProjection {
        discord_bot_tokens: Vec<String>,
        default_model: String,
        openai_api_base: Option<String>,
        openai_api_key: Option<String>,
    }

    impl RuntimeConfigProjection {
        // `Config` and `Config::from_env` live privately in the binary crate (`main.rs`), so lib
        // unit tests cannot name them without widening the production API or booting the binary.
        // Parse the importer's key/value map with the same token, required-key, and optional-key
        // semantics instead. This avoids process-global environment mutation in parallel tests.
        fn from_values(values: &BTreeMap<String, String>) -> crate::Result<Self> {
            let mut discord_bot_tokens = Vec::new();
            for key in ["DISCORD_BOT_TOKEN", "DISCORD_BOT_TOKENS"] {
                if let Some(tokens) = values.get(key) {
                    for token in tokens.split(',') {
                        let token = token.trim().trim_matches('"').trim_matches('\'');
                        if !token.is_empty() && !discord_bot_tokens.iter().any(|item| item == token)
                        {
                            discord_bot_tokens.push(token.to_owned());
                        }
                    }
                }
            }
            if discord_bot_tokens.is_empty() {
                return Err(OmonError::Config(
                    "missing required environment variable DISCORD_BOT_TOKEN".into(),
                ));
            }
            let default_model = values.get("DEFAULT_MODEL").cloned().ok_or_else(|| {
                OmonError::Config("missing required environment variable DEFAULT_MODEL".into())
            })?;
            Ok(Self {
                discord_bot_tokens,
                default_model,
                openai_api_base: values.get("OPENAI_API_BASE").cloned(),
                openai_api_key: values.get("OPENAI_API_KEY").cloned(),
            })
        }
    }

    fn fixture() -> FakeMigrationEnv {
        FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 14, 30, 45).unwrap())
    }

    #[test]
    fn dotenv_round_trip_special_values() {
        // Given dotenv syntax and YAML secrets that must survive the real loader.
        for secret in [
            "a # b",
            "$HOME ${HOME}",
            " both ' and \" quotes \\",
            "plain",
        ] {
            let env = fixture();
            write(
                &env,
                "/hermes/config.yaml",
                &format!(
                    "model:\n  default: gpt-4o\n  api_key: {}\n",
                    serde_json::to_string(secret).unwrap()
                ),
            );
            write(
                &env,
                "/hermes/.env",
                "export DISCORD_BOT_TOKEN='x' # comment\n",
            );
            let raw = "# untouched\r\nexport KEEP='literal $HOME # value' # keep\r\nMULTI=\"first\nsecond\"\n";
            for existing in [false, true] {
                if existing {
                    write(&env, "/gateway/.env", raw);
                }
                // When importing to a new or existing document.
                let result = import_config(
                    &env,
                    Path::new("/hermes"),
                    Path::new("/gateway/.env"),
                    false,
                )
                .unwrap();
                let rendered = env.read_to_string(Path::new("/gateway/.env")).unwrap();
                let temp = tempfile::tempdir().unwrap();
                let file = temp.path().join("fixture.env");
                std::fs::write(&file, &rendered).unwrap();
                let loaded = dotenvy::from_path_iter(&file)
                    .unwrap()
                    .collect::<std::result::Result<BTreeMap<_, _>, _>>()
                    .unwrap();
                // Then real dotenvy reload yields the original values, without env mutation.
                assert_eq!(
                    loaded.get("DISCORD_BOT_TOKEN").map(String::as_str),
                    Some("x")
                );
                assert_eq!(
                    loaded.get("OPENAI_API_KEY").map(String::as_str),
                    Some(secret)
                );
                if existing {
                    assert!(rendered.starts_with(raw));
                }
                assert_eq!(
                    result.values.get("OPENAI_API_KEY").map(String::as_str),
                    Some(secret)
                );
                println!("C04 secret={secret:?} existing={existing} reload_equal=true raw_preserved=true");
            }
        }
    }

    #[test]
    fn dotenv_raw_documents_round_trip() {
        // Given unrelated documents, including multiline values and missing final newlines.
        for raw in [
            "",
            "# comment without newline",
            "export KEEP='literal $HOME # value' # comment",
            "# prefix\r\n\r\nexport KEEP='literal $HOME # value' # keep\r\nMULTI=\"first\n# second\nthird\"\n# tail",
        ] {
            let env = fixture();
            write(&env, "/hermes/config.yaml", "{}");
            write(&env, "/gateway/.env", raw);
            // When importing an empty overlay.
            import_config(&env, Path::new("/hermes"), Path::new("/gateway/.env"), false)
                .unwrap();
            // Then every original byte, including EOF and line endings, remains intact.
            assert_eq!(env.read_to_string(Path::new("/gateway/.env")).unwrap(), raw);
            println!("C04 raw_bytes={} exact_round_trip=true", raw.len());
        }
    }

    #[test]
    fn dotenv_existing_assignments_reload_special_values() {
        // Given existing exported keys surrounded by unrelated raw bytes.
        let env = fixture();
        let secret = "a # b $HOME ${HOME} 'single' \"double\" \\";
        write(
            &env,
            "/hermes/config.yaml",
            &format!(
                "model:\n  default: gpt-4o\n  api_key: {}\n",
                serde_json::to_string(secret).unwrap()
            ),
        );
        write(
            &env,
            "/hermes/.env",
            "export DISCORD_BOT_TOKEN=\"x\\$HOME \\\\ path \\\"quote\\\"\" # comment\n",
        );
        let prefix = "# keep prefix\r\nexport KEEP='literal $HOME # value' # keep\r\n";
        let tail = "# keep tail without newline";
        write(
            &env,
            "/gateway/.env",
            &format!(
            "{prefix}export OPENAI_API_KEY='old' # replaced\nexport DISCORD_BOT_TOKEN='old'\n{tail}"
        ),
        );
        // When the real importer replaces those keys and appends DEFAULT_MODEL after the tail.
        import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();
        let rendered = env.read_to_string(Path::new("/gateway/.env")).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("fixture.env");
        std::fs::write(&file, &rendered).unwrap();
        let loaded = dotenvy::from_path_iter(&file)
            .unwrap()
            .collect::<std::result::Result<BTreeMap<_, _>, _>>()
            .unwrap();
        // Then escaping, export replacement and the EOF separator survive actual dotenv reload.
        assert_eq!(
            loaded.get("OPENAI_API_KEY").map(String::as_str),
            Some(secret)
        );
        assert_eq!(
            loaded.get("DISCORD_BOT_TOKEN").map(String::as_str),
            Some("x$HOME \\ path \"quote\"")
        );
        assert_eq!(
            loaded.get("KEEP").map(String::as_str),
            Some("literal $HOME # value")
        );
        assert_eq!(
            loaded.get("DEFAULT_MODEL").map(String::as_str),
            Some("gpt-4o")
        );
        assert!(rendered.starts_with(prefix));
        assert!(rendered.contains(&format!("\n{tail}\nDEFAULT_MODEL=")));
        println!("C04 replacement_reload_equal=true source_escapes_equal=true raw_preserved=true eof_separator=true");
    }

    fn write(env: &FakeMigrationEnv, path: &str, contents: &str) {
        env.write(Path::new(path), contents.as_bytes()).unwrap();
    }

    #[test]
    fn routes_claude_model_to_anthropic_and_maps_runtime_keys() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            "model:\n  default: Claude-3-7-Sonnet\n  provider: custom:quotio\n  base_url: https://anthropic.example/v1\n  api_key: anthropic-secret\n  api_mode: messages\napprovals:\n  mode: smart\n",
        );
        write(
            &env,
            "/hermes/.env",
            "DISCORD_BOT_TOKEN=primary\nDISCORD_ALLOWED_USERS=1,2\nDISCORD_FREE_RESPONSE_CHANNELS=3\nDISCORD_HOME_CHANNEL=4\n",
        );

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        assert_eq!(
            result.values.get("DEFAULT_MODEL").map(String::as_str),
            Some("Claude-3-7-Sonnet")
        );
        assert_eq!(
            result.values.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://anthropic.example/v1")
        );
        assert_eq!(
            result.values.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some("anthropic-secret")
        );
        assert!(!result.values.contains_key("OPENAI_API_BASE"));
        assert!(!result.values.contains_key("OPENAI_API_KEY"));
        for key in [
            "DEFAULT_MODEL",
            "DISCORD_BOT_TOKEN",
            "DISCORD_ALLOWED_USERS",
            "DISCORD_FREE_RESPONSE_CHANNELS",
            "DISCORD_HOME_CHANNEL",
            "APPROVAL_MODE",
        ] {
            assert!(
                result.values.contains_key(key),
                "Config::from_env key missing: {key}"
            );
        }
        assert!(!result.diff.contains("anthropic-secret"));
        assert!(!result.diff.contains("primary"));
        println!(
            "claude mapping keys={:?}",
            result.values.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn imported_values_round_trip_through_runtime_environment_parsing() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            "model:\n  default: gpt-5.6-luna\n  provider: custom:quotio\n  base_url: https://quotio.example/v1\n  api_key: quotio-secret\n",
        );
        write(&env, "/hermes/.env", "DISCORD_BOT_TOKEN=primary\n");

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();
        let parsed = RuntimeConfigProjection::from_values(&result.values).unwrap();
        assert_eq!(parsed.discord_bot_tokens, ["primary"]);
        assert_eq!(parsed.default_model, "gpt-5.6-luna");
        assert_eq!(
            parsed.openai_api_base.as_deref(),
            Some("https://quotio.example/v1")
        );
        assert_eq!(parsed.openai_api_key.as_deref(), Some("quotio-secret"));
        assert_eq!(
            result.values.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "DEFAULT_MODEL",
                "DISCORD_BOT_TOKEN",
                "LLM_PROVIDER",
                "OPENAI_API_BASE",
                "OPENAI_API_KEY"
            ]
        );
    }

    #[test]
    fn routes_non_claude_custom_provider_to_openai_compatible_keys() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            "model:\n  default: gpt-5.6-luna\n  provider: custom:quotio\n  base_url: https://quotio.example/v1\n  api_key: quotio-secret\n  api_mode: chat_completions\n",
        );
        write(&env, "/hermes/.env", "DISCORD_BOT_TOKEN=primary\n");

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        assert_eq!(
            result.values.get("OPENAI_API_BASE").map(String::as_str),
            Some("https://quotio.example/v1")
        );
        assert_eq!(
            result.values.get("OPENAI_API_KEY").map(String::as_str),
            Some("quotio-secret")
        );
        assert!(!result.values.contains_key("ANTHROPIC_BASE_URL"));
        println!("custom provider tolerated; openai-compatible keys emitted");
    }

    #[test]
    fn dedupes_profile_tokens_in_stable_order_and_strips_quotes() {
        let env = fixture();
        write(&env, "/hermes/config.yaml", "model:\n  default: gpt-4o\n");
        write(
            &env,
            "/hermes/.env",
            "DISCORD_BOT_TOKEN='primary'\nAPPROVAL_MODE=\"always\"\n",
        );
        write(
            &env,
            "/hermes/profiles/zeta/.env",
            "DISCORD_BOT_TOKEN=third\n",
        );
        write(
            &env,
            "/hermes/profiles/alpha/.env",
            "DISCORD_BOT_TOKEN=\"second\"\n",
        );
        write(
            &env,
            "/hermes/profiles/beta/.env",
            "DISCORD_BOT_TOKEN='primary'\n",
        );

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        assert_eq!(
            result.values.get("DISCORD_BOT_TOKEN").map(String::as_str),
            Some("primary")
        );
        assert_eq!(
            result.values.get("DISCORD_BOT_TOKENS").map(String::as_str),
            Some("second,third")
        );
        assert_eq!(
            result.values.get("APPROVAL_MODE").map(String::as_str),
            Some("always")
        );
        println!("profile token union order=primary,second,third (values masked in importer diff)");
    }

    #[test]
    fn env_parser_ignores_comments_and_uses_last_value_in_a_file() {
        let env = fixture();
        write(&env, "/hermes/config.yaml", "model:\n  default: gpt-4o\n");
        write(
            &env,
            "/hermes/.env",
            "# old token below is superseded\nDISCORD_BOT_TOKEN=old\n\nDISCORD_BOT_TOKEN=\"primary\"\n",
        );

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        assert_eq!(
            result.values.get("DISCORD_BOT_TOKEN").map(String::as_str),
            Some("primary")
        );
    }

    #[test]
    fn omits_missing_and_empty_values() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            "model:\n  default: gpt-4o\n  base_url: ''\n  api_key: null\n",
        );
        write(
            &env,
            "/hermes/.env",
            "DISCORD_BOT_TOKEN=primary\nDISCORD_ALLOWED_USERS=\n",
        );

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        assert_eq!(result.values.len(), 2);
        assert!(result.values.contains_key("DEFAULT_MODEL"));
        assert!(result.values.contains_key("DISCORD_BOT_TOKEN"));
        assert!(!result.values.contains_key("DISCORD_BOT_TOKENS"));
        assert!(!result.values.contains_key("OPENAI_API_BASE"));
        assert!(!result.values.contains_key("DISCORD_ALLOWED_USERS"));
    }

    #[test]
    fn merges_into_existing_env_and_backs_up_original() {
        let env = fixture();
        write(&env, "/hermes/config.yaml", "model:\n  default: gpt-4o\n");
        write(
            &env,
            "/hermes/.env",
            "DISCORD_BOT_TOKEN=primary\nDISCORD_ALLOWED_USERS=42\n",
        );
        write(
            &env,
            "/hermes/profiles/alpha/.env",
            "DISCORD_BOT_TOKEN=secondary\n",
        );
        write(
            &env,
            "/gateway/.env",
            "# gateway settings\nDATABASE_URL=sqlite://custom.db\nDEFAULT_MODEL=old\nOMON_WORKSPACE_ROOT=/x\n",
        );
        let writes_before = env.write_calls().len();

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        let original = "# gateway settings\nDATABASE_URL=sqlite://custom.db\nDEFAULT_MODEL=old\nOMON_WORKSPACE_ROOT=/x\n";
        let merged = "# gateway settings\nDATABASE_URL=sqlite://custom.db\nDEFAULT_MODEL=gpt-4o\nOMON_WORKSPACE_ROOT=/x\nDISCORD_ALLOWED_USERS=42\nDISCORD_BOT_TOKEN=primary\nDISCORD_BOT_TOKENS=secondary\n";
        let backup = PathBuf::from("/gateway/.env.bak-20260815T143045Z");
        assert_eq!(result.backup_path.as_deref(), Some(backup.as_path()));
        assert_eq!(env.rename_calls().len(), 1);
        assert_eq!(env.read_to_string(&backup).unwrap(), original);
        assert_eq!(
            env.read_to_string(Path::new("/gateway/.env")).unwrap(),
            merged
        );
        let writes = &env.write_calls()[writes_before..];
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[0].0, backup);
        assert_eq!(writes[0].1, original.as_bytes());
        assert_eq!(writes[1].0.parent(), Some(Path::new("/gateway")));
        assert_eq!(
            env.rename_calls()[0],
            (writes[1].0.clone(), PathBuf::from("/gateway/.env"))
        );
        assert_eq!(writes[1].1, merged.as_bytes());
        assert!(result.diff.contains("= DATABASE_URL="));
        assert!(result.diff.contains("= OMON_WORKSPACE_ROOT="));
        assert!(result.diff.contains("~ DEFAULT_MODEL="));
        assert!(result.diff.contains("+ DISCORD_BOT_TOKEN="));
        assert!(!result.diff.lines().any(|line| line.starts_with("- ")));
    }

    #[test]
    fn existing_discord_tokens_survive_when_hermes_supplies_none() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            "model:\n  default: gpt-4o\ndiscord:\n  enabled: false\n",
        );
        write(&env, "/hermes/.env", "");
        let original = "DISCORD_BOT_TOKEN=keep-me\nDISCORD_BOT_TOKENS=a,b\n";
        write(&env, "/gateway/.env", original);

        import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        let rendered = env.read_to_string(Path::new("/gateway/.env")).unwrap();
        assert!(rendered.contains("DISCORD_BOT_TOKEN=keep-me\n"));
        assert!(rendered.contains("DISCORD_BOT_TOKENS=a,b\n"));
    }

    #[test]
    fn dry_run_returns_masked_diff_and_performs_zero_writes() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            "model:\n  default: gpt-4o\n  api_key: api-secret\n",
        );
        write(&env, "/hermes/.env", "DISCORD_BOT_TOKEN=bot-secret\n");
        write(
            &env,
            "/gateway/.env",
            "DATABASE_URL=sqlite://custom.db\nDEFAULT_MODEL=old\nOMON_WORKSPACE_ROOT=/x\n",
        );
        let writes_before = env.write_calls().len();
        let renames_before = env.rename_calls().len();

        let result =
            import_config(&env, Path::new("/hermes"), Path::new("/gateway/.env"), true).unwrap();

        assert_eq!(env.write_calls().len(), writes_before);
        assert_eq!(env.rename_calls().len(), renames_before);
        assert!(result.diff.contains("DEFAULT_MODEL"));
        assert!(result.diff.contains("OPENAI_API_KEY"));
        assert!(result.diff.contains("= DATABASE_URL="));
        assert!(result.diff.contains("= OMON_WORKSPACE_ROOT="));
        assert!(!result.diff.lines().any(|line| line.starts_with("- ")));
        assert!(!result.diff.contains("api-secret"));
        assert!(!result.diff.contains("bot-secret"));
        assert!(result.backup_path.is_none());
        println!("dry-run diff:\n{}", result.diff);
    }

    #[test]
    fn malformed_yaml_is_typed_and_never_writes_target() {
        let env = fixture();
        write(&env, "/hermes/config.yaml", "model: [unterminated\n");
        write(&env, "/hermes/.env", "DISCORD_BOT_TOKEN=primary\n");
        let writes_before = env.write_calls().len();

        let error = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap_err();

        assert!(matches!(error, OmonError::Config(_)));
        assert_eq!(env.write_calls().len(), writes_before);
        assert!(env.rename_calls().is_empty());
        println!("malformed yaml rejected before target write: {error}");
    }

    #[test]
    fn malformed_hermes_root_path_is_typed_and_never_writes_target() {
        let env = fixture();
        write(&env, "/hermes", "not a directory");
        let writes_before = env.write_calls().len();

        let error = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap_err();

        assert!(matches!(error, OmonError::Config(_)));
        assert!(error.to_string().contains("not a directory"));
        assert_eq!(env.write_calls().len(), writes_before);
    }

    #[test]
    fn root_scalar_values_win_over_profile_fallbacks() {
        let env = fixture();
        write(&env, "/hermes/config.yaml", "model:\n  default: gpt-4o\n");
        write(
            &env,
            "/hermes/.env",
            "DISCORD_BOT_TOKEN=primary\nAPPROVAL_MODE=never\n",
        );
        write(
            &env,
            "/hermes/profiles/alpha/.env",
            "DISCORD_BOT_TOKEN=second\nAPPROVAL_MODE=always\nDISCORD_ALLOWED_USERS=99\n",
        );

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        assert_eq!(
            result.values.get("APPROVAL_MODE").map(String::as_str),
            Some("never")
        );
        assert_eq!(
            result
                .values
                .get("DISCORD_ALLOWED_USERS")
                .map(String::as_str),
            None
        );
    }

    #[test]
    fn profile_access_control_never_imported_globally() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            "model:\n  default: gpt-4o\ndiscord:\n  allowed_users: [\"42\"]\n",
        );
        write(&env, "/hermes/.env", "DISCORD_BOT_TOKEN=primary\n");
        write(
            &env,
            "/hermes/profiles/alpha/.env",
            "DISCORD_ALLOWED_USERS=99\nDISCORD_ALLOWED_CHANNELS=7\nDISCORD_ALLOWED_ROLES=8\nDISCORD_ALLOW_ALL_USERS=true\nDISCORD_IGNORED_CHANNELS=9\nDISCORD_FREE_RESPONSE_CHANNELS=10\nDISCORD_HOME_CHANNEL=11\n",
        );

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        assert_eq!(
            result
                .values
                .get("DISCORD_ALLOWED_USERS")
                .map(String::as_str),
            Some("42")
        );
        for key in [
            "DISCORD_ALLOWED_CHANNELS",
            "DISCORD_ALLOWED_ROLES",
            "DISCORD_ALLOW_ALL_USERS",
            "DISCORD_IGNORED_CHANNELS",
            "DISCORD_FREE_RESPONSE_CHANNELS",
            "DISCORD_HOME_CHANNEL",
        ] {
            assert!(
                !result.values.contains_key(key),
                "{key} leaked from profile"
            );
        }
    }

    #[test]
    fn profile_approval_mode_never_overrides_root_config() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            "model:\n  default: gpt-4o\napprovals:\n  mode: always\n",
        );
        write(&env, "/hermes/.env", "DISCORD_BOT_TOKEN=primary\n");
        write(
            &env,
            "/hermes/profiles/yolo/.env",
            "APPROVAL_MODE=never\nAPPROVALS_DESTRUCTIVE_SLASH_CONFIRM=false\n",
        );

        let result = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();

        assert_eq!(
            result.values.get("APPROVAL_MODE").map(String::as_str),
            Some("always"),
            "a profile's approval mode must not override the root config"
        );
        assert!(
            !result
                .values
                .contains_key("APPROVALS_DESTRUCTIVE_SLASH_CONFIRM"),
            "slash-confirm must not leak from a profile"
        );
    }

    #[test]
    fn imports_effective_discord_policy() {
        let env = fixture();
        write(
            &env,
            "/hermes/config.yaml",
            r#"
model:
  name: gpt-4o
discord:
  enabled: true
  allowed_users: ["42"]
  ignored_channels: ["99"]
"#,
        );
        write(&env, "/hermes/.env", "DISCORD_BOT_TOKEN=token_x\n");

        write(
            &env,
            "/hermes/profiles/disabled_bot/config.yaml",
            r#"
discord:
  enabled: false
  token: disabled_token
"#,
        );

        let res = import_config(
            &env,
            Path::new("/hermes"),
            Path::new("/gateway/.env"),
            false,
        )
        .unwrap();
        assert_eq!(
            res.values.get("DEFAULT_MODEL").map(String::as_str),
            Some("gpt-4o")
        );
        assert_eq!(
            res.values.get("DISCORD_ALLOWED_USERS").map(String::as_str),
            Some("42")
        );
        assert_eq!(
            res.values
                .get("DISCORD_IGNORED_CHANNELS")
                .map(String::as_str),
            Some("99")
        );
        assert_eq!(
            res.values.get("DISCORD_BOT_TOKEN").map(String::as_str),
            Some("token_x")
        );
        assert!(
            !res.values.contains_key("DISCORD_BOT_TOKENS"),
            "Disabled bot token must not be included in active tokens"
        );
    }
}
