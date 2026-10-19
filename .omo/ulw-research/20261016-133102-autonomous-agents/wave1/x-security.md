<!-- task=st_01a1450a -->
st_01a1450a [completed] model mahoquot/claude-sonnet-5-5
Ask: security research for always-on credentialed agents - wanted: a sourced report for the gateway design. For you: lethal trifecta, Rule of Two, CaMeL, adaptive-attack results, the Cline/OpenClaw incident and sandbox egress controls are verified from primary pages. The OWASP list and the Meta blog text are not. Now: report delivered. Next: none.

# Security of always-on autonomous agents with credentials

Web search was unavailable. I used direct fetches of primary pages and the HN Algolia API. OWASP's genai.owasp.org and securityboulevard.com returned bot challenges, and the Meta blog's body was truncated. The Rule of Two text therefore comes from Simon Willison's quotation of it.

## Identity

This is a cross-cutting topic, not one product. The canonical entities, with confidence:

| Entity | Details | Confidence |
|---|---|---|
| Lethal trifecta | Simon Willison, personal blog, 16 Jun 2025. https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/ | High |
| Agents Rule of Two | Meta AI blog, 31 Oct 2025, no listed authors, shared by Meta researcher Mick Ayzenberg. https://ai.meta.com/blog/practical-ai-agent-security/ | High on identity and date (via Willison), medium on exact wording |
| CaMeL | "Defeating Prompt Injections by Design", Google DeepMind with ETH Zurich (Tramèr). arXiv 2503.18813, submitted 24 Mar 2025, v2 24 Jun 2025. Code at github.com/google-research/camel-prompt-injection. https://arxiv.org/abs/2503.18813 | High |
| OWASP Top 10 for Agentic Applications for 2026 | Published 9 Dec 2025 by the OWASP GenAI Security Project. https://genai.owasp.org/2025/12/09/owasp-top-10-for-agentic-applications-the-benchmark-for-agentic-security-in-the-age-of-autonomous-ai/ | High that it exists (HN item), unverified content |
| Sandbox runtime | Anthropic, announced 20 Oct 2025, Apache-2.0, repo now at anthropics/sandbox-runtime. https://github.com/anthropics/sandbox-runtime | High |
| OpenClaw (the "claw" agents) | Viral open-source always-on agent. It is the main real-world case study (Feb 2026 incidents). HN says it "dropped in November" 2025. | Medium |

## Findings

**Frameworks**

- The lethal trifecta is private data, exposure to untrusted content, and the ability to communicate externally. An agent with all three can be tricked into stealing the data. [https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/]
- Email is named as a perfect untrusted source ("an attacker can literally email your LLM"). Any outbound HTTP request, including image loads or a link the user clicks, can serve as the exfiltration channel. [https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/]
- Willison says guardrail products are not reliable: "95%" detection is a failing grade in application security. Once you combine tools yourself, the only safe path is to avoid the trifecta. [https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/]
- Meta's Rule of Two says that within a session an agent should satisfy at most two of three properties:
  - [A] processes untrustworthy inputs;
  - [B] accesses sensitive systems or private data;
  - [C] can change state or communicate externally.
  If all three are needed without a fresh context window, the agent must not run autonomously and needs human-in-the-loop approval or other reliable validation. [https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/]
