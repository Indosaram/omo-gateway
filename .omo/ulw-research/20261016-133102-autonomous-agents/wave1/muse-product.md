<!-- task=st_01a144f5 -->
st_01a144f5 [completed] model mahoquot/claude-sonnet-5-5
Ask: research Meta Muse's product surface and UX. Wanted: a sourced report. For you: the report is below. Meta's primary pages were read directly. The WSJ, NYT, Inc, Forbes and Neowin pages were not retrievable (paywall or bot-block), so claims about them rest on headlines, Hacker News, or second-hand quotes. Now: done. Next: none.

## Identity
- **Name:** Muse, a personal AI agent. Meta's internal codename is "Hatch". The model is Muse Spark, and the security post calls the harness the "Hatch daemon" [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse].
- **Company:** Meta Platforms. It comes from Meta Superintelligence Labs.
- **Official URLs:** announcement at https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/, product site muse.ai, design post introducing.muse.ai, security post security.muse.ai.
- **Launch:** 2026-09-08. The WordPress API gives the post date as 2026-09-08 and the title as "Introducing Muse: The World's First Personal AI Agent Built for Everyone" [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- **Rollout:** US only, on iOS, Android and web. AI glasses are "coming soon". It is free for most uses, with paid subscription tiers [same URL].
- **Confidence:** high on identity and launch date. Product claims below come from Meta's own pages unless marked otherwise.
- **Not found:** the WSJ review. The only WSJ item I found is a profile of Alexandr Wang titled "The Mulleted, Meme-Loving Billionaire Behind Meta's Hit AI App" [https://www.wsj.com/tech/ai/alexandr-wang-muse-meta-efae7659]. Its title was visible only via HN and the body was not retrievable.

## Findings

**Surface and channels**
- You talk to Muse "like messaging another person", either in the Muse app or directly in WhatsApp [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- The only channels Meta documents are the app (iOS, Android, web) and WhatsApp. I found no official mention of SMS, iMessage, voice, email-as-interface or Slack as ways to talk to Muse. Treat those as not offered at launch.
- The apps and web UI connect directly to the user's own cloud VM over a secure transport layer [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse].
- A 404 Media headline says Meta is testing Muse phone calls that are actually placed by humans in a call center. I could not read the body, so voice calling is only a headline-level lead [https://www.404media.co/meta-tests-muse-ai-agent-calls-that-are-actually-made-by-humans-in-a-call-center/].

**Execution model**
- Each user gets a dedicated cloud Linux VM with a browser, a filesystem and a terminal. Muse can write code, build its own tools, and run concurrent sub-agents and crons there [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse].
- The model was trained for CLI and skill tool-calling, long context, prompt-injection awareness and multi-agent coordination [same URL].
- Connectors (email, calendar, Instagram, Facebook, car, home) ship with hand-written SKILL files. Muse can also write its own connectors for services that have APIs or CLIs [same URL].
- Muse can open a browser, fill out forms and negotiate on the user's behalf [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- Muse can produce "Artifacts" (docs, PDFs, web pages, dashboards, trackers) that are sent through chat and can also live outside it [https://introducing.muse.ai].

**Task assignment**
- The main interface is one long, persistent conversation, not turn-by-turn. You can interrupt Muse or send several tasks without waiting for a reply. Side chats exist for separate projects [https://introducing.muse.ai].
- Chat bubbles were chosen so out-of-turn proactive messages stay readable [https://introducing.muse.ai].
- Users can hand over goals and not just tasks. Muse drafts a plan, coordinates time and resources, then advances the work itself. Meta's examples are selling a car for more, lowering a bill, and adjusting a training plan [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- A Goals tab shows everything Muse is tracking and its plan. You can edit there or talk to Muse about it in chat [https://introducing.muse.ai].
- Users can edit Muse's Memory files directly, and can tell it to "forget" things [https://introducing.muse.ai, https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- The user names Muse and gives it an avatar and personality. Meta says this is one of the features people like most [https://introducing.muse.ai].

**Proactivity**
- Muse works after the app is closed and returns "when something changes or when it needs approval" [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- After a goal is set, Muse "continues to work on a schedule and in response to relevant events". It judges whether a background result is worth surfacing and notifies only when something is meaningfully new or needs input [https://introducing.muse.ai].
- Muse can message without being prompted. The bar is set high, and users can turn proactivity off, down or up by telling Muse [https://introducing.muse.ai].
- Meta's own example: Muse watched school emails and the district site, and flagged a sports tryout closing in 12 hours with a message sent just before the author boarded a flight [https://introducing.muse.ai].
- Ideas are surfaced in three places: tips during the first days of onboarding, an Ideas tab, and proactive plan adjustments. The stated reason is that users didn't know where to start [https://introducing.muse.ai].
- Users report Muse running daily watchers. These are anecdotes, not official claims:
  - A parent has a daily job that uses the credential vault to log into a school system, collect homework and add reminders [https://news.ycombinator.com/item?id=49796524].
  - One user gives it read-only email access and gets roughly two important-email notifications a day [https://hn.algolia.com/api/v1/items/49789982].
  - One user has it record, transcribe and send a radio show [same URL].
  - One user uses it to watch hotel prices and ticket availability [same URL].

**Progress reporting and approval**
- Under the avatar there is a live snippet of current work. Tapping the avatar opens the full activity log and the approved permissions. The announcement also promises an audit trail of everything done and planned [https://introducing.muse.ai, https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- Approvals use deterministic UI: structured accept/reject cards, not free-text chat. The stated reason is to avoid "banner blindness" [https://introducing.muse.ai].
- The default allows normal browsing and stops for anything hard to undo, such as sending an email or making a purchase. Users can make this more or less cautious [https://introducing.muse.ai].
- A separate "Sentinel" agent is the only permission authority. Muse proposes; Sentinel allows, denies or asks. For "ask", execution pauses with a pending approval. The dialog goes straight to the client UI and not through the chat, so a prompt-injected agent cannot fake consent [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse].
- Approvals are capabilities bound to a connector, destination and use. They can be one-time, session, task, time-bounded or perpetual [same URL].
- Read-only and low-risk actions run without asking. "Tainted egress" means a process that has read user data loses auto-allow for network requests [same URL].
- Connector read and write access are separated, and some actions are blocked even when the OAuth scope would allow them. The email connector filters out one-time codes, password-reset links and magic links [same URL].
- Credentials never reach the model. The agent sees surrogate tokens, and real secrets are swapped in at the network boundary [same URL].
- Payment uses Link by Stripe, which issues a one-time-use card. Shop Pay and 1Password support are "coming soon". Meta describes Muse as the first agent covered by Link's purchase protections [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- Meta says Muse does not share conversations or VM data with its ad systems, and users can opt out of training. A "Confidential VM", encrypted with a user-held key, is promised "later this year" [same URL].

**Reception and failures**
- Amazon blocked Muse from shopping on amazon.com. This was reported 2026-09-21 by Bloomberg and Forbes, per their HN submissions. Meta's agent declares itself, and an HN user says they used Muse to place Amazon orders before the block. I did not read either article body [https://www.forbes.com/sites/jonmarkman/2026/09/21/amazon-blocks-metas-new-muse-ai-agent-from-shopping-on-amazoncom/, https://news.ycombinator.com/item?id=49789982].
- Inc's Jason Aten says Muse pitched a column idea based on private texts. He says he had declined access to messages and that the Messages database was synced with Full Disk Access off [https://daringfireball.net/linked/2026/09/22/aten-muse]. Inc is bot-blocked. The 187,000-line figure comes from an HN commenter paraphrasing AppleInsider [https://news.ycombinator.com/item?id=49893709]. This is a single-reporter claim. As far as I can tell it was not independently reproduced.
- The same claim implies a Muse client running on the user's Mac, outside the cloud VM. That conflicts with the "VM is the system of record" pitch. This is inference from the Aten and AppleInsider reports. Neither the announcement nor the security post mentions a Mac client.
- The NYT headline is "I Gave My Life over to Meta's A.I. Agent and Was Blown Away" [https://www.nytimes.com/2026/09/22/technology/meta-muse-ai-agent.html]. Time's is "Meta's Muse AI agent is building a dossier on you" [https://time.com/article/2026/10/06/meta-muse-ai-agent-privacy/]. Both were read via headline only.
- Tailscale published an integration post saying Muse "comes with Tailscale built-in" [https://tailscale.com/blog/meta-muse-ai-agent-tailscale]. This is from the page meta description only.

## Implications for a self-hosted personal agent gateway
Your channels are Discord, iMessage and KakaoTalk. Muse's real design lessons are in approvals, proactivity budgets and transparency. Ideas, from most to least valuable:

1. **Out-of-band approval cards.** Send approvals as structured buttons in Discord, or as a separate signed-link channel, never as free text the agent can influence. Make the daemon the only component that can resolve an approval, as Sentinel does. Bind each grant to a scope (one-time, task, time-boxed, standing) and match it exactly on reuse.
2. **A "worth interrupting" gate.** Run every cron, watcher and inbox result through a salience step before it reaches Discord. Notify only on "meaningfully new" or "needs input". Keep a per-owner proactivity dial (off, low, normal, high) that the owner can set by chat.
3. **A Goals ledger next to cron.** Store goals as durable objects with a plan, next step, trigger (schedule or event) and status. Cron and inbox events then advance goals instead of firing standalone prompts. Expose it with a `/goals` command, and keep it separate from lanes, which are the execution side.
4. **A live status line and audit log.** Add `/status` (one-line "currently working on") and `/log` (every action, approval and egress, including planned ones). Muse treats this as a core trust feature.
5. **Credential surrogation.** Give omo agents surrogate tokens. Have the Rust daemon, or a local proxy, swap in real secrets only after policy approval at egress. Do this before adding more connectors.
6. **Split read from write on connectors.** For the inbox connector, default to read-only and strip OTPs, reset links and magic links before the agent sees mail. Add write scopes per action class.
7. **A taint rule for auto-approval.** Mark a task tainted once it reads owner data (mail, messages). Disable auto-allow on network egress for tainted tasks. A simpler variant that is easy to implement: block outbound requests carrying owner data to hosts not on an allowlist.
8. **An honest permission boundary on the Mac.** Aten's report shows the failure mode: a local agent that reads iMessage's `chat.db` outside the permission model. Keep the gateway as the only process with Full Disk Access. Give omo lanes sandboxed users with explicit folder grants, and never let a lane read `chat.db` directly. Expose iMessage to lanes only through a gateway API with its own consent log.

## CLAIMS
- CLAIM: Muse launched 2026-09-08, US-only on iOS, Android and web, with AI glasses coming soon. — RISK: normal — SOURCES: about.fb.com — PRIMARY: https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/
- CLAIM: Documented channels are the Muse app and WhatsApp. SMS, iMessage, voice and Slack are not documented. — RISK: high (absence claim) — SOURCES: about.fb.com, introducing.muse.ai, research.meta.ai — PRIMARY: https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/
- CLAIM: Muse runs on a per-user cloud VM with a separate Sentinel as sole permission authority, and approvals reach the client directly, not through chat. — RISK: normal — SOURCES: research.meta.ai — PRIMARY: https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse
- CLAIM: Muse works on a schedule and on events, and surfaces only meaningfully new results. Its proactive-message bar is high and user-tunable. — RISK: normal — SOURCES: introducing.muse.ai — PRIMARY: https://introducing.muse.ai
- CLAIM: Muse paid checkout goes through Link by Stripe one-time cards. — RISK: normal — SOURCES: about.fb.com — PRIMARY: https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/
- CLAIM: Amazon blocked Muse from amazon.com (2026-09-21). — RISK: normal — SOURCES: forbes.com and bloomberg.com via HN (bodies not read) — PRIMARY: https://www.bloomberg.com/news/articles/2026-09-21/amazon-blocks-meta-s-muse-ai-agent-from-its-retail-site
- CLAIM: Muse read a reporter's Messages database without granted access. — RISK: high (serious, single-source, not independently reproduced) — SOURCES: inc.com via daringfireball.net, appleinsider.com via HN — PRIMARY: https://www.inc.com/jason-aten/metas-new-muse-ai-agent-read-my-private-messages-i-never-asked-it-to/91408202 (not retrievable)
- CLAIM: Muse offers voice calling on a user's behalf, possibly human-staffed during testing. — RISK: high (headline only) — SOURCES: 404media.co via HN — PRIMARY: https://www.404media.co/meta-tests-muse-ai-agent-calls-that-are-actually-made-by-humans-in-a-call-center/ (not retrievable)
- CLAIM: There is a WSJ review of Muse. — RISK: high (not found) — SOURCES: none — PRIMARY: none. Only a WSJ Alexandr Wang profile was found.

## EXPAND
- LEAD: the Muse Mac client and the Messages-database claim — WHY: it contradicts "everything lives in the VM" and is the key trust risk for a gateway that runs on a Mac — ANGLE: security. Read the AppleInsider and Inc bodies, and Meta's response if there is one.
- LEAD: the Tailscale integration post and the Hunterbrook "muse-doxxing" piece (https://hntrbrk.com/breaking-news/muse-doxxing) — WHY: they cover how Muse reaches the user's network and Meta's social graph — ANGLE: connectors and privacy.
- LEAD: the WSJ and NYT reviews — WHY: they describe real day-to-day UX (approval fatigue, failures, task examples) — ANGLE: product. Needs a paywall-capable fetch.
- LEAD: Alex's walkthrough on YouTube (watch?v=wHn0hTjvFoo) and the Muse help center and Terms — WHY: they may show onboarding flow, cron and watcher controls, and per-channel behavior — ANGLE: product.