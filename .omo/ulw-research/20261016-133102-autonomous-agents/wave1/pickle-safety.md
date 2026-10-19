<!-- task=st_01a144ef -->
st_01a144ef [completed] model mahoquot/claude-sonnet-5-5
Ask: research Pickle's trust, safety and economics - wanted: an evidence-backed report. For you: the report is complete, but pricing numbers and prompt-injection defenses are not publicly documented, and the help-center pages could not be fetched. Now: final report. Next: none.

## Identity
- **Name:** Pickle (Korean listing "피클: 나만의 AI 에이전트"). The company is Pickle, Inc.; the App Store developer name is "Pickle Company".
- **Official URLs:** https://pickle.com (site, legal pages, security pages) and https://www.ycombinator.com/companies/pickle-2. The open-source component is https://github.com/pickle-com/credential-enclave.
- **YC listing:** "The personal AI agent for Korea", founded 2024 by Sanio Jung, Emmett Kim, Daniel Park and Hojin Yu, 6 employees, San Francisco. [https://www.ycombinator.com/companies/pickle-2]
- **iOS app:** id6795790129, bundle `com.pickle.client`, released 2026-08-10, version 1.1.12 as of 2026-10-13, free, 17+, iOS 17 or later. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- **Confidence:** high that this is the right entity. The App Store id, seller name, Korean-market description and YC blurb all match, and `pickle.com` is the `sellerUrl`. I did not confirm that the YC page links to pickle.com (its HTML was truncated). It is not Pickle Robot or ApplyPilot.
- **Tagline:** I could not find "Raise your AI" verbatim. The privacy policy describes "an app where users raise their own AI" (이용자가 자신의 AI를 키우는 앱), which is close. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- **Sourcing caveat:** pickle.com is a JS single-page app. I read its legal and security text from the bundled JS assets, not from rendered pages. The Terms of Service JS says "Effective October 1, 2026". The Korean privacy policy says "시행일 2026년 10월 22일" (effective October 22, 2026), which is after today's date (2026-10-16). Treat it as a pending version.

## Findings

### Approval gates
- The App Store text says payments and bookings need user approval before proceeding, and "anything that spends money or can't easily be undone waits for your approval first". [https://itunes.apple.com/lookup?id=6795790129&country=us]
- The security page says irreversible actions run only within the scope the user delegated, and Pickle asks first when scope is unclear. The app shows an approval card in chat with order details, total, and a pay button. [https://pickle.com/assets/security-n1i5zLW2.js]
- The FAQ says Pickle shows what it will do and the cost first. If the amount rises right before payment, it shows the increase and asks again. [https://pickle.com/assets/faq-pFbILNx9.js]
- With Link (Stripe's wallet), each payment is requested from Link and approved there before any charge. Link's spending limits apply. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]
- The Terms say the agent is "designed to check with you before spending your money and before other significant or irreversible steps". That is design intent, not a hard guarantee. The Terms also put review of results on the user, and the Terms limit liability to the greater of $100 or what you paid in 12 months. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]
- The user can watch phone calls and the browser live, take over the browser (for passkeys or bank approvals), and stop tasks. Steps already completed with a third party cannot be undone. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]
- Outbound calls come from a Pickle number. The agent says it is an AI assistant and that the call is transcribed. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]

### Permission model and agent-to-agent
- Connections are opt-in per app. The user can disconnect any of them, which deletes the stored token and revokes provider-side access where the provider supports it. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- Access to cards and connections is time-boxed: 30 days after you last opened the app, or a shorter period you choose. After that, work that needs them waits until you reopen the app. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]
- Sharing is set per friend and per connected app. Sensitive topics (health, money, private life, beliefs, precise location, account security) are not shared by default. There is also a user-editable "never share" list. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- When your Pickle talks to outsiders it can only read and talk. It has no tools to book, pay, or change files. The user can read every agent-to-agent conversation. [https://pickle.com/assets/security-n1i5zLW2.js]
- Items that need the user's decision go back to the user before being finalized. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- Overnight, the agent can browse the community board, comment, and start conversations on its own. The results are reported in the morning, and the user can stop it by telling it to. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]

### Audit logs
- Every use of a card or connected account writes a log entry before the use happens. The entry is sealed to the user's log key, so only the user can read it. [https://pickle.com/assets/security-n1i5zLW2.js]
- The log is an S3 bucket with Object Lock in compliance mode, 365-day retention. The README says not even the operator or the AWS root user can delete entries. If the entry is not stored (the store must answer HTTP 200), the enclave refuses to use the credential. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- Because of this, account deletion does not remove these logs before 365 days. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- Pickle staff can view conversations and memories only through an authenticated internal tool that logs every view and masks card numbers. It cannot show vault values, account tokens, or camera images. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]

