<!-- task=st_01a144fb -->
st_01a144fb [completed] model mahoquot/claude-sonnet-5-5
Ask: Research xAI's Grok Bot (trust, safety, economics) - wanted: a sourced report. For you: the product is verified from primary sources, but retention, regions and audit-log details are not publicly documented. I could not find "Automations" safety docs beyond the webhook page. Now: done. Next: none.

I did not have `web_search`. I used curl against x.ai, docs.x.ai, HN Algolia and a few third-party pages. I read the x.ai blog and /bot page via r.jina.ai because x.ai blocks direct curl behind Cloudflare. Some pages hit rate limits (r.jina.ai 429, businessinsider.com blocked). I did not read the x.ai pricing, news-index, guides or Automations overview pages.

## Identity
- **Name:** Grok Bot, "your team of always-on agents". It is the xAI "bot" product.
- **Company:** the x.ai site brands itself "SpaceXSI". A third-party writeup says "SpaceXAI", and a talk transcript says the team works "at SpaceX". Treat these as the same entity.
- **Official URLs:** https://x.ai/news/introducing-grok-bot and https://x.ai/bot. Automations docs are at docs.x.ai/grok/automations/*.
- **Launch:** beta announced 2026-08-11 (blog dateTime). The page also lists later additions, such as its own email inbox on 2026-10-09.
- **Confidence:** high on identity and launch date, because I read the primary page. The product's download links and contact-sales form point at Cursor domains, which suggests Cursor-built infrastructure. That is my inference.

