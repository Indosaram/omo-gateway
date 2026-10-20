# R1 citation resolution against current worktree

Line positions may have shifted after fixes; these are current excerpts at each reported Location. Semantic verification is recorded separately in CONFIRMED and delta reports.

## web-core-shell.md: web/src/tracker/hotkeys.ts:88

```text
  const editable = isEditableTarget(event.target)

  if (!editable && !event.metaKey && !event.ctrlKey && !event.altKey && buffer.length > 0) {
    const key = eventKey(event)
    if (key.length === 1) {
      const candidate = [...buffer, key]
```

## web-core-shell.md: web/src/tracker/store.tsx:380

```text
    favorites,
    issues,
    notifications,
    unreadCount: notifications.filter((n) => !n.read_at && !n.archived_at && !isSnoozed(n)).length,
    usersById,
    teamsById,
```

## web-core-shell.md: web/src/tracker/shell/CommandPalette.tsx:69

```text
    if (issue) return [issue.id]
  }
  if (t.focusedIssueId) return [t.focusedIssueId]
  return []
}

```

## web-core-shell.md: web/src/tracker/store.tsx:222

```text
        trackerApi.notifications(),
      ])
      setData(boot)
      setIssues(list.items)
      setNotifications(notes.items)
      setError(null)
```

## web-core-shell.md: web/src/tracker/api.ts:51

```text
  if (response.status === 204) return undefined as T
  const text = await response.text()
  const payload = text ? JSON.parse(text) : null
  if (!response.ok) {
    throw new TrackerApiError(
      payload?.error ?? `${response.status} ${response.statusText}`,
```

## web-core-shell.md: web/src/tracker/router.ts:42

```text
  const [pathname, query = ''] = path.split('?')
  const params = new URLSearchParams(query)
  const parts = pathname.split('/').filter(Boolean).map(decodeURIComponent)
  const [head, a, b] = parts
  switch (head) {
    case 'inbox':
```

## web-core-shell.md: web/src/tracker/TrackerApp.tsx:189

```text
      }
    })()
    document.title = `${title} - Omon`
  }, [route, t.teamsByKey, t.issuesByIdentifier])

  const showDesktopSidebar = !mobile && !t.sidebarCollapsed
```

## web-core-shell.md: web/src/tracker/format.ts:91

```text
export function dueShortcuts(now = new Date()): { id: string; label: string; date: string }[] {
  const today = startOfDay(now)
  const add = (days: number) => toDateOnly(new Date(today.getTime() + days * DAY))
  const endOfWeek = 5 - today.getDay()
  return [
    { id: 'today', label: 'Today', date: add(0) },
```

## slack-ingress.md: src/slack/backfill.rs:153

```text
        if ingress.intercept_replayed_lane_reply(&event).await? {
            continue;
        }
        let known_thread = ingress.is_known_thread(&event).await?;
        if let Some(inbound) = ingress.map_replayed(&event, known_thread) {
            match ingress.route_awaiting_turn(inbound).await {
```

## slack-ingress.md: src/slack/ingress.rs:781

```text
        }
        if let Err(error) = crate::lanes::runner::wake(&s.pool).await {
            tracing::warn!(%error, channel, thread_ts, "lane wake after Slack owner reply failed");
        }
        Ok(true)
    }
```

## slack-ingress.md: src/slack/ingress.rs:222

```text

        if is_stop(&inbound.content) {
            self.debouncer.cancel(&inbound.session).await;
            let interrupted = crate::discord::commands::stop_session(
                &s.pool,
                &self.approvals,
```

## slack-ingress.md: src/slack/backfill.rs:149

```text
            break;
        }
        let event = replay_event(message, channel, thread_ts, bot);
        // intercept_lane_reply advances the cursor itself when it consumes the reply.
        if ingress.intercept_replayed_lane_reply(&event).await? {
            continue;
```

## slack-ingress.md: src/slack/backfill.rs:107

```text
    let pool = ingress.pool();
    let bot = ingress.bot_user_id();
    let api = ingress.api();
    let mut messages = if thread_ts.is_empty() {
        api.conversations_history(channel, cursor.as_deref(), None, PAGE_LIMIT)
            .await?
```

## slack-ingress.md: src/slack/backfill.rs:54

```text
            targets.insert((channel, String::new()));
        }
        Err(error) => tracing::warn!(%error, "slack backfill could not open the owner DM"),
    }
    let sessions: Vec<String> = sqlx::query_scalar(
        "SELECT session_key FROM sessions WHERE platform = ? AND thread_id IS NOT NULL",
```

## slack-ingress.md: src/slack/debounce.rs:137

```text
            .await
            .entry(session.clone())
            .or_default()
            .clone()
    }

```

## web-projects-ui.md: web/src/tracker/ui/pickers.tsx:296

```text
      width={260}
      selectedIds={current === undefined ? [] : [current ?? 'none']}
      onSelect={(item) => apply({ project_id: item.id === 'none' ? null : item.id })}
    />
  )
}
```

## web-projects-ui.md: web/src/tracker/ui/pickers.tsx:56

```text
        return
      }
      toast.error(errorMessage(error))
    })
  }
}
```

## web-projects-ui.md: web/src/tracker/projects/cycles.tsx:161

```text
  const defaultStart = latestEnd > Date.now() ? new Date(latestEnd) : new Date()
  const [name, setName] = useState('')
  const [startsAt, setStartsAt] = useState(() => toDateOnly(defaultStart))
  const [endsAt, setEndsAt] = useState(() => toDateOnly(new Date(defaultStart.getTime() + (team.cycle_weeks || 2) * 7 * DAY)))
  const [saving, setSaving] = useState(false)

```

