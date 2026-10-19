# GitHub Search Sweep: AI Agent Kanban & Task Board Tools (2025–2026)

Seen: Trello, Jira, Linear, Notion, Asana, ClickUp, monday, Plane, WeKan, Kanboard, Planka, Vikunja, Focalboard, Taiga, Huly, Vibe Kanban, Task Master, Backlog.md, Crystal, Conductor, Claude Squad, Sculptor, Omnara, Happy, Shrimp, opcode, claudecodeui, Agent Deck, CCManager, emdash, Superset, Catnip.

### Nimbalyst
- URL: https://nimbalyst.com/
- Kind: agent-native
- What: An open-source visual workspace and task management system built specifically for orchestrating Claude Code, Codex, and OpenCode. Provides parallel git worktrees, interactive visual editors, red/green diff reviews, and session-level kanban boards.
- Agent integration: Direct CLI execution and orchestration for Claude Code, OpenAI Codex, OpenCode, GitHub Copilot (alpha), and Gemini; built-in MCP client and server support; git worktree isolation per session.
- Mobile: Native iOS companion app ("iOS companion app" on App Store / TestFlight; provides session dashboard, push alerts, voice replies, diff approvals; Android companion planned; cite: README.md)
- Pricing: Open-source free (Free & MIT licensed desktop application and companion app as of 2026-10-16)
- Self-host: Yes (Local desktop app for macOS DMG, Windows .exe, Linux .deb/.AppImage; local markdown and config files stored in git repository, no cloud database required)
- License: MIT
- Activity: 1,908 stars; last pushed 2026-10-15; latest release v0.80.6 on 2026-10-09; active development.
- Pros: True visual workspace with inline WYSIWYG diff reviews for markdown, diagrams, and mockups; Dedicated native iOS companion app with push alerts and mobile diff approvals; Built-in MCP client that renders tool results as interactive widgets; Clean git-worktree isolation preventing concurrent agent file collisions; Completely local-first with plain text file storage.
- Cons: Early stage project (v0.80.x) with frequent updates; Android native companion is not yet released; Extension SDK ecosystem is still emerging.
- Sources: https://github.com/nimbalyst/nimbalyst, https://nimbalyst.com/, https://docs.nimbalyst.com/
- Confidence: high (verified via repository README, releases, and documentation)

### Agent Teams AI
- URL: https://agentteams.live
- Kind: agent-native
- What: An autonomous multi-agent engineering platform where users command an AI company from a high-level Kanban board while agents self-organize, message each other, and review code. Integrates 300+ models and CLI agents into cooperative teams with voice control.
- Agent integration: CLI execution and API integrations for Claude Code, OpenAI Codex, OpenCode, Cursor, SuperGrok, GitHub Copilot, Z.AI, MiniMax, Kiro; MCP server support; inter-agent messaging protocol.
- Mobile: Responsive web interface optimized for mobile browsers ("From a phone, users should be able to see only the high-level tasks that matter and give the lead agent instructions by voice"; cite: README.md Vision section); no native app store app.
- Pricing: Open-source free (100% free desktop app; built-in free tier with zero-auth community models; bring your own API keys/CLI auth as of 2026-10-16)
- Self-host: Yes (Desktop app for macOS/Windows/Linux, standalone local web server; zero cloud requirement)
- License: AGPL-3.0
- Activity: 2,265 stars; last pushed 2026-10-16; latest release v2.17.10 on 2026-10-09; extremely active.
- Pros: High autonomy with agents conversing, negotiating, and reviewing each other on the board; Zero-configuration runtime with 300+ LLM models and local CLI auto-detection; Real-time Kanban board showing high-level status with drill-down terminal views; Built-in voice input for commanding agents hands-free.
- Cons: AGPL-3.0 copyleft license restricts closed commercial derivatives; Autonomous agent loops can consume significant tokens without strict limits; Mobile access requires manual local network or tunnel exposure.
- Sources: https://github.com/777genius/agent-teams-ai, https://agentteams.live/
- Confidence: high (verified via repository README, architecture docs, and release notes)

