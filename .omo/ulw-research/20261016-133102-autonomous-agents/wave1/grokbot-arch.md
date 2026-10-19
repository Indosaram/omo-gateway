<!-- task=st_01a144fa -->
st_01a144fa [completed] model mahoquot/claude-sonnet-5-5
Ask: Research Grok Bot architecture, with sources. Wanted: a report with identity, findings, implications and claims. For you: the report is below. It is built from primary x.ai pages, the HN launch thread, and one third-party reverse-engineering post. I found no developer API/SDK docs, no model disclosure, and no x.ai "Automations" product. Now: done. Next: nothing is open.

## Identity
- **Name:** Grok Bot, beta; the page title is "Introducing Grok Bot | SpaceXSI".
- **Company:** x.ai now brands itself SpaceXSI. HN titles call it "SpaceXAI". Grok Bot's download and sales links point at Cursor domains (`api2.cursor.sh`, `cursor.com/contact-sales?product=grok-bot`), so it is a joint xAI/Cursor product.
- **Official URLs:** https://x.ai/news/introducing-grok-bot, https://x.ai/bot, https://x.ai/bot/guides
- **Launch:** Aug 11, 2026, confirmed by the page date and the HN submission timestamp.
- **Confidence:** high on identity and launch date. Medium on architecture, because the primary sources are marketing and guides. The deep technical detail comes from one third-party teardown.
- **Pricing mismatch:** the task says about $120/month. x.ai/bot currently lists $30, $100 and $200 tiers. The HN launch thread says "120/200 USD per month, per employee". Treat $120 as unverified.

