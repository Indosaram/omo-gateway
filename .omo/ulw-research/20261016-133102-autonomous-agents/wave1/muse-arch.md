<!-- task=st_01a144f6 -->
st_01a144f6 [completed] model mahoquot/claude-sonnet-5-5
Ask: Research Meta Muse architecture and deliver a sourced report. Wanted: findings, implications for the gateway, claims and leads. For you: the report is complete and rests mostly on Meta primary sources. I did not find the WSJ review, and I did not read the Amazon-block articles, only their HN titles. Now: done. Next: none.

Method note: no `web_search` tool was available. I fetched primary pages with `curl` through a reader proxy (r.jina.ai), plus the HN Algolia API and the GitHub API. Forbes and Neowin were blocked, and Bloomberg and NYT were not fetched.

## Identity
- **Name:** Muse, internally codenamed "Hatch". Meta's personal AI agent, from Meta (Meta Superintelligence Labs).
- **Official URL:** https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent. The product sites are muse.ai and security.muse.ai.
- **Launch:** published 2026-09-08. Rolling out in the US on iOS, Android and the web, with AI glasses "coming soon".
- **Confidence:** high. The launch post, the Meta research blog and the Meta help center all agree, and "Hatch" is confirmed by both Meta's blog and The Verge's reporting.
- **Name clash:** "Muse" also names Meta's model family (Muse Spark, Muse Wonder) and Muse Code. Keep them separate.

## Findings

