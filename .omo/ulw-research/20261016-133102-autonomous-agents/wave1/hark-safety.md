<!-- task=st_01a144ff -->
st_01a144ff [completed] model mahoquot/claude-sonnet-5-5
Ask: research Hark's trust, safety and economics from primary sources. Wanted: a sourced report. For you: the entity is verified and the key policies were read. The trust center, the acceptable-usage policy and the Rockingrobots article were not retrieved. Now: done. Next: none.

I read this as an information ask, so I only read and made no edits. I'm stopping here because the report is complete.

## Identity
- **Name and company:** Hark Labs, Inc., 4000 N First St, San Jose, CA. The product is Hark Pro, which is the web, iOS and Android app. The computer-use agent inside it is **Handoff**. [https://hark.com/terms] [https://hark.com/using-hark]
- **Official URL:** hark.com. The trust center is trust.hark.com, but it returned empty when fetched. [https://hark.com/security]
- **Timeline:**
  - Brett Adcock founded Hark in late 2025 with $100M of his own money.
  - It left stealth around March 2026.
  - It raised more than $700M at a $6B valuation in May 2026.
  - Handoff was announced around 2026-08-05 (the HN submission date).
  - Hark Pro launched on 2026-10-06.
  - Hardware is due in 2027.
  - [https://thenextweb.com/news/hark-series-a-funding-adcock-ai-hardware] [https://hn.algolia.com/api/v1/search?query=hark.com&tags=story] [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- **Confidence:** high. The seed claims (free, $20 and $100 tiers, groceries, car booking, bills, 2027 devices) match both TNW and Hark's own pages.
- **Disambiguation:**
  - Other "Hark" items are unrelated, such as the old hark.com audio site (2011) and the open-source voice-prompt tool FPurchess/hark. [https://hn.algolia.com/api/v1/search?query=hark%20pro&tags=story]
  - I did not find any "Hark decision intelligence" listing, so I can't say which one it is. It is not this company.
  - I also did not retrieve the Rockingrobots article.

## Findings
**Permission and approval gates**
- By default, Hark asks you to confirm before it spends your money or sends a communication to others. The terms say "Unless you tell us otherwise." [https://hark.com/terms]
- Standing permission per category of transaction is supported. Every transaction inside that permission counts as authorized, and the user is liable for it. [https://hark.com/terms]
- Scheduled and proactive actions run only within the permissions you gave. You can pause or cancel them. Messages are sent in your name. [https://hark.com/terms]
- Hark can also decline, pause or stop any action it believes is unlawful, harmful or a security risk, even if you authorized it. It says it will tell you unless law or security prevents that. [https://hark.com/terms]
- The security page says tasks start "by defaulting to consent" and that Hark "remembers the actions you want it to take" over time. That implies permissions are learned, not just configured. This is my inference from marketing copy. [https://hark.com/security]
- Action Buttons ("Pay" a bill, "Cancel" a subscription) turn proactive suggestions into one-tap actions. They are generated from email and conversation context. [https://hark.com/using-hark]
- The launch post says that "if you allow it," Hark can fix problems on its own, such as topping up a lunch card. [https://hark.com/articles/introducing-hark-pro]
- Hark identifies itself as an AI acting for you when the law requires it or when a person is likely to read the message. [https://hark.com/terms]
- Auto top-up for usage credits is opt-in, with a user-set ceiling and "protections" against overspend. [https://hark.com/using-hark]

**Observability and audit**
- The security page says "You can watch our CUA perform actions" and "There's always a button to cancel your order." That is live viewing and a kill switch, not a persistent audit log. [https://hark.com/security]
- I found no user-facing audit log or per-action receipt feature in the pages I read. Users are told to review each confirmation. [https://hark.com/terms]
- Hark keeps task execution logs for debugging and security. They include shell commands, outputs, uploaded files and generated code. [https://hark.com/privacy-policy]
- Hark says it limits and audits its own employees' access to accounts. [https://hark.com/using-hark]

**Sandboxing and credentials**
- Each request gets a dedicated virtual computer with its own browser, file system and terminal. Handoff can run up to six browsers at once. TNW corrected an earlier "36" to six. [https://hark.com/articles/introducing-hark-handoff] [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- A vault called "Secured by Hark" holds cards and passwords. Hark claims it cannot see them, and that secrets are tokenized and only "detokenized on the actual page" of the destination site. [https://hark.com/using-hark] [https://hark.com/articles/introducing-hark-pro]
- Other connections go through direct APIs ("Services", such as Google Workspace and Microsoft 365) and MCP. [https://hark.com/articles/introducing-hark-pro]
- The terms say stored credentials may include session cookies and tokens. Users can remove them at any time. The "Access Features" clause covers using login sessions on the user's own device. [https://hark.com/terms]
- Liability for actions taken with stored credentials mostly stays with the user, except where Hark fails its own obligations. [https://hark.com/terms]

**Prompt injection**
- The only mention I found is a terms clause banning users from "prompt injecting" or bypassing Hark's safety systems. [https://hark.com/terms]
- Handoff's own post calls the web "a hostile place" but discusses bot-blocking and pop-ups, not injection. I found no published defenses. This is a gap in what I could read, not proof that none exist. [https://hark.com/articles/introducing-hark-handoff]

**Privacy and retention**
- Hark does not train on inputs, outputs, memory, connected-account data or sandbox data. It says no opt-out is needed. [https://hark.com/privacy-policy]
- Hark's third-party model providers must contractually bar training. The security page says vetted third-party models have "0 data retention." [https://hark.com/privacy-policy] [https://hark.com/security]
- Retention is "lifetime of your account," with legal-hold exceptions. Backups roll off within 90 days, but that figure is stated only in the biometric section. The terms say account closure deletes memory and stored credentials. [https://hark.com/privacy-policy] [https://hark.com/terms]
- Memory is user-editable by asking the agent to forget something. [https://hark.com/using-hark]
- The policy asks users not to give it sensitive data such as financial account numbers. That sits oddly next to the payment-and-bills use case. [https://hark.com/privacy-policy]
- Data is transferred to the US. SCCs are used for EU data, and EU and UK GDPR representatives are listed. [https://hark.com/privacy-policy]

**Pricing and limits**
- All features are free, with a usage cap. Pro² is $20 a month for 2x usage. Pro³ is $100 a month for 10x usage. [https://hark.com/articles/introducing-hark-pro] [https://hark.com/pricing]
- The paid tiers add usage only. Hark does not publish the cap as a number. Credits and top-ups are non-refundable. [https://hark.com/pricing] [https://hark.com/terms]
- TNW says Meta's Muse launched four weeks earlier at the same three prices. That claim is secondary and I did not verify it. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]

**Availability and regions**
- Hark Pro runs on web, iOS and Android. [https://hark.com/using-hark]
- TNW says Hark has not said whether it is available in Europe. It also says software approving a payment has no carve-out from PSD2 strong customer authentication. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- The terms and privacy policy include EEA/UK cancellation rights and GDPR representatives. That suggests Hark expects EU users, but it is not a confirmed launch. [https://hark.com/terms]

**Liability**
- Hark says it is not the merchant of record. Refunds and chargebacks go through the merchant or card issuer. [https://hark.com/terms]
- Liability is capped at the greater of 12 months of fees or $100 (US), or six months of fees or €100 (EU). [https://hark.com/terms]

**Benchmarks (vendor-reported)**
- Hark claims the top spot on the Online-Mind2Web human-eval leaderboard. [https://hark.com/articles/introducing-hark-handoff]

## Implications for a self-hosted gateway
1. **Three-state action policy per category** (`ask` | `standing` | `deny`), covering payments, outbound messages and deletes. Default to `ask`. Hark's model is ask-by-default with per-category standing permission. Add a hard per-action and per-day spend ceiling as well, because Hark's standing permissions have no visible cap.
2. **Approval over the owner channels.** Send a Discord, iMessage or KakaoTalk message showing the action, payee, amount and a diff. Accept approve or deny with a timeout that defaults to deny. Bind each approval to a hash of the exact action so it cannot be replayed.
3. **Append-only action ledger.** Log each action's lane, agent, tool call, inputs and result to an append-only file or SQLite table. Add `gateway audit tail` and a daily digest sent to the owner. Hark offers live viewing and a cancel button but shows no durable per-action record.
4. **Kill switch.** One owner command in any channel (`/stop`) cancels all running lanes. This mirrors Hark's "button to cancel your order."
5. **Taint tracking for inbox ingestion.** Mark anything from email or the web as untrusted. Untrusted-origin content cannot trigger `standing` actions and always forces `ask`. Hark publishes no injection defenses, so this is where you can do better.
6. **Secret broker.** Agents get opaque handles and a small broker fills credentials at the point of use. This is a local analogue of "Secured by Hark." Keep secrets in the macOS Keychain and out of agent context and logs.
7. **Cron guardrails.** Give scheduled jobs a frozen permission scope, an owner-visible pause, and a "who authorized this" record. Hark's terms say scheduled actions act only within the permissions you gave.
8. **Usage budgets per lane** (tokens or dollars, with a ceiling). Add an opt-in auto-top-up equivalent only if a paid API is involved.

## CLAIMS
- CLAIM: Hark Pro asks for confirmation before spending money or sending messages by default, and supports standing per-category permissions — RISK: high — SOURCES: hark.com — PRIMARY: https://hark.com/terms
- CLAIM: Passwords and cards are held in a vault Hark cannot see and are detokenized only on the destination page — RISK: high — SOURCES: hark.com — PRIMARY: https://hark.com/using-hark (vendor claim, unverified)
- CLAIM: Hark does not train on user data, and third-party model providers have zero retention — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/privacy-policy
- CLAIM: Pricing is free, $20 a month for 2x usage and $100 a month for 10x usage, with all features on every tier — RISK: normal — SOURCES: hark.com, thenextweb.com — PRIMARY: https://hark.com/articles/introducing-hark-pro
- CLAIM: No published prompt-injection defenses or user-facing audit log were found — RISK: high — SOURCES: hark.com — PRIMARY: none (absence of evidence, and trust.hark.com was not readable)
- CLAIM: EU availability is unconfirmed and PSD2 strong customer authentication is unresolved — RISK: normal — SOURCES: thenextweb.com — PRIMARY: none (secondary only)
- CLAIM: Hardware ships in 2027 and Handoff is the top performer on Online-Mind2Web — RISK: normal — SOURCES: hark.com, thenextweb.com — PRIMARY: https://hark.com/articles/introducing-hark-handoff (vendor-reported)

## EXPAND
- LEAD: trust.hark.com (SOC 2 and security posture) — WHY: it rendered empty when fetched, and it is the likeliest place for injection, pen-test and compliance details — ANGLE: safety
- LEAD: hark.com/acceptable-usage-policy and /other-policies — WHY: they were not retrieved because of rate limits, and may hold agent-specific rules — ANGLE: safety
- LEAD: The Bloomberg 2026-10-06 article and the Rockingrobots Handoff article — WHY: the Bloomberg article is the original press source, and the Rockingrobots article was a seed I did not read — ANGLE: independent coverage
- LEAD: Early-user reports on Reddit and X about wrong purchases and refunds — WHY: only vendor and policy text exists so far, with no field evidence of failure modes — ANGLE: safety