### Routa
- URL: https://phodal.github.io/routa/
- Kind: agent-native
- What: A workspace-first multi-agent coordination platform for software delivery created by Phodal Huang, featuring shared specifications, Kanban orchestration, and automated fitness/review gates. Uses a dual-runtime architecture combining a Next.js web application and a Tauri desktop app backed by Rust Axum.
- Agent integration: Model Context Protocol (MCP), Agent Client Protocol (ACP), Agent-to-Agent (A2A), AG-UI, A2UI, REST API, SSE streaming; supports Claude Code, Cursor, Codex, and custom LLM worker loops.
- Mobile: Responsive web (Next.js web frontend accessed via mobile browser; no native iOS/Android binary; cite: README.md architecture diagram)
- Pricing: Open-source free (MIT license, free self-hosted web and desktop binaries as of 2026-10-16)
- Self-host: Yes (Docker container, Node/Next.js self-hosted web app, or Tauri native desktop app on macOS/Linux/Windows backed by embedded Axum daemon)
- License: MIT
- Activity: 1,828 stars; last pushed 2026-10-14; latest release v0.19.0 on 2026-06-22; active development.
- Pros: Dual-runtime architecture (lightweight local Tauri app or multi-user self-hosted Next.js web app); Comprehensive protocol support including MCP, ACP, and A2A; Built-in fitness functions, review gates, and automated verification before closing Kanban cards; Shared specs and documentation integration alongside tasks.
- Cons: Setup can be complex due to Rust/Next.js dual-backend dependencies; Mobile experience is limited to standard browser viewport without offline support; Documentation partially split between English and Chinese.
- Sources: https://github.com/phodal/routa, https://phodal.github.io/routa/, https://www.npmjs.com/package/routa-cli
- Confidence: high (verified via codebase README, architecture diagram, and crates.io/npm metadata)

### Claude Code Agent Monitor (CCAM)
- URL: https://hoangsonww.github.io/Claude-Code-Agent-Monitor/
- Kind: agent-native
- What: A real-time local-first monitoring control room and Kanban status board for Claude Code, Codex, and Cursor sessions. Visualizes tool calls, agent execution hierarchies, token costs, and task statuses with live WebSocket updates.
- Agent integration: Claude Code native hooks, Cursor transcript discovery, OpenAI Codex session polling, MCP server health monitoring, REST/WebSocket API.
- Mobile: Responsive web (served on local port, viewable on mobile Safari/Chrome via LAN or Tailscale; cite: README.md responsive dashboard section); no native app.
- Pricing: Open-source free (MIT license, 100% free and open-source as of 2026-10-16)
- Self-host: Yes (Native macOS DMG / Windows NSIS desktop app via Electron 35, or local Node.js/Express + Vite server with SQLite3 storage)
- License: MIT
- Activity: 1,080 stars; last pushed 2026-10-11; latest release v2.3.0 on 2026-10-10; highly active.
- Pros: Zero friction setup with automatic detection of Claude Code and Codex sessions; Real-time Kanban board paired with live token/cost analytics; Detailed subagent hierarchy trees and tool execution inspection; Multi-platform packaged desktop app (macOS DMG & Windows installer) plus web UI.
- Cons: Primary focus is observability/monitoring rather than bidirectional task assignment; Electron desktop packaging increases bundle size; Lacks native mobile push notification daemon.
- Sources: https://github.com/hoangsonww/Claude-Code-Agent-Monitor, https://hoangsonww.github.io/Claude-Code-Agent-Monitor/
- Confidence: high (verified via repo README, release v2.3.0 notes, and package architecture)

