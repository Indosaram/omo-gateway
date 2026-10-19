<!-- task=st_01a14517 -->
st_01a14517 [completed] model mahoquot/claude-sonnet-5-5
Ask: fetch the OpenAI Dots primary pages and confirm eight scope items - wanted: a sourced report with the retrieval route per source. For you: the two openai.com pages and the YouTube description were not retrievable. OpenAI's own docs mirror (learn.chatgpt.com) was, and it confirms most items. "4000+ apps" is unconfirmed. Now: done. Next: none.

## Findings

**Retrieval routes**
- openai.com/index/introducing-dots/: direct curl returned a Cloudflare challenge. The jina.ai route returned an abuse block for openai.com. The Wayback route overflowed the output buffer. Not read directly.
- openai.com/index/how-we-build-safety-security-and-privacy-into-dots/: not fetched directly, same Cloudflare block. Its existence is confirmed only via HN item 49897058, which has no content.
- help.openai.com/en/articles/20001530-getting-started-with-your-dot: Cloudflare challenge. Its existence is confirmed via HN story 49896565.
- YouTube uXspbC2srEQ: the watch page overflowed the buffer, and the Invidious route returned 403. oEmbed worked but gives only the title, "Introducing dots, always-on agents built to handle everything." (OpenAI channel). **The description was not retrieved.**
- learn.chatgpt.com `.md` twins: **worked with plain curl**. These are OpenAI's own docs, so I treat them as PRIMARY, though they are not the four pages you named.
- hn.algolia.com: worked, used as SECONDARY.

