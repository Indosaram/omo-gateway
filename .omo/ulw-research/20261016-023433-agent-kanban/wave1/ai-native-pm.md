### Height
- URL: https://height.app
- Kind: agent-native
- What: Autonomous project management tool that introduced autonomous AI "workers" to organize, triage, update, and close project tasks automatically.
- Agent integration: REST API (API key and OAuth 2.0; endpoints for tasks, lists, activities, field templates, webhooks); third-party Linear export tools; no official MCP server before shutdown.
- Mobile: Native iOS and Android mobile apps previously offered (responsive web app also supported).
- Pricing: Free tier ($0/member/month, unlimited members, 50MB storage); Team ($6.99/mo annual or $8.50/mo monthly); Business ($11.99/mo annual or $14.99/mo monthly); Enterprise custom pricing.
- Self-host: no (cloud-only proprietary SaaS).
- License: proprietary
- Activity: Service officially shut down on September 24, 2025; inactive / shut down.
- Pros: Pioneer in autonomous AI project management; clean UI with spreadsheet-meets-kanban feel; automated task triage and duplicate detection; comprehensive REST API and webhooks.
- Cons: Permanently shut down on September 24, 2025; no self-hosting option; proprietary data format requiring custom export scripts (e.g. height-to-linear-export); no official MCP server was released.
- Sources: https://height.app, https://github.com/api-evangelist/height, https://raw.githubusercontent.com/adamski/height-to-linear-export/main/README.md, https://raw.githubusercontent.com/adamski/height-to-linear-export/main/IMPORT_GUIDE.md
- Confidence: high

### Taskade AI Agents
- URL: https://www.taskade.com
- Kind: agent-native
- What: Collaborative workspace combining AI business app generation (Genesis), customizable 24/7 autonomous AI agent teams, multi-agent workflows, and dynamic views including Kanban boards.
- Agent integration: Official MCP server integration for external tools (Claude Desktop, Cursor, VS Code, Codex); REST API, webhooks, mailhooks, and HTTP actions; built-in multi-agent orchestration (TSK-1 kernel, OpenAI GPT, DeepSeek, Qwen, BYOK Claude).
- Mobile: Native iOS app, native Android app, native desktop apps (macOS, Windows, Linux), and responsive web app.
- Pricing: Free ($0/mo, 2 seats, up to 3 apps, limited credits); Pro ($10/mo billed annually or $20/mo monthly for 10 seats, 50,000 credits/mo, unlimited agents, MCP integration); Business ($25/mo billed annually for unlimited seats, 150,000 credits/mo, custom domains); Max ($100/mo billed annually); Enterprise ($250/mo billed annually, BYOK Claude).
- Self-host: no (cloud-hosted multi-tenant SaaS; no Docker or local self-host option).
- License: proprietary
- Activity: Highly active production SaaS with weekly releases throughout 2025-2026.
- Pros: Official native MCP server for Claude Code/Cursor/VS Code; unlimited AI agents and teams with persistent memory; flexible views including Kanban, list, mind map, and calendar; cross-platform native mobile apps (iOS/Android) with offline support.
- Cons: Proprietary cloud platform with no on-premise self-hosting; credit-based token consumption model that can exhaust during heavy agent builds; complex multi-tier pricing with AI credit packs; deep agent-to-agent logic runs inside their proprietary TSK-1 kernel rather than purely local workflows.
- Sources: https://www.taskade.com, https://www.taskade.com/pricing, https://www.taskade.com/create, https://www.taskade.com/downloads
- Confidence: high

### Motion AI
- URL: https://www.usemotion.com
- Kind: general-kanban-with-agent-integration
- What: AI-driven productivity, project management, and automated time-blocking platform that prioritizes tasks and schedules work dynamically onto team calendars and Kanban views.
- Agent integration: Official REST API (`https://api.usemotion.com/v1` for tasks, projects, workspaces, comments, schedules); community Python and TypeScript SDKs (`python-motion`, `motion-sdk`); webhook triggers via Zapier; no official MCP server as of October 2026.
- Mobile: Native iOS app (App Store), native Android app (Play Store), desktop apps (macOS, Windows), and responsive web app.
- Pricing: 7-day free trial; Individual plan at $19/mo (billed annually at $228/yr) or $34/mo (monthly); Team plan at $12/user/mo (billed annually) or $20/user/mo (monthly).
- Self-host: no (cloud-only proprietary SaaS, SOC 2 Type 2 certified).
- License: proprietary
- Activity: Active commercial product with regular platform iterations in 2025-2026.
- Pros: Outstanding automated time-blocking and algorithmic schedule optimization; integrated Kanban boards, docs, calendars, and meeting notetakers; REST API allows external agents (Claude, Codex) to programmatically push and query tasks; native iOS and Android apps.
- Cons: No native MCP server (external agent integrations require REST API scripts or community SDK wrappers); expensive entry price with no permanent free tier; proprietary cloud service without self-hosting; AI features are primarily internal planning heuristics rather than autonomous autonomous developer agent workers.
- Sources: https://www.usemotion.com, https://www.usemotion.com/pricing, https://docs.usemotion.com, https://github.com/rhymiz/python-motion
- Confidence: high

