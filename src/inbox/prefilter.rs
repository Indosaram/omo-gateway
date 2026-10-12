use std::collections::HashSet;
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};

use super::{InboxItem, InboxSourceKind};
use crate::{OmonError, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PrefilterRules {
    pub owner_kakao_name: String,
    pub kakao_keywords: Vec<String>,
    pub excluded_kakao_rooms: HashSet<String>,
    pub owner_discord_id: String,
    pub bot_discord_id: Option<String>,
}

impl Default for PrefilterRules {
    fn default() -> Self {
        Self {
            owner_kakao_name: String::new(),
            kakao_keywords: Vec::new(),
            excluded_kakao_rooms: ["18491246151592761".to_owned()].into_iter().collect(),
            owner_discord_id: "414011632306618368".to_owned(),
            bot_discord_id: None,
        }
    }
}

impl PrefilterRules {
    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| OmonError::Config(format!("invalid inbox filter rules: {error}"))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let rules = Self::default();
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|error| OmonError::Config(error.to_string()))?;
                }
                let bytes = serde_json::to_vec_pretty(&rules)
                    .map_err(|error| OmonError::Config(error.to_string()))?;
                std::fs::write(path, bytes)
                    .map_err(|error| OmonError::Config(error.to_string()))?;
                Ok(rules)
            }
            Err(error) => Err(OmonError::Config(error.to_string())),
        }
    }

    pub fn drop_reason(&self, item: &InboxItem) -> Option<&'static str> {
        let author = item.author.as_deref().unwrap_or_default();
        let title = item.title.as_deref().unwrap_or_default();
        let text = format!("{author} {title} {}", item.snippet).to_lowercase();
        match item.source {
            InboxSourceKind::Mail => {
                if item
                    .mail_categories
                    .iter()
                    .any(|category| category == "promotions")
                    || contains_any(&text, &["category:promotions", "[promotions]"])
                {
                    return Some("mail_promotions");
                }
                if item
                    .mail_categories
                    .iter()
                    .any(|category| category == "social")
                    || contains_any(&text, &["category:social", "[social]"])
                {
                    return Some("mail_social");
                }
                if item
                    .mail_categories
                    .iter()
                    .any(|category| category == "updates")
                    || contains_any(&text, &["category:updates", "[updates]"])
                {
                    return Some("mail_updates");
                }
                None
            }
            InboxSourceKind::Kakao => {
                let room = item.thread_ref.as_deref().unwrap_or(&item.source_ref);
                if self.excluded_kakao_rooms.contains(room) {
                    return Some("kakao_excluded_room");
                }
                let room_type = item.room_type.as_deref().unwrap_or_default();
                if matches!(room_type, "OM" | "OD" | "PlusChat") {
                    let matches_owner = !self.owner_kakao_name.is_empty()
                        && text.contains(&self.owner_kakao_name.to_lowercase());
                    if !matches_owner
                        && !self
                            .kakao_keywords
                            .iter()
                            .any(|keyword| text.contains(&keyword.to_lowercase()))
                    {
                        return Some("kakao_noise_room_no_keyword");
                    }
                } else if !matches!(room_type, "DirectChat" | "MultiChat") {
                    return Some("kakao_unsupported_room_type");
                }
                None
            }
            InboxSourceKind::Imessage => {
                let sender = item.author.as_deref().unwrap_or_default();
                if sender.len() >= 2
                    && sender.len() <= 6
                    && sender.chars().all(|ch| ch.is_ascii_digit())
                {
                    return Some("imessage_shortcode");
                }
                if item.snippet.starts_with("[Web발신]") || item.snippet.starts_with("(광고)") {
                    return Some("imessage_ad");
                }
                None
            }
            InboxSourceKind::Discord => {
                let author = item.author.as_deref().unwrap_or_default();
                if author == self.owner_discord_id
                    || self
                        .bot_discord_id
                        .as_deref()
                        .is_some_and(|bot_id| author == bot_id)
                {
                    return Some("discord_author_excluded");
                }

                let owner_mention = text.contains(&format!("<@{}>", self.owner_discord_id))
                    || text.contains(&format!("<@!{}>", self.owner_discord_id));

                let room_type = item.room_type.as_deref().unwrap_or_default();
                let is_dm = room_type == "dm" || room_type.split(',').any(|s| s.trim() == "dm");
                let is_owner_reply = room_type == "owner_reply"
                    || room_type.split(',').any(|s| s.trim() == "owner_reply");

                if !owner_mention && !is_dm && !is_owner_reply {
                    return Some("discord_not_owner_mention_or_dm");
                }
                None
            }
        }
    }

    pub fn direct_class(&self, item: &InboxItem) -> Option<&'static str> {
        if item.source != InboxSourceKind::Mail {
            return None;
        }
        let author = item.author.as_deref().unwrap_or_default().to_lowercase();
        let text = format!(
            "{} {}",
            item.title.as_deref().unwrap_or_default(),
            item.snippet
        )
        .to_lowercase();
        let known_notification = ["github.com", "discord.com", "linkedin.com"]
            .iter()
            .any(|sender| author.contains(sender));
        let escalated = contains_any(
            &text,
            &["failed", "error", "review requested", "mentioned you"],
        );
        (known_notification && !escalated).then_some("fyi")
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub async fn record_drop(
    tx: &mut Transaction<'_, Sqlite>,
    source: InboxSourceKind,
    rule: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO inbox_filter_stats(day, source, rule, count) VALUES(?, ?, ?, 1) \
         ON CONFLICT(day, source, rule) DO UPDATE SET count=count+1",
    )
    .bind(Utc::now().date_naive().to_string())
    .bind(source.as_str())
    .bind(rule)
    .execute(&mut **tx)
    .await
    .map_err(OmonError::from)?;
    Ok(())
}
