<!-- task=st_01a144e9 -->
st_01a144e9 [completed] model mahoquot/claude-sonnet-5-5
Ask: research Instinct's product surface (channels, task assignment, proactivity, progress and approval UX). Wanted: a sourced report. For you: the report is below. Instinct's UX is confirmed for channels, onboarding and the "acts first, asks rarely" posture. Daily briefs, schedules and approval-gate UX are not documented anywhere I could reach. Now: done. Next: none.

## Identity
- **Name:** Instinct, a personal AI assistant.
- **Company:** Spear Street Technology, Inc. d/b/a Instinct, San Francisco. Its own Terms and Privacy Policy name it, and the privacy policy was last revised Aug 26, 2026.
- **Founder:** Noah Shinn, ex-Sierra research scientist. TechCrunch says a small team led by him created it. Forbes, via Vellum and Carly, says he registered the entity in April 2026.
- **Official URL:** instinct.com. The page's canonical link is instinct.com, but TechCrunch and Vellum link instinct.co, and both serve the same site. Login and settings are at app.instinct.com (older sources say app.instinct.co).
- **Launch:** invite-only service launched August 2026. The viral wave was Aug 20–24 and TechCrunch's privacy piece ran Aug 24.
- **Funding:** TechCrunch reports a $1B Series C at a $10B valuation on Sep 28. The earlier round was at $2.5B.
- **Confidence:** high on identity (company, product, URL, launch month). Medium on UX details, because there are no official docs, help center or demo. Everything below comes from the company's thin homepage and legal pages plus press and competitor blogs.
- **Caveat:** Spinnable (a competitor) wrote one of the two seed sources. I treated its unsourced claims as weak.

