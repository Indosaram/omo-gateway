# AI Agent Kanban / Task Board Survey: Korean, Japanese & Chinese Sources Sweep

Seen: Trello, Jira, Linear, Notion, Asana, ClickUp, monday, Plane, WeKan, Kanboard, Planka, Vikunja, Focalboard, Taiga, Huly, Vibe Kanban, Task Master, Backlog.md.

---

### KanVibe
- URL: https://github.com/rookedsysc/kanvibe
- Kind: agent-native
- What: A keyboard-first desktop Kanban workspace for AI coding agents that automatically tracks task states through CLI hooks and embeds xterm.js terminals per card.
- Agent integration: Claude Code, Gemini CLI, OpenAI Codex CLI, and OpenCode hooks; installs scripts into `.claude/hooks/` to auto-transition cards on prompt sent, AI question asked, user answer, and completion; automatic git worktree and tmux/zellij session management.
- Mobile: responsive web (accessible through mobile web browsers for board monitoring, but embedded terminal operations require desktop keyboard; cite: `docs/README.ko.md`).
- Pricing: free (100% open-source MIT/AGPL model; optional donations via Buy Me a Coffee; no paid tier as of 2026-10-16).
- Self-host: yes via `bash start.sh` or Docker / Docker Compose (Next.js 16 + React 19 + PostgreSQL + xterm.js + WebSocket; no enterprise restrictions).
- License: AGPL-3.0
- Activity: 147 GitHub stars, 20 forks, last release v1.4.0 on 2026-08-31, active commits through 2026-10-15; active.
- Pros: Automatic status movement triggered by Claude Code lifecycle hooks without manual drag-and-drop; Embedded web terminal eliminates tmux window switching; Automatic git worktree creation and cleanup upon completion; Real-time AI quota meter for Claude, Codex, and Gemini.
- Cons: Mobile interaction is constrained by terminal keyboard requirements; Requires self-hosted PostgreSQL database setup; Tight dependency on local terminal multiplexers (tmux/zellij).
- Sources: https://news.hada.io/topic?id=26737, https://github.com/rookedsysc/kanvibe, https://github.com/rookedsysc/kanvibe/releases
- Confidence: high

---

### Hermes KanbanWebUI
- URL: https://github.com/PriuS2/HermesKanban
- Kind: agent-native
- What: A standalone, Trello-style WebUI for the Hermes Agent Kanban SQLite database featuring a visual AI Workflow Designer to decompose complex goals into DAG task plans.
- Agent integration: Hermes Agent CLI (`hermes_cli.kanban_db`), direct shared SQLite database access, AI Workflow Designer mapping requirements to agent profiles; supports all LLM backends configured in Hermes Agent.
- Mobile: responsive web (modern responsive CSS/flexbox UI accessible from mobile browsers; no native iOS/Android binary; cite: `README.md`).
- Pricing: free (open-source Apache-2.0, no subscription or paid tiers as of 2026-10-16).
- Self-host: yes via local Python/Node server reading the local `hermes_cli.kanban_db` SQLite database directly (no cloud account required).
- License: Apache-2.0
- Activity: 39 GitHub stars, 11 forks, created 2026-05-04, active commits through 2026-10-15; active.
- Pros: Shares existing Hermes Agent SQLite database without creating redundant task schemas; Visual AI Workflow Designer generates previewable DAG task graphs before execution; Visual parent-child dependency line rendering across multi-step agent workflows.
- Cons: Strictly dependent on the Hermes Agent CLI ecosystem; Lacks multi-tenant enterprise access control in OSS; No standalone native mobile application.
- Sources: https://news.hada.io/topic?id=29268, https://github.com/PriuS2/HermesKanban, https://zenn.dev/zennai_ryutaro/articles/20260525-hermes-kanban-swarm-usage
- Confidence: high

---

