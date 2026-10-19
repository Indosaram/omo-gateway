<!-- task=st_01a1451a -->
st_01a1451a [completed] model mahoquot/claude-sonnet-5-5
Ask: research x.ai Grok Bot pages (guides, 101, automations, team-bots, pricing, BI leak incident) - wanted: a sourced report with design patterns for a Rust agent gateway. For you: every assigned source was read except the automations URL, which is a 404 (the real page is `docs.x.ai/grok-bot/skills-routines-and-automations`). Now: report delivered. Next: none.

**Routes used:**
- Plain curl to x.ai returns the Next.js shell, truncated.
- `r.jina.ai/<url>` was the working route for almost everything. It rate-limits per IP, so I retried each fetch until it returned.
- hn.algolia.com worked for the incident thread.
- archive.is hit a CAPTCHA, and `www.businessinsider.com` was blocked on r.jina.ai. The BI Africa mirror via r.jina.ai worked.
- I did not try web.archive.org or Reddit, because the other routes sufficed.

## Findings

**Routines and triggers**
- Bots are reached three ways: chat, routines/triggers (a bot sets its own schedule or listens to Slack/GitHub events), and other bots. [https://x.ai/bot/guides/grok-bot-101] PRIMARY, jina
- Limits: 50 routines per bot, the last 20 run records kept per routine, and schedules at least 5 minutes apart. Event triggers are separate from the Slack/GitHub plugins and may need their own connection. Docs say to avoid broad listeners and to give every routine a stale-data policy and idempotent retries. [https://docs.x.ai/grok-bot/skills-routines-and-automations] PRIMARY, jina
- Webhook-started routines receive the request body (0.27.0). Routines can be paused and resumed (0.52.0). A GitHub routine can watch one PR and removes itself when the PR merges or closes (0.25.0). Routines may be auto-paused after a long absence. [https://x.ai/changelog/bot] PRIMARY, jina (changelog truncated at v0.25)
- Engineering guide examples:
  - A 30-minute PR-sweep routine checks Bugbot findings, CI and merge conflicts. It auto-merges only when review confidence is high and blast radius is low.
  - A 3 a.m. nightly audit.
  - A "P0" temporary 5-minute transcript-watch routine, with a warning that it burns tokens.
  - A 5 a.m. "ops bot" that holds 1:1s with the other bots and runs postmortems that update a shared playbook.

  [https://x.ai/bot/guides/grok-bot-for-engineering] PRIMARY, jina
- Support guide: refunds are reviewed against policy and then wait for human approval. An hourly alert routine is the "self-driving" example. [https://x.ai/bot/guides/grok-bot-for-support] PRIMARY, jina

**Plugins, connectors, MCP**
- Grok Bot supports the same MCP servers, plugins and skills as Cursor. Multiple accounts per service are allowed. [https://x.ai/bot/guides/grok-bot-101] PRIMARY, jina
- The product is built on Cursor infrastructure. Docs reference the Cursor account, dashboard and privacy policy, and the download is an `api2.cursor.sh` URL. [https://docs.x.ai/grok-bot/approvals-security-and-privacy] PRIMARY, jina
- Bots can claim their own email address (0.70.2). Outgoing mail from it waits on a draft card for your approval. [https://x.ai/changelog/bot] PRIMARY, jina
- I found no mention of AgentMail in any page I retrieved. The HN search for "AgentMail Grok Bot" returned nothing. The native bot-email feature may be the same thing, but that is unverified. [https://hn.algolia.com/api/v1/search?query=AgentMail%20Grok%20Bot&tags=comment] direct
- Team Bot plugin auth, by type:

  | Plugin type | Whose access the bot uses |
  |---|---|
  | OAuth | The person asking |
  | Key or token | The bot's own credential |
  | Remote HTTPS MCP | Bot credential, or per-user OAuth |
  | Command MCP | Runs on the computer that conversation uses; secrets allowed only in the owner's chat |

  [https://docs.x.ai/grok-bot/team-bots] PRIMARY, jina
- Team Bot secrets: up to 25 per bot, values at least 8 characters and at most 4,096 bytes. Values are encrypted, never visible to the bot, and replaced with `[REDACTED]` in command output, files and plugin calls. [https://docs.x.ai/grok-bot/team-bots] PRIMARY, jina

**Per-bot permissions**
- The 101 guide says rules are written in natural language in Settings → General → Agent. A separate review agent can allow, block or escalate. The guide says "you are trusting the model." Another line says that if you log into a site with one bot, every other bot can reach it. [https://x.ai/bot/guides/grok-bot-101] PRIMARY, jina
- The docs are more precise. Auto Review has "Ask first" and "Allow automatically" rules, and Ask first wins on a conflict. Admin team rules are locked and personal rules can only make things stricter. [https://docs.x.ai/grok-bot/approvals-security-and-privacy] PRIMARY, jina
- Approval cards for unattended work (routine, trigger, message from another bot) expire after about 10 minutes and do not run. An expired card can add an Always-allow rule without running the action. [same page] PRIMARY, jina
- All your bots share one cloud computer per user, and the docs say "Do not use separate Bots as a security boundary." [same page] PRIMARY, jina
- Execution on the local computer is Ask every time / Always allow / Never allow, per machine. [same page] PRIMARY, jina
- Changelog hardening:
  - Before an irreversible action (send, post, buy, delete) the bot checks your own messages for approval (0.70.2).
  - After a bank or brokerage connector returns data, the bot asks before sharing it elsewhere (0.70.2).
  - Email and Slack drafts need approval by default, with "Disable drafts for this Bot" as an opt-out (0.64.0, 0.45.0).
  - Slack approvals post a card only you can answer (0.63.0).
  - When more than one account could send, the bot asks which address to use (0.70.2).

  [https://x.ai/changelog/bot] PRIMARY, jina

**Team bots**
- Team Bots shipped 2026-09-26 (v0.61.0) and were announced 2026-09-28, in public beta on Teams and Enterprise. They bundle context, plugins, credentials and memories. Each person's chats and memories stay private. [https://x.ai/news/team-bots] PRIMARY, jina
- Memory splits into team memory (read by everyone) and private per-person notes. [https://docs.x.ai/grok-bot/team-bots] PRIMARY, jina
- Routines are personal, run as the person who made them, and never run team-wide. [same page] PRIMARY, jina
- Compute placement: owner chat runs on the owner's machine, a teammate's 1:1 on theirs, and Slack channels and threads on one shared machine. The docs warn that everything in a channel thread is visible to everyone in it. [same page] PRIMARY, jina
- In shared conversations the bot must ask before using your personal connectors. The options are Allow once, Always for this bot, Always for all Team Bots, and Skip. [same page] PRIMARY, jina
- Where nobody can answer an approval card, the bot runs without Auto-review within its set-up permissions, unless the team requires Auto-review. [same page] PRIMARY, jina
- Slack bots answer only users who have linked their Slack identity. Usage bills to the asking teammate. [same page] PRIMARY, jina
- The SDR guide describes "CLEAR vs FLAG" intake, "nothing auto-sends", and one bot per job. [https://x.ai/bot/guides/grok-bot-for-sdrs] PRIMARY, jina
- The legal-ops guide describes draft-then-human-send, preview and confirmation before any write to a contract system, and read paths separated from write paths. [https://x.ai/bot/guides/grok-bot-for-legal-ops] PRIMARY, jina

**Pricing** (all one subscription)

| Tier | Price | Notes |
|---|---|---|
| Free | $0 | Limited |
| Plus | $8/mo | Shared usage pool |
| Premium | $30/mo | 4x Plus; Grok Bot first listed here |
| Super | $100/mo | 15x |
| Ultra | $200/mo | 50x |
| Teams | $30+/user/mo | Pooled |
| Enterprise | Custom | |

- Usage is shared across Grok, Cursor and Bot, and refreshes weekly. Top-ups are available.
- "Access, network, and audit controls for Grok Bot" appears as a row under Team management. The per-tier checkmarks were lost in the extraction, so I can't say which tiers include it.

[https://x.ai/pricing] PRIMARY, jina

**BI incident**
- On Oct 1, Shane Mac (CEO of XMTP Labs) had a personal "CFO" bot with read-only bank access. Its monthly audit went to Slack channel "Exec-team" instead of his personal group chat. His head of product noticed. [https://www.businessinsider.com/personal-ai-agent-grok-bot-posted-bank-details-company-slack-2026-10, text via https://africa.businessinsider.com/news/my-personal-ai-agent-posted-my-bank-details-on-company-slack-its-made-me-rethink-how/hw9t2xn] SECONDARY (as-told-to essay), jina
- He says it was a same-name channel confusion, and that all his agents use the same underlying connections. He says Grok shipped a fix requiring explicit user permission before data moves to other channels. [same] SECONDARY
- I have no first-party x.ai post-mortem. The 0.70.2 changelog lines are consistent with the fix but not confirmed as that fix (inference).
- HN thread, 63 points and 70 comments. The most useful comment: prompts can't stop leaks, so the agent for one domain must be deterministically blocked from other domains. Another: reference channels by permanent ID, not name. [https://hn.algolia.com/api/v1/items/50033517] SECONDARY, direct

## Design patterns worth copying into a Rust agent gateway

1. **Effect classes on every tool call.** Tag each call `Read | Draft | Write | Irreversible | Money` and key policy on the class.
   - `Irreversible` and `Money` always need a human or "your own message" approval.
   - Bank-class connector output gets a taint flag.
2. **Taint and egress rule.** Data tagged `Sensitive(source)` may flow only to destinations on an allowlist. Enforce it in the gateway as a hard check, not in the prompt. This directly prevents the BI leak.
3. **Destination binding by immutable ID.** Slack channels, emails and chats are bound to `(workspace_id, channel_id)` at config time. Reject any display-name-only target. When two destinations share a name, fail closed and ask.
4. **Per-bot credential scoping.** Don't copy their shared-computer model, since their docs admit bots aren't a security boundary. Give each bot its own `ConnectionGrant { bot_id, connector, account, scopes, allowed_destinations }`, its own filesystem, and its own browser profile.
5. **Approval state machine.** `Proposed → Pending(expires_at) → Approved | Denied | Expired`. Interactive requests wait. Unattended ones (routine, trigger, bot-to-bot) expire in about 10 minutes and do not run. Offer an "add allow rule" action on expiry. Rules are `Ask | Allow`, and Ask wins. Layered rules (admin, then personal) can only tighten.
6. **Rule matching should be deterministic first.** A structured matcher over tool, target and scope runs before any LLM reviewer. The reviewer is an extra layer that can only tighten.
7. **Routine record.**

   ```rust
   struct Routine {
       owner_bot: BotId,
       run_as: UserId,
       trigger: Cron | Webhook | EventFilter,
       min_interval: Duration,   // 5m floor
       on_missing_data: Fail,
       approval_policy: ...,
       delivery_target: DestId,
       paused: bool,
   }
   ```

   Cap routines per bot (50) and run history (20). Event triggers need a narrow filter, and a "match everything" filter is rejected. Persist an idempotency key per run.
8. **Draft-by-default outbound.** Email and chat sends produce a `Draft` object. Sending needs approval, and the opt-out is per bot and explicit. Include a "which identity am I sending as" step.
9. **Team and shared mode if you ever add it.**
   - OAuth plugins use the asker's identity. Key plugins use a bot credential.
   - Secrets are referenced by name and redacted from all tool output.
   - Memory is split into team and per-person scopes.
   - Shared-room conversations require a per-person consent card for personal connectors.
10. **Secure handoff.** Passwords, 2FA and payments go through a masked secret form or a human takeover, never through chat or model context.

## CLAIMS

- CLAIM: Grok Bot permissions are natural-language rules checked by a model reviewer. RISK: Prompt-level control can be talked around; their own guide says "you are trusting the model." SOURCES: 101 guide, approvals doc. PRIMARY: yes.
- CLAIM: All of a user's bots share one cloud computer, and logins carry across bots. RISK: A bot is not a security boundary, which is the root cause class of the BI leak. SOURCES: approvals doc, 101 guide. PRIMARY: yes.
- CLAIM: The Oct 1 leak was caused by a same-named Slack channel ("Exec-team"), and Grok then required explicit permission to move data across channels. RISK: The account is first-person and I found no x.ai post-mortem. SOURCES: BI essay (Africa mirror). PRIMARY: no.
- CLAIM: 0.70.2 added "ask before sharing bank/brokerage connector data elsewhere." RISK: I'm inferring it is the incident fix, with no vendor statement. SOURCES: changelog. PRIMARY: yes (for the line itself).
- CLAIM: Team Bots are in public beta on Teams and Enterprise, and use the asker's OAuth identity. RISK: Beta behavior may change. SOURCES: team-bots news post, docs. PRIMARY: yes.
- CLAIM: Unattended approvals expire in about 10 minutes and do not run. RISK: Silent task failure, but this is the safe default. SOURCES: approvals doc. PRIMARY: yes.
- CLAIM: Grok Bot is first listed at the Premium tier ($30/mo). RISK: The tier-by-tier checkmarks were stripped, so enterprise controls are unattributed. SOURCES: pricing page. PRIMARY: yes.
- CLAIM: AgentMail is integrated. RISK: Unverified. I found only native bot email addresses. SOURCES: changelog 0.70.2 only. PRIMARY: n/a.

## EXPAND

- Fetch the full changelog beyond v0.25.0. The page was truncated, and the email and trigger history is probably there.
- Fetch `docs.x.ai/grok-bot/teams-and-enterprises` for the connector policy, Team Rules and Auto-review enforcement details.
- Fetch `docs.x.ai/grok-bot/bots` and `computer-and-apps` for any trigger or webhook specifics.
- Pull the BI follow-up on users deleting agents (`businessinsider.com/early-users-delete-personal-ai-agents-privacy-scares-blunders-2026-10`) once the r.jina.ai block on that domain lifts (about 14:36 UTC).
- Search for any first-party x.ai statement on the incident.