## web-projects-ui.md: web/src/tracker/projects/Milestones.tsx:47

```text
  }

  const add = async () => {
    if (!name.trim()) return
    try {
      const created = await trackerApi.addMilestone(projectId, { name: name.trim(), target_date: date || null })
```

## autonomy-effect.md: src/autonomy/effect.rs:907

```text
                    "-f" | "-F" | "--field" | "--raw-field" | "--input"
                )
            }) && !args.iter().any(|a| a == "graphql");
            if method == "GET" && !has_field_mutation {
                Classification::new(Effect::Read, "gh api", target, summary)
            } else if method == "DELETE" {
```

## autonomy-effect.md: src/autonomy/effect.rs:828

```text
            let force = sub_args
                .iter()
                .any(|a| a.starts_with("-f") || a == "--force");
            if force {
                Classification::new(Effect::Delete, "git clean", repo_target, summary)
            } else {
```

## autonomy-effect.md: src/autonomy/effect.rs:1066

```text
    let tokens = tokenize_shell(cmd);
    for (i, tok) in tokens.iter().enumerate() {
        if (tok == ">" || tok == ">>" || tok == "1>" || tok == "1>>") && i + 1 < tokens.len() {
            let target = &tokens[i + 1];
            if !target.starts_with('&') && target != "/dev/null" {
                return Some(target.clone());
```

## autonomy-effect.md: src/autonomy/effect.rs:621

```text
                || sql.contains("alter ")
                || sql.contains("create ");
            if is_readonly || (!mutates && (sql.starts_with("select") || sql.starts_with("pragma")))
            {
                Classification::new(Effect::Read, "sqlite3", target, summary)
            } else if is_draft_path(db_path.unwrap_or(""), cwd) {
```

## autonomy-effect.md: src/autonomy/effect.rs:238

```text

/// JS calls in an eval cell that act outside the shell and the file helpers.
fn eval_opaque_api(code: &str) -> Option<&'static str> {
    const APIS: &[(&str, &str)] = &[
        ("Bun.spawn", "Bun.spawn"),
        ("child_process", "child_process"),
```

## autonomy-effect.md: src/autonomy/effect.rs:485

```text
                .is_none_or(|w| c.effect.rank() > w.effect.rank())
            {
                worst = Some(c);
            }
        }
        return match worst {
```

## autonomy-effect.md: src/autonomy/effect.rs:93

```text
        let target = normalize_target(&target.into());
        let summary = summary.into();
        let mut signature = format!("{}:{}:{}", effect.as_str(), program, target);
        // An exact precedent for a destructive or outward action covers only the exact
        // command the owner saw (rm -rf ., push --force, another PR are different cases).
        if matches!(
```

## autonomy-effect.md: src/autonomy/effect.rs:855

```text
                )
            });
            let branch = positionals
                .get(1)
                .or_else(|| positionals.first())
                .map(String::as_str)
```

## autonomy-effect.md: src/autonomy/effect.rs:642

```text
        }
        "sed" => {
            let in_place = args.iter().any(|a| a == "-i" || a.starts_with("-i"));
            let (_, positionals) = split_flags_and_positionals(args, &["-e", "-f"]);
            let file = positionals.last().map(String::as_str).unwrap_or("");
            if in_place {
```

## autonomy-effect.md: src/autonomy/effect.rs:765

```text

    match sub {
        "status" | "diff" | "log" | "show" | "rev-parse" | "ls-files" | "ls-remote"
        | "cat-file" | "describe" | "shortlog" | "blame" | "reflog" | "grep" => {
            Classification::new(Effect::Read, format!("git {sub}"), repo_target, summary)
        }
```

## autonomy-effect.md: src/autonomy/effect.rs:476

```text
            if body.is_empty() || body == "cd" || body.starts_with("cd ") {
                if let Some(dir) = body.strip_prefix("cd ") {
                    seg_cwd = Some(dir.trim().trim_matches('"').trim_matches('\'').to_string());
                }
                continue;
            }
```

## autonomy-effect.md: src/autonomy/effect.rs:949

```text
    all.contains("stripe charges")
        || all.contains("stripe payment")
        || all.contains("billing")
        || all.contains("--pay")
        || all.contains("purchase")
        || all.contains("checkout --confirm")
```

## autonomy-effect.md: src/autonomy/effect.rs:246

```text
        ("fetch(", "fetch"),
        ("rmSync", "fs delete"),
        ("rm(", "fs delete"),
        ("unlink", "fs delete"),
        ("rmdir", "fs delete"),
        ("renameSync", "fs rename"),
```

## autonomy-effect.md: src/autonomy/effect.rs:689

```text
            let target = resolve_target_scope(None, cwd);
            match sub {
                "check" | "clippy" | "test" | "bench" | "doc" | "metadata" | "tree" | "version" => {
                    Classification::new(
                        Effect::Read,
                        format!("cargo {sub}").trim().to_string(),
```

## inbox.md: src/inbox/sources/slack.rs:61

```text
                      ELSE 'channel' \
                    END AS calculated_room_type \
             FROM message_search_documents d \
             WHERE d.platform = 'slack' \
               AND d.rowid > ? \
               AND d.author_id <> '' \
```

## inbox.md: src/tools/message_context_slack.rs:593