### Kandev
- URL: https://kandev.ai
- Kind: agent-native
- What: A power-user AI Kanban and development environment that orchestrates dozens of parallel coding agent CLIs in isolated worktrees. Combines Kanban and pipeline views with an integrated file editor, terminal, browser preview, and review-gate system.
- Agent integration: Direct CLI orchestration supporting 20+ agents (Claude Code, Codex, GitHub Copilot, Gemini CLI, Cursor, Devin, OpenCode, Hermes, etc.); Task-Agent MCP server; external streamable HTTP MCP API.
- Mobile: Responsive web with remote access guide ("mobile remote-access guide" via Tailscale/Cloudflare Tunnel; cite: README.md & docs/public/mobile-remote-access.md); no native app.
- Pricing: Open-source free (AGPL-3.0, free self-hostable binary with zero cloud dependency or telemetry as of 2026-10-16)
- Self-host: Yes (Single self-contained Go binary with embedded web frontend, Docker container, or macOS/Linux Homebrew package)
- License: AGPL-3.0
- Activity: 954 stars; last pushed 2026-10-16; latest release v0.97.0 on 2026-10-04; very active.
- Pros: Single static Go binary makes self-hosting trivially easy; Supports over 20 agent CLIs with fine-grained multi-step agent pipelines (e.g. Opus plans, Copilot builds, Codex reviews); First-class review workspace with visual diffs, file tree, terminal, and live browser previews; Bidirectional MCP server allows agents to create subtasks and inspect sibling tasks.
- Cons: AGPL-3.0 license restricts closed-source enterprise redistribution; UI density can be overwhelming for casual users; Mobile usage is purely web-based over tunnel.
- Sources: https://github.com/kdlbs/kandev, https://kandev.ai, https://github.com/kdlbs/kandev/releases
- Confidence: high (verified via repository documentation, feature specs, and releases)

### Veritas Kanban
- URL: https://sstb.ai
- Kind: general-kanban-with-agent-integration
- What: A local-first, privacy-focused task management and Kanban board with deep modular AI agent orchestration layers. Features a 42-tool MCP server, multi-agent capability profiles, and strict phase-transition governance journals.
- Agent integration: Official Model Context Protocol (MCP) server with 42 tools, CLI adapter, REST API, webhooks; supports Claude Code, OpenAI Codex, Copilot CLI, Hermes, OpenClaw, Grok Build.
- Mobile: Progressive Web App (PWA) with responsive mobile interface and documented mobile offline architecture (cite: README.md & docs/architecture/ADR-0003-post-ga-native-mobile-offline.md); packaged desktop app for macOS.
- Pricing: Open-source free (MIT license, free local-first core with optional self-hosted extensions as of 2026-10-16)
- Self-host: Yes (macOS desktop cask via Homebrew `brew install --cask veritas-kanban`, Node.js/pnpm local server, or Docker container)
- License: MIT
- Activity: 841 stars; last pushed 2026-10-14; latest release v6.2.1 on 2026-09-10; mature and active.
- Pros: Extremely robust architectural rigor with formal ADRs, phase capability profiles, and transition journals; Unmatched MCP integration with 42 discrete tools for board and task manipulation; Flexible progressive layering (use as pure human kanban, add agent orchestration as needed); PWA support with documented offline sync architecture.
- Cons: High learning curve due to extensive governance concepts; Multi-tier setup requires pnpm and environment configuration; Complex documentation structure.
- Sources: https://github.com/BradGroux/veritas-kanban, https://sstb.ai, https://github.com/BradGroux/veritas-kanban/blob/main/CHANGELOG.md
- Confidence: high (verified via comprehensive repository documentation and v6 changelog)

### Claude Task Viewer
- URL: https://github.com/L1AD/claude-task-viewer
- Kind: agent-native
- What: A lightweight web-based Kanban board specifically built for observing Claude Code tasks in real time. It monitors local Claude Code session files and presents task breakdowns, active dependencies, and execution progress in a clean visual board.
- Agent integration: Direct file-system monitoring of Claude Code tasks (`~/.claude/` session state and task files); reads task dependencies (`blockedBy`/`blocks`); read-only observation with optional manual task cleanup.
- Mobile: Responsive web (accessible over local browser; no native app or PWA manifest; cite: README.md)
- Pricing: Open-source free (MIT license, 100% free as of 2026-10-16)
- Self-host: Yes (Local Node.js/Vite server running on localhost; runs directly on user machine with zero external network dependencies)
- License: MIT
- Activity: 765 stars; last pushed 2026-02-14; active community fork/usage (beta status).
- Pros: Zero configuration required—instantly reads Claude Code's native task store; Visualizes critical paths and dependency blockers that are invisible in standard terminal view; Audio chimes and desktop browser notifications when tasks complete; Minimal footprint and fast startup.
- Cons: Read-mostly observer board (cannot dispatch or trigger new Claude sessions directly); Claude Code-specific (no native support for Codex, Cursor, etc.); Development activity quieted down after early 2026.
- Sources: https://github.com/L1AD/claude-task-viewer
- Confidence: high (verified via repository README, codebase, and issue discussions)

