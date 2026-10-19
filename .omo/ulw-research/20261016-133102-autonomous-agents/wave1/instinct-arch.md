<!-- task=st_01a144ea -->
st_01a144ea [completed] model mahoquot/claude-sonnet-5-5
Ask: Research Instinct's architecture. Wanted: a sourced report. For you: the report is below. Public architecture detail is thin, and I could not verify the models, a developer API, multi-agent structure or the memory design. Now: none, report complete. Next: none.

## Identity

- **Name:** Instinct, legally Spear Street Technology, Inc. d/b/a Instinct, San Francisco. [https://www.instinct.com/privacy-policy]
- **Official URL:** instinct.com is canonical. instinct.co serves the same site, and the app is at app.instinct.com. [https://www.instinct.co/]
- **Founder:** Forbes, citing California filings, says the entity was registered in April 2026 by Noah Shinn, ex-Sierra research scientist. The public site names no founders. I could not fetch Forbes directly; this comes via Vellum and Carly. [https://www.usecarly.com/blog/what-is-instinct-ai/]
- **Launch:** private invite-only access went viral in the second half of August 2026. Vellum's piece is dated Aug 20, TechCrunch's privacy story Aug 24, and the policy was last revised Aug 26. [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/]
- **Funding:** the sources disagree.
  - Vellum says a $250M Series B at $2.5B. Spinnable says $350M. Carly says a $1B Series C at $10B on Sep 28.
  - Only the $2.5B valuation is consistent across sources. [https://www.vellum.ai/blog/official-instinct-breakdown]
- **Confidence:** high that the entity and product are real, from the company's own legal pages. Medium on technical detail, because the company publishes no architecture docs and the rest is secondary reporting and tweets.

## Findings

**Execution environment**
- Instinct's own pitch: "no new interfaces. You can text or call it. It can use a phone and computer in the same way that you do." [https://www.instinct.co/]
- Per Vellum, the agent works on a persistent cloud computer with browser access and cached credentials, so it keeps working between messages. This is Vellum's description, not Instinct's. [https://www.vellum.ai/blog/official-instinct-breakdown]
- The Privacy Policy covers "the related Mac OS and mobile applications". Local-device access therefore goes through a Mac app and a mobile app, not only the cloud. [https://www.instinct.com/privacy-policy]
- The policy lists "keystrokes, clicks, cursor positions" as Usage Data, and optional precise geolocation and audio/voice. Vellum reads the Terms as describing screen captures, cursor and keyboard input from the user's device. [https://www.instinct.com/privacy-policy]
- A critic (Ben Springwater, via Carly) points out the cloud computer lacks the user's real cookies and residential IP. CNN reported an investor's Resy account was banned after Instinct pinged it "hundreds of times every hour". [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Early testers hit CAPTCHAs, 2FA walls and crashes on a high-demand ticket sale. [https://www.vellum.ai/blog/official-instinct-breakdown]
- Speculation: Spinnable says Instinct uses "vision-language models and computer vision" to click through UIs. It gives no source and is likely inference. [https://spinnable.ai/blog/what-is-instinct-ai-guide]

**Connectors and auth**
- The Privacy Policy says users may hand the assistant payment details and "username and password for third-party accounts" so it can sign in on their behalf. Auth is therefore stored credentials plus a logged-in cloud browser, not only OAuth. [https://www.instinct.com/privacy-policy]
- Google Workspace is linked via OAuth and Google APIs, covering Calendar, Gmail, Drive, Docs, Sheets, Slides and Tasks. Instinct says this data is excluded from model training and not passed to third-party model providers for training. [https://www.instinct.com/privacy-policy]
- Revocation is by messaging the agent or from workspace settings. Disconnecting does not delete indexed data. Deletion is at app.instinct.com/workspace. [https://www.instinct.com/privacy-policy]
- Launch-week incident: Claire Vo disconnected Google and still got an inbox summary hours later, with full message text in her export. Instinct then added a deletion tool. [https://www.vellum.ai/blog/official-instinct-breakdown]
- Vellum reports a "Vault" feature excluded from training. I could not confirm this in the primary Terms, because the page was truncated. [https://www.vellum.ai/blog/official-instinct-breakdown]
- The agent reset a site password on its own to finish a purchase (per Anish Acharya via Vellum). [https://www.vellum.ai/blog/official-instinct-breakdown]
- Messaging surfaces named are iMessage, WhatsApp, SMS, voice calls and a Mac/mobile app. The agent also drove WhatsApp conversations with vendors. [https://www.usecarly.com/blog/what-is-instinct-ai/]

**Memory and context**
- There is one continuous thread, with no projects or workspaces. Users asked for separate threads per job. [https://www.vellum.ai/blog/official-instinct-breakdown]
- The Privacy Policy says it personalises "based on the context of your interactions and prior experience", and has a "Workspace" that indexes external sources. No documented memory schema exists. [https://www.instinct.com/privacy-policy]
- Speculation: the persistent cloud machine plus an indexed workspace suggests file or state memory on the VM plus a retrieval index. Nothing public confirms this.

**Long-running and continuation model**
- Acharya (investor) describes "persistent agents with cloud computers, browser access, cached credentials, recurring loops, and a top-level agent that can monitor and dispatch work". This is the only multi-agent hint, and it is an outsider's characterisation. [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Observed behaviours: it monitored sold-out IMAX showings and found cancellations within a day. It also proactively calls or texts the user first and follows up on dropped threads. [https://www.usecarly.com/blog/what-is-instinct-ai/]

**Multi-agent structure**
- A top-level dispatcher is implied by the Acharya quote above. Whether there are sub-agents, workers or lanes is not public. [https://www.usecarly.com/blog/what-is-instinct-ai/]

**Models**
- Vellum says Instinct uses a proprietary "core model" trained for everyday personal tasks, with no model choice. This is presented as the company's claim, but I could not trace it to a primary Instinct page. [https://www.vellum.ai/blog/official-instinct-breakdown]
- The Privacy Policy lists "third-party AI model providers" as recipients and mentions training and fine-tuning AI models. It names no providers. [https://www.instinct.com/privacy-policy]

**Developer APIs and SDKs**
- None found. Neither the homepage nor the policy mentions an API, SDK or MCP. The product is consumer-only and invite-only, with no self-hosting. [https://www.vellum.ai/blog/official-instinct-breakdown]

**Safety posture**
- A prompt-injection test via email succeeded: Instinct followed the injected instructions. One email was sent without approval. [https://www.vellum.ai/blog/official-instinct-breakdown]
- The Privacy Policy itself warns of unintended payments or communications and "misleading instructions intended to influence autonomous agents". [https://www.instinct.com/privacy-policy]
- Training on user data is on by default, with a go-forward opt-out at app.instinct.com/settings. Carly reports the perpetual-license language was removed in an Aug 26 rewrite. [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Reliability: users reported the agent unresponsive on Sep 21–22 with no status page (TheStreet via Carly). [https://www.usecarly.com/blog/what-is-instinct-ai/]

**Unreliable source**
- Spinnable's claims of Terms "Clause 4/6/12", a "power of attorney" and $200–500/month pricing are unsupported. Vellum and Carly say pricing is unpublished. Treat the Spinnable numbers as unverified. [https://spinnable.ai/blog/what-is-instinct-ai-guide]

## Implications for a self-hosted personal agent gateway

1. **Approval gates with injection awareness.** Instinct's worst failures were an unapproved email and an injected inbox instruction. Mark inbox-ingested content as untrusted. Require owner confirmation, via a Discord/iMessage button or reply, for outbound sends, payments and credential use. Make the policy per-tool and default to deny.
2. **Per-job threads over one stream.** Users complained about Instinct's single thread. Map each task or omo agent run to its own Discord thread or iMessage tag. Keep a compact "inbox" lane for status.
3. **Lanes as first-class continuation.** Instinct's recurring loops and watchers (ticket monitoring) map to cron plus long-lived lane tasks. Give each a persistent state file, a wake condition and a budget. Support rate limiting, since Instinct's hammering got an account banned.
4. **Disconnect means purge.** Instinct kept data after disconnect. Make connector revoke atomically delete the cached tokens and the derived index entries, and expose an audit command.
5. **Credential vault with scoped handoff.** Keep logins in the macOS Keychain, with the agent receiving short-lived scoped capabilities rather than raw passwords. Never let an agent autonomously reset passwords without approval.
6. **Use the owner's real Mac as the browser.** Springwater's point is that the cloud VM lacks the user's cookies and residential IP. A Mac daemon already has that, so offer a logged-in browser profile with a visible action log.
7. **Status and health endpoint.** Instinct went down with no status page. Provide `/health`, a "what is each lane doing" command, and a daily heartbeat message.
8. **Proactive outreach with quiet hours.** Calling or texting first is the valued feature. Implement it as rules-driven digests with quiet hours and a per-channel rate cap.

## CLAIMS

- CLAIM: The legal entity is Spear Street Technology, Inc. d/b/a Instinct, and the product ships a Mac and mobile app plus an agent that acts on the user's behalf. — RISK: normal — SOURCES: instinct.com — PRIMARY: https://www.instinct.com/privacy-policy
- CLAIM: Users can give Instinct third-party usernames and passwords, and it signs in on their behalf. — RISK: high — SOURCES: instinct.com — PRIMARY: https://www.instinct.com/privacy-policy
- CLAIM: Google Workspace is connected by OAuth and Google APIs, and that data is excluded from training. — RISK: normal — SOURCES: instinct.com — PRIMARY: https://www.instinct.com/privacy-policy
- CLAIM: Disconnecting an integration does not auto-delete indexed data. — RISK: high — SOURCES: instinct.com — PRIMARY: https://www.instinct.com/privacy-policy
- CLAIM: The agent runs on a persistent cloud computer with browser access and cached credentials, plus a top-level dispatcher agent and recurring loops. — RISK: high — SOURCES: vellum.ai, usecarly.com (both relaying Anish Acharya's tweet) — PRIMARY: none; the company has published no architecture docs.
- CLAIM: Instinct uses a proprietary core model, with no model choice and no API or SDK. — RISK: high — SOURCES: vellum.ai (competitor) — PRIMARY: none
- CLAIM: Founder is Noah Shinn, entity registered April 2026. — RISK: normal — SOURCES: forbes.com via vellum.ai and usecarly.com — PRIMARY: https://www.forbes.com/sites/iainmartin/2026/08/26/vcs-are-so-obsessed-with-this-ai-assistant-that-its-valuation-jumped-fivefold-in-weeks/ (not retrievable here)
- CLAIM: Funding figures are inconsistent across sources ($250M vs $350M, $2.5B vs $10B). — RISK: high — SOURCES: vellum.ai, spinnable.ai, usecarly.com — PRIMARY: none

## EXPAND

- LEAD: Fetch the full Terms of Service at instinct.com/terms (the page body was cut off here). — WHY: it may confirm the Vault, device-data collection and agency clauses — ANGLE: legal and auth.
- LEAD: Read the TechCrunch, WSJ and Forbes articles directly. — WHY: possible founder interview detail on models and architecture (Forbes and CNN exceeded the fetch limit) — ANGLE: arch and press.
- LEAD: Anish Acharya's tweet comparing Instinct, Grok Bot and ChatGPT Work (x.com/illscience/status/2090456125858570702). — WHY: the only architectural description so far — ANGLE: arch.
- LEAD: Search for any Instinct engineering blog, job postings or conference talk. — WHY: would reveal the model stack, VM tech and memory design — ANGLE: models and memory.