```text
    MessageSearchDocument {
        platform: PLATFORM.into(),
        guild_id: session.guild_id.clone(),
        channel_id: conversation.index_key().to_owned(),
        thread_id: conversation.thread_ts.clone(),
        message_id: message.id.clone(),
```

## inbox.md: src/inbox/triage.rs:295

```text
        for item in items {
            // Backlog that arrives late is history, not a new task for the owner.
            if now - item.received_at > chrono::Duration::hours(STALE_AFTER_HOURS) {
                classified.push((item, Class::Fyi, None));
                continue;
            }
```

## inbox.md: src/inbox/prefilter.rs:180

```text
                let owner_mention = item
                    .snippet
                    .contains(&format!("<@{}>", self.owner_slack_id));
                let room_type = item.room_type.as_deref().unwrap_or_default();
                if !owner_mention && !matches!(room_type, "dm" | "owner_reply") {
                    return Some("slack_not_owner_mention_or_dm");
```

## inbox.md: src/inbox/cli.rs:116

```text
            .await
            .map_err(|e| e.to_string())?;
    let poll = source
        .poll(cursor.as_deref())
        .await
        .map_err(|e| e.to_string())?;
```

## main-runtime.md: src/main.rs:813

```text
        "invalid_auth",
        "not_authed",
        "account_inactive",
        "token_revoked",
        "token_expired",
        "missing_scope",
```

## main-runtime.md: src/dashboard.rs:2458

```text
        } else {
            let decision = match suffix {
                "once" => omon_gateway::ApprovalDecision::Once,
                "session" => omon_gateway::ApprovalDecision::Session,
                "always" => omon_gateway::ApprovalDecision::Always,
                _ => omon_gateway::ApprovalDecision::Deny { reason: None },
```

## main-runtime.md: src/main.rs:837

```text
}

/// `auth.test`, persona copy and (when Slack is home) the owner DM lead. `None` when Slack is
/// off or its token is rejected; the caller then runs Discord only.
async fn boot_slack(
    pool: &SqlitePool,
```

## multiplexer-discord.md: src/discord/adapter.rs:3408

```text
            return Ok(());
        };
        let (claim_stripped, _extracted_claims) = if crate::report::claims::report_claims_enabled()
        {
            crate::report::claims::strip_and_extract_claims(content)
        } else {
```

## multiplexer-discord.md: src/discord/adapter.rs:3622

```text
                reply_to,
            } => {
                let filtered_content = filter_reasoning(&content);
                if is_explicit_silence(&filtered_content) {
                    self.typing.lock().await.remove(&session.storage_key());
                    return Ok(());
```

## multiplexer-discord.md: src/discord/adapter.rs:1388

```text
                let ledger_owner_required =
                    crate::agent::agent_workspace::approval_ledger_enabled();
                let is_discord_owner = user_id.to_string() == crate::inbox::proposals::OWNER_ID;
                if (ledger_owner_required && !is_discord_owner)
                    || (!is_paired
                        && !is_user_authorized(
```

## multiplexer-discord.md: src/multiplexer/actor.rs:809

```text
                            }
                            TurnOutcome::Completed(Err(error)) => {
                                driver.finish(Utc::now(), Err(error.to_string())).await
                            }
                            TurnOutcome::Stopped(_) => {
                                driver
```

## multiplexer-discord.md: src/discord/buttons.rs:106

```text
}

fn is_token(segment: &str) -> bool {
    (1..=128).contains(&segment.len())
        && segment.starts_with(|c: char| c.is_ascii_alphanumeric())
        && segment
```

## multiplexer-discord.md: src/discord/pairing.rs:342

```text

        // 3. Enforce bounded pending count, propagating count error
        let active_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_codes WHERE expires_at > ?")
                .bind(now)
                .fetch_one(&mut **tx)
```

## sessions-a.md: src/sessions/overview.rs:458

```text
const COLLECT_BUDGET: Duration = Duration::from_secs(23);

pub async fn collect<R: CommandRunner + 'static>(
    control: &SessionControl<R>,
    text_limit: usize,
) -> Overview {
```

## sessions-a.md: src/sessions/watch.rs:345

```text
                Ok(notices) => notices,
                Err(error) => {
                    tracing::warn!(%error, "session watcher tick failed; will retry");
                    continue;
                }
            };
```

## sessions-a.md: src/sessions/status_reply.rs:69

```text
pub fn owner_thread(key: &SessionKey, author: &str, owner: &str) -> bool {
    if key.platform == crate::slack::PLATFORM {
        return slack_home_conversation(
            key,
            author,
            &crate::slack::owner_id(),
```

## sessions-a.md: src/sessions/overview.rs:152

```text

        let snap_res = snapshot_map.get(m_str);

        if let Some(Err(err_msg)) = snap_res {
            lines.push(format!(
                "⚠️ {}: {}",
```

## sessions-a.md: src/sessions/watch.rs:264

```text
        self.streaming = Some(current);
        for notice in &notices {
            if notice.kind == "completion" {
                self.pending_completions
                    .insert(notice.dedupe_key.clone(), notice.clone());
            }
```

## slack-egress.md: src/slack/egress.rs:220

```text
    async fn stream(&self, session: &SessionKey, chunk: StreamChunk) -> Result<()> {
        // Partial chunks are never shown (a posted message is never edited or deleted later),
        // and they allocate nothing: a turn that ends without a final chunk leaves no state.
        if !chunk.is_final {
            return Ok(());
        }
```

## slack-egress.md: src/slack/egress.rs:227

