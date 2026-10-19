<!-- task=st_01a144f0 -->
st_01a144f0 [completed] model mahoquot/claude-sonnet-5-5
Ask: research Pickle's reception and failure modes, critique angle. Wanted: a sourced report. For you: I found no hands-on reviews, user complaints or Amazon-blocking reports for the agent app. The main critique-relevant finding is that this company is the same one Daring Fireball and HN called fraudulent over its AR glasses in January 2026. Now: done. Next: none.

I had no web_search tool, only read-only gh/curl. I covered the YC page, the iTunes lookup API, pickle.com (through a text proxy), the GitHub org, and HN Algolia. Reddit and Korean press returned nothing usable.

## Identity
- **Name:** Pickle, shown in the KR App Store as "피클: 나만의 AI 에이전트". Seller is Pickle, Inc.; developer is "Pickle Company". Bundle ID is `com.pickle.client`.
- **Official URLs:** https://pickle.com, https://www.ycombinator.com/companies/pickle-2, https://github.com/pickle-com/credential-enclave.
- **Launch:** the app was released 2026-08-10. The latest version is 1.1.12, from 2026-10-13. Google Play is "coming soon".
- **YC page:** "personal AI agent for Korea", founded 2024 by Sanio Jung, Emmett Kim, Daniel Park and Hojin Yu, with 6 employees in San Francisco.
- **Confidence:** high on the entity (YC page, App Store id 6795790129 and the GitHub org all agree). Medium-high that this is the same company as the discredited "Pickle 1" AR glasses. Daring Fireball links the same `pickle-2` YC page and names the same CEO, Daniel Park. The footer of pickle.com links x.com/danifesto, which I did not resolve to Park.
- **Not to be confused with:** picklebrowser.com (an unrelated "Show HN: Pickle agent browser"), Pickle Robot, ApplyPilot.

