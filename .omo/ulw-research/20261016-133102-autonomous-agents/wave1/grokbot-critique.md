<!-- task=st_01a144fc -->
st_01a144fc [completed] model mahoquot/claude-sonnet-5-5
Ask: research Grok Bot reception and failure modes, critique angle. Wanted: a sourced report of what works, what fails, and what it means for your gateway. For you: the report below. Direct evidence is strong on cost, reliability and security incidents. I found no evidence of an Amazon block, and I could not verify the ~$120/month price or an x.ai "Automations" product. Now: done. Next: none.

I did not use web_search, so the results come from curl and the HN Algolia API. Reddit returned the page shell only (no usable results), x.ai/news, x.ai/bot and x.ai/pricing sit behind Cloudflare, and the Business Insider and ABC pages gave me only their metadata. Where a claim rests on a headline or a forum post, I say so.

## Identity
- **Name:** Grok Bot, "your team of always-on agents". The page title is "Introducing Grok Bot | SpaceXSI".
- **Company:** SpaceXSI, which the Cursor forum treats as the owner of Cursor ("after the acquisition"). The macOS download is served from `api2.cursor.sh`, so Cursor ships the app. Both points are inference from the page and forum posts.
- **Official URL:** https://x.ai/news/introducing-grok-bot, dated Aug 11, 2026. It is a beta, on desktop (macOS) and iOS, with a separate usage pool from the Grok and Cursor plans. Enterprise is waitlist only.
- **Confidence:** high on identity and launch date, because I fetched the official page. Low on "$120/month". I saw no such price. The official page says it is bundled with SuperGrok, SuperGrok Plus, SuperGrok Heavy and the Cursor Pro, Pro+, Ultra and Teams tiers. Forum posts mention a $200 Ultra tier. Since Oct 12 it is also folded into "The X Subscription".
- **"x.ai Automations":** I found no such product. The Cursor forum has an `automations` tag, but I did not confirm it is a separate x.ai product. Grok Bot's own pitch is "no automations to set up" (see the first Findings bullet).

## Findings