```text
        // One stream can carry several finals (a background run relayed before the turn's own
        // reply); only a retried copy of the same final, same sequence, is a duplicate.
        let done = (key.0.clone(), key.1, chunk.sequence);
        if self.completed.lock().contains_key(&done) {
            return Ok(());
        }
```

## slack-egress.md: src/slack/egress.rs:894

```text
    ])
}

fn text_blocks(text: &str) -> Value {
    json!([{ "type": "section", "text": { "type": "mrkdwn", "text": escape(text) } }])
}
```

## slack-egress.md: src/slack/egress.rs:613

```text
    ) -> Result<()> {
        let channel = session.channel_id.as_str();
        self.skip_dead(channel).await?;
        let mentions = approval_mentions(&self.approval_mentions, self.approval_mentions_enabled);
        let fallback = format!(
            "{}Approval needed: {}",
```

## slack-egress.md: src/slack/mrkdwn.rs:101

```text
    if let Some(last) = chunks.last_mut() {
        let budget = limit.saturating_sub(TRUNCATION_NOTICE.len() + FENCE_CLOSE.len());
        if last.len() > budget {
            let mut end = budget;
            while end > 0 && !last.is_char_boundary(end) {
                end -= 1;
```

## tracker-a.md: src/tracker/seed.rs:92

```text
pub async fn ensure_sessions_team(pool: &SqlitePool) -> Result<String> {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM tracker_teams WHERE key = 'SES'")
            .fetch_optional(pool)
            .await?;
    if let Some(id) = existing {
```

## tracker-a.md: src/tracker/workspace.rs:876

```text
        None => Err(TrackerError::not_found("team")),
        Some(true) => Err(TrackerError::conflict(
            "the agent team's workflow mirrors lane statuses and cannot be edited",
            "system_team",
        )),
        Some(false) => Ok(()),
```

## tracker-a.md: src/tracker/workspace.rs:387

```text
    .await?;
    let members = input.member_ids.unwrap_or_else(|| vec![me.clone()]);
    set_project_links(&pool, &id, input.team_ids.as_deref(), Some(&members)).await?;
    Ok(created(load_project(&pool, &id).await?))
}

```

## tracker-a.md: src/tracker/workspace.rs:769

```text
    };
    let parent_id = patch.parent_id.unwrap_or(current.parent_id);
    if parent_id.as_deref() == Some(id.as_str()) {
        return Err(TrackerError::bad_request(
            "a label cannot be its own parent",
        ));
```

## context-report.md: src/context/pack.rs:405

```text
                Some(id) if id == settings.slack_owner => "owner".to_owned(),
                Some(id) => format!("<@{id}>"),
                // A user-less post (webhook, bot) picks its own display name, so it must never
                // render as "owner": `pack_has_foreign_text` keys the taint check on that label.
                None => format!(
                    "bot {}",
```

## context-report.md: src/report/claims.rs:78

```text
        } else {
            let safe_prefix = hold_incomplete_marker_suffix(chunk_text);
            let (stripped, _) = strip_and_extract_claims(safe_prefix);
            (stripped, Vec::new())
        }
    }
```

## context-report.md: src/report/claims.rs:140

```text
    let mut escaped = false;
    let bytes = s.as_bytes();
    let mut i = 2;
    while i + 1 < bytes.len() {
        let b = bytes[i];
        if escaped {
```

## context-report.md: src/context/pack.rs:95

```text
            }
            let wt_rows: Vec<(String, String)> =
                sqlx::query_as("SELECT repo, path FROM lane_worktrees WHERE lane_key = ?")
                    .bind(key)
                    .fetch_all(pool)
                    .await
```

## context-report.md: src/context/pack.rs:35

```text
impl ThreadReader for SlackApi {
    async fn thread_replies(&self, channel: &str, thread_ts: &str) -> Result<Vec<Value>> {
        self.conversations_replies(channel, thread_ts, None, REPLIES_FETCH_LIMIT)
            .await
    }
}
```

## context-report.md: src/report/status.rs:50

```text
        return;
    }
    tool_registry()
        .lock()
        .insert(session.storage_key(), tool.chars().take(60).collect());
}
```

## context-report.md: src/report/claims.rs:509

```text

```

## migrations-storage.md: migrations/0047_turn_status_messages.sql:1

```text
CREATE TABLE IF NOT EXISTS turn_status_messages (
    session_key TEXT NOT NULL,
    event_id TEXT NOT NULL,
    platform_message_id TEXT NOT NULL,
    started_at DATETIME NOT NULL,
    state TEXT NOT NULL DEFAULT 'running' CHECK (state IN ('running', 'done', 'failed')),
```

## tracker-c.md: src/tracker/lanes.rs:168

```text
            )
            .bind(&issue_id)
            .fetch_optional(pool)
            .await?;
            let status_changed = last_lane_status.as_deref() != Some(lane_status.as_str());
            if !status_changed {
```

## tracker-c.md: src/tracker/lanes.rs:553

```text
        lead: None,
        model,
    };

    let judge = production_judge(pool.clone());
    let guard = SYNC_LOCK.lock().await;
```

## tracker-c.md: src/tracker/sessions.rs:92

```text

async fn current(refresh: bool) -> ApiResult<(Arc<Overview>, Value)> {
    let mut cached = cache().lock().await;
    if !refresh {
        if let Some(collected) = cached.as_ref() {
            if collected.at.elapsed() < CACHE_TTL {
```

## tracker-c.md: src/tracker/threads.rs:359