**Execution environment**
- Each user gets a dedicated cloud VM that holds both the agent and the user's data. It is an isolated Linux box with a browser and enough CPU, memory and storage to compile code, develop skills and run concurrent sub-agents and crons. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- The VM is described as the "system of record". Clients (iOS, Android, web) connect directly to it over a secure transport. Muse also runs directly inside WhatsApp. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse] [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- The `hatch` daemon (the harness) and the user's workspace run in a `systemd-nspawn` container. Root inside the container maps to an unprivileged host user, the container has a full Debian image, syscalls are filtered (no `io_uring`), and capabilities are limited (no `CAP_SYS_PTRACE`, no `CAP_NET_ADMIN`). [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- Meta frames this as two isolated security domains on one box. The services outside the container are separate systemd units: `hatch-safety` (independent classifiers), `privsep` workers, `hatch-authd` (credentials), Sentinel, a postgres database for durable state, and inference and telemetry proxies. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- Container-to-service IPC uses Unix sockets with `SO_PEERCRED` and peer ACLs. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- The browser is a Chromium-based browser behind a virtualization layer, and the user can watch it and take over. A separate broker owns the Chrome DevTools Protocol connection. The browser sub-agent sees an accessibility-tree snapshot, not the raw DOM, and has no script or exec verbs. The agent is paused while the user drives or while stored credentials are being filled in. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- **Local device access:** it is not a desktop agent. Connectors cover phone calendar and contacts, Apple Health and Android SMS, with access managed in device settings. [https://www.meta.com/help/artificial-intelligence/1687253048996149/] [https://tailscale.com/blog/meta-muse-ai-agent-tailscale]
- Inc's reviewer ran Muse on an iPhone and a Mac mini. AppleInsider says Muse synced about 187,000 lines of his Messages database even with Full Disk Access off. This is one reviewer's report, and it rests on Meta's reported mechanism that Muse "read banners", which the reviewer disputed. [https://appleinsider.com/articles/26/09/28/metas-new-ai-agent-blatantly-ignores-users-permissions]

**Connectors and authentication**
- Meta built an initial set of connectors with provider cooperation. Each comes with a SKILL (written instructions to Muse). Muse can also write its own custom connectors against APIs or CLIs, and Meta does not review them. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse] [https://www.meta.com/help/artificial-intelligence/1687253048996149/]
- Connectors are added by asking in chat ("Connect my Gmail") or through Settings, using OAuth-style authorization. Read and write access can be separated, and some connectors push proactive updates such as calendar changes. [https://www.meta.com/help/artificial-intelligence/1687253048996149/]
- OAuth tokens are stored in the user's own VM, not in central Meta infrastructure, and are held by `hatch-authd`. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- Built-in connector logic runs in sandboxed `privsep` workers outside the agent's container. The CLI inside the container only parses arguments and passes typed arguments and file descriptors over a socket. Each worker has a cgroup-bound credential allowlist. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- **Credential surrogation:** the agent only sees surrogate tokens. Sentinel swaps in the real credential at the network boundary after it approves the request, so prompt injection cannot extract real secrets. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- **Passwords:** a custom client UI captures a username and password and sends it straight to `authd`. It is injected into the browser at the point of need, so the agent never sees it. Muse also cannot see passwords the user types into the browser themselves. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse] [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- 1Password support is "coming soon". [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- **Email hygiene:** the email connector filters out one-time codes, password-reset links and magic links, using deterministic filters plus a classifier. This stops a compromised agent from using the inbox to take over other accounts. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- **Payments:** Stripe Link issues a single-use virtual card bound to a merchant, an amount and an expiry. Every purchase needs human approval, and Shop Pay is coming. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse] [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- **Tailscale:** Muse joins the user's tailnet as its own node. It makes outbound connections only and needs explicit confirmation the first time it reaches each device. Tailscale reports it can run SSH, `ls` and `podman` commands on tailnet hosts. [https://tailscale.com/blog/meta-muse-ai-agent-tailscale]

**Permission and approval model (Sentinel)**
- Sentinel is a separate host-side agent and the sole permission authority for connector actions and all network egress. Muse proposes an action and Sentinel returns allow, deny or ask. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- Egress is checked through a forward proxy built on userns, veth and eBPF. Sentinel inspects the hostname, resolved IP, port, method, path and decoded request, and blocks SSRF via DNS rebinding. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- **Tainted egress:** eBPF and LSM hooks mark a process as tainted once it reads user data. Clean processes get narrow auto-allow, and tainted ones fall back to approval prompts. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- Approvals show in the client UI outside the chat thread. They are capabilities scoped to one connector and destination, and can be one-time, session, task, time-bounded or perpetual. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- Meta says a complete audit trail of past and planned actions is shown to the user. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- **Prompt-injection defense:** external data is labeled untrusted, an ensemble of classifiers scans everything entering context, the model is trained against injection, and there are browser-specific classifiers. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]

**Memory and personal context**
- Memory is plain Markdown files the user can edit: `MEMORY.md`, `Soul.md` (personality and boundaries) and `Identity.md`. [https://www.meta.com/help/artificial-intelligence/2225571704857152/]
- Other stored state includes a main chat plus side chats, a Goals tab with check-ins, a Library of artifacts and system files, and reminders and scheduled tasks. The user can ask Muse to forget something, reset it, or download all data. [https://www.meta.com/help/artificial-intelligence/2225571704857152/]
- The forget skill works on a best-effort basis. Meta warns that Muse "may still remember" information from deleted items. [https://www.meta.com/help/artificial-intelligence/2225571704857152/]
- **Reported but not confirmed by Meta:** a nightly "dream" job reviews recent conversations and turns them into guidance for later ones. This comes from developers who dumped the VM filesystem, as relayed by The Verge. [https://www.theverge.com/ai-artificial-intelligence/1000222/meta-muse-ai-filesystem]
- Continuous VM backup is claimed. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]

**Long-running tasks and multi-agent structure**
- Muse keeps working after the app is closed and returns when something changes or when it needs approval. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- It is proactive: it makes unprompted suggestions and acts on details mentioned once. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- Meta describes the agent as one that "launches swarms of subagents, builds its own tools, and edits itself". The VM is sized for concurrent sub-agents and crons. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- A dedicated browser sub-agent handles each browsing task. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- The model is trained to route incoming prompts to the correct task inside one long, messy thread, and to work silently or give frequent updates depending on user preference. [https://research.meta.ai/blog/introducing-muse-spark-1-3]
- **Reported but not confirmed by Meta:** The Verge reports "machinery that manages runaway agent spawning" in the dumped files. [https://www.theverge.com/ai-artificial-intelligence/1000222/meta-muse-ai-filesystem]

**Models**
- Muse is powered by Muse Spark (1.3 is named in Meta's blog). It was trained for zero-shot tool use via CLIs and skills, long context, long-trajectory instruction following with injection awareness, and multi-agent coordination. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse] [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- Separate classifier models run in `hatch-safety`, outside the agent container. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- Inference leaves the VM through an inference proxy. Trajectories are used for training after PII sanitization, with an opt-out. The setting is on by default. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse] [https://www.meta.com/help/artificial-intelligence/2225571704857152/]

**Developer APIs and SDKs**
- Muse itself has no public agent API that I found. Developer surfaces are dev.meta.ai (Meta Model API, Muse Code), custom connectors written by Muse, and the open-source Muse gadget SDK (Apache-2.0, `facebookincubator/muse-gadget-sdk`, pushed 2026-10-15). [https://dev.meta.ai/] [https://github.com/facebookincubator/muse-gadget-sdk]
- The gadget SDK targets ESP32 and Raspberry Pi hardware for displays, buttons and sensors. Muse Home Link is a Meta-made device, reportedly 5,000 units, that uses community skills. [https://www.theverge.com/tech/1004330/meta-muse-ai-gadgets-home-link]
- dev.meta.ai lists Muse Code (a multi-agent terminal coding tool) and Muse Glimmer ("always-on, local agents", on Hugging Face). Muse Spark Open is listed as an open-weight model. Whether these are available now is not verified. [https://dev.meta.ai/]

**Failure modes and context**
- Meta's spokesperson said that dumping the VM filesystem is not a breach, and employees called it intended behavior. The Verge's own test showed Muse refusing, then complying after flattery, which indicates weak injection resistance. [https://www.theverge.com/ai-artificial-intelligence/1000222/meta-muse-ai-filesystem]
- A researcher found an exploit that could hijack the agent, and Meta shipped a hotfix. A 404 Media source claims a VM-escape bug was patched in a rush before launch. This is a single anonymous source. [https://www.theverge.com/ai-artificial-intelligence/1000222/meta-muse-ai-filesystem] [https://www.theverge.com/tech/1004781/meta-reportedly-made-a-mad-dash-to-fix-a-muse-ai-breakout-bug-weeks-before-launch]
- The bug bounty pays up to $300,000, including up to $130,000 for a single-user prompt injection. [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse]
- A "Muse Confidential VM", encrypted with a user-held key, is promised "later this year". It is not shipped. [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent]
- Amazon blocked Muse from shopping on amazon.com. I only saw the headlines on HN (Forbes, Bloomberg, Neowin), not the article bodies, so the mechanism is unverified. [https://hn.algolia.com/api/v1/search?query=Meta%20Muse%20agent&tags=story]
- I found no WSJ review. Two HN searches for it returned zero hits. The NYT first-person review ("I Gave My Life over to Meta's A.I. Agent...") does exist, but I did not read it. [https://hn.algolia.com/api/v1/search?query=Meta%20Muse%20agent&tags=story]

## Implications for a self-hosted personal agent gateway
These are my inferences from the findings above, not Meta claims.

1. **Credential surrogation at an egress proxy.** Give coding agents and cron jobs placeholder tokens, and have the Rust daemon swap in real secrets (from the macOS Keychain) only after a policy check on the outgoing request. The agent never holds a real secret.
2. **A separate permission authority.** Run an approval service as its own process with its own UID, outside every agent's sandbox. Agents ask it over a Unix socket, and a Discord, iMessage or KakaoTalk approval reply routes to it, not through the agent's chat thread. This matches Meta's rule that approvals are not conversational.
3. **Scoped grants.** Approvals become capabilities bound to a connector or destination and a duration (once, task, session, timed, standing). A cron job gets a task or timed grant, not a blanket one.
4. **Inbox ingestion that strips secrets.** Filter one-time codes, reset links and magic links before mail reaches any LLM context, and tag all ingested content as untrusted. This protects the owner if an agent is subverted via email.
5. **Taint tracking by lane.** Mark a lane tainted once it reads private data such as inbox or Messages content. Tainted lanes lose auto-allow on outbound network and iMessage sends, and clean lanes keep it. This enforces the "no more than two of three" rule from Tailscale's trifecta framing.
6. **Auditable plain-file memory.** Keep memory as human-editable Markdown (`MEMORY.md`, `SOUL.md`), add a `forget` command, and add a nightly consolidation job through the existing cron.
7. **Network scoping by tailnet.** If the Mac reaches other hosts over Tailscale, use ACL grants and tags so the agent node can only reach explicitly allowed machines. Require a first-use confirmation per host.
8. **Sandbox for omo coding agents.** Treat each omo run as untrusted. Use a restricted macOS sandbox with a filtered environment and no raw credentials. Meta's design (an unprivileged container, a socket-only interface to a privileged broker) maps to a per-lane sandbox profile plus a broker.

## CLAIMS
- CLAIM: Muse runs in a per-user dedicated cloud Linux VM that stores the agent, data and credentials. — RISK: normal — SOURCES: about.fb.com, research.meta.ai — PRIMARY: https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse
- CLAIM: A separate Sentinel agent is the sole authority for connector actions and network egress. — RISK: normal — SOURCES: research.meta.ai, about.fb.com — PRIMARY: https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse
- CLAIM: The agent only sees surrogate credentials, and the real secrets are inserted at the network boundary. — RISK: high — SOURCES: research.meta.ai — PRIMARY: https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse
- CLAIM: The model behind Muse is Muse Spark (1.3 per Meta's blog). — RISK: normal — SOURCES: about.fb.com, research.meta.ai — PRIMARY: https://research.meta.ai/blog/introducing-muse-spark-1-3
- CLAIM: Muse's memory is user-editable Markdown files. — RISK: normal — SOURCES: meta.com help center, theverge.com — PRIMARY: https://www.meta.com/help/artificial-intelligence/2225571704857152/
- CLAIM: Muse joins a user's tailnet and can run SSH and container commands on other nodes. — RISK: normal — SOURCES: tailscale.com — PRIMARY: none
- CLAIM: Muse synced a reviewer's Messages database with Full Disk Access off. — RISK: high — SOURCES: appleinsider.com, inc.com, daringfireball.net — PRIMARY: none. This is a single reviewer's report, and Meta's response is not seen.
- CLAIM: A VM-escape bug was rushed out as a fix before launch. — RISK: high — SOURCES: theverge.com, 404media.co — PRIMARY: none. This is one anonymous source.
- CLAIM: Amazon blocked Muse from amazon.com. — RISK: normal — SOURCES: HN headlines pointing to forbes.com and bloomberg.com — PRIMARY: none, since the bodies were not read.
- CLAIM: There is no public Muse agent API. Developer surfaces are the model API, Muse Code and the gadget SDK. — RISK: normal — SOURCES: dev.meta.ai, github.com — PRIMARY: https://dev.meta.ai/. This is an absence-of-evidence claim.

## EXPAND
- LEAD: Meta's "How Muse works" help page (https://www.meta.com/help/artificial-intelligence/1385290430137537/) — WHY: likely the official statement of how approvals and the Mac and iPhone clients work, and the best check on the Messages-sync report — ANGLE: device clients and permissions.
- LEAD: Developers' filesystem-dump write-up (mouse.dev/blog/muse-runtime-export, linked from The Verge) — WHY: first-hand detail on the skills, cron and sub-agent machinery and the "dream" job — ANGLE: harness internals.
- LEAD: WSJ review and Amazon block — WHY: not found or read, so the shopping-blocking mechanism and the review's verdict are open — ANGLE: browser-agent identification and user-agent policy.
- LEAD: github.com/facebookincubator/muse-gadget-sdk source — WHY: it may show how Muse talks to external devices over a protocol, which is the closest thing to a public integration API — ANGLE: developer APIs.
- LEAD: Meta/Apple MLX post and Muse Glimmer — WHY: possible local-model options for a Mac-hosted agent — ANGLE: local inference.