### Paperclip
- URL: https://paperclip.ing
- Kind: agent-native
- What: Open-source orchestration and governance platform for multi-agent AI companies, featuring a 7-column Kanban task board, org charts, budget gates, and multi-agent coordination.
- Agent integration: First-class native adapters and heartbeat execution for Claude Code, Codex, Cursor/Cursor Cloud, Gemini CLI, OpenCode, Pi, Hermes/Hermes Gateway, Grok Build, Kimi Code; MCP Tool Gateway & Apps; REST API, webhooks, and CLI (`paperclipai`).
- Mobile: Responsive web app designed for mobile viewport management (`pnpm dev:mobile` serves dedicated mobile-optimized interface on port 3101; no standalone App Store native binary).
- Pricing: Open-source free under MIT; Paperclip Cloud managed SaaS on waitlist/beta.
- Self-host: yes + via Node.js CLI (`npx paperclipai@latest onboard --yes`), Docker (`docker compose`), or local source build (`pnpm dev`) with embedded PostgreSQL or external Postgres; local loopback, LAN, or Tailnet bind presets.
- License: MIT
- Activity: GitHub repository `paperclipai/paperclip`, over 2,700 merged PRs, active commits through October 2026.
- Pros: Completely open-source under MIT with no artificial feature gates; 7-column Kanban board with atomic task checkout and concurrency locks; broad multi-harness compatibility across Claude Code, Codex, and Cursor; built-in budget caps, org chart governance, and mobile-friendly responsive UI.
- Cons: Self-hosting requires Node.js 24+ and PostgreSQL infrastructure; no dedicated native iOS/Android binary in app stores (relies on mobile browser / PWA view); requires configuring provider API keys or local CLI logins per agent; multi-agent execution requires careful resource budgeting to avoid local host saturation.
- Sources: https://paperclip.ing, https://github.com/paperclipai/paperclip, https://docs.paperclip.ing
- Confidence: high

### Agent Teams AI
- URL: https://agentteams.live
- Kind: agent-native
- What: Open-source desktop-first orchestration platform and Kanban board for autonomous multi-agent teams across 300+ models and coding harnesses.
- Agent integration: Direct native integration with local Claude Code (`~/.claude/`), Codex, OpenCode, Cursor, SuperGrok, GitHub Copilot, Z.AI, MiniMax, Kiro; built-in MCP server (`mcp-server` directory); IPC and local HTTP API handlers.
- Mobile: none (desktop application only for macOS Apple Silicon/Intel DMG, Windows EXE, and Linux AppImage/deb/rpm/pacman; web interface currently under active development).
- Pricing: Free and open-source under AGPL-3.0; free built-in models without authentication or API keys; bring your own API keys or subscriptions for frontier models.
- Self-host: yes + standalone local desktop application (Electron 40 + React 19 + TypeScript 5) and optional local/trusted HTTP dashboard.
- License: GNU AGPL-3.0
- Activity: GitHub repository `777genius/agent-teams-ai`, 2,265+ stars, active commits as of October 16, 2026.
- Pros: Fully visual real-time 5-column Kanban board with per-task diff review and execution logs; zero-setup free tier requiring no API keys or accounts; supports mixed agent teams (e.g. Claude Code lead + Codex reviewer + Cursor builder); native git worktree branch isolation per agent.
- Cons: Desktop-only without mobile support (iOS/Android apps do not exist); AGPL-3.0 license imposes copyleft obligations on modifications; resource-heavy Electron footprint with multiple concurrent local LLM processes; web dashboard is restricted to local/trusted-network use.
- Sources: https://agentteams.live, https://github.com/777genius/agent-teams-ai
- Confidence: high