```text
    lanes::execute_lane_action(&pool, &issue_id, input)
        .await
        .map_err(|error| TrackerError::conflict(error.to_string(), "lane_command_failed"))?;
    Ok(Json(fetch_issue_detail(&pool, &issue_id).await?))
}

```

## slack-transport.md: src/slack/runtime.rs:100

```text
                _ = tokio::time::sleep(RECONCILE_INTERVAL) => {}
            }
            match super::backfill::run_settled(&self.ingress, Some(RECONCILE_SETTLE)).await {
                Ok(0) => {}
                Ok(routed) => tracing::warn!(
                    routed,
```

## slack-transport.md: src/slack/runtime.rs:91

```text
    /// owner replies at 11:50 never arrived and were only picked up by a restart), so the cursor
    /// sweep is what guarantees every message is eventually answered.
    pub async fn reconcile(self: Arc<Self>, shutdown: CancellationToken) {
        if !self.config.missed_backfill {
            return;
        }
```

## slack-transport.md: src/slack/mod.rs:167

```text
            missed_backfill: flag(lookup, "MISSED_BACKFILL", true),
            destructive_slash_confirm: parse_flag(destructive_slash_confirm.as_deref(), true),
            allow_all_users: flag(lookup, "ALLOW_ALL_USERS", false),
        }
    }

```

## autonomy-ledger.md: src/autonomy/loop.rs:89

```text
        let candidates = scan_unseen_candidates(&self.pool, &self.lead_session_key).await?;
        let mut added = 0usize;
        let mut batch = self.pending_batch.lock().await;
        for item in candidates {
            if batch.iter().any(|queued| queued.item_key == item.item_key) {
                continue;
```

## autonomy-ledger.md: src/autonomy/ledger.rs:700

```text
    // tool approval frames die with the process.
    let pending: Vec<ActionProposal> = sqlx::query_as(
        "SELECT * FROM action_proposals
         WHERE state = 'pending' AND origin IN ('engine_approval','tool_approval')
         ORDER BY id ASC",
    )
```

## autonomy-ledger.md: src/autonomy/ledger.rs:895

```text
                &sig_for_scope,
                effect,
                &proposal.target,
                PrecedentDecision::Deny,
                scope,
                "owner_decide",
```

## autonomy-ledger.md: src/autonomy/loop.rs:333

```text
            one_line(&reason, 80)
        );
        out.push(LoopItem {
            item_key: format!("inbox:{item_id}"),
            kind: format!("inbox_{class}"),
            source,
```

## sessions-b.md: src/sessions/external.rs:262

```text
            .as_deref()
            .and_then(|c| c.parse::<i64>().ok());
        let read_cursor = if is_numeric_cursor.is_some() {
            None
        } else {
            adoption.cursor.as_deref()
```

## sessions-b.md: src/sessions/external.rs:626

```text
        .collect();
    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn entry_text(entry: &Value) -> Option<String> {
    let content = entry
```

## sessions-b.md: src/sessions/probe.rs:109

```text
        match target_str.parse::<Target>() {
            Ok(target) => control::transport_argv(&target, &inner_words),
            Err(_) => inner_words,
        }
    }

```

## sessions-b.md: src/sessions/probe.py:202

```text
    matches = [s for s in sessions if target in (s["session_id"], s["routing_id"], s["name"])]
    if len(matches) != 1:
        emit({"ok": False, "error": "session not found" if not matches else "ambiguous session"}, 3)
    return matches[0]


```

## sessions-b.md: src/sessions/probe.rs:264

```text
            })
        };
        map.insert(item.id.clone(), val);
    }

    serde_json::Value::Object(map)
```

## sessions-b.md: src/sessions/cli.rs:221

```text
            let target = match control.resolve(&target).await {
                Ok((resolved, _)) => resolved,
                Err(_) => target,
            };
            let database = database().await?;
            let driver =
```

## sessions-b.md: src/sessions/control.rs:391

```text
        cursor: Option<&str>,
    ) -> Result<Value, ControlError> {
        let (resolved_target, _session) = self.resolve(target).await?;
        let mut words = vec![
            "omo".into(),
            "thread".into(),
```

## agent-daemon-deploy.md: scripts/deploy.sh:212

```text
if [[ "$ONLY_VERIFY" == 1 ]]; then
    # Before the first pinned-path deploy the gateway still runs from target/release.
    if [[ -z "$(gateway_pid)" ]]; then gateway_pid() { pgrep -f "$TARGET_BIN run" | head -1 || true; }; fi
    verify "0000" "" || die "running gateway is not healthy"
    exit 0
fi
```

## agent-daemon-deploy.md: scripts/deploy.sh:329

```text
        loaded_env="$( { launchctl print "gui/${UID:-$(id -u)}/$LABEL" 2>/dev/null || true; } \
            | sed -n '/^\tenvironment = {/,/^\t}/p' | sed -n 's/^\t\t\([^ ]*\) => \(.*\)$/\1=\2/p' | sort)"
        plist_env="$(/usr/libexec/PlistBuddy -c "Print :EnvironmentVariables" "$PLIST" 2>/dev/null \
            | sed -n 's/^ *\([^ ]*\) = \(.*\)$/\1=\2/p' | sort)"
        if [[ -n "$loaded_env" ]] && ! comm -23 <(printf '%s\n' "$plist_env") <(printf '%s\n' "$loaded_env") | grep -q .; then
            :
```

## agent-daemon-deploy.md: scripts/deploy.sh:302