**What the vendor claims**
- Bots get their own cloud computer, sign into tools, and work apps "with no clean API or MCP". You message them like a colleague, they come back only for approval, and they learn routines by watching you work. The page also pitches "no automations to set up", so there is no separate Automations product in the pitch. [https://x.ai/news/introducing-grok-bot]
- Early users are quoted on the page. All the examples are internal SpaceXSI (sales CRM updates, invoice processing from Gmail, bug repro). There are no independent benchmarks. [https://x.ai/news/introducing-grok-bot]

**What works, per users (mostly enthusiasts, so skew positive)**
- One reviewer says it is "really good" and the first polished packaging of proactive, inter-talking bot teams. [https://hn.algolia.com/api/v1/search?query=%22grok%20bot%22&tags=comment&hitsPerPage=40]
- One user runs it as a sysadmin bot with a root SSH key and a notes repo. It is controlled through a phone app with prompts like "run updates Monday 6am, confirm site is online, roll back if needed". [same HN search]
- A mobile bot is used to instruct coding agents (Opus, Codex) on other machines. [same HN search]
- A forum power user runs 13–14 bots in Slack with a QA bot testing in a live browser. They still gate PRs by hand. [https://forum.cursor.com/t/anyone-used-grokbot-on-the-api-very-high-costs/169551.json]
- A reviewer says "best AI app I have ever used", but the same posts are about running out of quota. [same forum thread]

**Cost and quota are the dominant complaint**
- A user's $50 of API credit was gone in about 10 minutes of light tasks ($27 in the first measurement). [https://forum.cursor.com/t/anyone-used-grokbot-on-the-api-very-high-costs/169551.json]
- Ultra ($200) users report the weekly pool exhausted in 1–3 days. One reports "one week's usage lasts ~8 hours" with 13 bots. Another hit 65% on day one. [same thread; https://forum.cursor.com/tag/grok-bot.json lists the related Ultra-allowance topic]
- Cursor staff admitted limits were "depleting quickly" and reset everyone's weekly limits on Aug 26. [https://forum.cursor.com/t/anyone-used-grokbot-on-the-api-very-high-costs/169551.json]
- One commenter blames quadratic cost growth in long conversations and advises splitting into new bot instances. That is a user's explanation, not confirmed by the vendor. [same thread]
- Users say it defaults to the most expensive model, and one saw an old Claude Sonnet used with no way to force or blacklist models. [same thread]
- On Oct 14, a user on the new Premium plan lost 7% of the weekly pool to one short chat (a web lookup and a stock chat). They linked it to an active "service degradation incident". [https://forum.cursor.com/t/premium-plan-7-of-weekly-included-usage-gone-after-one-short-casual-grok-bot-chat/174749.json]
- Staff would not give a clear on-demand price. One user extrapolated about $438k/year for their usage. That figure is their own extrapolation. [https://forum.cursor.com/t/anyone-used-grokbot-on-the-api-very-high-costs/169551.json]
- The Oct 12 bundle was criticized for dropping third-party model usage from the plans (billed at API list price), which makes the pooled limits harder to predict. [https://forum.cursor.com/t/introducing-the-x-subscription/174450.json]

**Reliability and platform issues**
- A user who tried it for support, auto-trading and research says the VM "resets without warning every 1–2 weeks", losing installed packages and mid-run jobs, with sessions kicked after about 50 minutes. This is a single HN comment with no vendor confirmation. [https://hn.algolia.com/api/v1/search?query=%22grok%20bot%22&tags=comment&hitsPerPage=40]
- The vendor's own docs say VM snapshots preserve state, so this report conflicts with the design. Treat it as anecdotal. [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/]
- The Cursor forum's grok-bot tag has 936 topics, with fresh bug reports on Oct 14–16. Examples:
  - "Can't reach computer, delete fails, Update does nothing" (macOS 0.68.1)
  - "No computer-use handler is registered on this exec daemon"
  - "thinks my computer is offline or sleeping" (for a week)
  - an old grok.com message re-sent to bots
  - the X connector stuck on "turning on API access" for 72+ hours
  - no notifications in the Grok app
  - the 1Password connector failing
  - Stop not stopping the bot
  - "Cannot load my bots"
  
  [https://forum.cursor.com/tag/grok-bot.json]
- One user asks "what exactly is the purpose of Grok Bot" after 22 posts of confusion. [https://forum.cursor.com/tag/grok-bot.json]

**Security incidents**
- Business Insider (Oct 9): the CEO of XMTP Labs says his personal "CFO agent" posted his monthly bank audit to his company's exec Slack channel. HN commenters say it picked a same-named channel and that a personal bot sat in a work Slack. [https://www.businessinsider.com/personal-ai-agent-grok-bot-posted-bank-details-company-slack-2026-10; https://hn.algolia.com/api/v1/items/50033517]
- HN commenters conclude prompting cannot stop leaks and that domains must be isolated physically and deterministically. They also say write actions should go through an outbox, and that personal and work accounts should be kept apart. [https://hn.algolia.com/api/v1/items/50033517]
- A Business Insider piece is linked from HN as "AI assistant hacks gym website" (ABC News, Aug 10). It is the same product class but may not be Grok Bot. I only saw the headline, so treat that as unverified. [https://www.abc.net.au/news/2026-08-10/ai-assistant-hacks-gym-website-aus-cyber-attack/107007986]
- Researcher "nightd" says the sandbox (found in a created bot's desktop environment) exposes a signing endpoint. From the bot's own container you can extract the renewal credential and forge Grok Bot's Web Bot Auth signatures, which bypassed a Cloudflare challenge. They disclosed Sep 23, got an acknowledgement, and published Oct 7 with no fix noted. This is a single researcher's claim. [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/]
- The same post says the browser is under-patched and loops forever on Cloudflare **managed** challenges. [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/]

**Blocked by sites (e.g. Amazon)**
- I found no report of Amazon blocking Grok Bot. The nearest evidence is an HN thread on Amazon v. Perplexity (CFAA, Ninth Circuit), where a commenter says "Grok Bots can do the same" logged-in browsing. [https://hn.algolia.com/api/v1/items/49704214]
- Signed Web Bot Auth requests mean Cloudflare can identify it, which can work in or against a bot's favour. [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/]
- A site-detection library, "Botscent", already lists Grok Bot among detectable agents. [https://hn.algolia.com/api/v1/search_by_date?query=grok%20bot&tags=story&hitsPerPage=30]

**Competitors named by users**
- Muse (Meta), Instinct, Dots, Hermes, OpenClaw, ChatGPT, and several open-source clones (Errand, OpenBot, Codync, Personal Jarvis, and a self-hosted alternative billing itself as private).
- Meta's Muse was called "the most pro-user" because Grok Bot and Instinct reportedly hide how they work. That is one commenter's claim. [https://hn.algolia.com/api/v1/search?query=%22grok%20bot%22&tags=comment&hitsPerPage=40]
- One user left Hermes for Grok Bot for easier setup, and misses model choice. Another says a local model through Hermes is better value. [https://hn.algolia.com/api/v1/items/49980264]
- A skeptic argues most people lack a use that justifies the complexity or the privacy tradeoff, and that half of one enthusiast's post was about managing Grok Bot's cost. [https://hn.algolia.com/api/v1/search?query=%22grok%20bot%22&tags=comment&hitsPerPage=40]

**Architecture (secondary source, from a talk)**
- One reviewer recaps a Lee Robinson talk. It describes a per-conversation queue where direct user messages interrupt a turn but helper results queue. It runs on durable workflows with step results saved, so nothing is redone after an interrupt or crash. Helper agents (general, computer-use, video, cloud-coding) keep the main context small. A separate model reviews actions before they run, and external content is treated as data, not instructions. Irreversible actions come back as drafts. One `SendToUser` tool fans out to every client, and compaction triggers after 10–60 minutes idle or at about 200K tokens. [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/]
- The post also says all of a user's bots share one computer but each gets its own desktop. Some of this is the author's inference. [same URL]

## Implications for a self-hosted personal agent gateway
1. **Per-domain isolation by construction, not prompts.** Give each lane its own credentials and own outbound targets, as the HN consensus on the Slack incident says. A "finance" lane physically cannot reach a work-chat adapter. Resolve destinations by immutable ID and reject names that match more than one target.
2. **Outbox with approval for irreversible actions.** Queue sends, posts, payments and email as drafts for owner approval in Discord, iMessage or KakaoTalk. Auto-approve only for an allowlist of idempotent targets.
3. **Cost and quota guardrails first.** Per-lane token budgets, a running spend meter pushed to the owner, a hard cap, and per-model routing so cheap models handle routine work. Quadratic growth in long threads means proactive compaction or thread rotation.
4. **Durable interrupt-safe turns.** Persist every model and tool step so a user message can interrupt a running coding agent without losing completed work. Keep user messages interrupting and everything else (cron, inbox, helper results) queued.
5. **No in-sandbox signing or secrets.** The forged-signature report shows exposed credential endpoints are the risk. Keep secrets and request signing in the daemon, outside the agent sandbox, and never keep a renewal token in the sandbox's process environment.
6. **Treat inbox ingestion as untrusted data.** Tag content from email, web and chat as data, and run action review on a separate model. Scope inbox lanes to read-only plus draft by default.
7. **Visibility the vendor lacks.** A per-lane usage ledger showing which agent spent what, a timeline of tool calls, and a stuck-job detector with a stop that actually stops. Users asked for exactly this.
8. **Snapshot and restore for the host.** Because users report VM resets, back up agent memory, skills and workflows as plain files (markdown) so a bad machine state costs nothing. Make agent memory portable.

## CLAIMS
- CLAIM: Grok Bot launched in beta on Aug 11, 2026 under SpaceXSI, shipped via Cursor, for macOS desktop and iOS. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/news/introducing-grok-bot
- CLAIM: Weekly usage limits burn very fast, with Ultra ($200) users reporting the pool gone in 1–3 days; staff acknowledged and reset limits on Aug 26. — RISK: normal — SOURCES: forum.cursor.com — PRIMARY: https://forum.cursor.com/t/anyone-used-grokbot-on-the-api-very-high-costs/169551.json
- CLAIM: A Grok Bot agent posted an XMTP CEO's bank audit to a company exec Slack channel after picking a same-named channel. — RISK: high — SOURCES: businessinsider.com, news.ycombinator.com — PRIMARY: https://www.businessinsider.com/personal-ai-agent-grok-bot-posted-bank-details-company-slack-2026-10 (I saw its headline and metadata, plus HN commenters quoting the article)
- CLAIM: One researcher says Grok Bot's Web Bot Auth signatures can be forged from inside its sandbox, and that its browser loops forever on Cloudflare managed challenges. — RISK: high — SOURCES: nightd.dev — PRIMARY: https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/ (single source, vendor fix not confirmed)
- CLAIM: A user reports the VM resets every 1–2 weeks, losing packages and mid-run jobs. — RISK: high — SOURCES: news.ycombinator.com — PRIMARY: none (single HN comment; the vendor's design claims snapshot persistence)
- CLAIM: Grok Bot costs about $120/month. — RISK: high — SOURCES: none found — PRIMARY: none (unverified; plans I saw are tied to SuperGrok, Cursor and the X Subscription)
- CLAIM: Amazon (or other sites) block Grok Bot. — RISK: high — SOURCES: none found — PRIMARY: none (no direct evidence; only the Amazon v. Perplexity analogy)
- CLAIM: x.ai has a product called "Automations". — RISK: normal — SOURCES: none confirmed — PRIMARY: none (the official page contrasts itself with automations; the Cursor forum has an unrelated-looking `automations` tag)

## EXPAND
- LEAD: the full text and comment thread of the Business Insider "bank details" article, plus Shane Mac's X posts — WHY: confirm the incident details and whether Slack-channel disambiguation was the root cause — ANGLE: incident postmortem
- LEAD: Lee Robinson's talk (https://www.youtube.com/watch?v=L1OBRtfPS34) and the transcript at https://www.peterp.me/transcripts/how-grok-bot-works/ — WHY: primary source for the queue, interrupt, compaction and action-review design — ANGLE: architecture to borrow
- LEAD: x.ai/pricing and the x.ai/news/x-subscription page (blocked by Cloudflare for me) — WHY: pin down real plan prices, which would settle the "$120/month" claim and the weekly-pool sizes — ANGLE: pricing
- LEAD: Reddit threads (r/singularity, r/ClaudeAI, r/LocalLLaMA) and an X search for "Grok Bot" Amazon/CAPTCHA reports — WHY: I could not read Reddit, and X search was not available to me — ANGLE: site-blocking and hands-on reviews
- LEAD: the ABC News "AI assistant hacks gym website" piece in full — WHY: check whether the agent involved was Grok Bot or another assistant — ANGLE: incident attribution