### Kanbots
- URL: https://kanbots.dev
- Kind: agent-native
- What: A local collaboration Kanban board where every card is an AI coding agent executing in its own isolated git worktree. Allows running up to 11 agent CLIs in parallel with live tool streaming, branch previews, and autopilot task splitting.
- Agent integration: Native CLI orchestration for 11 agents (Claude Code, OpenAI Codex, Gemini CLI, Cursor, GitHub Copilot, Amp, OpenCode, Droid, CCR, Qwen Code, and any ACP-compatible CLI); official `kanbots-mcp-server`.
- Mobile: Responsive web (viewable on mobile browser over local network or tunnel; cite: README.md); no native mobile app.
- Pricing: Open-source free (MIT license, free local-first software as of 2026-10-16)
- Self-host: Yes (Local web application running on local Node runtime with SQLite storage; git worktrees managed locally)
- License: MIT
- Activity: 615 stars; last pushed 2026-05-25; latest release v1.2.0 on 2026-05-25; stable.
- Pros: Broadest out-of-the-box CLI support (11 agents + any ACP stdio binary); Autopilot mode splits backlog items into subtasks and runs parallel execution slots automatically; One-click branch preview spins up local dev servers per card; Built-in MCP server allows external agents to drive the board.
- Cons: Parallel worktrees and heavy agent concurrency can strain local CPU and disk storage; Requires CLI agents to be pre-installed and authenticated on PATH; No native iOS/Android client.
- Sources: https://github.com/leodavinci1/kanbots, https://kanbots.dev
- Confidence: high (verified via repository README and releases)

### KaibanJS (Kaiban Board)
- URL: https://www.kaibanjs.com/
- Kind: agent-native
- What: A JavaScript/TypeScript-native framework for multi-agent systems built around the Kanban methodology, accompanied by Kaiban Board UI. Coordinates teams of specialized agents where tasks move visually through Kanban columns as agents execute and pass deliverables.
- Agent integration: Direct JS/TS framework API, LangChain integration, ExternalCodingAgent adapter for Claude Code (`claude --bare`), OpenCode, and Codex; MCP tool integrations.
- Mobile: Responsive web (Kaiban Board web playground and embedded React components adapt to mobile screens; cite: README.md and documentation); no native app.
- Pricing: Open-source free (MIT license, 100% free community framework with hosted playground on kaibanjs.com as of 2026-10-16)
- Self-host: Yes (npm package `kaibanjs`, standalone Vite/React board UI, deployable via Node.js or Docker)
- License: MIT
- Activity: 1,480 stars; last pushed 2026-05-15; latest release v0.24.1 on 2026-05-01; active.
- Pros: First multi-agent framework natively built on the Kanban mental model; Embeddable React components allow embedding agent task boards directly into custom apps; ExternalCodingAgent enables invoking headless Claude Code and Codex directly from workflow tasks; Strong documentation and active web playground.
- Cons: Primarily a developer framework requiring JavaScript code to define agents and tasks; Not a drop-in binary dashboard for non-programmers; CLI agent integration is newer than its native LLM API integration.
- Sources: https://github.com/kaiban-ai/KaibanJS, https://www.kaibanjs.com/, https://docs.kaibanjs.com/
- Confidence: high (verified via repo, docs site, and npm package)