```text
rm -rf "$INSTALL_WEB.old"
INSTALL_WEB="$(cd "$INSTALL_WEB" && pwd)"

# Ensure launchd plist ProgramArguments[0] equals INSTALL_BIN
if [[ -f "$PLIST" ]]; then
    current_arg0="$(/usr/libexec/PlistBuddy -c "Print :ProgramArguments:0" "$PLIST" 2>/dev/null || true)"
```

## agent-daemon-deploy.md: scripts/deploy.sh:221

```text
# currently deployed silently reverts another session's fixes (2026-10-19: the full-access
# and Slack ingress fixes were overwritten twice). Refuse it unless forced.
deployed_rev="$(sed -n 's/^rev=\([0-9a-f]*\).*/\1/p' "$INSTALL_DIR/DEPLOYED" 2>/dev/null | head -1 || true)"
if [[ -n "$deployed_rev" && "${OMON_DEPLOY_ALLOW_REWIND:-0}" != 1 ]]; then
    if git -C "$REPO" cat-file -e "$deployed_rev^{commit}" 2>/dev/null; then
        git -C "$REPO" merge-base --is-ancestor "$deployed_rev" HEAD \
```

## agent-daemon-deploy.md: scripts/deploy.sh:1

```text
#!/usr/bin/env bash
# One-shot production deploy: build -> sign -> restart -> verify -> rollback on failure.
#
#   scripts/deploy.sh            build gateway + dashboard web, sign, restart, verify
#   scripts/deploy.sh --no-build skip both builds, install existing target/release binary and web/dist
#   scripts/deploy.sh --verify   only run the post-deploy checks against the running gateway
```

## tracker-b.md: src/tracker/issues.rs:570

```text
                "state",
                Some(&current.state_id),
                Some(state_id),
            )
            .await?;
            touched = true;
```

## tracker-b.md: src/tracker/issues.rs:802

```text
    let me = me_id(&pool).await?;
    let mut items = Vec::with_capacity(input.ids.len());
    for id in &input.ids {
        let issue_id = resolve_issue_id(&pool, id).await?;
        items.push(apply_patch(&pool, &issue_id, &me, input.patch.clone()).await?);
    }
```

## tracker-b.md: src/tracker/issues.rs:493

```text
        }
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        let number: i64 = sqlx::query_scalar(
            "UPDATE tracker_teams SET issue_seq = issue_seq + 1 WHERE id = ? RETURNING issue_seq",
        )
        .bind(team_id)
```

## tracker-b.md: src/tracker/issues.rs:686

```text
            me,
            "cycle_id",
            "cycle",
            current.cycle_id.clone(),
            cycle,
        )
```

## tracker-b.md: src/tracker/issues.rs:77

```text
    }
    let mut labels: HashMap<String, Vec<String>> = HashMap::new();
    for row in sqlx::query("SELECT issue_id, label_id FROM tracker_issue_labels ORDER BY label_id")
        .fetch_all(pool)
        .await?
    {
```

## agent-backend.md: src/agent/omo_backend.rs:2516

```text
                    event.clone(),
                    deadline,
                    effective_total_timeout,
                    None,
                    &mut fallback_eligible,
                    &mut side_effects,
```

## agent-backend.md: src/agent/omo_backend/follow.rs:185

```text
                }
                "turn/completed" => {
                    let followed = current.is_some() && turn == current;
                    let fresh_replay = current.is_none()
                        && !gateway_turn_running
                        && val
```

## agent-backend.md: src/agent/omo_backend/follow.rs:187

```text
                    let followed = current.is_some() && turn == current;
                    let fresh_replay = current.is_none()
                        && !gateway_turn_running
                        && val
                            .pointer("/params/turn/completedAt")
                            .and_then(Value::as_f64)
```

## agent-backend.md: src/agent/omo_backend.rs:1731

```text
                                        {
                                            active.open_tools = open_tool_ids.len();
                                        }
                                        let ask_started = tokio::time::Instant::now();
                                        let req_id_owned = req_id.clone();
                                        let pool_clone = pool.clone();
```

## agent-backend.md: src/agent/omo_backend.rs:458

```text
                        crate::agent::agent_workspace::ensure_workspace_permission_preset(
                            &omo_dir.join("settings.json"),
                            crate::agent::agent_workspace::approval_ledger_enabled(),
                            crate::agent::agent_workspace::WORKSPACE_PERMISSION_PRESET,
                        )
                        .await;
```

## web-pages.md: web/src/tracker/pages/InboxPage.tsx:106

```text
  useHotkey('u', () => selected && void patch(selected, { read: !selected.read_at }), { enabled: Boolean(selected) })
  useHotkey(['backspace', 'delete'], () => selected && remove(selected), { enabled: Boolean(selected) })
  useHotkey(['j', 'arrowdown'], () => step(1))
  useHotkey(['k', 'arrowup'], () => step(-1))
  useHotkey('h', () => {
    if (!selected) return
```

## web-pages.md: web/src/tracker/pages/ViewPage.tsx:38

```text
  const currentFilters = t.filtersFor(viewKey)
  const currentDisplay = t.displayFor(viewKey, view.display)
  const dirty = !sameJson(currentFilters, view.filters ?? {}) || !sameJson(currentDisplay, { ...DEFAULT_DISPLAY, ...view.display })
  const Icon = iconComponent(view.icon)
  const favorite = t.isFavorite('view', view.id)

```

## web-pages.md: web/src/tracker/pages/SearchPage.tsx:47