**Scope items**
- **DevDay link: confirmed.** OpenAI's DevDay 2026 page, dated September 29, 2026, lists "Meet your dot" under ChatGPT. PRIMARY. [https://learn.chatgpt.com/docs/whats-new/devday-2026.md] (direct curl)
  - HN comment 49898200 calls DevDay "lackluster" in the thread on the launch post. SECONDARY. [https://hn.algolia.com/api/v1/search?query=DevDay&tags=comment,story_49896604] (HN API)
- **4000+ apps: not found.** Searches for "4000", "4,000", "thousands of apps" and "apps" in the 647-comment HN thread returned no quote of it. The docs say only that dots use "supported plugins installed and enabled." I could not read the blog text where it presumably appears.
- **Auto-review: confirmed, and larger than "a check."**
  - Dots doc: before an action that could affect accounts or share information, an automatic review checks it against instructions, permissions, custom rules and built-in safety. The outcome is proceed, needs approval, or "a step you must do yourself." PRIMARY. [https://learn.chatgpt.com/docs/dots/controls.md] (direct curl)
  - The Codex implementation is a separate reviewer agent. It is triggered by escalation requests (sandbox escapes, blocked network, out-of-root edits, MCP tools requiring approval). It sees a compact transcript plus the exact request, not hidden reasoning.
  - A denial tells the main agent not to work around it. A per-turn circuit breaker fires at 3 consecutive denials or 10 in the last 50 reviews. `/approve` grants one retry of the exact denied action, which is still re-reviewed. PRIMARY. [https://learn.chatgpt.com/docs/sandboxing/auto-review.md] (direct curl)
  - Caveat: this page describes Codex. That Dots uses the same breaker numbers is my inference.
- **Read-only proactive research: confirmed.** Research tools "can't send messages, change app content, or control your browser or computer." Findings are private notes. The background agents report to the dot, and any follow-up action goes through normal permissions. PRIMARY. [https://learn.chatgpt.com/docs/dots/tasks-and-memory.md] (direct curl)
  - The blog sentence is quoted verbatim in HN comment 49897554. SECONDARY. [https://hn.algolia.com/api/v1/search?query=read-only&tags=comment,story_49896604] (HN API)
  - The Register reported a dot burning 300K+ tokens in a day with no prompt (155 traces, multiple subagents, read-only permissions). SECONDARY, I saw only the headline and metadata. [https://www.theregister.com/ai-and-ml/2026/10/13/openais-dot-assistant-burned-300k-tokens-in-a-day-without-a-single-prompt/5302913] (direct curl, body truncated)
- **Scheduling and triggers: confirmed.**
  - A dot can decide when to pause and wake itself.
  - Fixed schedules need a time zone and an end date, and the dot is asked to confirm what it saved. They appear under "Scheduled."
  - Event monitoring is available only where the connected service supports it, and "Adding it to a channel alone doesn't start monitoring."
  - PRIMARY. [https://learn.chatgpt.com/docs/dots/tasks-and-memory.md] (direct curl)
  - DevDay also lists MCP Events, which uses webhook delivery and requires MCP 2.0. PRIMARY. [https://learn.chatgpt.com/docs/whats-new/devday-2026.md] (direct curl)
- **Memory: confirmed.** The dot starts from ChatGPT memory and keeps its own notes. The notes are separate from ChatGPT saved memory and are not a transcript. They are shared across channels. Disconnecting an app does not delete what was already learned. PRIMARY. [https://learn.chatgpt.com/docs/dots/tasks-and-memory.md] (direct curl)
  - HN user mike_hearn says the session "compacts continuously" and keeps a diary and a topic wiki in its home directory. This is anecdotal. SECONDARY. [https://hn.algolia.com/api/v1/search?query=memory&tags=comment,story_49896604] (HN API)
- **Pricing and limits: partly confirmed.**
  - Eligible plans are Pro 100/200/500 (over 18, outside EEA/UK/Switzerland), Business Premium, and Enterprise (off by default).
  - Conversations with the dot don't count toward ChatGPT usage limits. Work and Codex tasks it starts do count. There is an "allowance for deeper work" with extended limits for the first month.
  - PRIMARY. [https://learn.chatgpt.com/docs/dots.md] (direct curl)
  - Pro 500 is $500/month. PRIMARY. [https://learn.chatgpt.com/docs/whats-new/devday-2026.md] (direct curl)
  - A subscriber email quoted on HN says Pro 200 usage drops from 20x to 10x Plus on Oct 30, and Pro 500 gives 25x. SECONDARY. [https://hn.algolia.com/api/v1/items/49901067] (HN API)
  - I found no per-dot token or compute cap in the docs.
- **Slack and Teams: confirmed, with a conflict.**
  - The dots doc says you can message the dot in Slack and Teams. By default it replies only to you.
  - It cannot be woken by other people. Only the owner's DM or mention starts work, though it may read others' messages as context.
  - Admins control "Add dots to Slack."
  - PRIMARY. [https://learn.chatgpt.com/docs/dots/channels.md] and [https://learn.chatgpt.com/docs/enterprise/dots-admin-guide.md] (direct curl)
  - **Conflict:** the admin guide says Teams is "limited to an invite-only alpha," while the consumer docs read as generally available.
  - Texting is listed as "coming soon."
- **Microsoft Agent 365: SECONDARY only.** HN comment 49898108 quotes the blog: "working with Microsoft to integrate specialist dots with their enterprise governance and security controls in Agent 365." I found nothing in the learn.chatgpt.com docs. [https://hn.algolia.com/api/v1/search?query=Agent%20365&tags=comment,story_49896604] (HN API)

## Design patterns worth copying into a Rust personal-agent gateway

1. **Four-valued action decision.** The enum is `Allow | AskUser | HandOff | Deny`, with `AskUser` carrying the exact action hash. `HandOff` is for steps the agent can never do, such as password changes or 2FA codes. Dots has a distinct hand-off verdict, and HN user neom reports it will not type a 2FA code even when it has email access.
2. **Custom rules as data, not authority.** Store `Rule { scope, mode }` with modes `Auto | OnExplicitRequest | Ask | HandOff`. Evaluate order is hard-coded deny list, then the rule engine, then the reviewer. Rules can only narrow or confirm; they never override the built-in deny list, and they never grant app access. Dots rejects overly broad rules at save time (neom's report, SECONDARY), so I would add a rule linter.
3. **Draft is not send.** Treat "draft" and "send" as separate capabilities. An instruction authorizes only actions inside its stated scope (who, what, when). Anything outside re-prompts.
4. **Reviewer as a separate process.** Run it with a narrower prompt and no chain-of-thought. Give it a transcript tail plus the exact tool call. A denial is a typed result with a rationale. The main agent gets a "do not work around this" instruction.
5. **Circuit breaker per turn.** Keep a `VecDeque<bool>` of the last 50 verdicts plus a consecutive-denial counter. Abort at 3 consecutive or 10 of 50. Treat timeouts separately: a timeout is not evidence of danger.
6. **One-shot override token.** Key `/approve` on `hash(tool, args, cwd)`. It is valid for one retry, recorded per task (cap about 10 recent denials), and still passes through the reviewer.
7. **Proactive-research tool profile.** Build a second tool registry with only read-only tools, enforced by type: a `ReadTool` trait with no `ReadTool -> WriteTool` conversion. Background agents write only to a private notes store and surface suggestions to the main agent. Never let this registry share credentials scopes with the action registry.
8. **Task state machine.** `Pending -> Running -> WaitingForUser{decision|signin|approval|app} -> Running -> Done|Failed|Cancelled`. Keep "pause main", "stop delegated task" and "cancel schedule" as three separate controls, as Dots does.
9. **Schedules need a contract.** The schema is `{instruction, cron, tz, ends_at, notify_when, deliver_to}`. The agent must echo the saved row back for confirmation. Event triggers are separate rows that require a source that supports subscription. Joining a channel never implies a monitor.
10. **Channel isolation.** Keep one agent identity and one notes store across channels, but separate transcripts. Cross-channel disclosure needs an explicit `audience_ok` check. Only the owner's identity can start work. Third-party messages become context only.
11. **Credential handoff outside the model.** Use a sign-in request that opens a private form posting straight to the browser worker, so the secret never enters the model context. Reusing a saved login still requires confirmation.
12. **Disclose cost.** Always-on loops burn tokens, as The Register's 300K-token result suggests (SECONDARY, headline only). Give each dot a daily token budget and a log of wake-ups.

## CLAIMS

| CLAIM | RISK | SOURCES | PRIMARY |
|---|---|---|---|
| Dots were announced at DevDay on 2026-09-29 | Low | DevDay page; HN 49898200 | Yes (docs mirror) |
| "4000+ apps" is stated on the launch page | Unverified | Not found anywhere I could reach | No |
| Auto-review gates actions that affect accounts or share information | Low | dots controls doc; auto-review doc | Yes (docs mirror) |
| Dots Auto-review uses the Codex reviewer and its 3/10-of-50 breaker | Medium: inferred, the doc describes Codex | auto-review doc | Partial |
| Proactive research tools are read-only and cannot send, change or control | Low | tasks-and-memory doc; blog quote via HN 49897554 | Yes (docs mirror) |
| Scheduling needs a time zone and end date; event triggers need source support | Low | tasks-and-memory doc | Yes (docs mirror) |
| Memory is separate dot notes plus ChatGPT memory | Low | tasks-and-memory doc | Yes (docs mirror) |
| Dots is included in Pro 100/200/500, Business Premium and Enterprise; chat does not count toward limits | Medium: the "allowance for deeper work" is undefined | dots doc | Yes (docs mirror) |
| Pro 500 costs $500/month; Pro 200 allowance halves on Oct 30 | Medium: email quoted on HN | DevDay page; HN 49901067 | Partial |
| Slack is supported and Teams is general | High: the admin guide says Teams is an invite-only alpha | channels doc; admin guide | Yes (docs mirror), conflicting |
| Microsoft Agent 365 integration is planned | Medium: quote relayed on HN only | HN 49898108 | No |
| A dot burned 300K+ tokens in a day with no prompt | Medium: vendor test per headline, article body not read | The Register | No |

## EXPAND

- The openai.com launch and safety posts, the help-center article and the video description still need a browser session that can pass Cloudflare. They would settle the "4000+ apps" and Agent 365 wording.
- Worth fetching next: `learn.chatgpt.com/docs/enterprise/chatgpt-slack-and-teams.md`, `teams.md`, `agent-security.md`, `chatgpt-work-cloud-security.md`, `pricing.md`, and `dots-october-9-2026.md` (already retrieved, no new claims). Also the Codex reviewer policy at `codex-rs/core/src/guardian/policy.md` via `gh api`.
- The open-source Dots clones from HN (diggerhq/opendots, Anil-matcha/open-dots, ychampion/melete) are worth reading for Rust-gateway design.