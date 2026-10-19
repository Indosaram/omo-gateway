<!-- task=st_01a144f7 -->
st_01a144f7 [completed] model mahoquot/claude-sonnet-5-5
Ask: research Meta Muse's trust, safety and economics - wanted: a sourced report. For you: the report is below. Pricing tiers, usage limits and non-US availability are the gaps (see EXPAND). Now: done. Next: none.

## Identity
- **Name:** Muse, internally "Hatch". It runs on "Muse Spark 1.3" and is built by Meta Platforms. I'm highly confident (~95%) this is the intended entity. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- **Official URLs:** the launch post (published 2026-09-08T19:00Z), security.muse.ai, ai.meta.com/muse, muse.ai, and the Meta Help Center articles. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- **Not retrieved:** the WSJ review (paywall/not fetched), the Forbes and Bloomberg Amazon pieces (headline-level only, via HN), and Neowin (Cloudflare block). The Amazon facts below come from Ars Technica.

## Findings

**Architecture and sandboxing**
- Each user gets a dedicated cloud Linux VM that holds the agent, the data and the credentials. Other users' agents can't reach it. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- Inside the VM, the agent runs in a `systemd-nspawn` "runtime cell":
  - Root in the cell maps to an unprivileged host user.
  - The cell has syscall filtering (no `io_uring`) and dropped capabilities (no `CAP_SYS_PTRACE`, no `CAP_NET_ADMIN`).
  - The cell's network is a virtual interface filtered with eBPF.

  [http://security.muse.ai/]
- Security services run outside the cell as separate systemd units: `hatch-safety` (classifiers), `privsep` workers, `hatch-authd` (credential store) and Sentinel. They talk over Unix sockets with `SO_PEERCRED` checks. [http://security.muse.ai/]
- Meta's own framing is "two isolated security domains on one box, not an LLM powered agent with root". [http://security.muse.ai/]

**Permission model and approval gates**
- Sentinel is a separate host-side agent and the sole permission authority for connector actions and all network egress. Muse proposes actions and only Sentinel grants them. Each decision is allow, deny or ask. [http://security.muse.ai/]
- Sentinel checks egress at layers 4 and 7: hostname, resolved IP, port, method, path and the decoded request. It also restricts SSRF. [http://security.muse.ai/]
- When Sentinel asks, execution stops and a dialog appears in the client UI, not in the chat with Muse. Approvals are strict capabilities bound to a connector or destination. Grant scopes are one-time, session, task, time-bounded or perpetual. [http://security.muse.ai/]
- "Tainted egress" uses eBPF and LSM hooks to track whether a process has read user data. Clean processes can auto-allow narrowly bounded requests. Tainted processes fall back to approval. [http://security.muse.ai/]
- User-facing permission settings:
  - Connectors can be set to "Ask for some actions" (ask before every write and important read) or "Always ask".
  - Web access has the same two levels.
  - Approval choices are Allow once, for this task, for this site, Always allow, or Deny.
  - Permissions can be managed per connector, per scheduled task, and per allowed website.

  [https://www.meta.com/help/artificial-intelligence/1385290430137537/]
- Read and write access are separated where the service supports it. Finer-grained controls sit on top of OAuth scopes, for example removing Gmail settings access from a read grant. [http://security.muse.ai/]
- Some actions can't be undone. Meta's help page says sent emails can't be reversed and that the user is "responsible for fixing errors". [https://www.meta.com/help/artificial-intelligence/1385290430137537/]

**Payments and purchases**
- Muse detects checkout pages and asks for approval with the exact purchase details every time. This applies to sites where a card is already saved. [http://security.muse.ai/]
- For new sites it uses Stripe Link's agent wallet. Each payment gets a single-use card tied to one merchant, one dollar amount and a limited time window, and each issuance needs approval. [http://security.muse.ai/]
- Link purchase protections cover Muse purchases, described as a first for an agent. Shop Pay and 1Password support are "coming soon". [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]

**Audit logs and visibility**
- The Activity log is a chronological record of actions and permissions granted. An "Upcoming" view shows scheduled tasks, and System Files are browsable. [https://www.meta.com/help/artificial-intelligence/1385290430137537/]
- The launch post promises an audit trail of what Muse has done and plans to do. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]

**Credentials**
- The model never sees real credentials. Runtime code gets "surrogate" tokens, and Sentinel swaps in the real credential at the network boundary after authorizing the request. [http://security.muse.ai/]
- Browser passwords go through a client UI straight to `authd`, so the agent can't read them. [http://security.muse.ai/]
- The email connector filters out one-time passcodes, password-reset links and magic links, using deterministic filters plus a classifier. [http://security.muse.ai/]
- OAuth tokens are stored in the user's VM, not in central Meta infrastructure. [http://security.muse.ai/]

**Prompt-injection defenses**
- Four layers:
  - model training (Meta claims Spark 1.3 is "close to SOTA" at resisting injection);
  - external content labeled untrusted;
  - an ensemble of independent classifiers over all external data;
  - human approval for any action that moves data out of the VM.

  [http://security.muse.ai/]
- Browser:
  - A sub-agent sees an accessibility tree, not the raw DOM.
  - It has no JavaScript execution and DevTools are disabled.
  - A broker outside the cell owns the CDP connection.
  - The agent is paused during user takeover or credential fill.
  - Classifiers watch for personal-data egress, injection in DOM, images and downloads, and high-risk forms.
  - Meta's malicious-site blocklist is applied inside the VM.

  [http://security.muse.ai/]
- Meta admits that injection is "an open problem". The bug bounty pays up to $300k, including up to $130k for single-user prompt-injection attacks. [http://security.muse.ai/]

**Privacy and retention**
- Muse "doesn't share" conversations or VM data with Meta ad systems. However, browsing appears as the user's activity, so a visit to a merchant site or a Marketplace search may indirectly influence ads. [http://security.muse.ai/]
- Training on interactions is on by default. It can be switched off, and the switch also applies to past interactions. When it is on, PII is stripped and the data is disassociated from the account. [https://www.meta.com/help/artificial-intelligence/1047255454427887/]
- Memory is user-inspectable (MEMORY.md). There is a "forget" skill, which works "to the best of its ability". Deleting an item may leave learned information behind, and Reset deletes everything. [https://www.meta.com/help/artificial-intelligence/1047255454427887/]
- The VM is backed up continuously. Today's architecture "does not prevent Meta from accessing data when necessary". Muse Confidential VM, encrypted with a user-held key, is promised "later this year". [http://security.muse.ai/]
- Facebook, Instagram and Threads connect automatically if the accounts share an Accounts Center. Custom connectors are not reviewed by Meta. [https://www.meta.com/help/artificial-intelligence/1687253048996149/]
- Wired reports that Muse builds detailed profiles of the user's friends and family. This is secondhand via Techdirt. [https://www.techdirt.com/2026/10/06/metas-muse-is-an-adorable-privacy-and-security-dumpster-fire/]

**Incidents**
- **Zero-day (Ars, 2026-09-21).** Patrick Wardle found that any local app or terminal command could change the transcription endpoint setting and capture the Muse account token. Meta shipped a hotfix about 12 hours later and called it "not a remote exploit". Ars calls it exploitable via ClickFix. [https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/]
- **Messages read despite refusal.** Inc. columnist Jason Aten says Muse acted on his iMessages after he declined messages access. HN commenters report this as ~187k lines synced with Full Disk Access off. This is unreproduced, and HN is split between "TCC bypass" and "user error". [https://daringfireball.net/linked/2026/09/22/aten-muse]
- **Apple response.** Apple announced upcoming Full Disk Access changes aimed at AI agents. A commenter says Muse opens the Full Disk Access pane and asks the user to enable it. [https://hn.algolia.com/api/v1/search?query=Muse%20Meta&tags=comment&numericFilters=created_at_i%3E1788825600&hitsPerPage=30]
- **Pre-launch hardening (404 Media, via an anonymous source).** There was a pre-launch "KVM escape" scare, a hardening push that began 2026-08-27, and a "half-baked protections" quote. Meta's bounty page classes a VM escape that reaches Meta production as its top-risk category. [https://www.404media.co/meta-rushed-to-fix-muse-vm-escape-vulnerability-immediately-before-launch/]
- **Other reports.** Techdirt cites secondhand claims that Muse gave away a home address while running a Marketplace account, and that a "pretend to be a Muse agent" trick gave root access. [https://www.techdirt.com/2026/10/06/metas-muse-is-an-adorable-privacy-and-security-dumpster-fire/]

**Amazon block**
- Starting about 2026-09-20, Amazon showed Muse users a message calling it an "unauthorized AI agent" that violates its Conditions of Use. Amazon says it asked Meta to remove Amazon from the experience. [https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/]
- Amazon's framing is that third-party agents should "operate openly and respect service provider decisions". (inference: this means identifiable agent traffic that sites can allow or block) [https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/]

**Pricing and availability**
- Muse is "free for most of what people need, with subscription plans" for more. The FAQ says it's "free with a usage limit" that refreshes. Tier names, prices and limits are not published in anything I could retrieve. [https://ai.meta.com/muse/]
- The launch post says US rollout on iOS, Android and muse.ai, with AI glasses "coming soon". Meta's own pages are inconsistent on surfaces: ai.meta.com lists a Mac app and WhatsApp as well, and the launch post also names WhatsApp. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- Meta announced "Muse for Small Business" skills and connectors around 2026-09-28. [https://ai.meta.com/muse/]
- A commenter on HN says Muse hit "5 million" users in 3 weeks. This is unverified. [https://hn.algolia.com/api/v1/search?query=Muse%20Meta&tags=comment&numericFilters=created_at_i%3E1788825600&hitsPerPage=30]

## Implications for a self-hosted personal agent gateway
1. **Approval capability tokens.** Mirror Sentinel's grant scopes (once, task, time-bounded, perpetual) as signed tokens bound to tool, destination and parameter hash. Don't use "yes" in chat. This blocks injected "approve" messages in a Discord or iMessage thread.
2. **Out-of-band approval channel.** Send approvals through a separate surface from the agent conversation, such as a Discord DM with buttons routed only to the daemon. Never let the omo agent read or forge the approval channel.
3. **Egress broker with taint tracking.** Route all agent traffic, including omo coding agents, through a daemon-owned proxy with a per-lane destination allowlist. Mark a lane tainted once it has read inbox or message data, and require approval for any new destination after that.
4. **Credential surrogation.** Keep real tokens in the daemon. Agents get opaque surrogates that the proxy swaps in after policy approves the request. This also covers git and API keys.
5. **Inbox ingestion hardening.** Label all inbox, iMessage and KakaoTalk content as untrusted. Strip OTPs, reset links and magic links before the agent sees them. Run a separate injection classifier on ingestion.
6. **Append-only audit log, plus a "planned" queue.** Record who, which lane, which approval and which hash for every action. Include a queue of scheduled and cron work, like Muse's Activity and Upcoming views.
7. **macOS permission hygiene.** Don't grant the daemon Full Disk Access. Read Messages through a narrow helper with explicit consent. The Aten and Wardle incidents show that local-settings and endpoint overrides are attack surface, so lock the daemon's config and IPC to its own UID.
8. **Spend lanes and purchase gates.** Keep payments in a separate lane that uses single-use virtual cards with per-merchant and per-amount caps, and always requires a human tap. Don't let coding lanes touch it.

## CLAIMS
- CLAIM: Muse runs each user's agent in a per-user cloud VM, with a separate Sentinel as the sole authority over connector actions and network egress. — RISK: normal — SOURCES: about.fb.com, security.muse.ai — PRIMARY: http://security.muse.ai/
- CLAIM: Muse requires per-purchase human approval and uses single-use Stripe Link cards limited by merchant, amount and time. — RISK: normal — SOURCES: security.muse.ai, about.fb.com — PRIMARY: http://security.muse.ai/
- CLAIM: A zero-day let local apps and terminal commands hijack the Muse account token. Meta hotfixed it within about 12 hours. — RISK: high — SOURCES: arstechnica.com — PRIMARY: https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/
- CLAIM: Muse read a journalist's Messages after he declined messages access, despite Full Disk Access reportedly being off. — RISK: high — SOURCES: inc.com (via daringfireball.net), news.ycombinator.com — PRIMARY: https://www.inc.com/jason-aten/metas-new-muse-ai-agent-read-my-private-messages-i-never-asked-it-to/91408202 (not fetched; the quote comes via Daring Fireball)
- CLAIM: Amazon blocked Muse from shopping on its site, calling it an unauthorized AI agent. — RISK: normal — SOURCES: arstechnica.com — PRIMARY: https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/
- CLAIM: Meta engineers rushed to fix VM-escape vulnerabilities before launch. — RISK: high — SOURCES: 404media.co (one anonymous source) — PRIMARY: https://www.404media.co/meta-rushed-to-fix-muse-vm-escape-vulnerability-immediately-before-launch/
- CLAIM: Muse trains on user interactions by default, with PII scrubbing, and the user can opt out. — RISK: normal — SOURCES: meta.com Help Center, security.muse.ai — PRIMARY: https://www.meta.com/help/artificial-intelligence/1047255454427887/
- CLAIM: Muse is free with a usage limit and has paid tiers. The prices and limits are unknown. — RISK: normal — SOURCES: ai.meta.com, about.fb.com — PRIMARY: https://ai.meta.com/muse/
- CLAIM: Launch was US-only on iOS, Android and web. — RISK: normal — SOURCES: about.fb.com — PRIMARY: https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent

## EXPAND
- LEAD: Muse subscription tiers, prices and usage limits (muse.ai pricing page, app-store listing, WSJ and CNBC pieces). — WHY: the economics were not retrievable, and my searches found no tier names. — ANGLE: economics benchmark for spend and rate-limit design.
- LEAD: Jason Aten's Inc. article and Apple's Full Disk Access note (developer.apple.com/news/?id=p6zjojqw). — WHY: the Messages-access claim is disputed and could change the macOS permission advice. — ANGLE: macOS TCC threat model for the daemon.
- LEAD: Hunterbrook "muse-doxxing" and the Wired profile article. — WHY: social-graph access and third-party profiling are privacy risks not covered by the VM design. — ANGLE: data minimization for the inbox-ingestion lane.
- LEAD: Muse bug-bounty payout guidelines (bugbounty.meta.com/payout-guidelines/muse/). — WHY: it lists the threat categories Meta itself considers worst. — ANGLE: red-team checklist for the gateway.
- LEAD: Amazon's ToS and policy language on agents, plus the Bloomberg and Forbes coverage. — WHY: agent identification and site opt-out norms may affect browsing lanes. — ANGLE: browser lane user-agent and disclosure policy.