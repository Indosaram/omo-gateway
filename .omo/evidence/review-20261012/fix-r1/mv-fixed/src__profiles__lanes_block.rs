use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

const BEGIN_MARKER: &str = "<!-- omon:lanes:begin -->";
const END_MARKER: &str = "<!-- omon:lanes:end -->";
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

fn validate_block(block: &str) -> std::result::Result<(), ApplyBlockError> {
    let begins = block.match_indices(BEGIN_MARKER).count();
    let ends = block.match_indices(END_MARKER).count();
    if begins != 1 || ends != 1 {
        return Err(ApplyBlockError::InvalidBlock(
            "expected exactly one begin and end marker".into(),
        ));
    }
    let begin = block.find(BEGIN_MARKER).expect("marker count checked");
    let end = block.find(END_MARKER).expect("marker count checked");
    if end < begin + BEGIN_MARKER.len() {
        return Err(ApplyBlockError::InvalidBlock(
            "end marker precedes begin marker".into(),
        ));
    }
    Ok(())
}

fn marked_block(block: &str) -> &str {
    let begin = block.find(BEGIN_MARKER).expect("validated begin marker");
    let end = block.find(END_MARKER).expect("validated end marker") + END_MARKER.len();
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

pub async fn apply_block(
    pool: &SqlitePool,
    bot_id: &str,
    block: &str,
) -> std::result::Result<ApplyBlockResult, ApplyBlockError> {
    validate_block(block)?;
    let block = marked_block(block);
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
    let begin = old_prompt.find(BEGIN_MARKER);
    let end = old_prompt.find(END_MARKER);
    let begin_count = old_prompt.match_indices(BEGIN_MARKER).count();
    let end_count = old_prompt.match_indices(END_MARKER).count();
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
            if begin_count == 1 && end_count == 1 && end >= start + BEGIN_MARKER.len() =>
        {
            let after_end = end + END_MARKER.len();
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

    sqlx::query("UPDATE bot_profiles SET system_prompt = ?, updated_at = CURRENT_TIMESTAMP WHERE bot_id = ?")
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