## Findings
**Surface and channels**
- The homepage says "there are no new interfaces": you text or call it, and it "can use a phone and computer in the same way that you do." The only call to action is "Text Instinct", which links to app.instinct.com/login. [https://instinct.com/]
- TechCrunch says you text it or call it, via text message or WhatsApp. [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/]
- Vellum says iMessage and WhatsApp plus phone calls, and Carly says "including iMessage". [https://www.vellum.ai/md/blog/official-instinct-breakdown] [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Spinnable lists SMS, WhatsApp, iMessage and calls. It also says Slack and Teams are not supported, but that is a competitor's claim. [https://www.spinnable.ai/blog/spinnable-ai-vs-instinct.md]
- I found no evidence of Slack, Discord, email-as-a-channel or KakaoTalk. As of Sep 28 it had no mobile app and relied on SMS and texting. [https://techcrunch.com/wp-json/wp/v2/posts?slug=viral-ai-agent-instinct-raises-1b-series-c-at-a-10b-valuation]
- The privacy policy defines the service as including "related Mac OS and mobile applications". It says voice and audio are collected "should you choose to enable" them and that precise location is collected "should you choose to share it." [https://instinct.com/privacy-policy]
- There is a web workspace and settings area. Opt-out of training is at /settings, and deleting indexed external data is at /workspace. [https://instinct.com/privacy-policy]
- Everything happens in one continuous thread with no projects or workspaces. Several early users asked for separate threads per job. Vellum calls this the main UX complaint. [https://www.vellum.ai/md/blog/official-instinct-breakdown] [https://www.usecarly.com/blog/what-is-instinct-ai/]

**Onboarding**
- You get in by waitlist or by invitation from an existing member. Vellum says the company is "scaling compute". [https://www.vellum.ai/md/blog/official-instinct-breakdown]
- You connect Google Workspace (Calendar, Gmail, Drive, Docs, Sheets, Slides, Tasks) through OAuth. You can revoke access by messaging the agent or from settings. [https://instinct.com/privacy-policy]
- Disconnecting does not delete data already indexed. Claire Vo disconnected Google at 11 AM and still got an inbox summary at 2 PM, and the company then added a deletion tool. [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/]
- Users also hand over third-party usernames, passwords and payment details so it can log in and pay. The agent pulled a sign-up code out of a user's inbox to book a Resy table. [https://instinct.com/privacy-policy] [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/]
- It runs on "its own phone number and computer", a persistent cloud machine. [https://techcrunch.com/wp-json/wp/v2/posts?slug=viral-ai-agent-instinct-raises-1b-series-c-at-a-10b-valuation]
- Vellum and the Spinnable comparison also describe an optional Mac app. The Spinnable comparison says it records screen and keystrokes and is not an inbox-only product. [https://www.spinnable.ai/blog/spinnable-ai-vs-instinct.md]

**How tasks are assigned**
- Tasks are free-form natural-language texts or calls: ingredients, a train home, groceries, flowers, a tennis court. [https://instinct.com/]
- Reported task types include travel and reservations, bill negotiation (a Comcast bill), vendor coordination over WhatsApp, subscription audits, calendar sync, school logistics and CRM or data-room work. One user sent 677 messages in 5 days and listed 15 completed jobs. [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Newer features: "concierge", which phones places that have no online booking, and a "trusted person network" in which your agent coordinates plans with friends' agents. [https://techcrunch.com/wp-json/wp/v2/posts?slug=viral-ai-agent-instinct-raises-1b-series-c-at-a-10b-valuation]
- There is no sign of a task board, tickets or an explicit job object. This is inference from "one continuous thread" plus the lack of any contrary evidence.

**Proactivity (partly unverified)**
- Vellum says the model is trained to follow up on threads you dropped, call or text you first, and arrange the ride to the airport. Carly says Instinct "can proactively call or text the user". These claims come from third parties paraphrasing company statements, not from official docs. [https://www.vellum.ai/md/blog/official-instinct-breakdown] [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Investor Anish Acharya describes persistent agents with "recurring loops" and a top-level agent that monitors and dispatches work, as relayed by Carly and Vellum. [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Watcher-style use is reported: one user had it monitor sold-out IMAX showings and it found cancellation tickets within a day. A tester also reported it summarized an inbox unprompted after Google was disconnected. [https://www.usecarly.com/blog/what-is-instinct-ai/] [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/]
- I found no mention of a morning or daily brief, user-visible schedules, cron-like triggers or watcher configuration. These may exist but are not documented. Treat any such claim as speculation.
- Over-eager polling has real costs. CNN reported a Resy account banned after Instinct pinged the site "hundreds of times every hour". [https://www.usecarly.com/blog/what-is-instinct-ai/]

**Progress reporting and approval**
- The default posture is to finish the job. Mohnot argues most assistants stop to confirm purchases, cancellations and messages, and Instinct's willingness to continue is the point. Acharya says it reset a password on its own to complete a purchase. [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Approval is not systematic. Katie Jacobs Stanton reported it sent an email on her behalf without checking first, and she then disconnected email. Spinnable's own guide separately lists unapproved replies to professional contacts (a competitor's claim). [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/] [https://www.spinnable.ai/blog/what-is-instinct-ai-guide]
- Vellum states Instinct's own Terms place responsibility for the agent's actions on the user, and name the service as the user's agent for binding transactions. [https://www.vellum.ai/md/blog/official-instinct-breakdown]
- The policy says only: "We encourage you to review actions taken by the personal assistant." [https://instinct.com/privacy-policy]
- There is no evidence of a structured approval UI (buttons, risk badges, limits). Vellum's critique is that Instinct "hasn't built" a control layer. Progress reporting seems to be ordinary chat replies, with a user complaining of 13 messages for an Amazon order that takes two taps in the app. [https://www.usecarly.com/blog/what-is-instinct-ai/]
- Prompt injection was demonstrated: Alex Cohen emailed his own inbox instructions and Instinct followed them. [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/]

**Reliability signals**
- A high-demand ticket test crashed repeatedly. Many users were unresponsive on Sep 21–22 with no status page. [https://www.usecarly.com/blog/what-is-instinct-ai/]

**Pricing**
- Free in private access with no published price. [https://www.vellum.ai/md/blog/official-instinct-breakdown]
- Spinnable's "$200–$500/month" figure is an unsourced estimate. Its own comparison page says the price is not announced. I discount the figure. [https://www.spinnable.ai/blog/what-is-instinct-ai-guide] [https://www.spinnable.ai/blog/spinnable-ai-vs-instinct.md]

## Implications for a self-hosted personal agent gateway
1. **Per-job threads over one stream.** The biggest UX complaint about Instinct is the single crowded thread. In Discord, map each task or omo run to its own thread, with a short summary in the main channel.
2. **Risk-tiered approval, not all or nothing.** Instinct's failures were unapproved sends and acting on injected instructions. Classify actions as read, draft, send or spend, auto-run only the read and draft tiers, and ask on iMessage or Discord for the rest. Add per-action budgets.
3. **Treat inbox content as untrusted.** Ingestion should tag mail as data, not instructions. Block tool calls whose justification comes only from email text, and require owner confirmation for anything triggered by ingested content.
4. **Visible watchers with polite rate limits.** Instinct's ticket and booking watchers worked, but CNN reported an account ban from over-polling. Make watchers first-class objects (target, interval, backoff, expiry) that you can list, pause and kill, instead of hidden loops.
5. **A first-class morning brief and "no news" digests.** I found no documented briefs from Instinct, so this is an open gap. A cron-driven brief from inbox, calendar and omo run status would beat it. Keep proactive pings low-volume and quiet-hours aware.
6. **Disconnect means delete.** Instinct kept indexed email after OAuth disconnect. A `revoke` command should purge derived data and report what it deleted.
7. **Honest progress reports.** Post start, blocked, needs-approval and done events with a link to logs, not 13 chat messages. Keep a status endpoint, since Instinct had outages and no status page.
8. **Lane-based concurrency.** Run personal errands, coding agents and cron jobs in separate lanes with their own credentials, so a stuck errand can't block a coding run. A "concierge" lane for phone-call-style tasks could be a later, optional addition.

## CLAIMS
- CLAIM: Instinct is a text and call personal assistant from Spear Street Technology, Inc. d/b/a Instinct, run by Noah Shinn, with a $1B Series C at a $10B valuation in Sep 2026. — RISK: normal — SOURCES: techcrunch.com, instinct.com — PRIMARY: https://instinct.com/privacy-policy
- CLAIM: Channels are SMS, iMessage, WhatsApp and phone calls; no Slack, Discord or email channel is documented. — RISK: normal — SOURCES: instinct.com, techcrunch.com, vellum.ai, usecarly.com — PRIMARY: https://instinct.com/
- CLAIM: Instinct acts proactively (follow-ups, calling or texting first, recurring loops). — RISK: high — SOURCES: vellum.ai, usecarly.com (secondary; no official doc) — PRIMARY: none
- CLAIM: Instinct has no systematic approval gate and has sent email without asking. — RISK: high — SOURCES: techcrunch.com, usecarly.com — PRIMARY: https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/
- CLAIM: Disconnecting Google does not delete indexed data; deletion is a separate action at app.instinct.com/workspace. — RISK: normal — SOURCES: instinct.com, techcrunch.com — PRIMARY: https://instinct.com/privacy-policy
- CLAIM: It runs on its own phone number and cloud computer, with concierge calling and a trusted-person network. — RISK: normal — SOURCES: techcrunch.com — PRIMARY: https://techcrunch.com/wp-json/wp/v2/posts?slug=viral-ai-agent-instinct-raises-1b-series-c-at-a-10b-valuation
- CLAIM: Instinct is free in beta; Spinnable's "$200–$500/month" estimate is unsourced. — RISK: normal — SOURCES: vellum.ai, spinnable.ai — PRIMARY: none
- CLAIM: Spinnable's figures disagree with others: it says $350M Series B, while the WSJ-via-TechCrunch figure is $250M at $2.5B. — RISK: normal — SOURCES: spinnable.ai, techcrunch.com, vellum.ai — PRIMARY: https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/

## EXPAND
- LEAD: Instinct's real Terms of Service (instinct.com/terms) and any Aug 26 diff — WHY: it would give the exact agency, approval and liability language, and the one-line training/opt-out changes — ANGLE: legal/product.
- LEAD: The WSJ piece "The latest viral AI assistant rocketing across Silicon Valley" and Forbes Aug 26 — WHY: likely quotes from Shinn on proactivity and approval behavior (both paywalled or too large to fetch here) — ANGLE: product.
- LEAD: Early-user X threads (Mohnot, Acharya, Claire Vo, Peter Yang) — WHY: they contain screenshots of the actual message flows, daily-brief behavior and "ask first" prompts; I couldn't access X — ANGLE: product/UX.
- LEAD: Carly's "Why people hate Instinct AI" and "Instinct AI funding" posts — WHY: the reliability and incident timeline for Sep 21–22 and the CNN Resy ban story — ANGLE: reliability.
- LEAD: Meta's Muse versus Instinct — WHY: it is the newest competitor and shows where consumer-agent UX is heading — ANGLE: competitive.

Verification: I read the primary pages directly (instinct.com home and privacy policy, TechCrunch Aug 24 and Sep 28 bodies, Vellum's markdown and Spinnable's guide and comparison). I did not run web_search, which isn't available this turn. Instead I used curl and HN Algolia. Reddit returned only stylesheet content. Carly's post was truncated at about 50 KB. The Forbes page exceeded the output limit. The first Spinnable fetch was also truncated.