### Agent Kanban (saltbo)
- URL: https://agent-kanban.dev
- Kind: agent-native
- What: An agent-first task board designed to take humans out of the execution loop by treating AI agents as first-class actors with verifiable execution provenance. Features machine-discoverable interfaces, automated task assignment, dependency resolution, and multi-agent skills.
- Agent integration: REST API, JSON endpoints, automated skill runner, CLI hooks; supports Claude Code, OpenAI Codex, and custom autonomous agent loops.
- Mobile: Responsive web (Cloudflare Pages/D1 powered modern web UI accessible from mobile browsers; cite: README.md and live site); no native mobile app.
- Pricing: Open-source free (FSL-1.1-ALv2 / Functional Source License, free self-hostable instance as of 2026-10-16)
- Self-host: Yes (Node.js 24 + pnpm, Cloudflare Workers / Wrangler local D1 SQLite database, or cloud edge deployment)
- License: FSL-1.1-ALv2 (Functional Source License, transitions to Apache-2.0 after 2 years)
- Activity: 492 stars; last pushed 2026-10-02; latest release v1.13.4 on 2026-05-19; active updates.
- Pros: Purpose-built data model treating agents as first-class citizens rather than human proxies; Cryptographic/verifiable execution provenance records runtime and exact model outputs; Cloudflare Workers/D1 edge architecture makes it fast and lightweight; Modular role-based skills directory for plug-and-play workflows.
- Cons: Uses Functional Source License (FSL) rather than standard MIT/Apache permissive license; Requires Node.js 24+ and Wrangler environment for local hosting; Smaller community footprint.
- Sources: https://github.com/saltbo/agent-kanban, https://agent-kanban.dev
- Confidence: high (verified via repository documentation, license files, and live site)

### dev-3.0
- URL: https://dev3.h0x91b.com/
- Kind: agent-native
- What: A mission control Kanban board for the "One Person Studio" where every task card represents a live AI coding agent operating inside its own isolated git worktree and terminal. Enables running dozens of agents simultaneously across Claude Code, Codex, and Gemini CLI with QR-code remote access.
- Agent integration: Direct CLI process manager and tmux orchestration for Claude Code, Codex, Gemini CLI, Cursor Agent, OpenCode; AI reviewer automated spawn; CLI agent self-install guide.
- Mobile: Responsive web with dedicated headless remote server mode (`dev3 remote` serves UI over Cloudflare Tunnel / SSH with QR code scan for phones; cite: README.md "It runs headless, and your phone is a client"); no native app store binary.
- Pricing: Open-source free (Apache-2.0 license, completely free and open-source as of 2026-10-16)
- Self-host: Yes (Precompiled binaries for macOS, Linux, and Windows; curl installer; local-first storage)
- License: Apache-2.0
- Activity: 312 stars; last pushed 2026-10-15; latest release v1.58.0 on 2026-10-11; very active commits.
- Pros: Seamless mobile phone companion workflow via `dev3 remote` QR code without manual network setup; Dedicated columns for agent lifecycle stages including "Has Questions" and "AI Review"; Robust disk hygiene with `dev3 doctor --worktrees` to prevent disk bloat; Built-in inline diff viewer with interactive commenting turned directly into agent prompts.
- Cons: Relatively new tool with rapid version increments (v1.58.0 within months); Requires local tmux installation on Unix systems; Windows experience relies on WSL/virtualization for best performance.
- Sources: https://github.com/h0x91b/dev-3.0, https://dev3.h0x91b.com/
- Confidence: high (verified via repository README, screenshots, and live release artifacts)

### AIF Handoff
- URL: https://aif.cutcode.dev
- Kind: agent-native
- What: An autonomous Kanban board built on the AI Factory framework where specialized AI agents plan, implement, and review tasks hands-off. Features an automatic rework convergence loop (Review → request_changes → Implementing) and explicit human escalation handoffs.
- Agent integration: `@aif/runtime` profiles with native adapters for Claude Code and OpenAI Codex; WebSocket live event streaming; REST API for task transitions.
- Mobile: Responsive web (Vite + Tailwind frontend accessible via mobile browser; cite: README.md); no native mobile application.
- Pricing: Open-source free (MIT license, free self-hosted web system as of 2026-10-16)
- Self-host: Yes (Node.js 20+/22+ with better-sqlite3 local database, Vite dev server, Docker container support)
- License: MIT
- Activity: 308 stars; last pushed 2026-10-15; active development (active main branch).
- Pros: Intelligent auto-review convergence detection that escalates to human review instead of looping indefinitely; Strict pipeline stages (Backlog → Planning → Plan Ready → Implementing → Review → Done); Dual runtime support for Claude and Codex swappable per task; Zero cloud lock-in with local SQLite storage.
- Cons: Native compilation of `better-sqlite3` requires Node version alignment; Lacks packaged single-binary distribution (requires npm install/build); No standalone mobile app or push notifications.
- Sources: https://github.com/lee-to/aif-handoff, https://aif.cutcode.dev
- Confidence: high (verified via repository codebase, README, and dependency configs)

