use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

/// Block name used by `apply_block` and the CLI when `--block` is omitted.
pub const DEFAULT_BLOCK_NAME: &str = "lanes";

/// The begin/end comment markers that delimit one named persona block,
/// e.g. `<!-- omon:lanes:begin -->` ... `<!-- omon:lanes:end -->`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMarkers {
    pub begin: String,
    pub end: String,
}

impl BlockMarkers {
    /// Markers for `omon:<name>`. The name is a CLI boundary value, so it is
    /// restricted to lowercase ASCII letters, digits and `-`.
    pub fn named(name: &str) -> std::result::Result<Self, ApplyBlockError> {
        let valid = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !valid {
            return Err(ApplyBlockError::InvalidBlock(format!(
                "block name must match [a-z0-9-]+: {name:?}"
            )));
        }
        Ok(Self {
            begin: format!("<!-- omon:{name}:begin -->"),
            end: format!("<!-- omon:{name}:end -->"),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct BotProfileExport {
    pub bot_id: String,
    pub name: String,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub enabled_toolsets: Option<String>,
    pub custom_settings_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptDigest {
    pub length: usize,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyBlockResult {
    pub before: PromptDigest,
    pub after: PromptDigest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyBlockError {
    UnknownBotId(String),
    InvalidBlock(String),
    Database(String),
}

impl std::fmt::Display for ApplyBlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownBotId(id) => write!(f, "unknown bot id: {id}"),
            Self::InvalidBlock(message) => write!(f, "invalid lane block: {message}"),
            Self::Database(message) => write!(f, "database error: {message}"),
        }
    }
}

impl std::error::Error for ApplyBlockError {}

impl ApplyBlockError {
    pub fn exit_code(&self) -> Option<i32> {
        matches!(self, Self::UnknownBotId(_)).then_some(2)
    }
}

fn digest(prompt: &str) -> PromptDigest {
    PromptDigest {
        length: prompt.chars().count(),
        sha256: format!("{:x}", Sha256::digest(prompt.as_bytes())),
    }
}

fn validate_block(markers: &BlockMarkers, block: &str) -> std::result::Result<(), ApplyBlockError> {
    let (begin_marker, end_marker) = (markers.begin.as_str(), markers.end.as_str());
    let begins = block.match_indices(begin_marker).count();
    let ends = block.match_indices(end_marker).count();
    if begins != 1 || ends != 1 {
        return Err(ApplyBlockError::InvalidBlock(
            "expected exactly one begin and end marker".into(),
        ));
    }
    let begin = block.find(begin_marker).expect("marker count checked");
    let end = block.find(end_marker).expect("marker count checked");
    if end < begin + begin_marker.len() {
        return Err(ApplyBlockError::InvalidBlock(
            "end marker precedes begin marker".into(),
        ));
    }
    Ok(())
}

fn marked_block<'a>(markers: &BlockMarkers, block: &'a str) -> &'a str {
    let begin = block.find(&markers.begin).expect("validated begin marker");
    let end = block.find(&markers.end).expect("validated end marker") + markers.end.len();
    &block[begin..end]
}

pub async fn export_profile(
    pool: &SqlitePool,
    bot_id: &str,
) -> std::result::Result<Option<BotProfileExport>, sqlx::Error> {
    sqlx::query_as::<_, BotProfileExport>(
        "SELECT bot_id, name, model, system_prompt, enabled_toolsets,
                custom_settings_json, created_at, updated_at
         FROM bot_profiles WHERE bot_id = ?",
    )
    .bind(bot_id)
    .fetch_optional(pool)
    .await
}

/// Apply the default `omon:lanes` block.
pub async fn apply_block(
    pool: &SqlitePool,
    bot_id: &str,
    block: &str,
) -> std::result::Result<ApplyBlockResult, ApplyBlockError> {
    let markers = BlockMarkers::named(DEFAULT_BLOCK_NAME)?;
    apply_block_with_markers(pool, bot_id, &markers, block).await
}

/// Insert or replace the block delimited by `markers` in the bot's system
/// prompt. Text outside those markers, including other named blocks, is kept
/// byte-for-byte; re-applying the same block is a no-op.
pub async fn apply_block_with_markers(
    pool: &SqlitePool,
    bot_id: &str,
    markers: &BlockMarkers,
    block: &str,
) -> std::result::Result<ApplyBlockResult, ApplyBlockError> {
    validate_block(markers, block)?;
    let block = marked_block(markers, block);
    let (begin_marker, end_marker) = (markers.begin.as_str(), markers.end.as_str());
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| ApplyBlockError::Database(e.to_string()))?;
    let prompt: Option<Option<String>> =
        sqlx::query_scalar("SELECT system_prompt FROM bot_profiles WHERE bot_id = ?")
            .bind(bot_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ApplyBlockError::Database(e.to_string()))?;
    let Some(prompt) = prompt else {
        tx.rollback()
            .await
            .map_err(|e| ApplyBlockError::Database(e.to_string()))?;
        return Err(ApplyBlockError::UnknownBotId(bot_id.to_owned()));
    };

    let old_prompt = prompt.unwrap_or_default();
    let before = digest(&old_prompt);
    let begin = old_prompt.find(begin_marker);
    let end = old_prompt.find(end_marker);
    let begin_count = old_prompt.match_indices(begin_marker).count();
    let end_count = old_prompt.match_indices(end_marker).count();
    let new_prompt = match (begin, end) {
        (None, None) => {
            if begin_count != 0 || end_count != 0 {
                tx.rollback()
                    .await
                    .map_err(|e| ApplyBlockError::Database(e.to_string()))?;
                return Err(ApplyBlockError::InvalidBlock(
                    "existing prompt contains malformed lane markers".into(),
                ));
            }
            let separator = if old_prompt.is_empty() || old_prompt.ends_with('\n') {
                ""
            } else {
                "\n\n"
            };
            format!("{old_prompt}{separator}{block}")
        }
        (Some(start), Some(end))
            if begin_count == 1 && end_count == 1 && end >= start + begin_marker.len() =>
        {
            let after_end = end + end_marker.len();
            if old_prompt[start..after_end] == *block {
                old_prompt.clone()
            } else {
                format!(
                    "{}{block}{}",
                    &old_prompt[..start],
                    &old_prompt[after_end..]
                )
            }
        }
        _ => {
            tx.rollback()
                .await
                .map_err(|e| ApplyBlockError::Database(e.to_string()))?;
            return Err(ApplyBlockError::InvalidBlock(
                "existing prompt contains malformed lane markers".into(),
            ));
        }
    };

    sqlx::query("UPDATE bot_profiles SET system_prompt = ? WHERE bot_id = ?")
        .bind(&new_prompt)
        .bind(bot_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApplyBlockError::Database(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ApplyBlockError::Database(e.to_string()))?;

    Ok(ApplyBlockResult {
        before,
        after: digest(&new_prompt),
    })
}

pub fn export_json(profile: &BotProfileExport) -> std::result::Result<String, serde_json::Error> {
    serde_json::to_string_pretty(profile)
}