### AIF Handoff
- URL: https://aif.cutcode.dev
- Kind: agent-native
- What: An autonomous Kanban board where AI agents plan, implement, and review software tasks hands-off with convergence-aware auto-review rework loops and explicit human handoff gates.
- Agent integration: Built on AI Factory framework (`@aif/runtime`) with native adapters for Claude Code, OpenAI Codex, and Cursor CLI; automated multi-stage subagent orchestration across Backlog, Planning, Plan Ready, Implementing, Review, and Done.
- Mobile: responsive web (responsive single-page web app accessible via mobile browser; no native app; cite: `https://aif.cutcode.dev`).
- Pricing: free (open-source MIT, no paid tier as of 2026-10-16).
- Self-host: yes via Node.js/TypeScript self-hostable service or Docker container (local SQLite/Postgres storage).
- License: MIT
- Activity: 308 GitHub stars, 45 forks, active commits through 2026-10-15; active.
- Pros: Convergence-aware review strategies (`full_re_review` vs `closure_first`) preventing endless automated review cycles; Explicit human handoff gate (`Done + manual review required`) when AI consensus fails; Clean separation of specialized subagent roles (planning vs implementation vs review).
- Cons: Deep coupling with the broader AI Factory ecosystem; Relatively small community compared to mainstream task trackers; Requires pre-configured local CLI agent credentials.
- Sources: https://github.com/lee-to/aif-handoff, https://aif.cutcode.dev
- Confidence: high

---

### Dooray! Project
- URL: https://dooray.com
- Kind: general-kanban-with-agent-integration
- What: Korea's leading enterprise all-in-one collaboration suite featuring Project Kanban boards integrated with Dooray AI for automated task summarization, drafting, and agent orchestration.
- Agent integration: Dooray AI Assistant, Dooray Open API (REST), Webhooks, and Dooray Bot framework; connects external AI agents and LLM automation pipelines to Kanban cards and comments.
- Mobile: native iOS app (Apple App Store: Dooray!), native Android app (Google Play: Dooray!), and responsive web (cite: `https://dooray.com/main/mobile`).
- Pricing: free tier (up to 25 users with basic collaboration); Business tier from 2,000 KRW (~$1.50) per user/month; Dooray AI add-on pricing per seat (as of 2026-10-16).
- Self-host: yes for enterprise (Private Cloud / On-Premises CSAP-certified deployment for public institutions and financial firms; standard version is SaaS).
- License: proprietary
- Activity: Active enterprise production platform operated by NHN Dooray, continuously updated weekly in 2026.
- Pros: Mature enterprise-grade UX with full Korean language support; Native iOS and Android mobile apps with push notifications and drag-and-drop board manipulation; Comprehensive suite integrating Kanban boards with email, chat, calendar, and wiki; Certified security compliance (CSAP).
- Cons: Commercial proprietary software; AI capabilities primarily focus on drafting and summarization rather than autonomous coding agents in git worktrees; Advanced automation requires workspace administrator tokens.
- Sources: https://dooray.com, https://inside.nhn.com/news/935, https://dooray.com/main/mobile
- Confidence: high

---