## Findings
- **No independent hands-on reviews found.** HN Algolia has no story about the Pickle agent app ("Raise your AI", "pickle ai agent korea", and Korean-language queries all returned zero hits). I did not find any press review either. Treat "reception" as unknown, not positive. [https://hn.algolia.com/api/v1/search?query=pickle%20ai%20agent%20korea]
- **App Store signal is tiny.** The KR listing shows 3 ratings averaging 5.0, release notes of just "버그 수정 및 개선", and a 17+ rating. Three ratings is not meaningful, and I suspect friends and team. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- **The listing admits unreliability.** It says Pickle "may perform inaccurate or unexpected actions" and tells users to watch progress and intervene. That is the vendor's own failure-mode disclosure. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- **Scope claimed.** The listing says Pickle:
  - phones businesses, including carrier plan changes, refunds and reservations;
  - orders on Coupang and Daiso Mall;
  - books KTX, flights and restaurants, and watches for cancellation seats;
  - negotiates meetups through friends' Pickles, with the agent-to-agent conversation viewable;
  - connects Naver, KakaoTalk, Gmail, Calendar, Gov24, carriers, delivery, insurance and airlines;
  - proactively suggests tasks from mail and calendar.
  
  All of this is vendor-stated and unverified by any reviewer. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- **Site blocking (Amazon etc.): no evidence found either way.** The listing names Coupang and Daiso, not Amazon, and does not mention bot-detection. Its stated model is that Pickle drives a browser and shows the user a live view. Any blocking would come from Korean sites' anti-bot measures, which I did not find evidence on. This is speculation. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- **Reputational baggage.** In January 2026 an HN thread titled "Pickle 1 AR Glasses (YC W25) May Be Fraudulent" reached 110 points and 32 comments. It was built on Matthew Dowd's analysis of the claimed waveguide, SLAM and latency specs. [https://hn.algolia.com/api/v1/items/46483429]
- **Gruber's take.** John Gruber called the glasses claims a "sham" taking $200 pre-orders and linked the YC `pickle-2` page. He described the company as "a 15-person startup founded in 2024 that has raised less than $10 million". [https://daringfireball.net/linked/2026/01/03/dowd-pickle-expose]
- **The debate was contested.** A commenter claiming a computer-vision degree argued that Dowd's technical claims were wrong in several places, but did not defend Pickle's capability claims. Another commenter noted the general pattern of startups relabelling Chinese OEM hardware. [https://hn.algolia.com/api/v1/items/46483429]
- **The pivot is visible.** The YC page now says 6 employees, down from the 15 Gruber cited. Earlier Pickle products were "Glass" (an open-source desktop assistant, HN July 2025), "Memory OS" and "Pickle 1". I infer the agent app is a pivot after the glasses fallout, but no source states that. [https://hn.algolia.com/api/v1/search?query=pickle.com] [https://www.ycombinator.com/companies/pickle-2]
- **Security posture is the strongest, best-documented part.** The open-source credential component holds user keys inside an AWS Nitro Enclave. It is Apache-2.0, created 2026-10-01, with 2 stars. It has reproducible builds, attestation and a CLI verifier. [https://github.com/pickle-com/credential-enclave]
- **The enclave README states its own limits.**
  - Vault values (passwords, card numbers, CVC) leave the enclave through `release` to the operator's worker, which fills the form. After that the enclave no longer protects them.
  - Data fetched with a token, such as mail, goes to the operator domain.
  - Slack and Notion lack PKCE, so the operator callback can exchange the code.
  - The iOS app is not open source ("the app is honest" is an assumption).
  - The operator can stop the service.
  
  [https://github.com/pickle-com/credential-enclave]
- **Pickle's own FAQ is more reassuring than that README.** It says passwords and cards are "never passed to Pickle (the AI)", and that staff can view conversations with every view logged. For payments it shows the cost first, and re-asks if the price rises just before paying. The "AI never sees it" claim is narrower than "Pickle's backend never sees it". [https://pickle.com/]
- **Integration pain is visible in the issue tracker.**
  - Naver IMAP needed a fix for UIDVALIDITY 0 plus UID FETCH item parenthesizing.
  - A mail parser was bounded and memory reserved per operation after a hardening pass.
  - Excess mail calls were kept out of a shared admission queue.
  - Nine PRs landed in 10 days (Oct 5 to 15), with patch releases v1.5.1 and v1.5.2.
  
  Fast iteration, but the mail layer was still being hardened after launch. [https://github.com/pickle-com/credential-enclave/issues?state=all]
- **Safety carve-outs exist in the device provider.** The SmartThings PR says the backend refuses some device commands (unlocking, opening, heat) per device, and leaves scenes out because they would bypass that refusal. It also notes the PR was produced with Claude Code. [https://github.com/pickle-com/credential-enclave/pull/6]
- **Competitor comparisons: none found.** I found no review comparing Pickle with OpenClaw-style gateways, Poke, Manus or Korean assistants.

## Implications for a self-hosted personal agent gateway
These are inferences from the findings above, not reviewer advice.
1. **Per-action approval gates for irreversible actions.** Show what will happen and the cost, and re-prompt if the amount changes before commit (Pickle's own FAQ pattern). Put them in lanes, with approval requests going over Discord, iMessage or KakaoTalk.
2. **A live, replayable action view.** Pickle exposes phone-call and browser sessions in real time. Give each run an append-only transcript plus a "take over" command in the chat channel.
3. **Agent-to-agent scheduling as a narrow, auditable protocol.** Do slot negotiation only, over a structured message type, and let the owner read the full exchange. Refuse free-form agent-to-agent tool use.
4. **Secret handling with an explicit trust boundary.** Pickle's README is candid that release-time plaintext reaches a worker. Keep credentials in the macOS Keychain, inject them only at the sink, and write an audit log entry before each use (Pickle's write-ahead log pattern). Document what the daemon itself can see.
5. **Inbox ingestion hardening.** Pickle's Naver IMAP fixes show real mail servers misbehave. Bound parser size and memory per operation, and keep a separate admission queue so a noisy mailbox cannot starve other lanes. Test against Naver and Kakao quirks such as UIDVALIDITY 0.
6. **Honest scope disclosure and a per-device deny list.** Ship a visible "may be wrong, check it" notice. Keep a default-deny list for physical-world commands (locks, heat, garage) and exclude "scene"-style macros that bypass it.
7. **Bot-block fallback.** Korean shopping and booking sites likely resist automation. Add a "handoff to owner" state that sends the page link and a pre-filled context instead of silently failing. This is speculative, since there is no evidence of Pickle's actual block rate.
8. **Proactive suggestions as drafts, never actions.** Pickle suggests tasks from mail and calendar. In a cron or inbox lane, produce a proposal and require an explicit yes before executing.

## CLAIMS
- CLAIM: Pickle (pickle-2 on YC) is the same company that sold the "Pickle 1" AR glasses, which HN and Daring Fireball called fraudulent in Jan 2026 — RISK: high — SOURCES: ycombinator.com, daringfireball.net, news.ycombinator.com — PRIMARY: https://www.ycombinator.com/companies/pickle-2
- CLAIM: Pickle's agent app has no independent reviews found, and its only public rating is 3 five-star ratings — RISK: normal — SOURCES: itunes.apple.com, hn.algolia.com — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=kr
- CLAIM: The open-source credential enclave is Apache-2.0 and Nitro-based, and its README documents that vault values reach the operator's worker at `release` — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/pickle-com/credential-enclave
- CLAIM: The agent is built for Korean services (Coupang, Naver, Kakao, KTX, Gov24) — RISK: normal — SOURCES: apps.apple.com via iTunes API — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=kr
- CLAIM: Pickle admits the agent can act inaccurately and tells users to supervise important tasks — RISK: normal — SOURCES: apps.apple.com via iTunes API — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=kr
- CLAIM: The agent app is a pivot from the glasses — RISK: high (inference, unsourced) — SOURCES: none direct — PRIMARY: none

## EXPAND
- LEAD: Matthew Dowd's original X thread and any Pickle or Daniel Park response, including whether glasses pre-orders were refunded — WHY: determines whether the trust problem is resolved and whether the pivot is a recovery — ANGLE: incident history, refunds
- LEAD: Korean press and communities (Naver blog, DC Inside, Clien, Threads, X, Korean app-store reviews) for "피클 AI 에이전트 후기" — WHY: the only likely source of real usage and failure reports, and my tools could not search it — ANGLE: hands-on reviews, Coupang and KTX failures
- LEAD: pickle.com/help and pickle.com/security pages, plus the Apple/Google review text for id6795790129 — WHY: help articles list supported sites, limits and known issues — ANGLE: documented failure modes, supported-site list
- LEAD: Pickle's YC launch post (Launch YC / Bookface) and pickle.com/research "how-we-make-critter-act-in-real-time" — WHY: funding, claims and the real-time voice architecture — ANGLE: voice latency, agent-to-agent design

Remaining uncertainty: the claim that reception is absent rests on the HN Algolia and iTunes queries only. No general web search, Reddit or Korean press was available to me.