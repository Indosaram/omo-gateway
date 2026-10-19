<!-- task=st_01a14505 -->
st_01a14505 [completed] model mahoquot/claude-sonnet-5-5
Ask: research sandbox_infra (E2B, Daytona, Browserbase/Stagehand, Steel, Anchor, Scrapybara, Cua, Apple container, Tart) and report. Wanted: a sourced report of at most 1800 words. For you: the report is done. All items were checked against primary sources, with a few gaps noted under CLAIMS and EXPAND. Now: none. Next: none.

## Identity

Queried 2026-10-16. Confidence is high for each entity unless noted.

| Entity | Company / owner | Official URL | Notes |
|---|---|---|---|
| E2B | FoundryLabs, Inc. (SF), founded 2023 | e2b.dev | Apache-2.0; repo created 2023-03 |
| Daytona | Daytona (company name not confirmed) | daytona.io | Repo created 2024-02; license not exposed by the API |
| Browserbase + Stagehand | Browserbase | browserbase.com, stagehand.dev | Stagehand is MIT, repo created 2024-03 |
| Steel | Steel (steel-dev) | steel.dev | `steel-browser` is Apache-2.0, repo created 2024-11 |
| Anchor Browser | Anchor | anchorbrowser.io | Closed cloud, no repo checked; medium confidence on company details |
| Scrapybara | Scrapybara | scrapybara.com | Now a pivot to "Capy" (capy.ai); the product is effectively gone |
| Cua | Cua AI, Inc. | cua.ai, github.com/trycua/cua | MIT core, repo created 2025-01-31 |
| Tart | Cirrus Labs, now OpenAI | github.com/openai/tart, tart.run | Repo created 2022-02; moved from `cirruslabs/tart` |
| Apple `container` | Apple | github.com/apple/container | Apache-2.0, repo created 2025-05-30 |

Launch dates are the repo creation dates. No source gave a product launch date.

## Findings

