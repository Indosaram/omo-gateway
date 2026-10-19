<!-- task=st_01a144fd -->
st_01a144fd [completed] model mahoquot/claude-sonnet-5-5
Ask: research Hark's product surface and UX. Wanted: a sourced report with identity, findings, ideas and claims. For you: I read the primary sources (Hark's own CMS-backed pages) and the TNW and RockingRobots articles. Hark does not offer SMS, iMessage, voice or Slack as chat channels, and I found no documented approval gate. Now: done. Next: none.

I could only pull the TNW article's headline and summary, not its body, because the page was truncated.

## Identity

- **Name:** Hark. Hark Pro is the app and Handoff is the computer-use agent. Hark's own FAQ says "Hark is the name of our company and the personal intelligence we're building. Hark Pro is the software presentation… on the web along with iOS and Android apps." [https://hark.com/using-hark]
- **Company:** an AI lab founded by Brett Adcock, who is also CEO of Figure AI. It describes itself as pairing its own foundation models with bespoke hardware. [https://www.rockingrobots.com/hark-unveils-handoff-an-ai-agent-that-completes-tasks-directly-in-a-browser/] Adcock is quoted as Hark's founder and CEO in the AT&T release. [https://hark.com/articles/at-and-t-and-hark-formalize-strategic-partnership]
- **Official URL:** https://hark.com. A Hacker News submission of hark.com (2026-10-06, 3 points, 0 comments) is titled "Hark Pro – Personal AI that can use the web and get things done for you". [https://hn.algolia.com/api/v1/search_by_date?query=%22Hark%20Pro%22&tags=story]
- **Timeline:**
  - 2026-05-20: $700M Series A at a $6B post-money valuation. [https://hark.com/articles/hark-announces-usd700-million-fundraising-round]
  - 2026-08-05: Handoff research preview. [https://hark.com/articles/introducing-hark-handoff]
  - 2026-08-20 and 08-27: AT&T and NVIDIA partnerships.
  - 2026-10-06: Hark Pro public launch. [https://hark.com/articles/introducing-hark-pro]
- **Confidence:** high that this is the right entity. The seed description (groceries, cars, bills, free/$20/$100 tiers, 2027 devices) matches Hark's own pages.
- **Unrelated "Hark decision intelligence" listing:** I did not fetch or verify it. The hark.com content shows no decision-intelligence product, so I treat it as a different company.

I read Hark's pages through their Sanity CMS API (the site is client-rendered). The content is first-party and equivalent to the live pages.

## Findings

**Positioning and surface**
- Hark Pro is an iOS, Android and web app, free to everyone at launch. The company says it is "not a chatbot, not a personal assistant, not an AI agent". [https://hark.com/articles/introducing-hark-pro]
- Everything happens in "a single, persistent chat thread", with no "agents" or "tokens" to juggle. [https://hark.com/using-hark]
- There are three main views:
  - **Home** holds Action Cards and Panels.
  - **Chat** is the main thread, where Handoffs happen.
  - **Projects** are topical chat threads that keep their own files. [https://hark.com/using-hark]
- The interface feels like a plain messaging app. Hark "speaks in short bursts rather than long novels", with charts and rich media only when needed. [https://hark.com/articles/introducing-hark-pro]
- The background is a live sky simulation for the user's location. This is cosmetic, but it shows how much weight they put on the feel of the app. [https://hark.com/articles/introducing-hark-pro]

**Channels**
- I found no SMS, iMessage, voice or email-as-chat-interface channel. The documented platforms are iOS, Android and web only. [https://hark.com/using-hark]
- Slack is listed as a tracked or summarised source, not a chat surface. The pricing page lists "Tracks Slack" and "Juggles email" as features. [https://hark.com/pricing]
- Voice and vision appear only as hardware-roadmap language. AT&T says users will "interact more naturally with computers through speech, vision, and memory". [https://hark.com/articles/at-and-t-and-hark-formalize-strategic-partnership]

**Assigning tasks**
- You ask in natural language. "To start a Handoff, all you do is ask Hark to do something that requires it to use a web browser. Hark will take it from there." [https://hark.com/using-hark]
- Use cases named in the FAQ: order groceries, manage your calendar, build mini apps, book flights, summarize Slack threads, track projects with Google Docs. [https://hark.com/using-hark]
- Hark is described as acting through browsers where there is no API, and through "Services" (direct API connections) elsewhere. The current Services are Google Workspace and Microsoft 365, "and more". [https://hark.com/using-hark]
- Handoff itself:
  - Hark spins up a dedicated virtual computer per request, with its own browser, filesystem and terminal. [https://hark.com/articles/introducing-hark-handoff]
  - The Pro launch post says it runs up to 6 browsers at once and logs in to "millions" of sites. [https://hark.com/articles/introducing-hark-pro]
  - It can also read files, use external databases and connect over MCP. [https://hark.com/articles/introducing-hark-pro]
- Onboarding: connecting email is explained as "the fastest way to get a snapshot of your life". Email also gives Hark a way to schedule your calendar and send messages. [https://hark.com/using-hark]
- Connected accounts and logins live in an encrypted vault called "Secured by Hark". Hark says it cannot see inside the vault, and credentials are detokenized only on the target page. [https://hark.com/using-hark]

**Proactivity (the strongest differentiator)**
- The Pro launch post calls "proactive thinking" one of three core components, alongside Handoff and memory. Hark "uses its own tools like Handoff, to actually solve challenges ahead before you even encounter them". [https://hark.com/articles/introducing-hark-pro]
- Example: after a movie-ticket purchase, Hark offers to reserve nearby parking and later sends the QR code. It "will pace these steps out so they arrive at just the right time" instead of piling them into chat. [https://hark.com/articles/introducing-hark-pro]
- Users can also create their own scheduled tasks and reminders. The FAQ gives "a coffee delivered every day" and "a weekly meeting" as examples. [https://hark.com/using-hark]
- Spontaneous suggestions were seen in practice. Hark spotted a student's lunch card running low and flagged a refill. "If you allow it", it can fix such problems itself, for example by topping off the card. [https://hark.com/articles/introducing-hark-pro]
- Action Buttons replace notifications. Cards on Home carry actions like "Pay" a bill or "Cancel" a subscription, generated from what Hark sees in your email and has learned from your conversations. [https://hark.com/using-hark]
- Panels are user-requested mini-apps (Strava, Venmo, ParentSquare, Netflix with Rotten Tomatoes scores). Each has a "Reply" action to redesign it or ask questions of its data. [https://hark.com/using-hark]
- Specialties are workflows Hark memorises to repeat multi-step tasks, such as a way you want returns negotiated. Hark "sets up these specialties automatically", or you can ask for one. [https://hark.com/using-hark]
- Order tracking has a lag. Hark "can have a few minute delay" tracking deliveries, so external push notifications may arrive first. This suggests it polls rather than receives push events. That is my inference. [https://hark.com/using-hark]
- I found no documented triggers, watchers or daily brief. "Home" is the closest thing, and it is a feed, not a morning digest.

**Progress reporting and approvals**
- There is no written description of a mid-task progress UI. The only evidence is promo videos of live screen recordings of Handoff working. [https://hark.com/articles/introducing-hark-handoff]
- No explicit pre-purchase confirmation flow is documented. The closest evidence is the opt-in wording "if you allow it" for autonomous fixes, and one-tap Action Buttons for suggested actions. Treat the approval model as undocumented. [https://hark.com/articles/introducing-hark-pro]
- The company's stated design principle is "Hark will always ask the least of you possible to do the most for you that it can". [https://hark.com/using-hark]
- TNW's angle is regulatory: in Europe, "software approving a payment has no carve-out from the SCA rules". This implies approval and authentication design matters for EU use. That is from the article's summary only; I did not read the body. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]

**Memory**
- Hark builds a portrait of preferences, such as no mayo, window seats and document formatting, and applies it to every interaction. You can ask it to forget a memory. [https://hark.com/articles/introducing-hark-pro]

**Pricing**
- Free tier with all features. Pro² is $20/month for "2x more usage" and Pro³ is $100/month for "10x more usage". [https://hark.com/pricing]
- All features are in every plan, and upgrading only raises usage. [https://hark.com/pricing]
- Optional auto top-up of credits has a user-set ceiling. [https://hark.com/using-hark]

**Hardware**
- The first physical product is promised for 2027, and "Hark Pro is the software foundation that comes first". [https://hark.com/using-hark]
- The AT&T release says devices will connect "completely untethered from your phone or Wi-Fi". [https://hark.com/articles/at-and-t-and-hark-formalize-strategic-partnership]

**Handoff model claims (vendor-reported)**
- Handoff tops the Online-Mind2Web human-evaluation leaderboard, per Hark's post. It claims to beat GPT 5.4 by about 8 points and Opus 4.8 by about 2 on average. [https://hark.com/articles/introducing-hark-handoff]
- RockingRobots notes the performance claims were not independently verified. It also says the preview model was post-trained, with a fully pre-trained one planned for later in 2026. [https://www.rockingrobots.com/hark-unveils-handoff-an-ai-agent-that-completes-tasks-directly-in-a-browser/]

## Implications for a self-hosted personal agent gateway

1. **One persistent thread plus a "Home" feed.** Keep a single owner conversation per channel (Discord, iMessage, KakaoTalk), and add a rolling "Home" digest as a pinned Discord channel or a daily brief. Hark's lack of an explicit brief is a gap you can fill.
2. **Action Buttons as approval primitive.** Turn each proposed side effect into a one-tap card: Discord components, or reply-keyword shortcuts on iMessage and KakaoTalk. Where Hark's approval model is undocumented, your gateway can make it explicit: per-action risk tier, auto-run only on opted-in categories ("if you allow it").
3. **Paced follow-ups.** Queue dependent suggestions with timed release, as in the movie-then-parking example, using your cron lane. Don't dump every step into chat at once.
4. **Inbox ingestion to proposals.** Derive candidate actions (pay bill, cancel subscription, refill) from ingested mail, and store them as pending cards with expiry. This matches Hark's Action Card source model.
5. **Specialties as saved playbooks.** Let an omo run that succeeded be promoted into a named, replayable workflow, and have the agent suggest the promotion automatically after repeated runs.
6. **Usage and budget guardrails.** Hark sells usage multiples and auto top-up with a ceiling. For a self-hosted gateway, mirror this as per-lane spend and run caps with a hard ceiling, so proactive runs can't surprise you.
7. **Pollers with visible lag.** Hark's few-minute delay on order tracking shows the cost of polling. Give watchers explicit intervals and say in the message how stale the data is.
8. **Vault-style credential isolation.** Keep secrets out of model context and inject them only at the point of use (the Secured by Hark pattern). This is a design idea only; there is no evidence of how Hark implements it.

## CLAIMS

- CLAIM: Hark Pro launched publicly on 2026-10-06 as a free iOS, Android and web app with $20 (2x usage) and $100 (10x usage) tiers. — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/articles/introducing-hark-pro
- CLAIM: Hark is Brett Adcock's lab and raised a $700M Series A at a $6B valuation. — RISK: normal — SOURCES: hark.com, rockingrobots.com — PRIMARY: https://hark.com/articles/hark-announces-usd700-million-fundraising-round
- CLAIM: Hark is proactive: it offers follow-up actions, spots problems like a low lunch card, and can fix some "if you allow it". — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/articles/introducing-hark-pro
- CLAIM: There is no SMS, iMessage, voice or Slack chat channel at launch; the platforms are iOS, Android and web. — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/using-hark (an absence claim; Hark may add channels or have unpublished ones)
- CLAIM: Handoff beats GPT 5.4 and Opus 4.8 on browser-use benchmarks. — RISK: high — SOURCES: hark.com, rockingrobots.com — PRIMARY: https://hark.com/articles/introducing-hark-handoff (vendor-reported; RockingRobots says it is unverified)
- CLAIM: First hardware ships in 2027. — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/using-hark (a company roadmap statement)
- CLAIM: The approval or confirmation flow for purchases is not publicly documented. — RISK: high — SOURCES: hark.com — PRIMARY: none (absence of evidence; I could not use the app)
- CLAIM: EU SCA rules apply with no carve-out to software approving payments. — RISK: high — SOURCES: thenextweb.com — PRIMARY: https://thenextweb.com/news/hark-pro-agent-hardware-europe (from the page's summary only, not the full text)

## EXPAND

- LEAD: Read the TNW article body (SCA and PSD2 analysis of agent payments). WHY: it is the only source on regulated approval requirements for agent-initiated payments, and I only got its summary. ANGLE: compliance and payments.
- LEAD: Hands-on teardown: App Store listing, screenshots, and Reddit or X user reports of how approval prompts and live progress actually look. WHY: the UX details of approvals and progress are undocumented in first-party copy. ANGLE: hands-on UX.
- LEAD: The "surprise lie detector test" section of the security page and the Secured by Hark architecture. WHY: it bears on whether credential isolation claims are verifiable. ANGLE: security.
- LEAD: Whether the unrelated "Hark decision intelligence" listing needs explicit disambiguation. WHY: I did not verify it. ANGLE: entity disambiguation.