### Multica
- URL: https://multica.ai
- Kind: agent-native
- What: A self-hostable Linear-style mission control kanban board where human engineers and 26+ AI coding agent CLIs collaborate in one mixed team across isolated git worktrees.
- Agent integration: 26+ agent CLIs supported through a unified Go `Backend` interface (Claude Code, OpenAI Codex, Cursor, Kimi, Qoder, Oh-my-pi, CodeArts, Trae, Zeroclaw, etc.); WebSocket daemon, tool-use streaming, ACP protocol families, git branch delivery.
- Mobile: native iOS app and native Android app under `apps/mobile` (React Native / Expo codebase) + responsive web (cite: `apps/mobile` directory in GitHub repository, Juejin post #7686331923204292623).
- Pricing: free for self-hosted internal organizational use; commercial managed cloud and enterprise license available upon inquiry (as of 2026-10-16).
- Self-host: yes via Docker Compose or Helm (Go backend + PostgreSQL 17 + Next.js 16 frontend + local execution daemon; commercial hosting for third parties restricted).
- License: Apache 2.0 with custom commercial/hosting restrictions (Part I: commercial hosting prohibited without license, brand-name lock on UI; non-OSI source-available).
- Activity: 52,945 GitHub stars, 6,491 forks, v0.4.44 released mid-October 2026, commits on 2026-10-16; exceptionally active.
- Pros: Massive multi-agent CLI adapter suite (26+ CLIs unified under one protocol abstraction); Strict git worktree isolation ensuring zero dirty workspace pollution; Production-hardened execution timeouts, token accounting, and session recovery taxonomy; Cross-platform support including desktop Electron and native mobile apps.
- Cons: Restrictive license terms on third-party commercial hosting and branding; High architectural complexity (PostgreSQL 17, Go daemon, WebSocket hub); Rapid breaking changes across 0.4.x releases.
- Sources: https://juejin.cn/post/7686331923204292623, https://juejin.cn/post/7662325201879318562, https://github.com/multica-ai/multica, https://multica.ai
- Confidence: high

---

### Star-Office-UI
- URL: https://github.com/ringhyacinth/Star-Office-UI
- Kind: agent-native
- What: A pixel-art AI office kanban and status visualization board that maps AI agent work states (idle, writing, researching, executing, syncing, error) to interactive visual office zones.
- Agent integration: OpenClaw native skill integration (`SKILL.md`), REST API (`/set_state`, `/agent-push`), Python push scripts (`office-agent-push.py`), multi-agent join keys, Gemini API for AI background generation.
- Mobile: responsive web (mobile browser optimized viewport for quick glance status checking) + Tauri transparent desktop pet mode (cite: `README.md`, `desktop-pet/README.md`, Juejin #7613971395929112603).
- Pricing: free (100% open-source MIT code license, as of 2026-10-16).
- Self-host: yes via lightweight Flask Python backend + Phaser 2D HTML/JS frontend; optional Tauri desktop wrapper.
- License: MIT (code/logic; bundled pixel art assets restricted to non-commercial learning/demo use).
- Activity: 7,522 GitHub stars, 317 forks, v1.0.0, commits on 2026-10-16; very active.
- Pros: Uniquely delightful pixel-art visual representation eliminating the cognitive burden of reading text logs; Multi-agent team status with visitor join-key mechanism; Trilingual support (Chinese, English, Japanese); "Yesterday's memo" card auto-extracted from daily agent memories.
- Cons: Visual status display rather than full code-review or merge tool; Art assets require replacement for commercial deployment; Lacks deep git worktree manipulation.
- Sources: https://juejin.cn/post/7613971395929112603, https://github.com/ringhyacinth/Star-Office-UI
- Confidence: high

---

### Agent Kanban (agent-kanban.dev)
- URL: https://agent-kanban.dev
- Kind: agent-native
- What: A mission-control task board for autonomous software agents where agents operate boards and tasks through OpenAPI and Realmroot Toolbox CLI tools, with verified execution provenance and human review gates.
- Agent integration: Live OpenAPI contract, Realmroot Toolbox CLI, Agent Skills (`ak-worker`, `ak-maintainer`, `ak-task`, `ak-plan`), Enbor runtime session binding, GitHub App webhook integration.
- Mobile: responsive web SPA (React + Vite running on Cloudflare Workers; no dedicated native mobile app; cite: `README.md`).
- Pricing: free & open-source (self-hostable on Cloudflare Workers/D1, as of 2026-10-16).
- Self-host: yes, deployed to Cloudflare Workers with Cloudflare D1 database and Wrangler CLI; local dev server via `pnpm dev`.
- License: Functional Source License 1.1 (FSL-1.1-ALv2, converts to Apache 2.0 after 2 years).
- Activity: 492 GitHub stars, 44 forks, commits through 2026-10-14; active.
- Pros: Serverless Cloudflare Worker + D1 architecture with ultra-low latency and minimal ops overhead; Rigorous task lifecycle and dependency-aware planning rejecting cycles and cross-tenant conflicts; Verified execution provenance binding tasks to signed Enbor runtime sessions; Enforced human review gates preventing agents from self-approving work.
- Cons: Requires Cloudflare/Wrangler ecosystem and Realmroot/Enbor credentials for full multi-agent features; FSL license is source-available rather than pure OSI open source initially; Less focus on raw local terminal execution.
- Sources: https://agent-kanban.dev, https://github.com/saltbo/agent-kanban
- Confidence: high

---

### Kander
- URL: https://github.com/dualface/kander
- Kind: agent-native
- What: A strict rule-driven multi-agent parallel development kanban board with built-in two-stage independent review gates (PMQA) and conflict-aware scheduling across isolated git worktrees.
- Agent integration: Multi-agent CLI orchestration (Claude Code, OpenAI Codex, etc.), automated reviewer agents, conflict-aware static analysis scheduling, git worktree isolation, and automated branch merging.
- Mobile: responsive web (web interface accessible from mobile browsers, no native mobile app; cite: `README.md`).
- Pricing: free & open-source (MIT, as of 2026-10-16).
- Self-host: yes via single Go binary or local source build, backed by SQLite (no enterprise gating).
- License: MIT
- Activity: 148 GitHub stars, 13 forks, v0.9.7 released on 2026-10-15, commits on 2026-10-16; very active.
- Pros: Battle-tested in real production (1,726 tasks completed across 5 projects in QuickTUI); Physical isolation between implementation agents and independent reviewer agents; PMQA review catches semantic and state-machine bugs that false-green unit tests miss; Static analysis conflict detection preventing parallel branch collisions.
- Cons: Strict rule-driven workflow can feel rigid for exploratory hacking; Relatively early release stage (v0.9.x); Documentation primarily in Chinese and English with emerging Japanese translations.
- Sources: https://github.com/dualface/kander, https://github.com/dualface/kander/releases, https://quicktui.ai/
- Confidence: high

---

### Nerve (OpenClaw Nerve)
- URL: https://nerve.zone
- Kind: agent-native
- What: A real-time web cockpit for OpenClaw featuring an automated agent Kanban board, voice conversations, sub-agent session tree, workspace control, and usage analytics.
- Agent integration: Deep native integration with OpenClaw agent daemon; real-time WebSocket event streaming, tool call monitoring, voice-to-agent commands, automated task creation and state transitions.
- Mobile: responsive web with explicit mobile UI layout (mobile screenshots documented in README; PWA installable; no native App Store binary; cite: `README.md`).
- Pricing: free & open-source (MIT, as of 2026-10-16).
- Self-host: yes via 60-second one-line installer (`curl -fsSL https://.../install.sh | bash`), Node.js/TypeScript backend.
- License: MIT
- Activity: 867 GitHub stars, 148 forks, commits through 2026-10-13; active.
- Pros: Full multimodal control room combining Kanban board with voice interaction and file explorer; First-class mobile web interface designed for on-the-go monitoring; One-click automated installer getting users running in 60 seconds; Granular sub-agent session hierarchy visualization.
- Cons: Tied primarily to the OpenClaw agent ecosystem; Heavy feature surface requiring more resources than minimal kanban boards; Voice feature requires browser audio permissions and model API support.
- Sources: https://nerve.zone, https://github.com/daggerhashimoto/openclaw-nerve
- Confidence: high

---

### GitHub Kanban MCP Server (Sunwood AI Labs)
- URL: https://github.com/Sunwood-ai-labs/github-kanban-mcp-server
- Kind: general-kanban-with-agent-integration
- What: A Model Context Protocol (MCP) server developed in Japan that enables LLM agents to manage GitHub issues and Projects as a Kanban board for automated task tracking and execution.
- Agent integration: Official MCP server protocol; integrates with Claude Desktop, Cursor, Cline, Roo Code, Claude Code; exposes tools for issue creation, label management, column transitions, and commenting.
- Mobile: none (runs as a local background Node.js MCP server process; client mobile status depends on MCP host client; cite: `README.md`).
- Pricing: free & open-source (MIT, as of 2026-10-16).
- Self-host: yes via local npm/Node.js package installed via `npm install` or `npx`.
- License: MIT
- Activity: 10 GitHub stars, created by prominent Japanese AI researcher Maki / Sunwood AI Labs; active.
- Pros: Plugs directly into standard GitHub Projects/Issues without requiring a separate web UI or database; Standard MCP compliance allowing instant plug-and-play in Cursor, Claude Desktop, and Claude Code; Light footprint with zero external infrastructure needs beyond GitHub PAT.
- Cons: Lacks its own visual dashboard (relies on GitHub's native web UI for human viewing); Limited to GitHub issue capabilities; Single-developer project with modest community size.
- Sources: https://github.com/Sunwood-ai-labs/github-kanban-mcp-server, https://www.pulsemcp.com/servers/sunwood-ai-labs-github-kanban
- Confidence: high

---

### KanBots
- URL: https://kanbots.dev
- Kind: agent-native
- What: A local-first kanban board desktop application that runs 11 agent CLIs in parallel in isolated git worktrees with autopilot personas, live tool streaming, and interactive decision prompts.
- Agent integration: Claude Code, OpenAI Codex, Gemini CLI, Cursor, Copilot, Amp, OpenCode, Droid, CCR, Qwen, and any ACP-compatible CLI; `kanbots-mcp-server` for MCP client integration; Sentry error import.
- Mobile: none (Electron desktop application for macOS, Windows, and Linux; no mobile client; cite: GeekNews #29830, `README.md`).
- Pricing: free & open-source for local desktop app (MIT); Cloud multi-user tier at $19/seat/month (or $190/seat/year, as of 2026-10-16).
- Self-host: yes via local Electron desktop app (.dmg, .exe, .AppImage) with local SQLite storage; Cloud tier is hosted.
- License: MIT (Desktop OSS) / Proprietary (Cloud features).
- Activity: 615 GitHub stars, v1.2.0 released on 2026-05-25, commits through 2026-10-15; active.
- Pros: Broad multi-agent CLI support (11 CLIs) running simultaneously across 4 parallel autopilot slots; Human-in-the-loop decision prompts stopping agents at critical forks for user guidance; Local-first privacy with SQLite storage in `.kanbots/` and zero telemetry in OSS; Built-in branch preview starting local dev servers in one click.
- Cons: Cloud collaboration features locked behind $19/user/month subscription; No mobile application or responsive web interface for the OSS edition; Requires local installation of respective agent CLIs.
- Sources: https://news.hada.io/topic?id=29830, https://github.com/leodavinci1/kanbots, https://kanbots.dev
- Confidence: high

---

### KaibanJS
- URL: https://www.kaibanjs.com/
- Kind: agent-native
- What: A JavaScript/TypeScript framework for multi-agent systems featuring a built-in "Kaiban" (Kanban) visual board to orchestrate, inspect, and monitor AI agent workflows in real time.
- Agent integration: LangChainJS integration, Node.js/React/Next.js native integration; supports OpenAI, Anthropic Claude, Gemini, and local LLMs; agents and tasks defined programmatically with real-time status dispatch.
- Mobile: responsive web (Kanban board UI embeds into responsive React/Next.js web apps; accessible on mobile browsers; cite: `https://www.kaibanjs.com/`).
- Pricing: free & open-source (MIT, as of 2026-10-16).
- Self-host: yes via npm package (`kaibanjs`) integrated directly into any Node.js or browser frontend application.
- License: MIT
- Activity: 1,480 GitHub stars, active npm package, commits through 2026-10-15; active.
- Pros: Developer-friendly JavaScript/TypeScript native library with zero Python dependency; Built-in Kanban component ("Kaiban Board") that can be embedded directly into custom web applications; Redux-inspired deterministic state management for multi-agent workflows; Live observability of token usage, operational costs, and agent logs.
- Cons: Focused on programmatic multi-agent orchestration rather than managing existing standalone CLI tools like Claude Code or Codex; Requires coding in JavaScript to define agents and workflows; Less suited for non-technical project managers.
- Sources: https://news.hada.io/topic?id=17823, https://www.kaibanjs.com/, https://github.com/kaiban-ai/kaibanjs
- Confidence: high

---

### Claw-Kanban
- URL: https://github.com/GreenSheep01201/Claw-Kanban
- Kind: agent-native
- What: An agent orchestration kanban board designed to route tasks across Claude Code, Codex CLI, Gemini CLI, OpenCode, Copilot, and Antigravity with role-based auto-assignment and real-time monitoring.
- Agent integration: Direct CLI integration for 6 major agent CLIs; role-based automatic agent dispatch, WebSocket real-time progress streaming, AI prompt one-line install capability.
- Mobile: responsive web (Next.js / Tailwind CSS web application accessible on mobile browsers; no native app; cite: `README.md`).
- Pricing: free & open-source (Apache-2.0, as of 2026-10-16).
- Self-host: yes via Node.js 22+ local web application via `npm run dev` or Docker.
- License: Apache-2.0
- Activity: 72 GitHub stars, version 1.0.3, commits through 2026-10-09; active.
- Pros: Straightforward role-based task routing across multiple competing agent CLIs; Multilingual documentation including dedicated Korean (`README.ko.md`) and English guides; Quick installation designed to be handed directly to an AI agent prompt.
- Cons: Smaller open-source community footprint; Relies on local CLI authentication; Limited advanced dependency graph visualizers.
- Sources: https://github.com/GreenSheep01201/Claw-Kanban, https://github.com/GreenSheep01201/Claw-Kanban/blob/main/README.ko.md
- Confidence: high

---

### Roadmap Skill
- URL: https://www.npmjs.com/package/roadmap-skill
- Kind: agent-native
- What: A local-first shared planning workspace for humans and AI agents providing a visual Kanban board and dependency DAG graph paired with a native Model Context Protocol (MCP) server.
- Agent integration: Native MCP server (`npx -y roadmap-skill`) and Agent Skills; supports Claude Code, OpenAI Codex, Cursor, and OpenCode; agents query and mutate tasks via MCP tool calls.
- Mobile: responsive web (local browser web UI accessible via mobile on local network; no native mobile app; cite: `README.md`).
- Pricing: free & open-source (MIT, as of 2026-10-16).
- Self-host: yes via zero-configuration local tool executed via `npx roadmap-skill` with local file persistence.
- License: MIT
- Activity: 81 GitHub stars, published on npm, commits through 2026-09-16; active.
- Pros: Dual-view model combining intuitive Kanban status with DAG dependency graph for execution ordering; Extremely simple single-command launch via npx without database setup; Official VS Code MCP one-click installer badge for instant agent setup.
- Cons: Designed for single-user or local repository planning rather than enterprise team boards; No cloud synchronization out of the box; Relatively minimal feature set beyond task DAGs.
- Sources: https://github.com/shiquda/roadmap-skill, https://www.npmjs.com/package/roadmap-skill
- Confidence: high

---

### It's a Plan (itsaplan)
- URL: https://itsaplan.dev/
- Kind: general-kanban-with-agent-integration
- What: An open-source, self-hosted project management and issue tracking platform (Linear/Plane alternative) built from the ground up for human teams and AI agents to ship products together.
- Agent integration: Built-in native Model Context Protocol (MCP) server, comprehensive REST API, outbound webhooks, automated agent teammate assignment, AI issue triaging.
- Mobile: responsive web (fully responsive web app on desktop, tablet, and mobile; PWA installable; no native iOS/Android binary; cite: `https://itsaplan.dev/docs/`).
- Pricing: free & open-source (AGPL-3.0); optional hosted cloud plans (free community tier, paid team tiers starting at ~$8-12/user/month as of 2026-10-16).
- Self-host: yes via one-click deploy on Docker, Railway, Coolify, or self-hosted servers.
- License: AGPL-3.0
- Activity: 983 GitHub stars, 162 forks, commits through 2026-10-16; very active.
- Pros: Complete enterprise-grade Linear alternative with projects, sprints, boards, roadmaps, and custom views; AI agents participate as first-class team members with native MCP support; One-click deployment templates on Railway, Coolify, and Docker; Full REST API and webhook infrastructure.
- Cons: Requires full server infrastructure (Node.js, PostgreSQL/Redis) compared to local-only SQLite apps; AGPL license requires source disclosure for network modifications; Heavier setup than lightweight CLI wrappers.
- Sources: https://itsaplan.dev/, https://github.com/croffasia/itsaplan
- Confidence: high

---

### Claude Code Agent Monitor
- URL: https://hoangsonww.github.io/Claude-Code-Agent-Monitor/
- Kind: agent-native
- What: A real-time monitoring dashboard and Kanban status board for Claude Code, Cursor, and Codex sessions tracking agent hierarchies, token usage, cost, and subagent orchestration.
- Agent integration: Real-time WebSocket hook streaming from Claude Code, OpenAI Codex, and Cursor CLI sessions; MCP 1.0 protocol integration; agent hierarchy tree inspection.
- Mobile: responsive web UI + native desktop apps for macOS and Windows (cite: `README.md`).
- Pricing: free & open-source (MIT, as of 2026-10-16).
- Self-host: yes via Node.js Express backend + SQLite3 + React/Vite/Tailwind frontend, locally runnable.
- License: MIT
- Activity: 1,080 GitHub stars, comprehensive multilingual documentation (KO, CN, VN, EN, ES), commits through 2026-10-15; active.
- Pros: Rich real-time analytics combining Kanban status board with token expense charts, session timeline, and tool call logs; Multilingual localization including full Korean and Chinese documentation; Local-first privacy using SQLite3 with no mandatory cloud account; Interactive companion buddy for visual engagement.
- Cons: Primarily an observation and telemetry cockpit rather than an active task dispatcher with worktree isolation; Requires running local Express/WebSocket daemon; Desktop native app is an Electron-style wrapper.
- Sources: https://hoangsonww.github.io/Claude-Code-Agent-Monitor/, https://github.com/hoangsonww/Claude-Code-Agent-Monitor
- Confidence: high

---

### OpenKanban
- URL: https://github.com/TechDufus/openkanban
- Kind: agent-native
- What: A terminal user interface (TUI) kanban board written in Go for orchestrating AI coding agents, providing git worktree isolation and embedded terminal monitoring per ticket.
- Agent integration: Direct CLI agent orchestration (Claude Code, Codex, etc.); auto-creates isolated git worktrees per ticket; embedded terminal pane to steer running agents.
- Mobile: none (pure terminal user interface / TUI running in terminal emulators on macOS/Linux; cite: `README.md`).
- Pricing: free & open-source (AGPL-3.0, as of 2026-10-16).
- Self-host: yes via single statically-linked Go binary running locally in user's terminal environment.
- License: AGPL-3.0
- Activity: 151 GitHub stars, binary releases on GitHub, commits through 2026-10-14; active.
- Pros: Zero browser overhead, runs completely inside terminal/SSH sessions; Embedded terminal emulator per card allows instant inspection without switching tmux windows; Automatic git worktree management keeping main codebase pristine; Ultra-lightweight resource footprint written in Go.
- Cons: TUI interface is keyboard-only and cannot be viewed in standard web browsers or mobile devices; Lacks visual drag-and-drop mouse UX; Fewer third-party webhooks/integrations.
- Sources: https://github.com/TechDufus/openkanban, https://yetanotherorchestrator.app/apps/openkanban/
- Confidence: high

---

### Hermes Agent Kanban (NousResearch)
- URL: https://hermes-agent.nousresearch.com/docs/user-guide/features/kanban
- Kind: agent-native
- What: Built-in multi-agent task board and swarm execution engine inside NousResearch's Hermes Agent, orchestrating complex task graphs (triage, todo, ready, running, blocked, done) with worker, verifier, and synthesizer personas.
- Agent integration: Native CLI commands (`hermes kanban`, `hermes kanban swarm`), SQLite database (`hermes_cli.kanban_db`), A2A protocol support, agent tool calls (`kanban_*`), and companion WebUIs (`HermesKanban`).
- Mobile: responsive web available via community WebUIs (e.g. HermesKanban); core CLI is terminal-only (cite: Zenn article 20260525-hermes-kanban-swarm-usage, NousResearch docs).
- Pricing: free & open-source (MIT, as of 2026-10-16).
- Self-host: yes, runs locally via Python package `hermes-agent` using local SQLite storage and local/remote LLM endpoints.
- License: MIT
- Activity: 254,155 GitHub stars, commits on 2026-10-16; exceptionally active.
- Pros: Built directly into the autonomous Hermes Agent runtime; Structured task graph pattern (`parallel workers → verifier → synthesizer`) preventing AI output quality degradation; Persistent SQLite task queue with resumption, blocking, and auditing; Idempotency-key support for safe cron and multi-agent invocation.
- Cons: Native interface is purely CLI-based; Full swarm orchestration requires configuring distinct agent profiles and skills; Complex multi-agent flows can consume significant token budgets.
- Sources: https://zenn.dev/zennai_ryutaro/articles/20260525-hermes-kanban-swarm-usage, https://juejin.cn/post/7661325178647330856, https://github.com/NousResearch/hermes-agent, https://hermes-agent.nousresearch.com/docs/user-guide/features/kanban
- Confidence: high

---

### Backlog (Nulab)
- URL: https://backlog.com
- Kind: general-kanban-with-agent-integration
- What: Japan's leading project management and issue-tracking Kanban platform by Nulab, enhanced with Backlog AI for automated issue breakdown, PR/task summarization, and kanban board automation.
- Agent integration: Backlog AI Assistant (OpenAI-powered native assistant), comprehensive REST API v2, Webhooks, community MCP servers (`mcp-server-backlog`); supports Claude and ChatGPT integrations via API/MCP.
- Mobile: native iOS app (Apple App Store: Backlog by Nulab), native Android app (Google Play: Backlog), plus responsive web (cite: `https://backlog.com/ja/mobile/`).
- Pricing: free tier (up to 10 users, 1 project, 100MB); Starter plan ~¥2,970/mo ($20/mo) for 30 users; Standard plan ~¥14,080/mo ($95/mo) unlimited users; Backlog AI features bundled or add-on as of 2026-10-16.
- Self-host: yes for enterprise (Backlog Enterprise self-hosted on-premises edition available via Red Hat / CentOS / Docker package), though primarily sold as SaaS.
- License: proprietary
- Activity: Established Japanese market leader (Nulab Inc., listed on Tokyo Stock Exchange); continuous weekly production releases in 2026; active.
- Pros: Highly polished and mature UX widely adopted across Japan and Asia; Native iOS and Android mobile apps with push notifications and full board management; Built-in Git/SVN repositories, Gantt charts, and wiki integrated directly into Kanban cards; Broad REST API and thriving community MCP ecosystem.
- Cons: Proprietary commercial SaaS platform with paid tiers for serious team usage; AI features are assistant-level rather than autonomous coding agents executing in isolated git worktrees; Self-hosted enterprise edition has significant licensing costs.
- Sources: https://backlog.com/ja/lp-ai-assistant/, https://backlog.com/ja/blog/backlog-ai-issue-creation/, https://qiita.com/tie_rrific/items/8df560b1cb27945bd4c9, https://backlog.com/ja/mobile/
- Confidence: high

---

## EXPAND
- LEAD: Apple Intelligence Board Reconciler (iloli macOS App) — WHY: Japanese developer iloli built a native macOS app leveraging Apple's Foundation Models framework (`@Generable`, `@Guide`) to automatically reconcile ambient AI chat turns against Kanban cards on-device without wasting cloud tokens. — ANGLE: search: `site:zenn.dev "Apple Intelligence" カンバン` and investigate on-device ambient agent reconcilers.
- LEAD: Star-Office-UI 3D Virtual Office Ecosystem (JuniorXcoder/hermes-virtual-office) — WHY: Growing Asian trend combining Kanban task queues with 3D / isometric pixel virtual office game environments (Phaser, Three.js) to monitor A2A agent fleet status visually. — ANGLE: search: `site:github.com "virtual office" agent kanban`.
- LEAD: Git Worktree vs MicroVM Execution Isolation in Asian Agent Boards — WHY: Emerging dichotomy between local worktree-based boards (Multica, KanVibe, Kander, KanBots) and MicroVM/Container-based cloud agents (Agent Kanban, Realmroot, Enbor). — ANGLE: search: `site:juejin.cn "git worktree" agent 看板` vs containerized agent sandboxes.