### Vibe Kanban (BloopAI)
- URL: https://www.vibekanban.com
- Kind: agent-native
- What: Developer-focused Kanban board that runs coding agents in isolated Git worktrees with dev servers, diff reviews, and interactive browser previews.
- Agent integration: Built-in dedicated MCP server binary (`vibe-kanban-mcp` with global and orchestrator modes); supports 10+ coding agents including Claude Code, Codex, Gemini CLI, GitHub Copilot, Cursor, Amp, OpenCode; REST API backend in Rust (Axum).
- Mobile: Responsive web application for browser access; no native mobile app (desktop app packaged via Tauri for macOS/Windows/Linux).
- Pricing: Free open-source tool (`npx vibe-kanban`); project announced sunset in mid-2026 with code remaining open source.
- Self-host: yes + via single NPX command (`npx vibe-kanban`), Docker container, or local Rust/Node build.
- License: Apache-2.0
- Activity: GitHub repository `BloopAI/vibe-kanban`, 28,370+ stars, 540+ open issues; repo README announced sunset in 2026 though codebase remains functional.
- Pros: High-performance Rust/Axum backend with fast SQLite storage; isolated Git worktree management per agent workspace; built-in dedicated MCP server supporting both global and orchestrator modes; comprehensive inline diff review and browser preview.
- Cons: Project status is officially sunsetting / in maintenance; no native mobile app; requires local agent CLI tooling installed; orchestrator mode requires tight coordination with backend container aliases.
- Sources: https://www.vibekanban.com, https://github.com/BloopAI/vibe-kanban, https://raw.githubusercontent.com/BloopAI/vibe-kanban/main/Cargo.toml, https://raw.githubusercontent.com/BloopAI/vibe-kanban/main/crates/mcp/src/task_server/mod.rs
- Confidence: high

### Gas Town
- URL: https://github.com/steveyegge/gastown
- Kind: agent-native
- What: Git-backed multi-agent workspace and orchestration system coordinating up to 20-30 coding agents via Beads issue tracking, convoys, and tmux-backed workers.
- Agent integration: CLI-first (`gt sling`, `gt convoy`, `gt mayor`); git-backed hooks; supports Claude Code, GitHub Copilot, Codex, Gemini CLI, Cursor, Kiro, Pi, and Oh My Pi; REST API / web dashboard on port 8080; DoltHub federation via Wasteland.
- Mobile: none (terminal TUI `gt feed` and local web dashboard on port 8080 intended for desktop browsers; no native mobile app).
- Pricing: Free and open source under MIT; users pay their own model API / subscription usage.
- Self-host: yes + via Go binary (`brew install gastown` / `go install`), source build, or Docker Compose.
- License: MIT
- Activity: GitHub repository `gastownhall/gastown`, active development through late 2026.
- Pros: Industrial-strength persistence where work survives agent crashes via git worktree hooks; scalable watchdog architecture (Witness, Deacon, Dogs); Bors-style bisecting merge queue (Refinery) for completed code; built-in OpenTelemetry logging and metrics.
- Cons: Steep learning curve with unconventional terminology (Polecats, Rigs, Beads, Molecules, Wasteland); requires tmux, Go, Dolt, and SQLite; web dashboard is an informational monitor rather than an interactive drag-and-drop Kanban board; no mobile support.
- Sources: https://github.com/steveyegge/gastown, https://github.com/steveyegge/gastown/blob/main/docs/glossary.md
- Confidence: high

### Dev Boards (Agent Relay)
- URL: https://github.com/GumbyEnder/agent-relay
- Kind: agent-native
- What: Open-source mission Kanban board built explicitly for AI agents implementing a five-verb contract: poll, claim, heartbeat, escalate, and deliver.
- Agent integration: REST API (`/api/agent` five-verb contract: `poll`, `claim`, `heartbeat`, `escalate`, `deliver`), scoped per-agent bearer tokens (`ark_...`), terminal UI CLI (`apps/tui`), and MCP support; compatible with Claude Code, Codex, Hermes, Grok.
- Mobile: Responsive web application (Vite + React 19 + Tailwind); no native iOS or Android app.
- Pricing: Free open source under MIT; optional hosted cloud version in beta.
- Self-host: yes + via Node.js/Vite with zero-config embedded PGLite database, or Docker Compose with PostgreSQL.
- License: MIT
- Activity: GitHub repository `GumbyEnder/agent-relay`, releases on 0.3.x in 2026 with recent commits on October 16, 2026.
- Pros: Dead-simple five-verb REST protocol tailored specifically for autonomous agent lifecycles; atomic task claiming with HTTP 409 concurrency conflict prevention; zero-dependency quickstart using embedded PGLite without needing external databases; clean web Kanban UI and companion TUI.
- Cons: Young early-stage project (v0.3.x) with evolving protocol specs; no native mobile applications; limited built-in agent execution sandbox (external agents must drive the HTTP API themselves); small open-source contributor community compared to Paperclip or Taskade.
- Sources: https://github.com/GumbyEnder/agent-relay, https://raw.githubusercontent.com/GumbyEnder/agent-relay/main/README.md, https://raw.githubusercontent.com/GumbyEnder/agent-relay/main/LICENSE
- Confidence: high