**Hosted agent sandboxes (E2B, Daytona)**
- E2B bills per second: $0.000014 per vCPU-second and $0.0000045 per GiB-second, with storage free. The default sandbox is 2 vCPU and 4 GiB, and the maximum is 8 and 8. [https://e2b.dev/pricing.md]
- E2B plans: Hobby is $0 with a one-time $100 credit, 20 concurrent sandboxes and 1-hour sessions. Pro is $150/month with 100 concurrent sandboxes and 24-hour sessions. Enterprise has a $3,000/month minimum and BYOC. [https://e2b.dev/pricing.md]
- E2B is CPU-only and Linux-oriented. Its schema.org markup lists `operatingSystem: Cloud, Linux`, and I saw no macOS option. [https://e2b.dev/pricing]
- E2B pause/resume saves filesystem and memory, including running processes. A pause takes about 4 seconds per GiB of RAM, and a resume takes about 1 second. [https://docs.e2b.dev/sandbox/persistence.md]
- E2B keeps paused sandboxes indefinitely with no TTL. Auto-pause is opt-in through `lifecycle.onTimeout: 'pause'`, and the default is `kill`. [https://docs.e2b.dev/sandbox/persistence.md]
- E2B also offers a filesystem-only snapshot mode that reboots on resume, plus `createSnapshot`. [https://docs.e2b.dev/sandbox/persistence.md]
- Daytona has Linux containers (under 90ms start), Linux VMs, Windows VMs, GPU and macOS sandbox classes. [https://www.daytona.io/docs/en/sandboxes.md]
- Daytona's VM classes support fork, pause/resume and memory ("hot") snapshots. Container snapshots are filesystem-only. [https://www.daytona.io/docs/en/snapshots.md]
- Daytona's macOS sandboxes are not first-party. The docs route them to the third-party use.computer platform (Computer Use, VNC, SSH). [https://www.daytona.io/docs/en/sandboxes.md]
- Daytona billing: a paused VM bills disk only, and the preserved memory is not billed. A stopped sandbox also bills disk only. [https://www.daytona.io/docs/en/billing.md]
- Daytona's default snapshot image ships Claude Code, Codex, opencode and openclaw preinstalled. [https://www.daytona.io/docs/en/snapshots.md]
- Daytona's docs say its Amazon ECR setup also covers a "Self-hosted" case. I did not read the self-host docs, so the self-host story is unverified. [https://www.daytona.io/docs/en/snapshots.md]

**Browser-only infrastructure**
- Browserbase describes itself as a "Browser Agent Platform": browsers, Fetch, Search, Agent Identity, Functions and a Model Gateway. [https://docs.browserbase.com/llms.txt]
- Browserbase plans: Free, Developer $20, Startup $99 (100 concurrent, 500 browser hours) and a custom Scale plan. [https://docs.browserbase.com/account/billing/plans.md]
- Browserbase session limits: Free allows 15 minutes and paid plans allow 6 hours. Overage is $0.12/hr on Developer and $0.10/hr on Startup. [https://docs.browserbase.com/account/billing/plans.md]
- Browserbase has persistent Contexts and keep-alive sessions. It also documents "Pausing and resuming runs", where an agent waits for human input. [https://docs.browserbase.com/llms.txt]
- Browserbase documents OpenClaw and Hermes Agent integrations. [https://docs.browserbase.com/llms.txt]
- Stagehand (MIT) is Browserbase's SDK. Steel's integrations page also lists "Run Stagehand on Steel", so Stagehand is not locked to Browserbase. [https://github.com/browserbase/stagehand] [https://docs.steel.dev/llms.txt]
- Steel pricing: Launch is $0 with $30 free credit. Scale is $250 with $100/month in credits. Browser hours are $0.10 on Launch and $0.08 on Scale. [https://docs.steel.dev/overview/pricinglimits.md]
- Steel session length is 15 minutes on Launch, 1 hour on Scale and up to 24 hours on Enterprise. Profiles persist auth and cookies. [https://docs.steel.dev/overview/pricinglimits.md]
- Steel self-hosts via Docker, Railway and a clustering guide. `steel-browser` is Apache-2.0. [https://docs.steel.dev/llms.txt] [https://github.com/steel-dev/steel-browser]
- Steel has a "Steel Local vs Steel Cloud" page. I did not read it, so how much the local version lacks is unconfirmed. [https://docs.steel.dev/llms.txt]
- Anchor Browser is a cloud-only browser with CDP and REST access. It has persistent volumes and profiles, managed authentication (identities with email MFA), live view, recordings and webhooks. [https://docs.anchorbrowser.io/llms.txt]
- Anchor adds OS-level control endpoints (mouse, keyboard, clipboard). It lists third-party sandbox integrations: CreateOS, Daytona and Tenki. [https://docs.anchorbrowser.io/llms.txt]
- Anchor's open-source MCP server is self-hostable, but the browsers themselves are not. [https://docs.anchorbrowser.io/llms.txt]
- Anchor's agent-onboarding docs say an agent can get an API key by solving a puzzle, with no dashboard account. [https://docs.anchorbrowser.io/llms.txt]

**Scrapybara**
- Scrapybara's homepage now reads "Scrapybara built computers for agents. Now we're building Capy, the AI software engineer." Treat the product as discontinued or pivoted. This is my inference from the wording; I saw no shutdown notice. [https://scrapybara.com]
- Its last public playground repo push was 2025-03-11. [https://github.com/Scrapybara/scrapybara-playground]

**Mac-local and macOS options**
- Cua is the strongest macOS-native option. Cua Spaces is a desktop app that gives agents full desktops: macOS VMs built locally, plus Linux/Omarchy images. [https://github.com/trycua/cua]
- Cua Spaces runs on your Mac, on other machines you own, or in your own AWS, GCP or Modal account. It is free for individuals, and Pro and Teams plans are "coming soon". [https://github.com/trycua/cua]
- Cua "Teleport" moves a signed-in app such as Chrome or Slack into a Space, with sessions held in an encrypted on-Mac Keyvault and released only after approval. [https://github.com/trycua/cua]
- Cua can host Spaces on a spare Mac via relay, Tailscale or SSH. Its `cua-spacesd` guest daemon exposes processes, files, screenshots and input, and the SDKs cover Python, TypeScript, Swift and Kotlin with a Rust core. [https://github.com/trycua/cua]
- Cua licensing: the SDK, Cua Driver and Lume are MIT. Cua Spaces and `cua-spacesd` are FSL-1.1-MIT, which forbids offering them as a competing hosted service and converts to MIT after two years. [https://github.com/trycua/cua]
- Cua is very active: 29.9k stars, and releases on 2026-10-16. [https://github.com/trycua/cua]
- Cua Driver runs apps in the background without moving the user's pointer or taking focus, "when the app and platform support it". [https://github.com/trycua/cua]
- Tart runs macOS and Linux VMs on Apple Silicon via Apple's `Virtualization.Framework`. It pushes and pulls VM images to any OCI registry. [https://github.com/openai/tart]
- Tart's last release in the API output was 2.41.1 on 2026-10-15. [https://github.com/openai/tart]
- Cirrus Labs announced on 2026-04-07 that it is joining OpenAI's Agent Infrastructure team. It said it would relicense Tart, Vetu and Orchard more permissively and stop charging licence fees. [https://cirruslabs.org/]
- The Tart `LICENSE` file in the repo is FSL-1.1-ALv2 (copyright OpenAI, 2022-2026). It allows internal use but forbids competing commercial offerings, and it converts to Apache-2.0 after two years. This is not the "more permissive" change that was promised. [https://raw.githubusercontent.com/openai/tart/main/LICENSE]
- Tart's README example still pulls `ghcr.io/cirruslabs/macos-tahoe-base:latest` and tells users to `brew install openai/tools/tart`. [https://github.com/openai/tart]
- Apple `container` runs Linux containers as lightweight VMs on Apple Silicon. It needs macOS 26, uses OCI images and is Apache-2.0 with 50.7k stars. [https://github.com/apple/container]
- Apple `container` is Linux-only. It cannot give you a macOS guest, so use Tart or Lume for that. [https://github.com/apple/container]

## Implications for a self-hosted personal agent gateway

1. **Make the sandbox backend a trait with a per-lane policy.** One interface (`create`, `exec`, `snapshot`, `pause`, `resume`, `kill`) would let you swap backends. Candidates are a local Tart or Lume macOS VM, Apple `container` for Linux work, and an optional remote E2B or Daytona backend. A coding lane would pick Linux/`container`, and a "use-my-Mac-apps" lane would pick a macOS VM.
2. **Run omo coding agents inside disposable Linux VMs by default.** Apple `container` is Apache-2.0 and runs OCI images in a lightweight VM per container. Fork a fresh one per task, then diff and merge the result back. This keeps the owner's Mac files off-limits, which matters for an unattended agent.
3. **Keep a persistent macOS "agent desktop" for GUI tasks.** Use a Tart or Lume VM, and drive it with Cua Driver or `cua-spacesd` instead of building your own input layer. Cua is MIT outside Spaces, but its Spaces layer is FSL and cannot be used to offer a competing hosted service, which a personal gateway would not do. Tart's licence also allows internal use. Both fit a single-owner install.
4. **Add snapshot-and-resume as a lane primitive.** E2B pauses with memory in about 1 second on resume. Daytona's VM hot snapshots do the same. A local equivalent is a Tart VM snapshot. Wire cron and inbox-ingestion lanes to resume a warm workspace, which avoids paying cold-start cost for every iMessage or KakaoTalk trigger.
5. **Use a self-hosted browser for web work, with sticky identity.** Steel (Apache-2.0, Docker) or Stagehand on a local Chrome gives the agent a browser with persisted profiles. Browserbase and Anchor are cloud-only, so they only fit if you accept third-party session data. Store cookies per lane in the Rust daemon's keychain, in the style of Cua Keyvault.
6. **Add human-in-the-loop handoff over the owner's chat channels.** Browserbase documents pause-and-resume runs and Anchor documents human-intervention webhooks. Map "agent is blocked (2FA, CAPTCHA, approval)" to a Discord or iMessage prompt, with a live-view link or screenshot. Resume the run on the owner's reply.
7. **Add per-lane quotas and TTLs.** Hosted sandboxes cap session length (E2B 1 hour on Hobby and 24 on Pro; Steel 15 minutes to 24 hours) and bill per second. Mirror this locally with max wall-clock, idle auto-pause and a hard kill. Add an explicit "never auto-delete paused workspaces" option, as E2B does, so long-running projects survive.
8. **Pin and mirror VM images from your own registry.** Tart pulls from any OCI registry, and its ownership and licence have just changed. Keep a local copy of your base images and do not depend on `cirruslabs/*` tags staying put.

## CLAIMS
- CLAIM: E2B bills about $0.000014/vCPU-s and $0.0000045/GiB-s, storage is free, and the Pro plan is $150/month with 24-hour sessions — RISK: normal — SOURCES: e2b.dev — PRIMARY: https://e2b.dev/pricing.md
- CLAIM: E2B pause preserves memory and processes, resumes in about 1 second, and paused sandboxes never expire — RISK: normal — SOURCES: docs.e2b.dev — PRIMARY: https://docs.e2b.dev/sandbox/persistence.md
- CLAIM: Daytona's macOS sandboxes are delivered through the third-party use.computer service, not natively — RISK: high — SOURCES: daytona.io — PRIMARY: https://www.daytona.io/docs/en/sandboxes.md
- CLAIM: Daytona VM sandboxes support fork, pause/resume and memory snapshots, and a paused VM is billed for disk only — RISK: normal — SOURCES: daytona.io — PRIMARY: https://www.daytona.io/docs/en/billing.md
- CLAIM: Steel Scale is $250/month with $0.08 browser hours and 1-hour sessions, and Launch has 15-minute sessions — RISK: normal — SOURCES: docs.steel.dev — PRIMARY: https://docs.steel.dev/overview/pricinglimits.md
- CLAIM: Browserbase Developer is $20, Startup is $99, and sessions run up to 6 hours on paid plans — RISK: normal — SOURCES: docs.browserbase.com — PRIMARY: https://docs.browserbase.com/account/billing/plans.md
- CLAIM: Tart is now an OpenAI-owned repo under an FSL-1.1-ALv2 licence, despite the "more permissive" relicensing promise — RISK: high — SOURCES: github.com, cirruslabs.org — PRIMARY: https://raw.githubusercontent.com/openai/tart/main/LICENSE
- CLAIM: Cirrus Labs agreed to join OpenAI on 2026-04-07, and Cirrus CI shut down on 2026-06-01 — RISK: normal — SOURCES: cirruslabs.org — PRIMARY: https://cirruslabs.org/
- CLAIM: Cua Spaces gives agents local macOS VMs on a Mac, is source-available (FSL-1.1-MIT) and is free for individuals — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/trycua/cua
- CLAIM: Scrapybara has pivoted to Capy, an AI software engineer, and is no longer a computer-for-agents product — RISK: normal — SOURCES: scrapybara.com — PRIMARY: https://scrapybara.com
- CLAIM: Apple `container` runs Linux containers only, in lightweight VMs on Apple Silicon, and requires macOS 26 — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/apple/container

## EXPAND
- LEAD: Daytona self-hosting (open-source status, licence, a runner on a Mac) — WHY: the GitHub API returned no licence and I did not read the self-host docs — ANGLE: `gh api repos/daytonaio/daytona/contents/LICENSE`, plus the Daytona docs pages on regions, "custom regions" and OSS deployment.
- LEAD: Daytona version discrepancy (docs show v0.223, but `gh releases` lists v0.190.0 as the newest) — WHY: this may be a different release channel — ANGLE: check the `daytonaio/daytona` tags and changelog at daytona.io/changelog.
- LEAD: Cua Spaces/Lume resource limits and macOS licensing (Apple's two-VM cap per Mac) — WHY: this limits how many macOS agent desktops one host can run — ANGLE: cua.ai/docs/lume and the Apple Virtualization EULA.
- LEAD: Anchor Browser pricing page and company identity — WHY: I read the docs index but not `docs.anchorbrowser.io/pricing.md` — ANGLE: fetch that page and search for funding news.
- LEAD: Steel Local vs Steel Cloud feature gap — WHY: it decides whether a fully local deployment is viable — ANGLE: `docs.steel.dev/overview/self-hosting/steel-local-vs-steel-cloud.md`.
- LEAD: web_search and x_search were not available, and HN search returned nothing useful for Tart or OpenAI. Community sentiment, security incidents (Cua's deprecated `cuabot` carries a security warning) and recent news are not covered — WHY: the brief asked for date-bounded social signal — ANGLE: Reddit `.json`, HN Algolia queries for "Cua", "E2B" and "Daytona" after 2026-06.