### AI Agent Board (DanWahlin)
- URL: https://github.com/DanWahlin/ai-agent-board
- Kind: agent-native
- What: An open-source drag-and-drop Kanban board created by Dan Wahlin that delegates software tasks to AI coding agent CLIs. Automatically detects installed agents, isolates tasks in git worktrees, and streams execution progress live to the board.
- Agent integration: CLI adapters for GitHub Copilot CLI, Claude Code (`claude`), OpenAI Codex (`codex`), OpenCode, Hermes (`hermes acp`), and OpenClaw; WebSocket event bus.
- Mobile: Responsive web (standard React/Tailwind frontend readable on mobile browsers; cite: README.md); no native mobile app.
- Pricing: Open-source free (MIT license, 100% free community project as of 2026-10-16)
- Self-host: Yes (Node.js/npm application running locally on localhost, optional Docker container)
- License: MIT
- Activity: 65 stars; last pushed 2026-09-19; active commits in late 2026.
- Pros: Auto-detection of local CLI agents on system PATH at startup; Batch task creation via "Task Groups" for parallel feature rollouts; Clean normalized `AgentEvent` format across 6 different agent providers; Git worktree isolation with automated branch-to-PR creation.
- Cons: Younger project with smaller community star count; Does not yet provide native mobile apps or PWA offline caching; Task assignment is manual drag-and-drop rather than fully autonomous scheduling.
- Sources: https://github.com/DanWahlin/ai-agent-board
- Confidence: high (verified via repository source code, architecture descriptions, and CI tests)

### Agetor
- URL: https://agetor.dev
- Kind: agent-native
- What: A local-first harness orchestrator and Kanban board for executing Claude Code, Codex, Cursor, Gemini CLI, and Antigravity in parallel across isolated git worktrees. Features visual pipeline graphs, restart-safe tmux sessions, and structured live transcripts.
- Agent integration: Native CLI orchestration for Claude Code, OpenAI Codex, Cursor, Gemini CLI, fx, Antigravity; non-invasive execution (drives real CLI without modifying CLAUDE.md or installing hooks); custom agent profiles and pipelines.
- Mobile: Responsive web / local dashboard (primarily desktop macOS Apple Silicon application with browser view; cite: README.md); no native mobile app.
- Pricing: Open-source free (MIT license, free download with optional GitHub Sponsors support as of 2026-10-16)
- Self-host: Yes (macOS prebuilt native binary built with Bun, local-first with zero accounts and zero telemetry)
- License: MIT
- Activity: 89 stars; last pushed 2026-10-15; latest release v1.0.2 on 2026-10-15; active.
- Pros: Zero invasive configuration (does not inject prompts or require special MCP servers); Restart-safe execution via tmux ensures running agent jobs survive app restarts; Canvas-based visual pipelines (Plan → Implement → Review) linked directly to Kanban cards; Rich UI components for structured agent transcripts (tool calls, diffs, subagents).
- Cons: Currently packaged primarily for macOS Apple Silicon (Linux/Windows require building from source); Single-user local tool without team collaboration backend; No mobile push notification system.
- Sources: https://github.com/alamops/agetor, https://agetor.dev, https://github.com/alamops/agetor/releases
- Confidence: high (verified via official website, GitHub repo, release binaries, and README)