### Platypus
- URL: https://platypus.chat
- Kind: agent-native
- What: Full-stack self-hosted AI agent platform featuring drag-and-drop Kanban boards, widget dashboards, background cron triggers, and A2A inter-agent communication.
- Agent integration: Native Model Context Protocol (MCP) server support, A2A open protocol endpoints, REST API (Hono.js), HMAC-signed webhooks, and Vercel AI SDK provider integrations (OpenAI, Anthropic, Ollama, vLLM).
- Mobile: Responsive web interface (Next.js + Tailwind + ShadCN) accessible on mobile browsers; no native mobile app binary.
- Pricing: Free open-source software under MIT; users supply their own hosted, proxied, or local LLM models (e.g. Ollama).
- Self-host: yes + via one-line Docker installer script (`curl -fsSL https://platypus.chat/install.sh | bash`) or manual Docker Compose stack.
- License: MIT
- Activity: GitHub repository `willdady/platypus`, active releases and commits in October 2026.
- Pros: Complete self-contained stack with interactive Kanban boards and execution dashboards; first-class MCP and A2A protocol support allowing external agents to query and update cards; support for fully offline local models via Ollama/vLLM for air-gapped security; multi-tenant organizations and workspaces.
- Cons: Docker Compose is required for hosting; lacks native iOS/Android apps; no built-in Git worktree branch management for coding agents; user authentication is basic email/password without SSO/SAML support.
- Sources: https://platypus.chat, https://github.com/willdady/platypus, https://docs.platypus.chat
- Confidence: high

### Roadmap Skill
- URL: https://github.com/shiquda/roadmap-skill
- Kind: agent-native
- What: Lightweight local-first planning tool providing a synchronized Kanban board and dependency Graph View for humans and AI agents.
- Agent integration: Native MCP server (`npx -y roadmap-skill`) compatible with Claude Code (`claude mcp add`), Codex (`codex mcp add`), Cursor, VS Code, OpenCode, and Windsurf; web UI tool call `open_web_interface`.
- Mobile: Responsive web interface served on `localhost:7860` (accessible via mobile browser if port forwarded or accessed over local network); no native mobile apps.
- Pricing: 100% free open-source software.
- Self-host: yes + runs locally via Node.js (`npx roadmap-skill`), storing data in `~/.roadmap-skill/` JSON files with zero external database dependencies.
- License: MIT
- Activity: GitHub repository `shiquda/roadmap-skill`, npm package `roadmap-skill`, active v0.3.x releases in 2026.
- Pros: Instant zero-config setup with a single npx command; seamless bi-directional synchronization between chat LLM commands and visual Kanban/Graph views; visual dependency DAG view for sequencing blocked/ready tasks; zero cloud dependencies with private local JSON storage.
- Cons: Single-user local tool without team collaboration or multi-tenant support; no native mobile application; limited to planning and status tracking (does not execute background code or run dev servers itself); no native budget or token tracking features.
- Sources: https://github.com/shiquda/roadmap-skill, https://www.npmjs.com/package/roadmap-skill
- Confidence: high

## EXPAND
- LEAD: Kanban-MCP (Planka integration) — WHY: Planka is one of the most popular self-hosted open-source Kanban boards (Trello alternative), and projects like bradrisse/kanban-mcp and gogogadgetbytes/planka-mcp bridge Planka directly into Cursor/Claude Code via standard MCP — ANGLE: Search GitHub for 'planka-mcp' and 'kanban-mcp' configuration and card update tool schemas
- LEAD: Linear AI Agents & Linear MCP — WHY: Linear is the de-facto developer standard issue tracker and recently launched native AI agent features alongside official MCP servers that allow coding agents (Claude Code, Cursor) to read/write board status natively — ANGLE: Search 'Linear MCP server' and 'Linear AI agents project management 2025 2026'
- LEAD: OpenRig TUI & RigSpec Task Graph — WHY: OpenRig (mvschwarz/openrig) provides an alternative to browser Kanban boards by managing agent seats, topologies, and queues through a terminal UI (TUI) and tmux daemon — ANGLE: Inspect OpenRig queue and seat status integration with coding agents
