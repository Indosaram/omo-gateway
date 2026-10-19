<!-- task=st_01a144ed -->
st_01a144ed [completed] model mahoquot/claude-sonnet-5-5
Ask: research Pickle's product surface and UX. Wanted: a sourced report. For you: the entity is confirmed, and most of the product detail comes from the App Store listing. pickle.com is a JS-only SPA, and the YC page body was truncated, so there is no official docs, help-center or blog text. Now: done. Next: none.

## Identity
- **Name:** Pickle (Korean app name 피클). The App Store title is "Pickle: Your personal AI agent" in the US and "피클: 나만의 AI 에이전트" in KR.
- **Company:** Pickle, Inc. (seller), developer account "Pickle Company". The YC page describes it as founded in 2024 by Sanio Jung, Emmett Kim, Daniel Park and Hojin Yu, with 6 employees in San Francisco. Its tagline is "The personal AI agent for Korea." [https://www.ycombinator.com/companies/pickle-2]
- **Official URLs:** pickle.com (App Store `sellerUrl`), App Store id6795790129 (bundle `com.pickle.client`), GitHub org `pickle-com`. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- **Launch date:** App Store first release 2026-08-10; current version 1.1.12 dated 2026-10-13. iOS 17+, free, 17+ rating. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- **Ratings:** 4.59 from 41 ratings in the US store; 5.0 from 3 in KR. That is a small sample.
- **Confidence:** high (about 95%) that this is the right entity. The YC slug, the App Store ID and the `pickle-com/credential-enclave` repo all agree. The repo's providers list includes NAVER Mail, and its README says "the iOS app of that account."
- **Probable history (inference):** Korean press in Feb and Apr 2025 covered a YC/NFX-backed "Pickle" building a video-meeting "clone", which is probably the same company before a pivot. I did not verify this link. [https://wowtale.net]
- **Not Pickle:** "피클플러스" (a different Korean firm co-developing a "personal hybrid AI agent" with AIVE, Aug 2026). [https://www.newswire.co.kr]

## Findings
Findings come from the App Store listing text (first-party marketing copy), the open-source README, and the YC page.

**Surface and channels**
- The primary surface is an iPhone app. The listing's "Message, voice, or phone" section says you can send a message, talk by voice, or call Pickle on the phone. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- The app asks for optional permissions: microphone for voice messages and calls with Pickle, notifications for task progress, completion and suggestions, camera, and photos/video shared in the Pickle conversation. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- I found no evidence of SMS, iMessage, email or Slack as chat channels. Slack, Notion, X, Google, Microsoft, Link, Granola, Mercury and NAVER Mail appear only as credential providers, which are data and action connections and not user-facing channels. The listing names Gmail and Calendar. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- The KR listing names Naver, KakaoTalk, Gmail, Google Calendar, Gov24, Coupang, telecom carriers, delivery, insurance and airlines as connectable services. The US listing names Gmail, Calendar, Amazon, DoorDash, Uber Eats, Instacart, OpenTable, United, Airbnb, Target and Uber. Which services appear is localized per market. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- The app is localized in EN, JA and KO. Invite links `/i/:token` carry a market (kr/jp), so the product is likely invite and referral driven (inferred from a code comment in the pickle.com HTML). [https://pickle.com/terms-of-service]

**How tasks are assigned and executed**
- Tasks are assigned by message: "Send Pickle a message and it carries out the task." It calls businesses, navigates websites, fills forms and completes checkout. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- It makes phone calls on the user's behalf: customer-service lines, bill reductions, promo rates, refunds and phone-only reservations. You follow the call in real time and get a summary at the end. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- It orders from stores like Amazon and Target, with US payment through Link by Stripe. You can track or cancel an order from the chat. In KR the listing names Coupang and Daiso Mall. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- It books restaurants, appointments, flights, stays, event tickets and KTX trains (KR listing). [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- Agent-to-agent scheduling: "Your Pickle and your friend's Pickle talk to each other" to find a time, pick a place and make the reservation. The user can view the conversation at any time. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- I saw no assignment mechanism beyond chat, voice and phone. There was no mention of a task board, email-to-task or share-sheet intake.

**Proactivity**
- Pickle suggests tasks from your email and calendar. The listed examples are a missing-package claim, contesting a parking ticket, canceling unused subscriptions, pausing a gym membership while travelling, and renewing a car registration. The KR version lists flight refund requests, checkup bookings and anniversary restaurant bookings. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- This is suggestion-first. Nothing says it executes these tasks unprompted. The notification permission text covers "suggestion alerts," which matches that reading. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- A narrow watcher exists: "When seats open up for something you wanted, it lets you know and books them on request." The KR version says it notifies when a cancellation ticket appears for a sold-out seat, and books immediately if asked. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- I found no evidence of daily briefs, user-defined cron/schedules or general triggers. This is an absence in marketing copy and is not proof the features don't exist.

**Progress reporting and approval**
- Anything that spends money or can't easily be undone waits for approval first. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- Live observability: you can watch calls and the browser in real time, open the browser yourself at any point (human takeover), and disconnect any app at any time. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- The listing carries a fallibility disclaimer: "Pickle can be inaccurate or take an unexpected action. Check in on larger tasks and step in when needed." [https://itunes.apple.com/lookup?id=6795790129&country=us]
- Progress notifications cover task progress, completion and suggestions. [https://itunes.apple.com/lookup?id=6795790129&country=kr]

**Credential component (open source)**
- `pickle-com/credential-enclave` is licensed Apache-2.0 and is written in Rust. It holds delegated user keys inside an AWS Nitro Enclave. The last push was 2026-10-15, and it is the only product repo besides a `tau2-bench` fork. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- Per-user master key: it is created on the iPhone, kept in iCloud Keychain, and never reaches the node or operator. [same README]
- Delegation: the app grants the node a user key for at most 30 days, held in memory only. The node erases it on expiry or revoke. [same README]
- Audit log: every use of a credential writes a sealed, signed log entry to S3 with Object Lock (365 days) before the act. Only the account holder can decrypt it. [same README]
- Vault release: `release` hands one field at a time (password, card number, CVC) to the operator's "worker," which fills a form. The README states plainly that protection ends at that point. [same README]
- The README admits the iOS app is not open source yet, so "the app is honest" is an assumption. [same README]
- Design takeaway: "log before act" with an append-only, user-readable audit trail is a core trust mechanism. This is the README's own design.

**Unverified items from the seed**
- "Each user has their own Pickle working in its own cloud": I found no first-party text confirming a per-user VM or sandbox. Reporting only mentions a per-user browser session, so this is unverified.
- "Raise your AI": I found no first-party text with this slogan. I did not find it in the App Store text or the pickle.com HTML.

## Implications for a self-hosted personal agent gateway
1. **Approval gate by action class.** Pickle auto-gates anything that spends money or is irreversible. Add a declarative `requires_approval` tag per tool or lane. Send the owner a Discord/iMessage/KakaoTalk prompt with a one-tap approve/deny and a timeout default of deny.
2. **Suggestion inbox, not auto-execution.** Pickle turns email and calendar into proposed tasks that the user accepts. Your inbox ingestion should emit "proposals" into a queue, and the owner promotes them to an omo job. Only run unattended after an explicit per-rule opt-in.
3. **Narrow watchers.** The "notify when a seat opens, book on request" pattern fits cron: a `watch` job with a condition, a notify action and an optional pre-authorized follow-up action. It is cheaper and safer than a general trigger system.
4. **Live progress feed and human takeover.** Stream status events (started, step N, waiting for approval, done) to the owner's chat thread. For browser or coding runs, add a `take over` command that pauses the agent and hands over the session.
5. **Append-only audit log written before the side effect.** Copy the credential-enclave rule: write the entry, wait for fsync or ack, then act. Make it readable by the owner via a `/log` command. A local hash-chained file is enough on a Mac.
6. **Credential custody as a separate process.** Keep tokens in a small Rust sidecar (macOS Keychain, not a Nitro Enclave). Agents get a handle and a short-lived grant (Pickle uses 30 days max), never plaintext. Let `release` return one field at a time and log each release.
7. **Agent-to-agent coordination, scoped narrowly.** Pickle's friend-to-friend scheduling shows the pattern. A minimal version is a signed request/response envelope between two gateways (propose slots, accept slot) with a viewable transcript. Treat it as optional and later.
8. **Per-channel UX differences.** Pickle is a single app surface. You have three chat channels, so treat the approval and progress UX as channel-agnostic and render per channel. Voice or phone is out of scope unless you add it.

## CLAIMS
- CLAIM: The Pickle App Store ID 6795790129 was first released 2026-08-10 by Pickle, Inc. — RISK: normal — SOURCES: itunes.apple.com — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=us
- CLAIM: The product handles calls, orders, bookings and friend-to-friend scheduling through agent-to-agent conversation, and gates spending and irreversible actions behind approval — RISK: normal — SOURCES: itunes.apple.com (vendor marketing copy) — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=us
- CLAIM: Pickle proposes tasks from email and calendar, and its only evidenced watcher is for seat or ticket availability — RISK: normal — SOURCES: itunes.apple.com — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=us
- CLAIM: The credential-enclave repo (Apache-2.0, Rust) holds user keys in a Nitro Enclave for at most 30 days, logs every use to S3 Object Lock for 365 days before acting, and the iOS app is not open source — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md
- CLAIM: The YC listing gives founders Sanio Jung, Emmett Kim, Daniel Park and Hojin Yu, founded 2024, 6 employees in San Francisco — RISK: normal — SOURCES: ycombinator.com — PRIMARY: https://www.ycombinator.com/companies/pickle-2
- CLAIM: Pickle (the current agent) is the same company as the 2025 "video-meeting clone" Pickle — RISK: high (unverified inference) — SOURCES: wowtale.net, etnews.com headlines only — PRIMARY: none
- CLAIM: Each user's Pickle runs in its own dedicated cloud — RISK: high (unverified) — SOURCES: none found — PRIMARY: none
- CLAIM: Pickle has no SMS, iMessage, email or Slack chat channel and no daily-brief feature — RISK: high (absence in marketing copy only) — SOURCES: itunes.apple.com — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=us

## EXPAND
- LEAD: pickle.com content (landing page, whitepaper, privacy-article) is JS-rendered and returned only the HTML shell. Fetch `/assets/main-*.js` or the language strings bundle, which my tool broker could not retrieve, or read via a real browser. WHY: it likely holds the first-party product narrative, the "Raise your AI" slogan and the per-user cloud architecture. ANGLE: product and architecture.
- LEAD: the repo docs `docs/protocol.md`, `docs/enclave.md` and `docs/egress-policy.md`. WHY: they describe the worker-side form-filling and what leaves the node. ANGLE: security and architecture.
- LEAD: the full YC page text (founder bio, "Launch YC" posts) beyond the truncated head. WHY: it would give launch positioning, traction and the 2025 history. ANGLE: company and funding.
- LEAD: Korean press from Aug to Oct 2026 about the launch. WHY: independent confirmation of features such as KakaoTalk integration. Google News RSS returned no direct hit, so try Naver or Kakao news and the Korean spelling "피클 AI 에이전트 샌프란시스코". ANGLE: secondary and market.