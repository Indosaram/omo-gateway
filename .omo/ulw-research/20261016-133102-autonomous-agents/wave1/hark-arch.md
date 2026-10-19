<!-- task=st_01a144fe -->
st_01a144fe [completed] model mahoquot/claude-sonnet-5-5
Ask: research Hark's architecture, with identity verification, as a sourced report. Wanted: a ≤1800-word report with sources and gateway ideas. For you: the identity is confirmed and the primary pages were read, but nothing public documents a developer API or SDK, and Hark has not published how its long-running tasks work. Now: done. Next: none.

I read this as an information ask, so the report below is the end state.

## Identity
- **Name and company:** Hark (Hark Labs, "Hark AI" on LinkedIn). It is an "artificial intelligence lab" founded and led by Brett Adcock, who also runs Figure AI. [https://hark.com/articles/hark-announces-usd700-million-fundraising-round]
- **Official URL:** hark.com. The consumer product is "Hark Pro" (iOS, Android, web). "Handoff" is the cloud computer-use agent and model inside it. Hark is the company and Hark Pro is the app. [https://hark.com/using-hark]
- **Dates:**
  - $700M Series A at a $6B post-money valuation was announced 2026-05-20. [https://hark.com/articles/hark-announces-usd700-million-fundraising-round]
  - Handoff research preview was published 2026-08-05. [https://hark.com/articles/introducing-hark-handoff]
  - Hark Pro launched 2026-10-06. [https://hark.com/articles/introducing-hark-pro]
  - First hardware is due in 2027. [https://hark.com/using-hark]
- **Seeds confirmed:** The TNW article (2026-10-06, Darius Popa) matches. It cites Bloomberg and says the tiers are free, $20 and $100. [https://thenextweb.com/news/hark-pro-agent-hardware-europe] The rockingrobots.com search page shows a snippet saying Brett Adcock's Hark "has introduced Handoff". I did not read that article body. [https://rockingrobots.com/?s=hark]
- **The "Hark decision intelligence" listing:** I found nothing matching it. The older "hark.com" hits on HN are a 2011 audio-clips site, a different entity. [https://hn.algolia.com/api/v1/search?query=hark.com] I could not check the decision-intelligence listing directly, so treat it as unrelated.
- **Confidence:** high (~95%) on the entity, because every fact below comes from hark.com or TNW. Architecture details are what Hark chose to disclose, so they are marketing-grade and thin.

## Findings

**Execution environment**
- For every request, Hark spins up a dedicated virtual computer with its own browser, file system and terminal. [https://hark.com/articles/introducing-hark-handoff]
- Handoff is "a computer in the cloud, operated by Hark". [https://hark.com/using-hark]
- Handoff runs up to 6 browsers at once. TNW first printed 36 and corrected it to 6 after Hark said so. [https://hark.com/articles/introducing-hark-pro] [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- Handoff is a pure computer-use agent. Its output is cursor and keyboard actions at x/y screen coordinates (click, type, scroll) against a live browser. It does not use DOM or accessibility-tree selectors. [https://hark.com/articles/introducing-hark-handoff]
- Hark chose the browser because it says 74.9% of user screen time (nearly 3M screen-minutes surveyed) is spent there, and fewer than 1 in 1000 sites have a public API. [https://hark.com/articles/introducing-hark-handoff]
- Hark publishes no local-device access for Hark Pro. I found no desktop agent or local file bridge. The agent runs on Hark's cloud, and hardware arrives in 2027. [https://hark.com/using-hark]
- The web agent reads on-screen pixels only. Hark itself says bot-blocking, pop-ups and ads are the main obstacle and that its modeling and engineering teams co-evolve the model with the harness. [https://hark.com/articles/introducing-hark-handoff]

**Connectors, integrations and auth**
- There are two integration paths. Handoff covers "thousands of sites" through the browser. Direct API connections, which Hark calls "Services", currently include Google Workspace and Microsoft 365, "and more". [https://hark.com/using-hark]
- Outside the browser, Hark reads files, accesses external databases and finds information through MCP. It says Google and Outlook connect via APIs. [https://hark.com/articles/introducing-hark-pro]
- Email access is treated as foundational. Hark says it needs your email to get a "snapshot of your life", schedule your calendar and send messages. [https://hark.com/using-hark]
- Panels (mini apps on Home) pull from connected accounts like Strava, Netflix, ParentSquare and Venmo. [https://hark.com/articles/introducing-hark-pro] [https://hark.com/using-hark]
- Credentials live in an in-house encrypted vault called "Secured by Hark". Cards and passwords are tokenized and only detokenized on the destination page, so Hark staff and Hark itself cannot see them. [https://hark.com/using-hark]
- The article adds that the password is encrypted from vault to website "until the moment it lands". [https://hark.com/articles/introducing-hark-pro]
- Whether the model sees raw credentials is not stated. The tokenize-then-detokenize-in-page wording implies it only sees placeholders. That is my inference. [https://hark.com/security]
- OAuth specifics are not documented. The pages mention neither OAuth nor per-service scopes. [https://hark.com/using-hark]
- A trust center exists at trust.hark.com, but it is gated behind a request-access form. [https://trust.hark.com/]

**Human-in-the-loop and safety**
- Users can watch the computer-use agent act live, and there is always a cancel button. [https://hark.com/security]
- Tasks "default to consent", and Hark learns over time which actions you want it to take. [https://hark.com/security]
- Action Buttons are the evolution of notifications. They are generated from email and conversation sources, and a tap executes ("Pay", "Cancel"). [https://hark.com/using-hark]
- Auto top-up for usage credits has a user-set ceiling, with "protections so you don't accidentally overspend". [https://hark.com/using-hark]
- TNW notes that a payment approved by software has no carve-out from EU strong customer authentication rules. Hark has not said whether Hark Pro is available in Europe. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]

**Memory and personal context**
- Memory is one of three pillars, alongside the cloud computer and proactive thinking. [https://hark.com/articles/introducing-hark-pro]
- It builds a "portrait" of you from what you say and what it learns. It applies that to every interaction and honors "forget this" requests. [https://hark.com/articles/introducing-hark-pro]
- Deletion is conversational ("ask Hark to forget"). Account-level deletion is at Settings → Profile → Delete Account. [https://hark.com/using-hark]
- Connected accounts (email, calendar) seed context, and Handoff acts with saved addresses, preferences and history. [https://hark.com/articles/introducing-hark-handoff]
- "Specialties" are multi-step workflows Hark memorizes (for example how you negotiate returns). Hark can create them automatically or on request. This is effectively learned playbooks, not documented as a skill format. [https://hark.com/using-hark]
- The storage backend, retrieval method and embedding approach are not published.

**Long-running and continuation model**
- The UI is one persistent chat thread. "Projects" are separate threads with their own files for long-horizon work. [https://hark.com/using-hark]
- Proactivity is a stated pillar. Hark spots things and paces follow-up steps over time instead of dumping them into chat. The example is a movie ticket, then a parking suggestion, then the QR code at the right moment. [https://hark.com/articles/introducing-hark-pro]
- Scheduled Tasks (a daily coffee order, a weekly reminder) are created by asking in natural language. [https://hark.com/using-hark]
- Order tracking can lag by minutes, so external push notifications may arrive first. This suggests polling rather than webhooks, but that is my inference. [https://hark.com/using-hark]
- Checkpointing, resume semantics and failure retry are not documented.

**Multi-agent structure**
- The documented structure is one chat agent that invokes Handoff when browser work is needed. The user "asks Hark to do something that requires a web browser" and Hark takes over. [https://hark.com/using-hark]
- Hark explicitly says users should not have to juggle "agents" or "tokens". [https://hark.com/using-hark]
- Six parallel browsers imply a concurrent sub-task structure, but Hark calls Handoff one model plus a harness and does not describe a planner/worker split. Speculation: a router plus parallel browser workers. [https://hark.com/articles/introducing-hark-pro]

**Models**
- Handoff is Hark's own model, post-trained with SFT then RL on live-web tasks. Hark says it has moved into mid-training and plans pre-training later in 2026. [https://hark.com/articles/introducing-hark-handoff]
- The base model is not named.
- Hark claims #1 on the Online-Mind2Web human-evaluation leaderboard, beating GPT 5.4 by 8 points on average and Opus 4.8 by 2. Hark says costs per token are an order of magnitude lower. These are self-reported. [https://hark.com/articles/introducing-hark-handoff]
- The latency note says about 10s of browser latency per step on top of model latency. [https://hark.com/articles/introducing-hark-handoff]
- The security page says Hark "prioritizes its own models whenever best". Vetted third-party models have 0 data retention. [https://hark.com/security]
- Infrastructure includes a B200 training data center and an NVIDIA partnership. The stack is Megatron for training, Dynamo for inference, Nemotron pretraining data and NVSentinel. The Vera Rubin platform is mentioned for later. [https://hark.com/articles/hark-announces-multi-year-partnership-with-nvidia] [https://hark.com/articles/hark-announces-usd700-million-fundraising-round]

**Pricing and hardware**
- All features are free with a usage cap. Pro² is $20/mo for 2x usage and Pro³ is $100/mo for 10x. [https://hark.com/using-hark]
- TNW notes Meta's Muse launched at the same three prices four weeks earlier. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- Hardware is due in 2027. Design head Abidur Chowdhury (ex-Apple) says "we don't really believe in glasses with cameras or a pin you can wear", and AT&T supplies cellular. [https://thenextweb.com/news/hark-pro-agent-hardware-europe] [https://hark.com/articles/at-and-t-and-hark-formalize-strategic-partnership]

**Developer APIs and SDKs**
- I found none. The sitemap lists no docs or developer pages, and the only product surface is the app. [https://hark.com/sitemap.xml]
- Handoff is described as "how to use" through the consumer app only. [https://hark.com/articles/introducing-hark-handoff]
- The consumer app MCP support I found is client-side only (Hark consumes MCP). Nothing says Hark exposes an MCP server. [https://hark.com/articles/introducing-hark-pro]

## Implications for a self-hosted personal agent gateway
Hark makes a few ideas visible. These are my suggestions, not claims about Hark.

1. **Action Buttons from the inbox.** Have the inbox ingestion step turn bills, subscriptions and flight emails into one-tap proposals (a Discord button, an iMessage reply token, a KakaoTalk quick reply). Execution happens only on tap. This is Hark's pattern with the safe default of consent first.
2. **Consent ledger with learned auto-approval.** Hark's "default to consent, learn what you want" model fits a per-action-class allowlist. Store approvals in the daemon and promote a class to auto-approve only after N explicit approvals with a spend ceiling. Auto top-up's ceiling is the same shape.
3. **Credential broker, never in the model context.** The Secured-by-Hark idea is a vault that injects secrets into the final page or process. Keep secrets in the macOS Keychain and let a daemon-side injector fill login fields for browser tasks, so omo agents only ever see placeholders.
4. **Watch and cancel for every lane.** Hark's "open kitchen" is live visibility plus a cancel button. Stream task status and screenshots to the owner's chat, and give every lane a `/cancel` that kills the process tree.
5. **Paced proactive follow-ups.** The "movie ticket then parking then QR code at the right moment" example is a cron-plus-state pattern. Store a plan as timed steps, deliver each when relevant, and avoid dumping a batch of messages.
6. **Specialties as saved playbooks.** Hark memorizes multi-step workflows. Let the owner promote a successful run into a named, versioned playbook that cron or a chat command can re-run.
7. **Projects as per-topic threads and memory.** Give each topic its own thread, files and memory scope next to the global profile, with a "forget this" command that deletes entries and is logged.
8. **Honest scope.** Hark's cloud browser cannot touch your Mac, whereas your daemon can. That is a differentiator, but it needs a stricter blast-radius policy for local file and shell access than Hark has to think about.

## CLAIMS
- CLAIM: Hark Pro launched 2026-10-06 as a free iOS/Android/web app with $20 and $100 tiers (2x and 10x usage). — RISK: normal — SOURCES: hark.com, thenextweb.com — PRIMARY: https://hark.com/using-hark
- CLAIM: Each Handoff request gets a dedicated cloud virtual computer with browser, file system and terminal. — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/articles/introducing-hark-handoff
- CLAIM: Handoff is Hark's own post-trained (SFT+RL) coordinate-based computer-use model that tops Online-Mind2Web. — RISK: high (self-reported benchmark) — SOURCES: hark.com — PRIMARY: https://hark.com/articles/introducing-hark-handoff
- CLAIM: Handoff runs up to 6 concurrent browsers. — RISK: normal — SOURCES: hark.com, thenextweb.com (corrected from 36) — PRIMARY: https://hark.com/articles/introducing-hark-pro
- CLAIM: Passwords and cards are stored in the "Secured by Hark" vault and are only detokenized on the destination page, and Hark cannot see them. — RISK: high (unverifiable vendor claim; trust center is gated) — SOURCES: hark.com — PRIMARY: https://hark.com/using-hark
- CLAIM: Hark Pro integrates Google Workspace and Microsoft 365 by direct API ("Services"), other sites through Handoff, and MCP. — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/using-hark
- CLAIM: Hark has raised $700M+ Series A at $6B, partners with NVIDIA, and plans hardware in 2027. — RISK: normal — SOURCES: hark.com, thenextweb.com — PRIMARY: https://hark.com/articles/hark-announces-usd700-million-fundraising-round
- CLAIM: No public developer API or SDK for Hark or Handoff exists. — RISK: normal (absence claim, based on the sitemap and articles) — SOURCES: hark.com — PRIMARY: https://hark.com/sitemap.xml
- CLAIM: Hark Pro's availability in Europe is unstated, and TNW says software-approved payments have no carve-out from EU strong customer authentication rules. — RISK: normal — SOURCES: thenextweb.com — PRIMARY: https://thenextweb.com/news/hark-pro-agent-hardware-europe

## EXPAND
- LEAD: Bloomberg's 2026-10-06 launch article, which may add details beyond the press materials (Bloomberg URL is in the TNW piece). — WHY: it may include founder-interview detail on memory, proactivity or safety that Hark's own pages omit. — ANGLE: press/reporting.
- LEAD: Hark Pro App Store and Play Store listings (id6772362941, com.hark.android) and the Privacy Policy / Terms. — WHY: data-type declarations and permission lists show which device permissions the app actually requests, and the privacy policy may name the subprocessors and model vendors. — ANGLE: privacy/permissions.
- LEAD: trust.hark.com, access request. — WHY: it likely holds the SOC 2/GDPR documents and subprocessor lists that would confirm or contradict the vault claims. — ANGLE: security.
- LEAD: Hark Handoff benchmark pages (Online-Mind2Web leaderboard, WebTailBench). — WHY: independent confirmation of the #1 claim. — ANGLE: model evaluation.
- LEAD: the full rockingrobots.com Handoff article, whose body I did not read. — WHY: it is a named seed that may add secondary commentary. — ANGLE: press.
- LEAD: X posts from @hark_labs and Brett Adcock around 2026-10-06 (x_search was not available in my tool set). — WHY: launch-day architecture hints and user reports of failures. — ANGLE: social signal.

I could not check the privacy policy, the app store listings or the trust center behind its access form, and none of the architecture claims above are independently verified.