## Findings
- **Launch scope.** Beta on desktop and iOS for SuperGrok / Plus / Heavy, Cursor Pro / Pro+ / Ultra, and Cursor Teams Standard / Premium. Grok Bot has its own usage pool, separate from Grok and Cursor plans. Enterprise is waitlist only. [https://x.ai/news/introducing-grok-bot]
- **Pricing correction.** The /bot page lists three standalone tiers: Premium $30/mo, Super $100/mo (3.5x Premium usage, priority at peak) and Ultra $200/mo (12x usage, earliest access). I found no ~$120/month tier, so that figure is unverified. The page says eligible plans include Grok Bot. Usage is "weekly" and the page gives no absolute caps. [https://x.ai/bot]
- **Execution model.** Bots get a cloud computer so jobs continue while you're away. They sign into apps and sites, including ones with no API or MCP, and "only come back when something needs your approval". [https://x.ai/news/introducing-grok-bot]
- **Sandbox.** Per Cursor's Lee Robinson at Stanford CS146S, the agent computer is a Firecracker microVM. It is snapshotted and disposable, and is described as "secured and locked down". The agent loop runs on the server, not in the VM. [https://www.peterp.me/transcripts/how-grok-bot-works/]
- **Shared computer (secondary source).** All of one user's bots share one computer: the same files, the same installed tools and the same browser logins. Each bot gets only its own desktop and browser window. [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/]
- **Action review.** A separate model reviews actions before they run, mostly shell commands ("auto review"). Robinson claims this beats human approval fatigue. This is a vendor claim with no published eval. [https://www.peterp.me/transcripts/how-grok-bot-works/]
- **Prompt injection.** Robinson says untrusted inputs (emails, webhooks) should not be treated as user instructions, and calls the industry's injection protection work still in progress. The secondary writeup states more firmly that external content is "always" treated as data. xAI's own docs do not say this. [https://www.peterp.me/transcripts/how-grok-bot-works/]
- **Approval gates.**
  - Irreversible actions (email, Slack) come back as a draft to edit and approve.
  - Passwords can be kept outside the model conversation (one-password style), with Stripe Link / X Money for payments.
  - The marketing page shows "You're in control" and a review-list flow for outbound drafts.
  - Robinson's escalation ladder is: context, then plugin / API, then web search, then browser, then desktop, and only then ask the user.
  - Payment-specific approval policy is not documented. [https://www.peterp.me/transcripts/how-grok-bot-works/] [https://x.ai/bot]
- **Real incident.** In early October 2026 a CEO's "CFO" bot with read-only bank access posted a monthly personal-finance report to the wrong Slack channel. It chose a channel named "Exec-team" over his private agent group chat, because the names matched. All of a user's agents shared the same underlying connections. xAI said it shipped a fix requiring explicit user permission before agents move information across channels. The CEO then disconnected everything. Read-only data access did not prevent the leak. The cause was a destination ambiguity, with no model or injection failure involved. [https://africa.businessinsider.com/news/my-personal-ai-agent-posted-my-bank-details-on-company-slack-its-made-me-rethink-how/hw9t2xn]
- **Community reaction.** HN comments split between "user error" and "product fault". Practical suggestions were hard network and domain separation, an outbox for world-actions, and permanent IDs instead of channel names. This is opinion. [https://news.ycombinator.com/item?id=50033517]
- **Agent identity and spoofing.** Grok Bot signs browser requests with Web Bot Auth (Signature, Signature-Input, Signature-Agent headers). A researcher showed that code inside the bot's VM could extract an inference renewal credential from a process's environment and mint valid signatures for any origin, so other traffic could impersonate the bot. Disclosed 2026-09-23, published 2026-10-07, acknowledged, and no fix was reported at publication. The same post shows the VM's start script unsets the gateway token, but a helper process still holds it. [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/]
- **Email.** Since 2026-10-09 a bot can claim a mail.grokbot.com address. Third-party coverage says xAI documents no send caps, retention or send approval for this inbox. It is one permanent address per user, shared across bots, and there is no webmail to inspect it. Team admins must enable it. This comes from a vendor blog quoting X posts, so treat it as secondary. [https://shipmail.to/blog/grok-bot-email-on-your-own-domain]
- **Automations webhooks.**
  - Each trigger has its own endpoint plus a `whsec_` signing secret, shown only once and rotatable.
  - Requests use Standard Webhooks (HMAC-SHA256 over `id.timestamp.body`).
  - Timestamps more than 5 minutes off are rejected, and bodies are capped at 1 MiB.
  - A valid request returns 202 and runs asynchronously. There is 429 rate limiting and replay protection via timestamp.
  - [https://docs.x.ai/grok/automations/webhooks]
- **Data and state.**
  - Conversation history, routines and workflow steps live in a server database. Files, memory and skills are backed up to object storage.
  - Compaction affects only the model's working context, not the stored history.
  - The agent loop is a durable Temporal workflow (secondary writeup).
  - Retention periods, training use and deletion controls were not found.
  - [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/]
- **Availability.** The product runs on desktop (macOS download) and iOS. Android and regional limits were not found. Slack is a supported surface (Robinson's talk). [https://x.ai/news/introducing-grok-bot]
- **Models and training (secondary, low confidence).** Elon Musk's tweet is cited on HN as saying Grok Bot will use Opus 5.5 and Midjourney. I did not verify it. Robinson says the team trains models on collected failures every few months. [https://twitter.com/elonmusk/status/2107724314451878104]

## Implications for a self-hosted personal agent gateway
Ideas for the Rust daemon, not claims about Grok Bot:
1. **Outbox with draft-approve for all external effects.** Queue every send, post, purchase and file-write as a structured draft. Show it on Discord / iMessage / KakaoTalk with approve / edit / reject. Deliver only after approval, and re-validate the destination at send time.
2. **Destination pinning by immutable ID.** Bind each lane to allowed recipients by stable IDs (Discord channel ID, chat GUID), never display names. Reject sends where a name matches more than one destination. This directly prevents the "Exec-team" incident.
3. **Per-lane credential and connection isolation.** Do not share one connection set across all agents. Each lane (finance, work, coding) gets its own scoped tokens and its own egress allowlist. No cross-lane data flow without an explicit grant.
4. **Trust labelling for ingestion.** Tag everything from the inbox ingester, webhooks and web fetches as untrusted data. Never let it become instructions. Put a deterministic policy layer, not a prompt, between untrusted content and tool calls.
5. **Independent action reviewer.** Add a small separate model or rule engine that reviews shell commands and outbound actions before the omo coding agents run them. Log its decisions. Pair it with hard sandboxing (per-lane working directories, no ambient credentials) instead of relying on the reviewer alone.
6. **Signed webhook triggers.** Mirror the Standard Webhooks scheme for cron and external triggers: per-trigger secret, 5-minute timestamp window, 1 MiB cap, 202-then-async execution, secret rotation, and a rate limit per trigger.
7. **Append-only audit log.** Record every trigger, tool call, reviewer verdict and approval with the initiating lane. Make it queryable from chat ("what did lane X do overnight?"). Grok Bot has no documented audit trail.
8. **Budget and quota controls.** Per-lane usage ceilings, weekly caps and peak-time priority. Alert the owner before a lane exhausts its limit.

## CLAIMS
- CLAIM: Grok Bot launched in beta on 2026-08-11 as xAI's "team of always-on agents" with its own cloud computer. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/news/introducing-grok-bot
- CLAIM: Standalone tiers are $30, $100 and $200 per month, with no confirmed ~$120 tier. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/bot
- CLAIM: Irreversible actions such as email or Slack posts come back as drafts for approval. — RISK: high — SOURCES: peterp.me (transcript of the Cursor talk) — PRIMARY: none (third-party transcript of the talk)
- CLAIM: A separate model reviews shell commands before they run, and untrusted content is treated as data. — RISK: high — SOURCES: peterp.me — PRIMARY: none
- CLAIM: The agent VM is a Firecracker microVM. — RISK: normal — SOURCES: peterp.me — PRIMARY: none
- CLAIM: All of one user's bots share one computer, files and browser logins. — RISK: high — SOURCES: peterp.me — PRIMARY: none
- CLAIM: A personal-finance report leaked to a company Slack channel because of a same-name channel confusion, and xAI shipped a fix. — RISK: high — SOURCES: businessinsider.com (Africa edition) — PRIMARY: none
- CLAIM: Bot-identity signing credentials can be extracted from inside the VM and used for impersonation. — RISK: high — SOURCES: nightd.dev — PRIMARY: https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/
- CLAIM: Automation webhooks use HMAC-signed Standard Webhooks with a 5-minute window and 1 MiB cap. — RISK: normal — SOURCES: docs.x.ai — PRIMARY: https://docs.x.ai/grok/automations/webhooks
- CLAIM: The mail.grokbot.com inbox has no documented send caps, retention or send approval. — RISK: normal — SOURCES: shipmail.to, alphasignal.ai (via Shipmail) — PRIMARY: none

## EXPAND
- LEAD: x.ai/bot/guides and Automations overview docs (docs.x.ai/grok/automations) — I could not fetch them (rate limited). They may hold the permission-model, trigger-type and usage-limit details — ANGLE: official docs.
- LEAD: Grok Bot privacy and data-retention policy (x.ai legal pages, Grok Bot help center) — retention and training-on-user-data were not found — ANGLE: privacy.
- LEAD: xAI's fix for the cross-channel leak and any permission-model changelog — only the CEO's account was found — ANGLE: xAI statement or release notes.
- LEAD: Region and enterprise availability, and the "SuperGrok at $30" claim in the Shipmail source against /bot tiers — pricing sources conflict slightly — ANGLE: pricing page, support docs.
- LEAD: Business Insider's piece on early users deleting personal agents over privacy scares (URL in the CEO article) — more incident data points — ANGLE: incidents.
- LEAD: Robinson's talk after the 37-minute mark (cut off in the transcript) — it may cover group-chat bot safety — ANGLE: YouTube L1OBRtfPS34.