```text
  useEffect(() => {
    const query = q.trim()
    if (!query) {
      setRemote(null)
      return
    }
```

## web-pages.md: web/src/tracker/pages/SettingsPage.tsx:130

```text
        <Row title="Interface theme" desc="Select or customize your interface color scheme.">
          <div className="ls-seg" role="radiogroup" aria-label="Interface theme">
            <button type="button" role="radio" aria-checked={false} onClick={() => t.setTheme(system)}>
              <Monitor size={13} /> System
            </button>
            <button type="button" role="radio" aria-checked={t.theme === 'light'} className={t.theme === 'light' ? 'is-on' : ''} onClick={() => t.setTheme('light')}>
```

## web-pages.md: web/src/tracker/pages/ProjectPage.tsx:176

```text
          placeholder="Project name"
          className="text-[26px] w-590 t-1"
          onCommit={(name) => name.trim() && void onSave({ name: name.trim() })}
        />
        <EditableText
          value={project.summary}
```

## web-pages.md: web/src/tracker/pages/MyIssuesPage.tsx:30

```text
        // TODO(FE-C): the issue list payload has no subscriber_ids; creators and assignees are auto-subscribed,
        // so this approximates subscriptions until the API exposes them on Issue.
        return live.filter((issue) => issue.creator_id === meId || issue.assignee_id === meId)
      case 'activity': {
        const since = Date.now() - 14 * 86_400_000
        return live.filter(
```

## cron.md: src/cron/scheduler.rs:2433

```text
            .map_err(|error| OmonError::ToolExecution(format!("cron report file: {error}")))?;
        prune_cron_reports(&dir).await;
        let outcome = if succeeded { "끝남" } else { "실패" };
        let body = format!(
            "크론 {} {outcome}. 전체 출력: {} . 이 출력을 그대로 옮기지 말 것. 직접 읽고 실제 결과(종료 코드, 건수, 로그, 수신 증거)를 확인해서 한국어로 짧게 보고할 것. 실패나 이상이 있으면 원인을 조사하고 할 수 있는 조치를 먼저 한 뒤, 사용자가 직접 해야 하는 일이 있으면 무엇을 왜 해야 하는지 분명히 요청할 것. 알릴 가치가 없으면 [SILENT].",
            job.id,
```

## cron.md: src/cron/scheduler.rs:2640

```text
    let Some(key) = target_session_key else {
        return Ok(false);
    };

    let label = format!("cron:{job_id}");
    crate::mirror::mirror_to_session(pool, &key, "assistant", content, Some(&label)).await
```

## cron.md: src/cron/scheduler.rs:2298

```text
                            if let Some(ack_command) =
                                ack_command.filter(|command| !command.trim().is_empty())
                            {
                                super::ack::run_ack_logged(&ack_command).await;
                            }
                        } else {
```

## cron.md: src/cron/scheduler.rs:2424

```text
            .await
            .map_err(|error| OmonError::ToolExecution(format!("cron report dir: {error}")))?;
        let path = dir.join(format!(
            "{}-{}.txt",
            job.id,
            (self.clock)().format("%Y%m%dT%H%M%S%.3f")
```

## cron.md: src/cron/scheduler.rs:1451

```text
                            // A next_run computed under an older weekday mapping (or before an
                            // expression edit) is not an occurrence of the current expression;
                            // moving it forward is a reschedule, not a missed run.
                            let is_occurrence = next_run_tz(
                                &expression,
                                next_run - TimeDelta::seconds(1),
```

## cron.md: src/cron/executor.rs:636

```text
/// silence token), no delivery can fail, so the ack is run here once the agent
/// run produced a result.
fn payload_session_key(payload: &serde_json::Value, job_id: &str) -> SessionKey {
    let deliver = payload
        .get("deliver")
        .and_then(serde_json::Value::as_str)
```

## cron.md: src/cron/executor.rs:1239

```text
        if pid > 1 {
            // SAFETY: libc::getpgid takes a process ID by value.
            let pgid = unsafe { libc::getpgid(pid) };
            if pgid > 1 && pgid != root {
                // SAFETY: libc::kill with negative pid signals the process group whose pgid equals `pgid`.
                unsafe {
```

## lanes.md: src/lanes/guard.rs:658

```text
                let mem_cleared = sample.mem_free_pct >= self.thresholds.mem_free_min_pct;
                let disk_cleared = sample.disk_free_gb >= self.thresholds.disk_free_min_gb;

                if load_cleared && mem_cleared && disk_cleared {
                    self.current_state = GuardState::Clear;
                    let reported = std::mem::replace(&mut self.episode_reported, false);
```

## lanes.md: src/lanes/runner.rs:122

```text
    }
    let now = Utc::now();
    let changed = sqlx::query("UPDATE lanes SET status = 'working', turns_run = 0, last_turn_outcome = NULL, claim_evidence = NULL, claim_at = NULL, objection = NULL, updated_at = ? WHERE key = ? AND status = ?")
        .bind(now)
        .bind(key)
        .bind(&from)
```

## lanes.md: src/lanes/cli.rs:552

```text
        if let Err(error) =
            super::runner::register_owned_worktree(pool, &lane.key, &tree.repo, &tree.path).await
        {
            tracing::warn!(key = %lane.key, %error, "auto worktree registration failed");
            warnings.push(format!("auto worktree not registered: {error}"));
        }
```

## lanes.md: src/lanes/runner.rs:496

