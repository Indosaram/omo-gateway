<!-- task=st_01a144f9 -->
st_01a144f9 [completed] model mahoquot/claude-sonnet-5-5
Handoff
Ask: research xAI's Grok Bot, product surface and UX. Wanted: a sourced report. For you: the report is below. Most claims come from primary x.ai pages, but the ~$120/month price you cited is not what x.ai shows, and SMS, iMessage and voice are not documented. Now: done. Next: nothing.

## Identity
- **Name:** Grok Bot. The product page title is "AI teammates that finish the work | Grok Bot".
- **Company:** SpaceXSI. The x.ai news page is branded "SpaceXSI". I take that to be the successor to xAI, which is an inference, since the pages never say it outright.
- **Official URLs:** https://x.ai/news/introducing-grok-bot (announcement), https://x.ai/bot (product page), https://x.ai/bot/guides (guides), and https://docs.x.ai/grok/automations/webhooks (Automations docs).
- **Launch:** the announcement is dated Aug 11, 2026. It is a beta on desktop (macOS download) and iOS.
- **Confidence:** high on identity and launch date (primary pages fetched). Medium on features after launch, since several come from third-party blogs.
- **Price discrepancy:** x.ai/bot lists Premium $30, Super $100 and Ultra $200 per month. I found no "$120/month" tier. Treat that figure as unverified or stale.
- **Fetch limits:** I could not fetch x.ai/news (Cloudflare block). r.jina.ai rate-limited several fetches, so I did not read the Business Insider article, the TeslaNorth piece, the "team-bots" post or the Automations overview page.

## Findings