## Findings
**Execution environment**
- Each bot "share[s] a computer of their own in the cloud", so jobs keep running when you step away. Bots work in apps, tools and websites "including platforms with no clean API or MCP". [https://x.ai/news/introducing-grok-bot]
- The cloud computer has a desktop, a filesystem, a terminal and apps. You can open the bot's window and use it as a remote desktop. [https://x.ai/bot/guides/grok-bot-101]
- Bots are not local-device agents. I found no sign of access to your own machine; the machine is a cloud VM. Desktop (macOS download, with a Windows OS listed in the page's JSON-LD) and iOS are the client surfaces. [https://x.ai/bot/guides/grok-bot-101]
- A teardown by a third party (Kevin, "nightd") reports a Docker container with Chrome driven over CDP. Its `/usr/local/bin` holds `sand-web-bot-auth.mjs` and a `start-exec-daemon`. The environment variables are named `SAND_GATEWAY_TOKEN`, `SAND_INFERENCE_RENEWAL_CREDENTIAL` and `SAND_EGRESS_TUNNEL_BEARER`. The author says the egress tunnel is the "sand" sandbox's network path and that it runs on AWS. Third-party, not vendor-confirmed. [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/]
- The same teardown says the daemon unsets those tokens from the environment, but they were still readable via `/proc/<pid>/environ`. Inference uses a short-lived access token minted from a renewal credential. [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/]
- The teardown says outbound browser requests get Web Bot Auth signatures (Ed25519, `Signature-Agent: https://cursorusercontent.com`) from a backend signing endpoint. The author forged signatures by calling that endpoint from inside the container, and says xAI was notified Sep 23 and had not responded by Oct 7. [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/]
- The bot's computer persists across sessions. For example, a Marketplace bot works because "Facebook is already logged in". [https://x.ai/bot/guides/grok-bot-101]

**Auth, credentials and human handoff**
- The model is "Log Grok Bot in once. It uses your apps and websites just like you would". The bot signs in as a user-like principal rather than through OAuth scopes alone. [https://x.ai/bot]
- When a flow needs a human, the bot hands the desktop back for a CAPTCHA or 2FA. It can also send a "secure form" for a password or API key. [https://x.ai/bot/guides/grok-bot-101]
- There is also a plugin and connector layer: the guide cites a "Cursor connection", GitHub and a "pstack" plugin. Non-standard plugins such as MCP servers and custom scripts are not carried in templates. [https://x.ai/bot/guides/templates-for-grok-bot]
- Templates exclude secrets and personal memories. The user can review the contents before sharing or installing, and the guide warns to treat third-party templates like untrusted software. [https://x.ai/bot/guides/templates-for-grok-bot]
- I found no documentation of the credential vault internals, such as encryption, per-bot scoping or audit logs. That is unknown. [none]
- Community pushback on HN centres on prompt injection and blast radius. Commenters suggest giving agents their own accounts and enforcing read-only on the service side. That is commentary, not vendor design. [https://news.ycombinator.com/item?id=49261514]

**Memory and personal context**
- Bots "remember conversations, learn how you like things done", pick up your voice and edge cases, and learn "when to ping versus keep going". [https://x.ai/news/introducing-grok-bot]
- Bots "keep context and learn from each other". [https://x.ai/bot]
- A bot is configured by three fields: Name, Title and Description (instructions), set in chat or Settings. [https://x.ai/bot/guides/grok-bot-101]
- The memory store, its retrieval mechanism and the cross-bot sharing semantics are not documented. [none]

**Long-running tasks and continuation**
- Bots "keep working 24/7" and "only come back when something needs your approval". [https://x.ai/news/introducing-grok-bot]
- Routines are the saved-workflow mechanism. A bot watches you do a job once, saves it as a routine, takes corrections, and re-runs it. A template's "routines are just the triggers... like how often it runs on its own". [https://x.ai/news/introducing-grok-bot] [https://x.ai/bot/guides/templates-for-grok-bot]
- The example bot "Tech Demos" runs each weekday, drafts a prompt, waits for approve or deny, then starts a Cursor Cloud Agent. When done it sends screenshots or a clip and opens a PR. [https://x.ai/bot/guides/grok-bot-101]
- Bots follow up on dropped threads, "nudge a stalled handoff" and resume work from earlier conversations. [https://x.ai/news/introducing-grok-bot]
- The pricing page lists "Routines on a schedule" in the base tier. [https://x.ai/bot]
- The same thread continues across phone and desktop. [https://x.ai/news/introducing-grok-bot]

**Multi-agent structure**
- Internally, people run several bots, with a "chief of staff" on top and one specialist per lane, such as inbox, expenses, recruiting, bug fixes and ops. [https://x.ai/news/introducing-grok-bot]
- Bots can message each other, share context in threads, and sit in group chats where they pass work and assign ownership. They pull the human in only for judgment calls. [https://x.ai/news/introducing-grok-bot]
- Guides describe a "Bot Boss as the single front door" and a pattern where each project gets a channel, a roster and a Notion board. [https://x.ai/bot/guides]
- The engineering example is a bug-repro bot that files the ticket and hands off to a debugging bot. Engineer bots also manage Cursor cloud agents. [https://x.ai/news/introducing-grok-bot] [https://x.ai/bot/guides]

**Models and developer surface**
- I found no statement of which models power bots. The pricing page mentions "Priority model access at peak times". [https://x.ai/bot]
- I found no public Grok Bot API or SDK. Extensibility is through plugins, skills, MCP servers, templates and the Cursor integration. The wider xAI API is separate (console.x.ai, docs.x.ai); I did not check it for bot endpoints. [https://x.ai/bot/guides/templates-for-grok-bot]
- Grok Bot usage is metered separately from existing Grok and Cursor plan usage. [https://x.ai/news/introducing-grok-bot]
- The ecosystem has imitators, which suggests the pattern is easy to copy. Runta's Errand is a cloud-computer-per-teammate design on the Pi agent framework. Its runs, events and artifacts API supports reconnect by replaying events from `after=-1`. [https://runta.com/blog/building-errand-in-one-week/]
- Other open-source clones appear in HN results: CopilotKit "Open Bot", Routi, Linubot and others. [https://hn.algolia.com/api/v1/search?query=grok+bot&tags=story]

**x.ai "Automations"**
- I found no x.ai product called Automations. The news-index page was blocked by Cloudflare, and an HN search for "x.ai automations" returned nothing. Grok Bot's own pitch is that you "message a Bot" instead of building automations first, with routines as the scheduled-work mechanism. Treat Automations as unresolved. [https://x.ai/news/introducing-grok-bot]

## Implications for a self-hosted personal agent gateway
1. **Secure-form credential handoff.** Add a single-use, expiring web form or Discord modal where the owner supplies a password or API key. It goes straight into the macOS Keychain, and the agent only receives a handle. This mirrors Grok Bot's "secure form" without leaving secrets in chat history.
2. **Remote-takeover for 2FA and CAPTCHA.** When an omo agent's browser run stalls, pause the lane and send a screen-share or noVNC link. Resume when the owner finishes. Runta's Errand does the same with VNC.
3. **Routine recorder.** Let the owner say "watch me do this once". Record the steps into a replayable routine file and attach a cron trigger. Keep a diff-and-correct step so each rerun can be corrected. Grok Bot's routines have the same shape.
4. **Chief-of-staff lane router.** Keep one front-door agent that owns the Discord, iMessage and KakaoTalk threads. It dispatches to specialist lanes (inbox, coding, ops) and only escalates judgment calls. Give lane-to-lane messaging a thread ID and shared context. This maps onto your lanes concept.
5. **Approval gates as a first-class state.** Model "waiting for approve/deny" as a durable task state. A message reply resumes the task, so cron and inbox-triggered work survive daemon restarts. Grok Bot's Tech Demos flow is the template.
6. **Event-log replay for continuation.** Persist each run as an append-only event log, so any client can reconstruct a run after a disconnect. Errand's `events?after=-1` pattern shows this works. It also gives you audit.
7. **Recipe-style export.** Export an agent as instructions, skills and routine triggers, with secrets and personal memory stripped. That is cheap to build and avoids Grok Bot's gap, where non-standard plugins don't travel.
8. **Do not copy the leaky token model.** The teardown shows tokens recoverable from `/proc/<pid>/environ` and a signing endpoint callable from inside the sandbox. On a Mac, keep credentials in the daemon, sign or broker from outside the agent process, and give each lane scoped, short-lived tokens.

## CLAIMS
- CLAIM: Grok Bot launched in beta on 2026-08-11 as a cloud-hosted multi-bot agent product from SpaceXSI (x.ai), with Cursor-linked distribution. — RISK: normal — SOURCES: x.ai, news.ycombinator.com — PRIMARY: https://x.ai/news/introducing-grok-bot
- CLAIM: Each bot gets a persistent cloud computer (desktop, filesystem, terminal, apps) and operates apps without needing an API or MCP. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/bot/guides/grok-bot-101
- CLAIM: Credentials are handled by logging the bot into apps on its own computer, with human handoff for 2FA/CAPTCHA and a "secure form" for passwords and API keys. — RISK: high — SOURCES: x.ai — PRIMARY: https://x.ai/bot/guides/grok-bot-101
- CLAIM: The sandbox leaks its own tokens and can be used to forge Web Bot Auth signatures as Grok Bot. — RISK: high — SOURCES: nightd.dev (single, unconfirmed by the vendor) — PRIMARY: https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/
- CLAIM: Multi-agent structure is a chief-of-staff bot plus per-lane specialists, with bot-to-bot messaging and group chats. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/news/introducing-grok-bot
- CLAIM: Scheduled work is done via "routines" learned by demonstration, with approval gates. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/bot/guides/grok-bot-101
- CLAIM: Templates package instructions, memories, skills and first-party plugins without secrets or personal memories. — RISK: normal — SOURCES: x.ai — PRIMARY: https://x.ai/bot/guides/templates-for-grok-bot
- CLAIM: Pricing is $30, $100 or $200 per month on x.ai/bot, and the task's "~$120/month" matches only an HN comment about launch-time per-employee pricing. — RISK: normal — SOURCES: x.ai, news.ycombinator.com — PRIMARY: https://x.ai/bot
- CLAIM: No public Grok Bot developer API/SDK or model disclosure was found. — RISK: normal (absence of evidence) — SOURCES: x.ai — PRIMARY: none

## EXPAND
- LEAD: Fetch the rest of the guides (support, engineering, multiple-teams, PMs). I was rate-limited on `r.jina.ai`, and the raw HTML is truncated by the tool. — They likely describe connectors, per-bot permissions, memory and routine details. — Architecture, credentials and routines.
- LEAD: Read Grok Bot 101 past the truncation point, especially the routines, plugins and skills sections. — It may name the connector and auth model. — Connectors.
- LEAD: Check the Cursor changelog and docs, plus docs.x.ai, for a Cloud Agents API or a "sand" runtime. — This is where any developer surface or model information would be. — APIs/SDKs and models.
- LEAD: Read the HN thread https://news.ycombinator.com/item?id=49261514 comments by `leerob` and other Cursor staff. — They are a possible source of unofficial architecture answers. — Models, sandbox, security.
- LEAD: Search for the x.ai "Automations" announcement via X posts from @spacexsi or @xai. — My fetches did not find it. — Product identity.