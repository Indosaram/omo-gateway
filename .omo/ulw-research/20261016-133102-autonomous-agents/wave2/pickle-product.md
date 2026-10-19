<!-- task=st_01a1451f -->
st_01a1451f [completed] model mahoquot/claude-sonnet-5-5
Ask: research Pickle's product behavior, approvals, spend caps, Korea integrations and pricing, with sources. For you: the report is below. I found no spend cap, no Toss or PASS integration, and none of the named Korean press. Now: done. Next: none.

## Findings

**Approvals and payments**
- PRIMARY: Pickle shows a card with **Approve** and **Decline** buttons for details you must decide on (items, total, booking conditions). It pays only within what you agreed to. [https://pickle.com/help/tasks/confirm-before-payment] — r.jina.ai worked. Direct curl returned only the SPA shell, and Wayback has no copy.
- PRIMARY: Re-approval triggers when the payment-screen amount exceeds the agreed amount, even for a small difference. The example is ₩25,000 agreed and ₩31,000 at checkout, which prompts again for the extra ₩6,000. A changed item or time also forces a re-check. [same URL]
- PRIMARY: Pickle does not decide amounts that are up to you, such as tip size. If you already told it to go ahead with a stated item and amount, it does not ask again. [same URL]
- PRIMARY: After paying, it checks the order record and confirmation number. If the screen cuts out, it checks whether the payment went through before paying again. After two failures with the same error it stops and reports the blocker. [same URL]
- PRIMARY: Before a task starts, it shows a card of what it will do and under what conditions. Tasks keep running after the app closes. It uses a real browser, and phone calls for offline businesses. [https://pickle.com/help/tasks/delegate] — jina.
- PRIMARY: I found no per-transaction or monthly spend cap on any fetched page. The only quantitative limit is "agreed amount". "Usage credits" refill weekly and meter compute, not money. Chat input is disabled and scheduled tasks are skipped when credits run out. [https://pickle.com/help/credits/usage-credits] — jina.
- PRIMARY: The plans page says "[To be added once decided]". [https://pickle.com/help/credits/plans] — jina.

**Privacy and access**
- PRIMARY: Four parties are involved: the agent, AI providers, Pickle staff, and other people. Passwords, card numbers, CVCs and TOTP keys are never passed to the agent. It can read addresses, phone numbers, site usernames, and the last four digits and expiry of cards. Staff views require authentication and are logged each time. [https://pickle.com/help/privacy/who-can-see] — jina.
- PRIMARY: Between friends' Pickles, it shares only within limits set in My Pickle → Privacy. By default it shares nothing sensitive: health, finances, personal life, beliefs, exact location, account security. [same URL]
- PRIMARY: "Keep access for" runs from 1 to 30 days and resets whenever you open the app. If you don't open the app within the period, card and account access turns off. You re-enable it from the Activity screen. [https://pickle.com/help/privacy/keep-access] — jina.
- PRIMARY: Activity is stored encrypted. The phone checks locally for missing or altered records and shows "All activity verified" or a specific gap message. [https://pickle.com/help/privacy/activity] — jina.

**Source: credential-enclave (Rust)**
- PRIMARY: Pickle's Rust credential enclave is public and Apache-2.0. It holds delegated user keys inside an AWS Nitro Enclave. [https://github.com/pickle-com/credential-enclave/tree/2dcf8957a4afb78a1c0400e3d37df0053faa97ad] — gh API plus raw.githubusercontent.com. The org has two repos: this one and a tau2-bench fork.
- PRIMARY: The user key is granted for at most 30 days and erased at expiry or revoke. This matches the "Keep access for" page. [https://raw.githubusercontent.com/pickle-com/credential-enclave/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- PRIMARY: Every credential use writes a log entry to S3 Object Lock (365 days) before the act. The entry is sealed to the account's log key, and the user verifies it on-device. [README above; enclave.md section 4]
- PRIMARY: Secrets are a `Secret<T>` type with no `Serialize` or `Display`, zeroized on drop. Response types must implement the marker trait `OperatorResponse`, so a secret field fails to compile. The functions that read a secret are a closed list, checked in CI by `scripts/secret-access.sh --check`. A canary test plants markers and scans every response. [https://raw.githubusercontent.com/pickle-com/credential-enclave/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/egress-policy.md]
- PRIMARY: Provider responses are scanned for the injected token (raw, base64, base64url, hex, and percent-decoded up to three times). A match fails with `response_withheld`. Redirects, `Set-Cookie` and method-override headers are refused or dropped. [egress-policy.md E4; enclave.md 5.8]
- PRIMARY: Korea-specific: the only Korean provider in the enclave is NAVER Mail, over IMAP/SMTP with an app password. The destinations are pinned to imap.naver.com:993 and smtp.naver.com:465. [egress-policy.md, "Mail credentials"]

**App Store listing (id6795790129)**
- PRIMARY: The KR listing is "피클: 나만의 AI 에이전트", version 1.1.12, released 2026-08-10, iOS 17+, 19+ rating. It lists in-app purchases Pickle Plus Monthly ₩15,000 and Yearly ₩99,000. [https://apps.apple.com/kr/app/id6795790129] — jina. [https://itunes.apple.com/lookup?id=6795790129&country=kr] — direct curl.
- PRIMARY: The listing says payments and bookings need approval before proceeding. It covers phone calls, Coupang and Daiso orders, KTX, restaurants and hospitals, and cancellation-seat alerts. Connectors named: Naver, KakaoTalk, Gmail, Google Calendar, Gov24, Coupang, carriers, delivery, insurance, airlines. It also says Pickle "can be inaccurate or take an unexpected action". [KR listing above]
- PRIMARY: Friends' Pickles negotiate availability, venue and reservation with each other, and the conversation is viewable. [KR listing above; https://pickle.com/help/friends/add — jina]
- PRIMARY: The US listing says payment goes through Link by Stripe and that anything spending money "waits for your approval first". Rating is 4.585 across 41 ratings. [https://itunes.apple.com/lookup?id=6795790129&country=us] — direct curl.
- SECONDARY: Reviews are mostly 5-star but only lightly informative. One 3-star review says the app lacks Google OAuth approval for calendar. A 2-star review on v1.1.12 complains about a theme change. The KR store has one review, which praises Korea-specific services. [https://itunes.apple.com/us/rss/customerreviews/page=1/id=6795790129/sortby=mostrecent/json] — direct curl.

**Matthew Dowd incident**
- SECONDARY: This is about the "Pickle 1" XR glasses from the same company. The HN thread title is "Pickle 1 AR Glasses (YC W25) May Be Fraudulent". Dowd's tweet and the HN comments name "Pickle's CEO - Daniel Park" and a Q2 2026 US delivery estimate. Comments dispute Dowd's technical claims, so fraud is alleged but not established. [https://news.ycombinator.com/item?id=46483429] — HN Algolia items API. [https://daringfireball.net/linked/2026/01/03/dowd-pickle-expose] is known only via the HN link; the fetch failed on jina rate limit.
- The glasses relate to the agent only as company history. The product has since pivoted to a phone agent.

**Not found**
- Platum, Maeil and Naver coverage of "피클 AI 에이전트" returned nothing. Google News RSS hits for "피클" are mostly unrelated: 피클플러스 (a different company), pickleball, and the pickle food. The Naver search page failed over jina.
- Possibly related: ZDNet Korea, "AI 아바타 스타트업 피클, 시드 투자 60억원 유치" (2025-06-09), and ETNews, "피클, 영상회의 '클론 생성'…" (2025-04-14). I saw titles only, and the company match is unverified.
- Not found: Toss and PASS integrations (searches of the App Store text and the help pages found no mention) and Reddit (not attempted).
- HN Algolia has no comment hits for "raise your AI" or Daniel Park's agent.

## Design patterns worth copying into a Rust gateway

1. **Approval state machine.** Use `Proposed{items,total,currency,expires} → Approved{cap=total} → Executing → {Settled, Escalated(delta), Failed}`. At the payment step, compare `observed_total > approved_cap` and re-enter `Proposed` with the delta. Apply the rule at any amount, with no tolerance band. Treat item or time changes as invalidating. Store an explicit `pre_authorized{item, max_amount}` flag, so "already agreed" is data, not an LLM judgment.
2. **Idempotent payment recovery.** On a UI or connection drop, move to `PaymentUnknown` and run a read-only status check (order record or confirmation number) before any retry. Cap retries at 2 per distinct error signature, then stop and surface the blocker.
3. **Secret types.** Use `Secret<T>` with no `Serialize` or `Display`, `Debug` printing a fixed string, and `zeroize` on drop. Require a sealed `OperatorResponse` trait for everything leaving the process, with `compile_fail` doctests. Keep a `secret-access.tsv` allow-list checked by a CI script. Add a canary test that plants markers and greps every response, including base64 and hex forms.
4. **Credential-injecting egress proxy.** The LLM issues a request, and the gateway injects the token. Pin the destination host and path prefix per provider, disable redirects, force `Accept-Encoding: identity`, and strip `Set-Cookie` and method-override headers. Reflection-scan the response for the token before returning it. Return diagnostics only from a closed vocabulary.
5. **Delegation with TTL.** Make a `Grant{key_id, not_after ≤ 30d}` that is checked on every read and zeroed lazily at expiry. Pickle's sliding "keep access for" is reset by app-open, and a self-hosted gateway should reset on authenticated user activity.
6. **Write-ahead audit log.** Append an entry to the log before the action, and only then execute. Chain entries per account with a hash, seal them to the owner's key, and write them to WORM storage where available. Surface "N records missing" in the UI. On a self-hosted box, use an append-only file plus a hash chain and a signed head.
7. **Friend-agent sharing.** Default-deny a sensitive-topic list (health, finance, location, security) and make the sharing policy per peer. Make the agent-to-agent transcript viewable by both owners.
8. **Credits versus money.** Meter compute (weekly refill) separately from spend. Pickle documents no money cap, so add a hard per-task and per-period `spend_cap` that the approval machine consults.

## CLAIMS

- CLAIM: Pickle enforces approval for all payments. — RISK: medium; the docs are marketing-adjacent and the card flow is not independently tested, though the enclave source supports credential handling. — SOURCES: confirm-before-payment page, KR and US App Store text. — PRIMARY: yes.
- CLAIM: The agent never sees card numbers or passwords. — RISK: medium; the `vault_card` and `vault_password` release path hands the value to the operator's worker, so the protection ends there. — SOURCES: who-can-see page, README ("A vault value is protected by the node up to `release`"). — PRIMARY: yes.
- CLAIM: Activity is tamper-evident. — RISK: low-medium; the README itself says the app is not open source and the app's honesty is assumed. — SOURCES: activity page, README. — PRIMARY: yes.
- CLAIM: Pickle has spend caps. — RISK: unsupported; no cap found. — SOURCES: absence across fetched help pages. — PRIMARY: n/a.
- CLAIM: Pickle integrates Kakao, Toss and PASS. — RISK: only KakaoTalk is listed, and as an app connector; Toss and PASS are unverified. — SOURCES: KR App Store text. — PRIMARY: yes for Kakao.
- CLAIM: Pickle Plus costs ₩15,000 a month or ₩99,000 a year. — RISK: low; in-app purchase prices may differ at checkout. — SOURCES: the KR listing and the iTunes lookup. — PRIMARY: yes.
- CLAIM: The Pickle 1 glasses were fraudulent. — RISK: high; alleged by Dowd, disputed in the HN comments, and the product was never delivered as far as I found. — SOURCES: HN thread 46483429. — PRIMARY: no.

## EXPAND

- Fetch the help pages for `help/connect/save-card`, `help/connect/add-connection` and `help/tasks/*` (6 articles). jina is rate-limited, so retry serially with delays.
- Fetch `docs/protocol.md` from credential-enclave for record AAD, grant and revoke formats.
- Fetch `pickle.com/security/{encryption,isolation,control}` and `/terms-of-service`, to check for liability or spend-limit terms.
- Try Korean press directly (platum.kr, mk.co.kr, news.naver.com search) with different queries. Try X via the `@danifesto` handle and old.reddit.com.