```text
        .map_err(io_error)?;
    let path = lanes_dir.join(key);
    let branch = format!("lane/{key}");
    let target = path.to_string_lossy().into_owned();
    git(&repo, &["worktree", "add", "-b", &branch, &target, "HEAD"]).await?;
    let path = tokio::fs::canonicalize(&path).await.map_err(io_error)?;
```

## web-issues.md: web/src/tracker/issues/BoardView.tsx:172

```text

    const from = groups.find((g) => g.key === fromGroupKey)
    const to = groups.find((g) => g.key === toGroupKey)
    if (!to) return

    let patch: IssuePatch = {}
```

## web-issues.md: web/src/tracker/issues/BoardView.tsx:144

```text
    </div>
  )
}

export function BoardView({ groups, t }: { groups: IssueGroup[]; t: TrackerState }) {
  const c = useCollection()
```

## web-issues.md: web/src/tracker/issues/model.ts:174

```text
  } catch (err) {
    if (err instanceof TrackerApiError && err.code === 'lane_not_verified') {
      toast.error(err.message, {
        action: {
          label: 'Force close',
          onClick: () => {
```

## web-issues.md: web/src/tracker/issues/TimelineView.tsx:60

```text
  const ticks: { label: string; left: number; today: boolean; weekend: boolean }[] = []
  for (let i = 0; i < days; i++) {
    const date = new Date(rangeStart + i * DAY)
    if (date.getDate() === 1 || i === 0) {
      months.push({ label: date.toLocaleDateString('en-US', { month: 'long', year: 'numeric' }), left: i * DAY_W })
    }
```

## slack-interactive.md: src/tools/message_context_slack.rs:262

```text
        let message = self
            .api
            .conversations_replies(&conversation.channel, ts, None, 1)
            .await?
            .iter()
            .find(|message| str_field(message, "ts") == ts)
```

## slack-interactive.md: src/tools/message_context_slack.rs:184

```text
                let mut replies = self
                    .api
                    .conversations_replies(
                        &conversation.channel,
                        thread_ts,
                        None,
```

## web-detail-create.md: web/src/tracker/detail/IssueView.tsx:237

```text
  useHotkey(['mod+shift+,', 'mod+shift+<'], actions.copyBranch, { enabled: ready })
  useHotkey('mod+shift+c', actions.copyLink, { enabled: ready })
  useHotkey('mod+shift+a', actions.toggleArchive, { enabled: ready })
  useHotkey('mod+backspace', actions.askDelete, { enabled: ready })
  useHotkey(['j', 'arrowdown'], () => goTo(1), { enabled: ready && mode === 'page' && !embedded && picker === null })
  useHotkey(['k', 'arrowup'], () => goTo(-1), { enabled: ready && mode === 'page' && !embedded && picker === null })
```

## web-detail-create.md: web/src/tracker/detail/Activity.tsx:246

```text
            label="Copy link to comment"
            icon={<Copy size={14} />}
            size="sm"
            onClick={() => {
              // Link to the issue itself: the hash router has no place for a second fragment.
              const issue = t.issuesById.get(comment.issue_id)
```

## web-detail-create.md: web/src/tracker/detail/Editors.tsx:102

```text
        setSaveState('idle')
        notifyError(err)
      })
  }

  // Closing the peek by clicking outside unmounts the editor before blur fires: flush the
```

## web-detail-create.md: web/src/tracker/detail/AgentPanel.tsx:23

```text
}

const ALLOWED: Record<LaneStatus, LaneAction[]> = {
  queued: ['send', 'close'],
  working: ['send', 'close'],
  blocked: ['send', 'close'],
```

## web-detail-create.md: web/src/tracker/detail/Editors.tsx:92

```text
    window.clearTimeout(timer.current)
    if (next === lastSaved.current) return
    const previous = lastSaved.current
    lastSaved.current = next
    setSaveState('saving')
    updateIssues([issueId], { description: next })
```

## web-detail-create.md: web/src/tracker/create/CreateIssueModal.tsx:65

```text

function initialForm(draft: CreateDraft, t: ReturnType<typeof useTracker>, fallbackTeam: ID): Form {
  const team_id = draft.team_id && t.teamsById.has(draft.team_id) ? draft.team_id : fallbackTeam
  const state = draft.state_id && t.statesById.get(draft.state_id)?.team_id === team_id ? draft.state_id : null
  return {
    team_id,
```

## web-detail-create.md: web/src/tracker/detail/SessionAgentPanel.tsx:33

```text
function shortenPath(path: string | null): string {
  if (!path) return ''
  return path.replace(/^\/Users\/indo(?=\/|$)/, '~')
}

export function SessionAgentPanel({
```

## slack-commands.md: src/slack/commands.rs:436

```text
    lines.join("\n")
}

/// DMs (`D…`) are per-user sessions; channels are shared per team, matching ingress keys.
pub fn session_key(invocation: &SlashInvocation, bot_user_id: &str) -> SessionKey {
    let key = if invocation.is_dm() {
```

## slack-commands.md: src/slack/commands.rs:610

```text
        }
    }

    async fn execute(&self, invocation: &SlashInvocation, command: SlackCommand) -> Result<Reply> {
        let mut key = session_key(invocation, &self.bot_user_id);
        // At a channel's top level the slash payload carries no thread, so `key` is the
```

## slack-commands.md: src/slack/commands.rs:137

```text
            user_id: field("user_id"),
            channel_id: field("channel_id"),
            team_id: field("team_id"),
            response_url: optional("response_url"),
            thread_ts: optional("thread_ts"),
        }
```