### Kander
- URL: https://dualface.dev/projects/
- Kind: agent-native
- What: A battle-tested multi-agent Kanban orchestration engine created by dualface (creator of QuickTUI), refined over 1,700+ production tasks in QuickTUI. Enforces strict rule-driven delivery gates, isolated git worktrees, and independent reviewer agents to eliminate code regressions and hallucinations.
- Agent integration: Direct CLI integration for OpenAI Codex, Claude Code, Grok, Cursor, and Pi; automated reviewer agent dispatch; QuickTUI direct terminal protocol.
- Mobile: Companion mobile terminal support via QuickTUI ("QuickTUI is a tool for remotely operating the agents on your computer; it supports iOS/Android/macOS/Linux/Windows and is free to use"; cite: README.md & https://quicktui.ai); Kander TUI itself runs in terminal.
- Pricing: Open-source free (MIT license, free CLI/TUI and free single-host companion mobile app QuickTUI as of 2026-10-16)
- Self-host: Yes (Homebrew formula `brew install dualface/tap/kander`, Linux standalone binary archives, runs locally with Git)
- License: MIT
- Activity: 148 stars; last pushed 2026-10-16; latest release v0.9.7 on 2026-10-15; active daily engineering.
- Pros: Real-world production pedigree with 1,700+ documented tasks and 1,600+ independent review runs; 49% review catch rate prevents broken agent code from landing silently; Mobile pairing through QuickTUI allows full terminal interaction and card tracking on iOS and Android; Fast keyboard-driven terminal Kanban interface with vim-like shortcuts.
- Cons: Terminal UI (TUI) focused rather than rich browser GUI; Relies on external QuickTUI companion for mobile visualization; Requires git worktree comfort.
- Sources: https://github.com/dualface/kander, https://dualface.dev/projects/, https://quicktui.ai/
- Confidence: high (verified via repository documentation, retrospective reports, and production stats)

### OverClick
- URL: https://github.com/ustoppble/overclick
- Kind: agent-native
- What: A privacy-first, self-hosted task board for hybrid human + AI-agent teams where agents execute cards over MCP and report back with verified telemetry. Designed with zero data collection, local Postgres storage, and native model token tracking.
- Agent integration: Official Model Context Protocol (MCP) server; connects any MCP-capable agent including Claude Code, Codex, Gemini CLI, Overclock; REST API.
- Mobile: Responsive web (modern web dashboard running on port 3000 accessible from mobile browsers; cite: README.md); no native mobile app.
- Pricing: Open-source free (MIT license, 100% free and open-source self-hosted system as of 2026-10-16)
- Self-host: Yes (Docker Compose with local PostgreSQL database, single-command deployment `docker compose up --build`)
- License: MIT
- Activity: 119 stars; last pushed 2026-09-24; latest release v0.4.0 on 2026-09-24; active beta.
- Pros: MCP-native architecture allows any compliant coding agent to discover and claim cards autonomously; Built-in telemetry tracking captures real measured tokens per model and duration directly from session transcripts; Absolute privacy guarantee with zero telemetry, analytics, or external phone-home; Simple Docker Compose deployment with Postgres persistence.
- Cons: Early stage (v0.4.0) with developing feature set; No native mobile app or push notification service; Requires Docker and mandatory `AUTH_SECRET` environment configuration.
- Sources: https://github.com/ustoppble/overclick, https://github.com/ustoppble/overclick/releases
- Confidence: high (verified via repository README, docker-compose configuration, and release v0.4.0)

## EXPAND
- LEAD: Obsidian-based agent kanbans (such as `dotpm/obsidian-pm` and `hasanyilmaz/operon`) — WHY: Many engineers prefer storing tasks as plain markdown notes within an Obsidian vault, driving agents via local REST or MCP endpoints directly against their existing markdown files without running a standalone server. — ANGLE: GitHub search sweep for 'obsidian agent kanban' OR 'obsidian coding agent task'.
- LEAD: OpenClaw dedicated mission-control cockpits (such as `daggerhashimoto/openclaw-nerve`, `Wadera/clawboard`, and `tmchow/hzl`) — WHY: The OpenClaw multi-agent ecosystem has produced specialized web dashboards with real-time kanban boards, DAG task dependencies, and voice-assisted steering specifically tailored for OpenClaw gateway tasks. — ANGLE: GitHub search sweep for 'openclaw kanban' OR 'openclaw mission control'.