**Positioning and onboarding**
- Bots are "AI teammates" with their own cloud computer. They sign into your tools, finish jobs end to end, and "only come back when something needs your approval". [https://x.ai/news/introducing-grok-bot]
- The launch targets SuperGrok, SuperGrok Plus/Heavy, Cursor Pro/Pro+/Ultra and Cursor Teams subscribers. It has its own usage pool, separate from Grok and Cursor usage. Enterprise is a waitlist. [https://x.ai/news/introducing-grok-bot]
- Onboarding is "download the app, create your first teammate". The pitch is "no workflows to set up first"; you just message a bot. [https://x.ai/news/introducing-grok-bot]
- A bot is three settings fields, editable in chat or in Settings. Standing one up takes about 10–15 minutes. [https://x.ai/bot/guides/grok-bot-101]
- Pricing tiers: Premium $30 (own computer, signs into tools, "routines on a schedule", weekly usage), Super $100 (3.5x usage, priority at peak), Ultra $200 (12x usage). [https://x.ai/bot]

**Channels**
- The official surfaces are desktop and iOS. You message a bot the way you'd text a colleague and pick up the same thread on either. [https://x.ai/news/introducing-grok-bot]
- The product page says "Work anywhere: desktop, mobile, and more". I found no first-party SMS, iMessage, voice or Slack-as-front-end channel, so that is unconfirmed. [https://x.ai/bot]
- Bots can also be tagged as @bot on X. They can message each other and sit in group chats. [https://shipmail.to/blog/grok-bot-email-on-your-own-domain] [https://x.ai/news/introducing-grok-bot]
- Since Oct 9, 2026 a bot can claim its own inbox at name@mail.grokbot.com (secondary source). [https://shipmail.to/blog/grok-bot-email-on-your-own-domain]
  - It is one permanent address per user, shared across that user's bots, and plus-aliases did not arrive.
  - There is no webmail, so you inspect it by asking the bot.
  - New mail does not wake the bot unless a routine exists. Team admins must enable the feature.
  - These details come from X posts quoted by Shipmail, not from xAI docs.

**Assigning tasks**
- There are three ways to talk to a bot: chat, routines and triggers, and other bots. [https://x.ai/bot/guides/grok-bot-101]
- You can teach a bot by recording a task or writing a prompt. A bot can also "follow along" while you do a job once. It saves that as a routine, takes corrections, and runs it alone next time. [https://x.ai/news/introducing-grok-bot] [https://x.ai/bot]
- Templates share a bot as a recipe, not a clone. [https://x.ai/bot/guides]
- The bot's computer is a cloud desktop with a filesystem and terminal. You can open it like a remote desktop. [https://x.ai/bot/guides/grok-bot-101]
- Bots support the same MCP servers, plugins and skills as Cursor, with multiple accounts per service. [https://x.ai/bot/guides/grok-bot-101]
- Native connectors include Gmail, Calendar, Drive, Slack and GitHub, according to one user. [https://jamespember.substack.com/p/grok-bot-is-really-good]
- The guides show "outer loop" bots that gather context and then hand a clean prompt to a Cursor cloud agent for the coding. [https://x.ai/bot/guides/grok-bot-101]

**Proactivity**
- A bot can set its own schedule or listen for events, such as a Slack thread or a GitHub PR. [https://x.ai/bot/guides/grok-bot-101]
- One user's bots watch the calendar, wait for specific Slack replies, and run on timed schedules ("every morning", "4 times a day"). [https://jamespember.substack.com/p/grok-bot-is-really-good]
- Daily briefs exist as a pattern. An "Intel Scout" bot reports at 8am and 5pm: what happened, open action items, and prep for upcoming meetings. [https://x.ai/bot/guides/grok-bot-for-work]
- A "Tech Demos" bot scouts X bookmarks each weekday and drafts a prototype prompt. It then asks the user to approve it before launching a Cursor cloud agent. [https://x.ai/bot/guides/grok-bot-101]
- Bots can "follow up on threads you dropped" and "nudge a stalled handoff". The announcement says they become more proactive over time. That is a marketing claim and not independently verified. [https://x.ai/news/introducing-grok-bot]
- Separately, Grok Automations accept a Webhook trigger. The docs describe this: [https://docs.x.ai/grok/automations/webhooks]
  - Each trigger has its own endpoint and a `whsec_` signing secret, shown once and rotatable.
  - Requests use the Standard Webhooks HMAC-SHA256 headers, with a five-minute timestamp window and a 1 MiB body cap.
  - A valid request returns `202`, and the run starts asynchronously.
  - Errors are `400`, `401`, `413`, `429` and `503`.
- A third party says the Grok webhook run took "a few minutes" to appear under Executions. The run is queued, not instant. [https://shipmail.to/blog/grok-bot-email-on-your-own-domain]

**Progress reporting and approval**
- Bots escalate for "judgment calls" and approvals. In group chats they pass work and assign ownership themselves. [https://x.ai/news/introducing-grok-bot]
- When a flow needs you, the bot hands the desktop back for a CAPTCHA or 2FA. It can also send a secure form for a password or API key. [https://x.ai/bot/guides/grok-bot-101]
- Guardrails are natural-language allow/block rules under Settings > General > Agent. A separate review agent checks proposed actions and can allow, block or escalate to you. The guide itself says you are trusting the model to follow what you wrote. [https://x.ai/bot/guides/grok-bot-101]
- Logging into a site with one bot makes that site reachable by every other bot. [https://x.ai/bot/guides/grok-bot-101]
- The product page shows a live "Computer · You're in control · Working" status and a "watching and learning 0:04" indicator for taught routines. [https://x.ai/bot]
- A "Bot Boss" front-door pattern has all specialist bots report into one bot that tells the human. That cuts the number of pings. [https://x.ai/bot/guides/grok-bot-for-work]
- Guides repeatedly describe draft-then-human-send for outbound work (SDR and legal). [https://x.ai/bot/guides]
- The pitch is "you watch them take action instead of approving every step". [https://x.ai/bot]

**Failure mode**
- A Business Insider story, reported on HN, says a user's personal agent posted bank details to a company Slack channel. The cause was two channels with the same name. I only saw the HN thread, not the article. [https://hn.algolia.com/api/v1/items/50033517]
- An HN commenter's summary: guardrails by prompt are not enough, and domains must be separated deterministically. That is opinion, not fact. [https://hn.algolia.com/api/v1/items/50033517]

## Implications for a self-hosted personal agent gateway
These are my inferences, not claims from the sources.

1. **Bot Boss lane.** Make one "front door" lane the only thing that messages the owner on Discord, iMessage or KakaoTalk. Other lanes (inbox, calendar, omo coding) report into it, and it batches and prioritizes. This matches the "Bot Boss" pattern and reduces notification noise.
2. **Approval queue with a reviewer stage.** Every outbound or irreversible action goes through a deterministic outbox (sender, recipient ID, content hash). A second lightweight policy check can then allow, block or escalate. Make it default to draft-then-human-send for email, Slack and Discord posts.
3. **Resolve destinations by permanent ID.** The Slack incident came from an ambiguous channel name. Pin each lane to explicit allowed channel and recipient IDs, enforced by the daemon rather than by the prompt.
4. **Signed webhook triggers plus cron.** Copy the Automations model: one endpoint per trigger with its own secret, Standard Webhooks HMAC, a five-minute replay window, a body cap and a `202` async ack. Put it next to cron and inbox-ingestion watchers, with per-trigger rate limits.
5. **Routine recorder.** Let the owner say "watch me do this once" and save the transcript as a named routine with a schedule or trigger. Version it and take corrections as edits. The key UX is "chat first, formalize later".
6. **Scheduled digests as a built-in.** Ship a morning and evening brief (changes, open items, meeting prep) as a default cron routine delivered through the front-door lane.
7. **Lane isolation and credential scoping.** Keep work and personal credentials in different lanes with different tool sets, and never let a bot reach another lane's logins. Grok Bot does the opposite, since one bot's login is available to all bots.
8. **Wake-on-mail ingestion.** Grok's own inbox does not wake the bot without a routine. Make your inbox ingestion push events into a lane automatically, with the sender, subject and message ID in the event body.

## CLAIMS
- CLAIM: Grok Bot launched in beta on 2026-08-11 for SuperGrok, Cursor and Teams subscribers on desktop and iOS, with its own usage pool. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/news/introducing-grok-bot
- CLAIM: Pricing is $30, $100 and $200 per month, not about $120. — RISK: high — SOURCES: x.ai — PRIMARY: https://x.ai/bot
- CLAIM: Bots run on a cloud computer and can hand control back to the user for CAPTCHA and 2FA. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/bot/guides/grok-bot-101
- CLAIM: Bots support schedule and event triggers (Slack threads, GitHub PRs) and can trigger each other. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/bot/guides/grok-bot-101
- CLAIM: Approval uses a natural-language policy plus a separate review agent that can allow, block or escalate. — RISK: high — SOURCES: x.ai — PRIMARY: https://x.ai/bot/guides/grok-bot-101
- CLAIM: Grok Automations accept signed webhook triggers (Standard Webhooks, 5-minute window, 1 MiB cap, `202` async). — RISK: normal — SOURCES: docs.x.ai — PRIMARY: https://docs.x.ai/grok/automations/webhooks
- CLAIM: Grok Bot got its own mail.grokbot.com inbox around Oct 9, 2026, with one address per user and no wake-on-mail. — RISK: normal — SOURCES: shipmail.to, which quotes X posts and AlphaSignal — PRIMARY: https://x.com/bot/status/2108609764766908772 (not fetched; cited via Shipmail)
- CLAIM: A Grok Bot user's agent posted bank details to a company Slack channel. — RISK: high — SOURCES: news.ycombinator.com (a Business Insider story, via the HN item) — PRIMARY: https://www.businessinsider.com/personal-ai-agent-grok-bot-posted-bank-details-company-slack-2026-10 (not read)
- CLAIM: There is no first-party SMS, iMessage or voice channel. — RISK: normal — SOURCES: x.ai (absence of evidence only) — PRIMARY: none

## EXPAND
- LEAD: x.ai/news/team-bots ("Grok Bot Teams") and the Grok Bot docs/help center — WHY: multi-user bots, admin controls, and any channel list beyond desktop and iOS — ANGLE: product and enterprise
- LEAD: https://x.com/bot status posts and the Elon Musk tweet "Grok Bot will use Opus 5.5, Midjourney" (HN 49991047) — WHY: model routing and post-launch feature cadence (email, plugins), which are likely a source of SMS or iMessage news — ANGLE: product roadmap
- LEAD: Business Insider's bank-details incident article (archive.is/JUbIC) — WHY: the exact approval and permission failure — ANGLE: safety and UX
- LEAD: x.ai/bot/guides (support, SDR, legal-ops, engineering) and the `x.ai/bot/<id>` shared-bot templates — WHY: concrete routine, trigger and approval configurations — ANGLE: product patterns
- LEAD: the Grok Bot plugin and connector docs (AgentMail, custom MCP connectors on grok.com) — WHY: how external events enter a bot — ANGLE: integration