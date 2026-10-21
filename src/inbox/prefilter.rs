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
    /// KakaoTalk sender names that are the owner himself; his own messages never need his action.
    pub owner_kakao_authors: Vec<String>,
    pub kakao_keywords: Vec<String>,
    pub excluded_kakao_rooms: HashSet<String>,
    pub owner_discord_id: String,
    pub bot_discord_id: Option<String>,
    pub owner_slack_id: String,
    pub bot_slack_id: Option<String>,
}

impl Default for PrefilterRules {
    fn default() -> Self {
        Self {
            owner_kakao_name: String::new(),
            owner_kakao_authors: vec!["윤인도".to_owned()],
            kakao_keywords: Vec::new(),
            excluded_kakao_rooms: ["18491246151592761".to_owned()].into_iter().collect(),
            owner_discord_id: "414011632306618368".to_owned(),
            bot_discord_id: None,
            owner_slack_id: crate::slack::owner_id(),
            bot_slack_id: None,
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
                if contains_any(
                    &text,
                    &[
                        "인증 코드",
                        "인증코드",
                        "인증번호",
                        "보안 코드",
                        "verification code",
                        "security code",
                        "one-time code",
                        "login code",
                    ],
                ) {
                    return Some("mail_otp");
                }
                // Nobody reads a reply to a no-reply sender, so such mail never waits on the
                // owner. Known notification senders (GitHub CI failures) keep their own rules.
                let sender = author.to_lowercase();
                if contains_any(
                    &sender,
                    &["no-reply", "noreply", "donotreply", "do-not-reply"],
                ) && !contains_any(&sender, &["github.com", "discord.com", "linkedin.com"])
                {
                    return Some("mail_no_reply");
                }
                None
            }
            InboxSourceKind::Kakao => {
                let room = item.thread_ref.as_deref().unwrap_or(&item.source_ref);
                if self.excluded_kakao_rooms.contains(room) {
                    return Some("kakao_excluded_room");
                }
                if self.owner_kakao_authors.iter().any(|name| name == author) {
                    return Some("kakao_owner_author");
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
                // Only messages marked as ads are dropped. Plain [Web발신] also carries bookings
                // and notices from banks and agencies, so jev judges those.
                let body = item.snippet.trim_start();
                let body = body.strip_prefix("[Web발신]").unwrap_or(body).trim_start();
                if body.starts_with("(광고)") {
                    return Some("imessage_ad");
                }
                if contains_any(
                    &text,
                    &[
                        "인증 코드",
                        "인증코드",
                        "보안 코드",
                        "verification code",
                        "security code",
                    ],
                ) {
                    return Some("imessage_otp");
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
            InboxSourceKind::Slack => {
                let author = item.author.as_deref().unwrap_or_default();
                if author == self.owner_slack_id
                    || self
                        .bot_slack_id
                        .as_deref()
                        .is_some_and(|bot_id| author == bot_id)
                {
                    return Some("slack_author_excluded");
                }
                // The Slack source classifies `mention` from the full message content, because the
                // stored snippet is capped at MAX_SNIPPET_CHARS and can hide a mention past the cap.
                let owner_mention = item
                    .snippet
                    .contains(&format!("<@{}>", self.owner_slack_id));
                let room_type = item.room_type.as_deref().unwrap_or_default();
                if !owner_mention && !matches!(room_type, "dm" | "owner_reply" | "mention") {
                    return Some("slack_not_owner_mention_or_dm");
                }
                None
            }
            InboxSourceKind::Github => None,
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
    // KST day, matching the digest/usage windows that read these counts.
    .bind(
        Utc::now()
            .with_timezone(&chrono_tz::Asia::Seoul)
            .date_naive()
            .to_string(),
    )
    .bind(source.as_str())
    .bind(rule)
    .execute(&mut **tx)
    .await
    .map_err(OmonError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn slack_item(author: &str, room_type: &str, snippet: &str) -> InboxItem {
        let mut item = InboxItem::new(
            InboxSourceKind::Slack,
            "1700000001.000100",
            snippet,
            Utc::now(),
        );
        item.author = Some(author.into());
        item.room_type = Some(room_type.into());
        item
    }

    #[test]
    fn slack_keeps_dm_mention_and_owner_reply_and_drops_the_rest() {
        let rules = PrefilterRules {
            owner_slack_id: "U0OWNER".into(),
            bot_slack_id: Some("U0BOT".into()),
            ..Default::default()
        };
        assert_eq!(rules.drop_reason(&slack_item("U0A", "dm", "hello")), None);
        assert_eq!(
            rules.drop_reason(&slack_item("U0A", "owner_reply", "ok")),
            None
        );
        assert_eq!(
            rules.drop_reason(&slack_item("U0A", "channel", "hey <@U0OWNER>")),
            None
        );
        assert_eq!(
            rules.drop_reason(&slack_item("U0A", "channel", "hey <@U0OTHER>")),
            Some("slack_not_owner_mention_or_dm")
        );
        assert_eq!(
            rules.drop_reason(&slack_item("U0OWNER", "dm", "self")),
            Some("slack_author_excluded")
        );
        assert_eq!(
            rules.drop_reason(&slack_item("U0BOT", "dm", "bot")),
            Some("slack_author_excluded")
        );
    }

    #[test]
    fn slack_keeps_mention_room_type_when_snippet_truncated() {
        let rules = PrefilterRules {
            owner_slack_id: "U0OWNER".into(),
            bot_slack_id: Some("U0BOT".into()),
            ..Default::default()
        };
        // A message longer than MAX_SNIPPET_CHARS with the owner mention past the cap: the source
        // decides `mention` from the full content, so the prefilter must not re-test the snippet.
        let content = format!(
            "{}<@U0OWNER> can you take a look?",
            "x".repeat(crate::inbox::MAX_SNIPPET_CHARS)
        );
        let item = slack_item("U0A", "mention", &content);
        assert!(!item.snippet.contains("<@U0OWNER>"));
        assert_eq!(rules.drop_reason(&item), None);
        // A channel row without a visible mention stays dropped; classifying it is the source's job.
        assert_eq!(
            rules.drop_reason(&slack_item("U0A", "channel", &content)),
            Some("slack_not_owner_mention_or_dm")
        );
    }

    #[test]
    fn mail_codes_and_no_reply_senders_never_reach_the_owner() {
        let rules = PrefilterRules::default();
        let mail = |author: &str, title: &str| {
            let mut item = InboxItem::new(InboxSourceKind::Mail, "t", "body", Utc::now());
            item.author = Some(author.into());
            item.title = Some(title.into());
            item
        };
        let code = mail(
            "Slack <no-reply-x@slack.com>",
            "Your Slack security code is 169399",
        );
        assert_eq!(rules.drop_reason(&code), Some("mail_otp"));
        let survey = mail(
            "Survey <noreply@survey.example>",
            "Please answer our survey",
        );
        assert_eq!(rules.drop_reason(&survey), Some("mail_no_reply"));
        let person = mail("REPROZ <reproz.customer@reproz.co.jp>", "Re: quote");
        assert_eq!(rules.drop_reason(&person), None);
        let ci = mail("GitHub <noreply@github.com>", "Run failed: ci");
        assert_eq!(rules.drop_reason(&ci), None);
        assert_eq!(rules.direct_class(&ci), None);
    }

    #[test]
    fn rules_files_without_slack_fields_load_with_defaults() {
        let rules: PrefilterRules = serde_json::from_str(r#"{"owner_discord_id":"1"}"#).unwrap();
        assert_eq!(rules.owner_slack_id, crate::slack::owner_id());
        assert_eq!(rules.bot_slack_id, None);
        assert_eq!(rules.owner_discord_id, "1");
    }
}
