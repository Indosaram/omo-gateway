<!-- task=st_01a144ee -->
st_01a144ee [completed] model mahoquot/claude-sonnet-5-5
Ask: research Pickle's architecture (YC, "Raise your AI"). Wanted: a sourced report. For you: the entity is verified and the architecture is well covered by primary sources, but there is no developer API/SDK and no web_search tool was available. I used curl, gh and the HN Algolia API instead. Now: done. Next: none.

## Identity

- **Name:** Pickle, in Korean 피클 ("피클: 나만의 AI 에이전트"). The legal entity is Pickle, Inc., 5 Pear Ct, Hillsborough, CA. [https://pickle.com/privacy-policy]
- **YC:** the profile is ycombinator.com/companies/pickle-2, Winter 2025, 6 people, San Francisco. Founders are Daniel Park (CEO), Sanio Jung, Emmett Kim and Hojin Yu (listed as "Sam Yu, CTO" in the old launch post). [https://www.ycombinator.com/companies/pickle-2]
- **Pivot:** the same YC page's launch post describes the 2024 product as an "AI body double for video calls". This is a pivot to a personal agent, not a separate company. [https://www.ycombinator.com/companies/pickle-2]
- **iOS app:** App Store id6795790129, bundle `com.pickle.client`, seller "Pickle, Inc.", first released 2026-08-10, version 1.1.12 (2026-10-13), iOS 17+, 17+ rating, free. Languages are EN/JA/KO and the seller URL is pickle.com. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- **GitHub:** the org `pickle-com` ("A personal AI you raise", created 2023-11-21, homepage pickle.com) has two public repos. One is `credential-enclave` (Rust, Apache-2.0, created 2026-10-01). The other is `tau2-bench`, a fork of sierra-research/tau2-bench. [https://github.com/pickle-com/credential-enclave]
- **Launch status:** YC says it is "live on the App Store in the US and launches in Korea next". The privacy policy says the service is offered in the US to people 18 or older, and the policy is dated "Effective October 22, 2026", which is after today's date. [https://www.ycombinator.com/companies/pickle-2] [https://pickle.com/privacy-policy]
- **Confidence:** high. The YC page, the App Store seller URL, the pickle.com policy text and the GitHub org all point at each other. This is not Pickle Robot or ApplyPilot.

## Findings

**Execution environment**
- Each user gets a Pickle, a 3D character, that "does things for you on its own computer". It acts through "its own web browser" and also places phone calls. [https://pickle.com/privacy-policy]
- The cloud browser is provided by Kernel. A per-user "browser profile" persists and is deleted on account deletion. [https://pickle.com/privacy-policy]
- Calls go out through Telnyx, and through Flover for Korean numbers. The agent says it is an AI assistant and that the call is transcribed. YC claims calls in 81 countries. [https://pickle.com/privacy-policy] [https://www.ycombinator.com/companies/pickle-2]
- Hosting is AWS in the western US. Real-time audio and video runs on LiveKit. Sign-in is Stytch plus Sign in with Apple, with phone verification. [https://pickle.com/privacy-policy]
- Local device access is limited to the iPhone app. Contacts, Calendar, Reminders, Photos, Apple Music, Motion and Screen Time are read on the phone only when enabled, and what is read is sent to the server in that conversation. [https://pickle.com/privacy-policy]
- Optional "Always" location lets the iPhone report arrivals, departures and significant moves. [https://pickle.com/privacy-policy]
- There is no evidence of any desktop or Mac daemon. All execution is cloud-side. [https://pickle.com/privacy-policy]

**Connectors and auth**
- Connected accounts are Google Workspace, Microsoft (including Teams for work accounts), Slack, Notion, X, Granola, Mercury (read only), GitHub and Link (Stripe's wallet). [https://pickle.com/privacy-policy]
- Banks, cards and investments connect through Plaid, read only. [https://pickle.com/privacy-policy]
- The App Store listing also names Naver, KakaoTalk, Gov24, Coupang, carriers, delivery, insurance and airlines. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- The listing says important actions such as payments and bookings need user approval, and that browser and phone work can be watched live. [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- Connector definitions ship as `definitions.json`, compiled in and measured as part of the enclave image. Each lists the OAuth URL, PKCE mode, token endpoint, allowed API hosts and path prefixes, and the header to inject, for example `Authorization: Bearer`. Google covers Gmail, Calendar, Drive, Docs, Sheets, Slides, Forms, Tasks and Meet. GitHub covers api, uploads and git smart-HTTP. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/enclave/src/providers/definitions.json]
- Granola and Mercury use MCP endpoints with dynamic client registration. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/enclave/src/providers/definitions.json]
- NAVER Mail uses an app password over fixed IMAP/SMTP only (imap.naver.com:993, smtp.naver.com:465). [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/egress-policy.md]

**Credential vault and password handling (the open-source component)**
- The credential enclave is a Rust workspace with four crates: `enclave` (node), `host` (parent pod relay), `protocol` and `verify`. It runs in an AWS Nitro Enclave with no disk. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- The iPhone app creates a master key kept in iCloud Keychain. The user key derived from it is granted to the enclave only after the app verifies the Nitro attestation, for at most 30 days. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- The enclave exchanges the OAuth code itself, with the PKCE verifier staying inside. `forward` injects the token into requests to allow-listed provider hosts and returns only the provider's response. No call returns the token. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/enclave.md]
- Responses are scanned for the injected token, including base64, hex and percent-encoded forms. A match is `response_withheld`. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/egress-policy.md]
- Vault values (passwords, card numbers, CVC) leave only through `release`, one field at a time. TOTP seeds never leave, only the 6-digit code. After `release` the value goes to the operator's worker, which "fills a form", so the node no longer protects it. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- The privacy policy states that the AI model never sees the filled value, only that a saved value was filled. [https://pickle.com/privacy-policy]
- Every use writes a log entry to S3 Object Lock (compliance mode, 365 days) before the act happens. Entries are sealed to a key only the user holds and are chained and signed per node. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- The build is reproducible, and `credential-enclave-verify` compares the measured image with what the running nodes attest. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]
- Self-stated limits: the iOS app is not open source, so "the app is honest" is an assumption. The OAuth client secret belongs to the operator. Slack and Notion have no PKCE. The operator can ship a later release of the enclave with zero notice period. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md]

**Memory and personal context**
- The agent writes a diary of the user's days (shown as "memory bubbles"), notes on who the user is, and learned topics. There is also a summary of long conversations and a searchable memory index. Users can read, edit and delete bubbles. [https://pickle.com/privacy-policy]
- Voice audio is not kept, only the transcript. Camera frames are sampled every few seconds into memory while the camera tile is on. [https://pickle.com/privacy-policy]
- Content fetched from connected accounts "becomes part of your conversation history and memory". [https://pickle.com/privacy-policy]
- Step-level work records (pages visited, results) are kept so the agent "can follow up". Detailed background-task records are kept for 30 days. [https://pickle.com/privacy-policy]

**Long-running tasks and continuation**
- YC describes a morning briefing and scheduled tasks. The App Store listing says it proposes tasks proactively from mail and calendar, and notifies when a sold-out seat frees up. [https://www.ycombinator.com/companies/pickle-2] [https://itunes.apple.com/lookup?id=6795790129&country=kr]
- "At night" the agent browses the community board on its own and reports back in the morning. [https://pickle.com/privacy-policy]
- The user chooses how long the agent may use cards and connections while they are away (the grant is up to 30 days, renewed when the app opens). [https://pickle.com/privacy-policy]
- YC says the agent "earns trust in steps", from small errands to accounts, cards and identity. [https://www.ycombinator.com/companies/pickle-2]
- The internal job and queue model is not published. Speculation: a background worker fleet calls the enclave over HTTP, since the enclave docs describe a "backend" and "worker" calling it. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/enclave.md]

**Multi-agent structure**
- Agents talk to each other. Two Pickles negotiate availability, venue and booking, and the transcript is visible to both owners. Anything needing a decision returns to the owner. [https://itunes.apple.com/lookup?id=6795790129&country=kr] [https://pickle.com/privacy-policy]
- Privacy controls cover sharing tiers for friends and non-friends, a "never shared" list, per-person rules, and per-connection "may use while talking to others" toggles that default to off. Health, money, beliefs, exact location and account security are private by default. Passwords and codes are never shared. [https://pickle.com/privacy-policy]
- The agents also meet in a shared 3D "sky/world" and on a community board. [https://pickle.com/privacy-policy]
- Whether there is an internal planner/worker split is not published.

**Models**
- LLM providers are Google, OpenAI, Anthropic and xAI. Hosting/inference providers are Azure, Baseten, Cerebras and OpenRouter. Some models run on Pickle's own servers. [https://pickle.com/privacy-policy]
- Speech-to-text is Soniox and Deepgram. Audio cleanup is Krisp and ai-coustics. Images come from fal.ai. Search is Exa and Serper. Maps are Google, Kakao and TMAP. [https://pickle.com/privacy-policy]
- A per-task "computing cost" is recorded and used to derive the character's "energy". [https://pickle.com/privacy-policy]
- Speculation: the `tau2-bench` fork suggests they use τ-bench style tool-agent-user evals. The fork is dated 2026-07-22, with no further detail. [https://github.com/pickle-com/tau2-bench]

**Developer APIs/SDKs**
- None found. The org has two repos, the site is consumer-only, and HN Algolia returned nothing relevant for "pickle raise your AI". The only public API is the enclave's internal HTTP surface (`/v1/forward`, `/v1/release`, `/v1/oauth/*`, `/v1/attestation` and others), which is for Pickle's own backend. [https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/enclave.md]

## Implications for a self-hosted personal agent gateway

1. **Brokered credentials.** Run a small credential broker daemon in the Rust gateway, modeled on `forward`. omo agents get a handle, never the token. The broker injects the `Authorization` header only for allow-listed hosts and path prefixes, and drops any response that reflects the token, including base64, hex and percent-encoded forms. On a Mac the Secure Enclave and Keychain stand in for Nitro.
2. **Write-ahead audit log.** Append a signed, hash-chained entry before each credentialed act. Store it append-only and make it viewable in Discord or iMessage ("Activity"). Pickle writes the entry first and acts only after the store confirms it.
3. **Time-boxed delegations.** Grant a lane access to a key for N days, shown to the owner as a countdown, and auto-expire it. Require an explicit owner "renew" message (for example from KakaoTalk), mirroring "30 days, renewed when you open the app".
4. **Approval gates by action class.** Payment, booking and send actions pause the lane and ask the owner over the chat channel. Reads and drafts proceed. Add a "trust ladder" config where each connector and action class is promoted separately.
5. **Memory as inspectable files.** Keep a diary plus notes plus topic notes in plain Markdown, with a search index. Make summaries editable and deletable by the owner. Keep step-level work records on a short TTL (Pickle uses 30 days).
6. **Agent-to-agent protocol, later.** If the owner wants a gateway-to-gateway handshake, copy these rules: transcripts visible to both owners, per-peer sharing tiers, a never-share list, and sensitive categories private by default.
7. **Morning briefing and overnight lane.** Use cron to run a scheduled briefing lane, plus an overnight "explore" lane that reports in the morning. Fed by inbox ingestion, it can propose tasks proactively (refund, appointment, anniversary booking).
8. **Redaction at the model boundary.** When a form fill needs a secret, let the agent say "fill field X from vault item Y" and let a separate non-LLM filler resolve it. The model sees only a "filled" marker, as Pickle does.

## CLAIMS
- CLAIM: Pickle (pickle.com, Pickle, Inc.) is the YC W25 company pickle-2 and the iOS app id6795790129. — RISK: normal — SOURCES: ycombinator.com, apple.com, pickle.com — PRIMARY: https://www.ycombinator.com/companies/pickle-2
- CLAIM: User credentials are held in a public-source AWS Nitro Enclave. The model and Pickle's servers cannot read tokens, and no enclave call returns one. — RISK: high — SOURCES: github.com, pickle.com — PRIMARY: https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/egress-policy.md
- CLAIM: The iOS app is not open source, so the enclave's guarantees rest on app honesty (self-stated). — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md
- CLAIM: The agent browses via Kernel's cloud browser and places calls via Telnyx and Flover, on AWS with LiveKit. — RISK: normal — SOURCES: pickle.com — PRIMARY: https://pickle.com/privacy-policy
- CLAIM: LLMs used include Google, OpenAI, Anthropic and xAI, hosted partly via Azure, Baseten, Cerebras and OpenRouter. — RISK: normal — SOURCES: pickle.com — PRIMARY: https://pickle.com/privacy-policy
- CLAIM: Agent-to-agent coordination exists (calendar/venue/booking negotiation, with transcripts visible to the owner). — RISK: normal — SOURCES: apple.com, pickle.com — PRIMARY: https://itunes.apple.com/lookup?id=6795790129&country=kr
- CLAIM: The service is US-only for adults, with Korea "next"; the Korean App Store listing is live. — RISK: normal — SOURCES: ycombinator.com, pickle.com, apple.com — PRIMARY: https://pickle.com/privacy-policy
- CLAIM: No public developer API or SDK exists. — RISK: normal — SOURCES: github.com, pickle.com (absence of evidence) — PRIMARY: none

## EXPAND
- LEAD: the full `docs/protocol.md` in credential-enclave (grant, envelope, log-chain and endorsement formats) — to copy a concrete delegation and audit-log wire format — protocol/crypto
- LEAD: the `enclave.md` §5.14 and §9 host-program sections (truncated in my fetch) — to see how the host relays vsock/TLS and drains accounts on shutdown — ops
- LEAD: the Terms of Service at pickle.com/terms-of-service and the whitepaper page, which renders client-side and returned only an error shell — to find agent action limits, liability and the "Pickle-to-Pickle" rules — product/legal
- LEAD: the `releases` and `.github/workflows/release.yml` in credential-enclave (Sigstore and Rekor attestation) — to copy the reproducible-build and transparency-log pattern — supply chain
- LEAD: X posts (@getpickleai, @danifesto) and the Korean press for the Korea launch — for model/stack details and the identity/payment rails (Gov24, PASS, Toss) — launch/Korea
- LEAD: the `tau2-bench` fork diff versus upstream — to confirm what Pickle evaluates (tool-agent-user flows) — evals

Uncertainty: no web_search tool was available, so everything above is from primary pages fetched directly. I did not reach the full body of `protocol.md` or the second half of `enclave.md`. Internal task and queue design, the multi-agent runtime and the model-routing policy are undocumented publicly.