### Sandboxing and secrets
- Three isolation zones are described. (1) A sealed vault built on AWS Nitro Enclaves, unlocked values held in memory only. (2) A per-user virtual browser: it closes after each task and only the login state is kept in encrypted storage. (3) An isolated file workspace that cannot reach the vault or Pickle's internal network. [https://pickle.com/assets/security-n1i5zLW2.js]
- The cloud browser vendor is Kernel. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- The enclave source is Apache-2.0, created 2026-10-01, with releases v1.0.0 through v1.5.2 and the latest on 2026-10-15. The latest commit I resolved is `2dcf8957a4afb78a1c0400e3d37df0053faa97ad`. It has a reproducible build (PCR0/1/2 measurements), a verification CLI, and a compile-time rule that secrets cannot be serialized. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- Secret handling:
  - The user's encryption key is created on the device and kept in iCloud Keychain. Pickle cannot recover it.
  - Passwords and card numbers are never passed to the model. The system decrypts only what a field needs and fills it in, and the model sees masked results. No screenshots are taken while a value is on screen.
  - TOTP seeds never leave the enclave; only 6-digit codes do.
  - [https://pickle.com/assets/security-n1i5zLW2.js]
- The README lists its own limits: it protects tokens, not the data the tokens fetch. `release` hands a password or card to the operator's worker, which fills the form. The iOS app is not yet open source. The transparency log is checked only for entry timestamps, not inclusion proofs. The operator can stop the service. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]

### Prompt injection
- I found no public prompt-injection documentation. This is a gap, not evidence of absence.
- The closest mitigations are structural. The model never holds secrets, the outward-facing agent has no action tools, and tasks run in per-user isolated browsers and file workspaces. [https://pickle.com/assets/security-n1i5zLW2.js]
- The `forward` and `release` calls still return data to the operator domain. Content fetched by the browser or mail (email bodies, web pages) is not protected by the enclave. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]

### Privacy and retention
- Stated commitments:
  - No selling, no ads.
  - No AI-training on user data, either by Pickle or by its AI providers' contract terms. Some providers may keep copies briefly to monitor abuse.
  - Voice is not recorded; only the transcript is kept.
  - [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- Conversations and memories are encrypted with a server-side key, not the user's key. [https://pickle.com/assets/security-n1i5zLW2.js]
- Retention figures:
  - Chat, memory and attachments are kept while the account exists.
  - Detailed background-task logs are kept 30 days.
  - Server logs are kept 90 days.
  - AI debug snapshots are kept 30 days (diagnostic mode is off by default).
  - Location transition records are kept 7 days.
  - DB backups are kept 7 days.
  - [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- Data is stored in AWS US-West. Subprocessors include OpenAI, Anthropic, Google, xAI, Azure, Baseten, Cerebras, OpenRouter, Stripe, Plaid, Telnyx, a Korean carrier partner (Plover), Kakao Map, TMAP, LiveKit, Stytch, Soniox and Deepgram. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- Account deletion is available in the app and starts immediately. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
- The Pickle team views data only for user-reported issues or safety reports. [https://pickle.com/assets/security-n1i5zLW2.js]

### Pricing and usage limits
- The app is free to download. [https://itunes.apple.com/lookup?id=6795790129&country=us]
- The Terms mention an optional "Pickle Plus" subscription, billed through Apple. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]
- Usage is metered by "energy", which reflects compute cost. When it runs out, new responses and tasks pause until it recovers. The company says it may apply other limits too. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]
- No price points or energy quotas were found. That is a gap in what I could retrieve.

### Availability and regions
- The English Terms say the service is offered in the United States, to adults 18 or older. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]
- The App Store lists Korean, English and Japanese. It is marketed in Korea with Coupang, Naver, KakaoTalk, Gov24 and KTX integrations. The security pages also exist in Korean, English and Japanese. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- The US and KR listings differ. The KR text lists Korean services, while the US text lists Amazon, Target, Uber Eats and DoorDash. The English Terms say US-only, which conflicts with the KR-market marketing. I read this as a legal-text lag, but that is my inference.

## Implications for a self-hosted personal agent gateway
1. **Approval cards as a first-class typed event.** Gate any spend, send, or irreversible action behind an approval with a structured payload (action, recipient, amount). Deliver it through Discord, iMessage or KakaoTalk with explicit approve and deny. Re-prompt if the amount rises before execution, as Pickle does at payment. [https://pickle.com/assets/faq-pFbILNx9.js]
2. **Pre-action audit log that blocks the action.** Write an append-only entry (WORM file or hash-chained log) before every outbound action, and refuse to act if the write fails. Pickle does this with Object Lock and a "no record, no use" rule. Add a daily self-verification job. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
3. **Time-boxed capability leases.** Grant each lane or cron job credentials for at most N days, and let them lapse if the owner has not checked in. This is the "30 days or shorter after you last opened the app" idea. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]
4. **Secrets broker that never hands secrets to the model.** The daemon injects passwords and tokens into tool calls and masks them in the output returned to the agent. This works on a Mac with the Keychain or Secure Enclave, without needing Nitro. A Rust type that cannot be serialized, like their secret type, is a cheap compile-time guard. [https://pickle.com/assets/security-n1i5zLW2.js]
5. **Two-tier agent roles by tool set.** Anything touching untrusted input (inbox ingestion, outsiders, web pages) runs in a read/talk-only lane with no write, pay or send tools. Only the owner-facing lane gets action tools. This is the structural prompt-injection defense Pickle uses for its external-facing agent. [https://pickle.com/assets/security-n1i5zLW2.js]
6. **Per-contact and per-source disclosure policy.** Give each contact and each connected source a sharing level and a "never share" list. Default sensitive categories (health, money, location, credentials) to off for third parties. [https://pickle.com/assets/privacy-policy-BtYR0eAZ.js]
7. **Isolated, disposable workspaces for omo coding agents.** Use one workspace per task with no route to the secrets store or to other lanes. Keep only the login state between runs, which is Pickle's browser model. [https://pickle.com/assets/security-n1i5zLW2.js]
8. **Cost and energy budgets per lane.** Add a compute-cost meter that pauses new tasks when exhausted, and surface it to the owner. [https://pickle.com/assets/terms-of-service-BQeY61qP.js]

## CLAIMS
- CLAIM: Pickle is a YC-listed personal AI agent for Korea, founded in 2024, 6 people in San Francisco. — RISK: normal — SOURCES: ycombinator.com — PRIMARY: https://www.ycombinator.com/companies/pickle-2
- CLAIM: The iOS app (id6795790129) was released 2026-08-10 and is currently v1.1.12 by Pickle, Inc. — RISK: normal — SOURCES: apple.com — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=us
- CLAIM: Payments and bookings require user approval, with a re-prompt if the total rises. — RISK: high — SOURCES: pickle.com, apple.com — PRIMARY: https://pickle.com/assets/faq-pFbILNx9.js
- CLAIM: Credential use is logged before the action in a 365-day S3 Object Lock store. — RISK: high — SOURCES: github.com — PRIMARY: https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md
- CLAIM: Passwords and card numbers never reach the AI model. — RISK: high — SOURCES: pickle.com, github.com — PRIMARY: https://pickle.com/assets/security-n1i5zLW2.js
- CLAIM: The README limits this protection to tokens and secrets, not to the data fetched with them. — RISK: high — SOURCES: github.com — PRIMARY: https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md
- CLAIM: Outsider-facing Pickles have no booking, payment or file tools. — RISK: high — SOURCES: pickle.com — PRIMARY: https://pickle.com/assets/security-n1i5zLW2.js
- CLAIM: Data is stored in AWS US-West, and AI providers include OpenAI, Anthropic, Google and xAI. — RISK: normal — SOURCES: pickle.com — PRIMARY: https://pickle.com/assets/privacy-policy-BtYR0eAZ.js
- CLAIM: User data is not used for AI training, though some providers may retain copies briefly for abuse monitoring. — RISK: normal — SOURCES: pickle.com — PRIMARY: https://pickle.com/assets/faq-pFbILNx9.js
- CLAIM: The English Terms say US-only and 18+, while marketing targets Korea. — RISK: normal — SOURCES: pickle.com, apple.com — PRIMARY: https://pickle.com/assets/terms-of-service-BQeY61qP.js
- CLAIM: Pricing tiers are not publicly found; only a free app, a "Pickle Plus" subscription and "energy" limits are named. — RISK: normal — SOURCES: pickle.com — PRIMARY: https://pickle.com/assets/terms-of-service-BQeY61qP.js
- CLAIM: No public prompt-injection defense documentation exists (absence in what I could reach). — RISK: high — SOURCES: none — PRIMARY: none

## EXPAND
- LEAD: Fetch the Pickle help-center pages `/help/tasks/confirm-before-payment`, `/help/privacy/who-can-see` and `/help/privacy/keep-access` — WHY: they likely hold the exact approval rules, spend caps and access-duration options — ANGLE: approval-gate details and numeric limits. `curl` of pickle.com's JS assets was size-capped, so a browser or `--range` fetch is needed.
- LEAD: Read `docs/egress-policy.md` and `docs/protocol.md` in credential-enclave — WHY: they spell out exactly what can leave the enclave and the log format — ANGLE: a template for a pre-action audit log.
- LEAD: Find Korean press (e.g. platum, Maeil Business, Naver News) and Pickle's X/Threads accounts for Pickle Plus pricing and launch coverage — WHY: no pricing was in the primary sources I could reach — ANGLE: economics and actual KR availability.
- LEAD: Compare against other agent-safety designs (e.g. OpenAI Operator confirmations) — WHY: calibrate which of Pickle's gates are standard — ANGLE: prompt-injection mitigations.