- Rule of Two adds state-changing actions, which the trifecta (exfiltration only) misses. Meta later relabeled the two-property overlaps from "safe" to "lower risk". Willison objects that A+C is still harmful. Meta's reply is that removing [B] means the agent only reaches non-critical systems, such as one in a tight sandbox. [https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/]
- CaMeL extracts control flow and data flow from the trusted user query, so untrusted data can never alter program flow. It also attaches capabilities to values and enforces policies at tool-call time to block exfiltration. [https://arxiv.org/abs/2503.18813]
- CaMeL solved 77% of AgentDojo tasks with provable security, against 84% for an undefended system. That is a real utility cost. [https://arxiv.org/abs/2503.18813]
- A design-patterns paper's core rule: once an agent has ingested untrusted input, it must be constrained so that input cannot trigger any consequential action. [https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/]

**Why detection and filtering are not enough**

- "The Attacker Moves Second" (arXiv 2510.09023, 10 Oct 2025, authors from OpenAI, Anthropic and Google DeepMind) bypassed 12 published defenses with adaptive attacks. Most had success rates above 90%, although many had originally reported near-zero. [https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/]
- Human red-teaming, with 500 participants and a $20k prize, defeated every defense tested. [https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/]
- OpenAI's CISO said that prompt injection "remains a frontier, unsolved security problem". Their mitigations for the Atlas browser agent are:
  - "logged out mode" (acts without the user's credentials);
  - "Watch Mode" (requires the tab to stay active on sensitive sites);
  - rapid response systems;
  - overlapping guardrails.
  [https://simonwillison.net/2025/Oct/22/openai-ciso-on-atlas/]
- Willison's comment on that post is that logged-in mode is for well-scoped actions on trusted sites, and that "review my emails and take whatever actions are needed" is the dangerous request shape. He also notes that delegating security decisions to end users is unfair to them. [https://simonwillison.net/2025/Oct/22/openai-ciso-on-atlas/]

**Incidents**

- A hacker used a prompt-injection hole in Cline's Claude-powered workflow, found by researcher Adnan Khan, to install OpenClaw on users' computers. The agents were not activated on install; the attacker could have installed anything. Khan had warned Cline weeks earlier, and it was fixed only after public disclosure. [https://www.theverge.com/ai-artificial-intelligence/881574/cline-openclaw-prompt-injection-hack]
- The same Verge piece says OpenAI introduced a "Lockdown Mode" for ChatGPT to limit what a hijacked agent can leak. [https://www.theverge.com/ai-artificial-intelligence/881574/cline-openclaw-prompt-injection-hack]
- OpenClaw-specific prompt-injection stories (Feb 2026):
  - a chat-history-spoofing injection, fixed [https://news.ycombinator.com/item?id=46973193];
  - a journalist "hacked my own computer" demo [https://www.androidauthority.com/openclaw-ai-prompt-injection-3636904/];
  - a Show HN claiming credential exfiltration through the chat interface within 2 minutes [https://github.com/clawshell/clawshell].
  These are self-reported or blog-level claims and I could not verify them.
- A security-research series ("BrokenClaw") ran injection experiments against OpenClaw with Opus 4.6 and GPT-5.4, in Mar-Apr 2026. I did not read the results. [https://veganmosfet.codeberg.page/posts/2026-04-08-openclaw_gpt5_4/]
- A Show HN from Seoul National University's compsec lab, "Defending OpenClaw against indirect prompt injection" (8 Jul 2026), exists. I could not fetch the page, so its method and results are unverified. [https://compsec.snu.ac.kr/blog/dualview]
- One HN post attributes "Wiz uncovered 1.5 million leaked API keys" to the Moltbook agent-social-network episode. I treat this as unverified, because the source is a speculative post. [https://news.ycombinator.com/item?id=46897428]
- Brave's security team showed browser agents from other vendors leaking private data through prompt injection (Oct 2025). I saw this only as a link from Willison and did not read Brave's post. [https://simonwillison.net/2025/Oct/22/openai-ciso-on-atlas/]

**Controls that work**

- Anthropic's sandbox needs both filesystem and network isolation. Without network isolation, a compromised agent exfiltrates SSH keys. Without filesystem isolation, it escapes the sandbox and reaches the network. [https://www.anthropic.com/engineering/claude-code-sandboxing]
- Implementation: macOS seatbelt and Linux bubblewrap. All network traffic goes through a Unix-domain-socket proxy outside the sandbox. The proxy enforces a domain allowlist and asks the user about new domains. The proxy can be customized to enforce arbitrary rules on outgoing traffic. [https://www.anthropic.com/engineering/claude-code-sandboxing]
- Anthropic reports that sandboxing cut permission prompts by 84%, and describes "approval fatigue" as a security failure mode. [https://www.anthropic.com/engineering/claude-code-sandboxing]
- In Claude Code on the web, git credentials and signing keys never enter the sandbox. A proxy holds the real token, validates the scoped in-sandbox credential and the content of each git operation (for example, push only to the configured branch), then attaches the token. [https://www.anthropic.com/engineering/claude-code-sandboxing]
- Community patterns that echo this (self-reported, unverified):
  - ClawShell keeps secrets in a separate OS-enforced process so a hijacked agent gets only a scoped identifier. [https://github.com/clawshell/clawshell]
  - A Cedar policy-as-code extension for OpenClaw blocks tool calls deterministically, for example `rm`, regardless of what the model was tricked into wanting. [https://securetrajectories.substack.com/p/openclaw-rm-rf-policy-as-code]
  - ContextFort's runtime controls check tool responses an attacker could modify with a small model, then require UI approval for externally visible writes. [https://github.com/ContextFort-AI/clawdbot-runtime-controls]
- The OWASP Top 10 for Agentic Applications exists and a community Cedar policy pack maps to it. I could not fetch the list. The ASI01-ASI10 categories below (goal hijack, tool misuse, identity/privilege abuse, supply chain, code execution, memory poisoning, inter-agent comms, cascading failures, human-trust exploitation, rogue agents) are from memory and unverified. [https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/]

## Implications for a self-hosted personal agent gateway

Rust daemon on a Mac; Discord/iMessage/KakaoTalk; omo coding agents; cron; inbox ingestion; lanes. These are my recommendations, derived from the sources above.

1. **Trust-tagged lanes enforcing the Rule of Two.** Give every lane a declared A/B/C profile. For example, an inbox-ingest lane has A (email and web) and nothing else: no secrets, no outbound network, no writes except to a staging queue. A coding lane has B and C (repo, tokens) but takes only owner-authored input. Refuse, at daemon level, to start a session that holds A+B+C without a human approval gate. Base this on [https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/].
2. **Fresh context on every trust boundary.** Rule of Two allows A+B+C only across new sessions. The inbox lane summarizes into a typed, size-limited structure. The daemon passes only that structure, never raw text, to a privileged lane. Raw email should never share a context window with credentials.
3. **Egress proxy per lane, default deny.**
   - Run each omo agent under seatbelt (or sandbox-exec) with a Unix-socket-only network path. The daemon owns the proxy: domain allowlist, a new-domain prompt to the owner on Discord or iMessage, and logging of every request. Based on [https://www.anthropic.com/engineering/claude-code-sandboxing] and [https://github.com/anthropics/sandbox-runtime].
   - Inspect URLs for data-carrying query strings, since any URL can carry data out [https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/].
4. **Credential brokering.** Agents never see real tokens. They get per-lane scoped handles, and the daemon-side broker validates each call (repo, branch, recipient, amount) and attaches the secret, as in Claude Code on the web's git proxy.
5. **CaMeL-lite for cron and inbox flows.** Make plans deterministic: the daemon (trusted code) fixes the sequence of tool calls from the owner's instruction or cron definition, and the LLM only fills typed slots from untrusted data. Add capability tags, so data derived from email cannot flow to the `send` tool without owner approval. Accept a utility cost: CaMeL lost about 7 points of task success [https://arxiv.org/abs/2503.18813].
6. **Approvals over the chat channels, built to avoid fatigue.** Send high-risk actions (external send, new domain, push, delete) to the owner with the exact payload and its provenance ("this recipient came from email X"). Batch and deduplicate low-risk ones. Never let message content in the approval channel authorize anything. Chat-history spoofing was a real OpenClaw issue [https://news.ycombinator.com/item?id=46973193]. Also treat inbound iMessage/KakaoTalk/Discord messages from non-owner senders as untrusted (A), and authenticate the owner by sender identity, not message content.
7. **Deterministic policy layer plus tamper-evident audit log.**
   - Keep a Cedar-style or Rust-native allow/deny table on tool calls, independent of the model: block `sudo`, credential paths, shell writes outside the workspace, and edits to the agent's own identity and config files.
   - Keep an append-only, hash-chained record of every tool call with its provenance tag, for forensics ("was it prompt injection?"). The idea comes from the Lightbox HN post [https://news.ycombinator.com/item?id=46783126].
8. **Treat the supply chain as untrusted input.** The Cline incident shows that a CI or workflow agent reading attacker-controlled issue text can install arbitrary software. Do not let omo agents install tools or MCP servers, or run package installs, outside the sandbox without approval. Pin and hash plugins. [https://www.theverge.com/ai-artificial-intelligence/881574/cline-openclaw-prompt-injection-hack]
9. **A kill switch and a "lockdown mode".** One owner command (or a daemon-side breaker on anomalies such as unexpected domains, bursts of sends or credential-path reads) drops all lanes to read-only and no egress, similar in spirit to ChatGPT's Lockdown Mode [https://www.theverge.com/ai-artificial-intelligence/881574/cline-openclaw-prompt-injection-hack].

Do not rely on a prompt-injection classifier as a primary control. Use it at most as telemetry, because adaptive attacks beat such defenses at above 90% [https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/].

## CLAIMS

- CLAIM: The lethal trifecta (private data, untrusted content, external communication) is sufficient for data theft via prompt injection, and guardrail products cannot be trusted to stop it. — RISK: high — SOURCES: simonwillison.net — PRIMARY: https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/
- CLAIM: Meta's Agents Rule of Two says to satisfy at most two of untrusted input, sensitive access, and state change or external communication per session, otherwise require human supervision. — RISK: high — SOURCES: simonwillison.net (quoting ai.meta.com) — PRIMARY: https://ai.meta.com/blog/practical-ai-agent-security/ (page fetched, body truncated; wording confirmed only via Willison)
- CLAIM: 12 published prompt-injection and jailbreak defenses were bypassed above 90% by adaptive attacks, and human red-teaming beat all of them. — RISK: high — SOURCES: simonwillison.net, arxiv.org — PRIMARY: https://arxiv.org/abs/2510.09023 (I read only Willison's summary, not the paper)
- CLAIM: CaMeL gives provable security on 77% of AgentDojo tasks versus 84% undefended. — RISK: normal — SOURCES: arxiv.org — PRIMARY: https://arxiv.org/abs/2503.18813
- CLAIM: An effective sandbox needs both filesystem and network isolation, and a proxy-based domain allowlist is the egress mechanism. — RISK: high — SOURCES: anthropic.com, github.com — PRIMARY: https://www.anthropic.com/engineering/claude-code-sandboxing
- CLAIM: Real credentials should live outside the agent sandbox and be attached by a validating proxy. — RISK: high — SOURCES: anthropic.com — PRIMARY: https://www.anthropic.com/engineering/claude-code-sandboxing
- CLAIM: A prompt injection against Cline's Claude workflow was used in Feb 2026 to install OpenClaw on users' machines. — RISK: high — SOURCES: theverge.com — PRIMARY: https://www.theverge.com/ai-artificial-intelligence/881574/cline-openclaw-prompt-injection-hack
- CLAIM: OpenAI's CISO acknowledged that prompt injection is unsolved and ships logged-out mode and Watch Mode as mitigations. — RISK: normal — SOURCES: simonwillison.net (quoting a tweet) — PRIMARY: https://twitter.com/cryps1s/status/1981037851279278414 (not fetched)
- CLAIM: OWASP published a Top 10 for Agentic Applications on 9 Dec 2025. — RISK: normal — SOURCES: hn.algolia.com — PRIMARY: https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/ (blocked; item list unverified)
- CLAIM: OpenClaw agents were exfiltrated within minutes and 70% of injection attempts succeeded in ZeroLeaks testing. — RISK: normal — SOURCES: hn.algolia.com (self-promotional or speculative posts) — PRIMARY: none (treat as unverified)

## EXPAND

- LEAD: Fetch the OWASP Agentic Top 10 PDF or the GitHub mirror and confirm the ASI01-10 names and mitigations. — WHY: I could not read it, and the gateway's policy table should map to it. — ANGLE: `gh api repos/OWASP/...` or the genai.owasp.org PDF download.
- LEAD: Read Meta's blog body and the CaMeL repo README (google-research/camel-prompt-injection). — WHY: exact Rule of Two wording, and whether CaMeL has a usable policy engine to port to Rust. — ANGLE: GitHub contents API.
- LEAD: Read the "Attacker Moves Second" paper (arXiv 2510.09023) and the BrokenClaw series. — WHY: concrete attack payloads against always-on agents with email and web tools, usable as red-team tests for the gateway. — ANGLE: arXiv HTML, codeberg.
- LEAD: Read the Cline incident write-up by Adnan Khan and the SNU "DualView" defense. — WHY: a first-hand, CI-style injection path, and a possible dual-context defense. — ANGLE: Khan's blog, compsec.snu.ac.kr (reset the connection on my fetch).
- LEAD: Read the sandbox-runtime source for the proxy and macOS seatbelt profile details. — WHY: it is the closest reference for building the daemon's egress proxy, and the repo has 287 open issues that may list known bypasses. — ANGLE: the issues list and `src/` in anthropics/sandbox-runtime.
- LEAD: Read the 3 Oct 2026 Willison post "default hard budget caps". — WHY: possible relevance to runaway-cost and loop guards for cron lanes. — ANGLE: simonwillison.net.