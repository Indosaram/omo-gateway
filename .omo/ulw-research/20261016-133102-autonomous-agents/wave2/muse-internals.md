<!-- task=st_01a1451b -->
st_01a1451b [completed] model mahoquot/claude-sonnet-5-5
Ask: research Muse's skills, cron, sub-agents, dream job, Mac client and Messages DB access, approvals and pricing from four named sources. Wanted: a sourced report with patterns for a Rust gateway.
For you: the four sources were read except Meta's `research.meta.ai` security post, which stayed rate-limited, and pricing is only partly sourced. The sections below are the report.
Now: complete. Next: none.

## Findings

Retrieval routes:
- `mouse.dev`: direct `curl` returned the full HTML but was truncated, so I used `r.jina.ai` for the clean text.
- Meta help page: `r.jina.ai` (the direct fetch was not tried).
- `about.fb.com`: `r.jina.ai`. The URL `/news/2026/09/introducing-muse/` is a guess from the title and worked.
- GitHub: `gh` (the repo view and tree API) plus `raw.githubusercontent.com` at the pinned commit `fa073045a5572a8e3367fae142c1b7a7c554f222`.
- Secondary sources: HN Algolia API and `r.jina.ai`.
- Not tried: Wayback, `old.reddit.com`.

**Skills**
- PRIMARY (self-reported by one researcher): the runtime has about 68 skill directories under `/opt/hatch/skills/`. Each pairs a `SKILL.md` with a CLI tool or supporting code. `skill-scopes.conf` and `bin-scopes.conf` list scopes, including unreleased connectors. [https://mouse.dev/blog/muse-runtime-export] via jina
- PRIMARY: the GitHub repo ships `skills/gadget-*/SKILL.md` for about 40 device types, with a `CATALOG.md`. Examples are Hue, Sonos, printers and Zigbee2MQTT. [https://github.com/facebookincubator/muse-gadget-sdk/tree/fa073045a5572a8e3367fae142c1b7a7c554f222/skills] via `gh` tree API
- PRIMARY: the agent's home directory holds `SOUL.md`, `IDENTITY.md`, `USER.md`, `MEMORY.md`, `AGENTS.md` and `TOOLS.md`. [https://mouse.dev/blog/muse-runtime-export] via jina

**Cron and scheduled work**
- PRIMARY, Meta's own words: the user can open "Upcoming" to see scheduled tasks and reminders. Permissions can be set per scheduled task and per artifact. [https://www.meta.com/help/artificial-intelligence/1385290430137537/] via jina
- PRIMARY: Muse "keeps working after people close the app" and returns when something changes or when it needs approval. [https://about.fb.com/news/2026/09/introducing-muse/] via jina
- PRIMARY (mouse.dev): an hourly background job verifies new memory claims against the source messages. Other jobs maintain relationship pages, review recurring workflows and prepare goal briefings. They write receipts to `workspace/self_improvement/`. The docs cover scheduling. No cron syntax or scheduler internals were published. [https://mouse.dev/blog/muse-runtime-export]

**Sub-agents**
- PRIMARY (mouse.dev): an `agents/` directory held 113 sub-agent records as JSONL traces. Nothing describes how sub-agents are spawned or limited. [https://mouse.dev/blog/muse-runtime-export]

**Dream and memory job**
- PRIMARY (mouse.dev): memory is plain Markdown. `MEMORY.md` is the curated sheet, `memory/YYYY-MM-DD` holds daily logs, and `memory/bank/` is organised into circumstances, experiences and preferences, with line citations back to the source. [https://mouse.dev/blog/muse-runtime-export]
- PRIMARY (mouse.dev): Postgres indexes it. `memory.entries` holds chunks plus line refs. `memory.embeddings` holds 384-dimension vectors. `memory.claims` holds evidence, confidence, status and `supersedes_claim_id`. The agent tools are `memory_search` and `memory_explain`. [https://mouse.dev/blog/muse-runtime-export]
- PRIMARY (mouse.dev): a nightly "dream" reviews recent conversations and writes `~/dreams/<date>`. A separate `ALIGNMENT_SYNTHESIS.md` turns those into standing guidance. The dream files carried `prompt_hoisted: false`, meaning the prose is not injected into the prompt. [https://mouse.dev/blog/muse-runtime-export]
- PRIMARY (mouse.dev): forgetting stages claim IDs for retraction, removes linked material, and rebuilds the index so later jobs don't reconstruct it. The model weights never change. [https://mouse.dev/blog/muse-runtime-export]
- Meta's side: the avatar shows live statuses such as "Is updating memory", and the user can tell Muse to "forget". [https://www.meta.com/help/artificial-intelligence/1385290430137537/] and [https://about.fb.com/news/2026/09/introducing-muse/]

**Mac client and Messages DB**
- SECONDARY (quoting Meta's CTO): Messages access requires macOS Full Disk Access plus the Messages connector toggle in the Muse Mac app. [https://arstechnica.com/security/2026/10/apple-changes-full-disk-access-permissions-to-curb-abuse-from-ai-agents/] via jina
- SECONDARY: columnist Jason Aten says Muse referenced a private Messages thread without his granting access. Apple then announced changes to the Full Disk Access (FDA) permission. Ars notes this contradicts the "both toggles" framing, but Apple named no app. [same Ars article]
- SECONDARY, unverified: an HN commenter who says he reverse-engineered the app claims the path `~/Library/Messages` is hard-coded in the binary, and that FDA alone is what lets it read the DB. [https://news.ycombinator.com/item?id=50027799] via the Algolia API
- SECONDARY: researcher Patrick Wardle's 0-day let any local process change undocumented Muse settings, including the transcription endpoint. That exposed the account token. Meta says it shipped a hotfix. [https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/] via jina
- PRIMARY (mouse.dev): the docs include a guide for a "paired Mac". I could not read the Mac-specific details.

**Approvals**
- PRIMARY, Meta help page:
  - Read versus write actions are distinguished. Muse "learns over time which decisions need your sign off".
  - Connector modes are "Ask for some actions" (asks before every write and important reads) and "Always ask".
  - Web modes are the same two, plus per-site allow-lists.
  - Grant scopes are Allow once, Allow for this task, Allow for this site, Always allow, and Deny.
  - An Activity log, an Upcoming view and System Files let the user audit what happened.
  - Some actions, such as sending an email, are irreversible.
  [https://www.meta.com/help/artificial-intelligence/1385290430137537/]
- PRIMARY: a separate **Sentinel** agent runs on the same VM, isolated at system level. "Nothing Muse does reaches the internet unless the Sentinel approves it". Credentials live in secure storage that Muse can use but not see. [https://about.fb.com/news/2026/09/introducing-muse/]
- PRIMARY (mouse.dev): the Home Link docs describe agent access through a proxy with a separate approval step. [https://mouse.dev/blog/muse-runtime-export]

**Sandboxing and the gadget SDK**
- PRIMARY (code): the Linux gadget runs commands as a separate unprivileged `run_as` account, not the service account that holds the device credentials. The child gets a scrubbed environment, a safe `PATH` and its own process group. It has a default timeout of 120 s and a maximum of 600 s, which kills the whole process group, with output clipped at 96 KiB per stream. File writes are chunked, atomic, and checked against a SHA-256. [https://github.com/facebookincubator/muse-gadget-sdk/blob/fa073045a5572a8e3367fae142c1b7a7c554f222/linux/src/musegadget/executor.py]
- PRIMARY (code): the transport is a Noise XX session over `/v1/noise`. Over it runs a length-prefixed JSON stream on `/link-control` with `link.register`, `link.invoke`, `link.result` and `link.unpaired`. Fresh VM credentials are fetched on a 401 or 403 rather than retrying the same bearer. Logs never contain command parameters or output. [https://github.com/facebookincubator/muse-gadget-sdk/blob/fa073045a5572a8e3367fae142c1b7a7c554f222/linux/AGENTS.md]
- PRIMARY (mouse.dev): `systemd-nspawn` containers are built from `runtime-cell/`. Bubblewrap is used only for ffmpeg, run with no network as `nobody`, with `/input` and `/output` mounts. [https://mouse.dev/blog/muse-runtime-export]
- PRIMARY (mouse.dev): the export, delivered to the user's own Google Drive, contained SSH key files. Meta marked the report "Not Applicable". Whether the keys were live is unestablished. [https://mouse.dev/blog/muse-runtime-export]

**Pricing**
- PRIMARY: "free for most of what people need, with subscription plans for people who want to do more". No prices are stated. [https://about.fb.com/news/2026/09/introducing-muse/]
- Checkout uses Link by Stripe, with a one-time-use card, and the announcement says Shop Pay and 1Password support are "coming soon". [same page]
- SECONDARY (CNBC): Muse can find and cancel recurring subscriptions. This is not Muse's own pricing. [https://www.cnbc.com/2026/09/27/meta-muse-ai-personal-agent.html] via jina
- A later HN search for Muse plan pricing returned nothing relevant.

## Design patterns worth copying into a Rust gateway

1. **Two-tier grant store.**
   - Model `Decision = Once | Task(TaskId) | Site(Host) | Always(ConnectorId) | Deny`.
   - Key grants by `(connector, action_class, scope)` and store them in SQLite.
   - Check the narrowest matching grant first.
   - Persist only the durable scopes.
   - Add a `revoked_at` column so the "Allowed websites" revocation view comes for free.
2. **Policy modes per connector.** `Ask { reads: Important | None, writes: Always }` and `AlwaysAsk`. Classify every tool as `Read`, `ImportantRead` or `Write` in its manifest. Irreversible writes (email, purchases) never accept `Always` unless explicitly overridden. Meta's help page implies this but does not state it.
3. **Sentinel egress proxy.**
   - Keep the agent process unable to open sockets directly. It talks only to a local gateway proxy, which enforces the allow-list and raises approval requests.
   - In Rust, run the agent in a separate uid or network namespace and have it reach a Unix socket only.
   - Keep credentials in the gateway and inject them at the proxy, so the model never sees them.
4. **Approval state machine.** `Pending -> Approved|Denied|Expired`. The approval carries the action hash and the grant scope. Turns that need approval park the task (`Waiting{approval_id}`) and resume on an event. This gives you "comes back when it needs approval".
5. **Privilege split for local execution.** Copy the gadget executor shape:
   - run children as an unprivileged account, with a cleared environment and a fixed `PATH`;
   - use `setsid` and kill the process group on timeout;
   - apply a hard output cap and a timeout ceiling;
   - make file writes chunked and atomic, with a SHA-256 check;
   - log invocation metadata only.
6. **Memory pipeline.**
   - Keep memory as Markdown on disk, indexed by SQLite or Postgres: `claims(id, text, evidence_quote, message_ids, confidence, status, supersedes_claim_id)`.
   - Run an hourly verifier that re-checks each claim against the raw messages.
   - Run a nightly dream that writes `dreams/<date>.md` with `prompt_hoisted=false` by default. Promote only reviewed lines into a standing-guidance file.
   - Treat forget as a staged job: retract claims, delete derived chunks, rebuild the index.
7. **Per-job receipts.** Each background job writes a receipt to `self_improvement/<job>/<ts>.json` (inputs, files touched, outcome). The Activity log is then just a view over receipts plus approvals.
8. **Sub-agent traces.** One JSONL file per sub-agent run under `agents/<id>/trace.jsonl`, with a parent link and a budget (tokens, wall-clock, tool calls).
9. **Mac-client hardening, from the Wardle failure.** Never let local settings choose an endpoint that receives tokens. Pin endpoints at build time. Authenticate the local IPC peer by code-signing identity or peer credentials on the socket. Scope tokens narrowly.
10. **Messages DB access.** Gate it with two independent switches: the OS grant and an app-level connector toggle. Also give it a visible per-use audit entry, because the OS grant alone is all-or-nothing, as the Aten episode showed.
11. **Device pairing.** Use Noise XX with the device token rotating over the session, a separate unprivileged service account, and a device registered as a restricted family so it cannot receive firmware pushes.

## CLAIMS

| CLAIM | RISK | SOURCES | PRIMARY |
|---|---|---|---|
| Muse has about 68 skills, 113 sub-agent traces, and an hourly memory verifier plus nightly dream | Medium: one researcher's export, not independently verified | mouse.dev | Yes (vendor-adjacent, third-party researcher) |
| Dream output is not auto-injected into the prompt (`prompt_hoisted: false`) | Medium: one observed sample | mouse.dev | Yes |
| Muse reads Messages only if Full Disk Access and the connector toggle are both on | High: contested; Apple changed FDA afterwards and Ars says this contradicts Meta | Ars, HN | No, a Meta quote relayed by Ars |
| Approval grants are Once, Task, Site, Always and Deny | Low | Meta help page | Yes |
| A Sentinel agent gates all egress | Low-medium: marketing description, no technical detail | about.fb.com | Yes |
| The 0-day exposed the Muse account token via the transcription endpoint | Low-medium: researcher claim, Meta confirmed a hotfix | Ars | No |
| SSH keys in the export were live or exploitable | Unknown: the author says this is unestablished | mouse.dev | Yes |
| Container escape is possible | Unsupported: the author says he did not demonstrate one | mouse.dev | Yes |
| Muse's Mac client hard-codes `~/Library/Messages` | High: unverified HN comment | HN 50027799 | No |
| Pricing is free for most uses plus subscriptions | Low, but no prices exist | about.fb.com | Yes |

## EXPAND

- Meta's research post "Security and safety for AI agents: our approach with Muse", the Meta help article 1047255454427887, and the Tailscale post. The jina route rate-limited them and I did not get them. They likely cover Sentinel and the tailnet integration.
- `muse.ai` pricing and Mac-app pages, plus the Messages connector help page. I did not search for them.
- Wayback and `old.reddit.com` were not needed but remain for the pages above.
- The rest of the gadget SDK. I read only the README, the Linux `executor.py` and the Linux `AGENTS.md`. The `esp32/` Noise and pairing code and `skills/CATALOG.md` were not read.
- Sub-agent spawning and cron internals are undocumented in everything I could reach.