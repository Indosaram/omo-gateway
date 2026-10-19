# Agent Kanban & Containerized Agent Dashboards Survey

### Catnip
- URL: https://github.com/wandb/catnip (also referenced at https://catnip.run, mirror at https://github.com/HarleyCoops/catnip)
- Kind: agent-native
- What: A containerized development harness and dashboard created by Weights & Biases (W&B) for running multiple AI coding agents (specifically Claude Code) in parallel across isolated Git worktrees and Docker/Apple Container environments.
- Agent integration: CLI-first orchestration designed for Claude Code (`claude`); spawns and controls sessions inside container sandboxes via PTY terminals, auto-commits changes to Git branches (`refs/catnip/$WORKSPACE_NAME` and `feature/*`), captures tool calls and todo lists, and forwards dev server ports automatically.
- Mobile: Responsive web interface (mobile breakpoint at 768px in `src/hooks/use-mobile.ts`) and an iOS native application distributed via TestFlight / companion service at https://catnip.run allowing remote session monitoring, terminal inspection, and approval from phones.
- Pricing: Free and open-source (100% free self-hosted; no paid subscription tier announced prior to repository transition).
- Self-host: Yes; self-hosted via local Go binary + embedded Vite SPA (`catnip run`) and Docker/Colima/Apple Container SDK (`wandb/catnip` base container with `--dind` Docker-in-Docker support and automatic SSH daemon `ssh catnip`).
- License: Apache-2.0
- Activity: ~491 GitHub stars; originally created July 2025, active push through September 2026 (~488-491 stars, 40 forks); upstream repository `wandb/catnip` returned HTTP 404/removal in autumn 2026 during W&B codebase reorganization, but full source tree and forks (e.g. HarleyCoops/catnip) remain preserved and active in community mirrors.
- Pros: True sandbox isolation using Docker or Apple Container SDK preventing rogue commands from damaging the host system; Automated Git worktree creation and synchronization allowing frictionless parallel branch development; Built-in port autodetection and reverse proxy for live web service previewing; Clean embedded React/Tailwind/TanStack UI with terminal streaming, diff inspection, and task/todo badges; Remote access through SSH pairing and mobile web/iOS TestFlight companion.
- Cons: Upstream repository at wandb/catnip is currently 404/relocated by Weights & Biases, requiring users to rely on mirrors or container snapshots; Primary optimization target is Claude Code CLI, with other agent harnesses (Codex, OpenCode) remaining on the unfinished roadmap; Requires a local container runtime (Docker Desktop, Podman, or Colima) running in the background; Mobile iOS app was limited to TestFlight beta rather than a general public App Store release.
- Sources: https://github.com/wandb/catnip, https://github.com/HarleyCoops/catnip, https://github.com/barstoolbluz/agentic-development-with-flox, https://github.com/bradAGI/awesome-cli-coding-agents, https://github.com/gabrielmoreira/awesome-ai-rabbit-holes, https://catnip.run
- Confidence: high (verified via source tree analysis of HarleyCoops/catnip mirror, flox environment package definitions, and git commit history).

### Vibe Kanban
- URL: https://github.com/BloopAI/vibe-kanban (official site: https://vibekanban.com)
- Kind: agent-native
- What: A high-profile open-source kanban board and multi-workspace orchestrator built by BloopAI to coordinate, plan, execute, and review 10+ AI coding agents running in isolated Git branches and dev environments.
- Agent integration: CLI execution harness supporting 10+ coding agents: Claude Code, OpenAI Codex CLI, Google Gemini CLI, GitHub Copilot CLI, Amp Code, Cursor Agent CLI, OpenCode, Droid CLI, CCR, and Qwen Code. Interacts via local process/worktree spawning, real-time terminal streaming, automated diff viewing, and inline code review comment passing back to agent sessions.
- Mobile: Responsive web interface accessible on tablet/mobile browsers; native cross-platform client codebase contains Tauri mobile build scaffolding (`crates/tauri-app` with Android and iOS app icons and manifest schemas), but primary usage was desktop web and localhost browser.
- Pricing: Free and open-source for self-hosting; Bloop previously operated a managed "Vibe Kanban Cloud" SaaS tier, which announced its sunsetting/shutdown in late 2026 with 30-day data export banners.
- Self-host: Yes; self-hosted locally via one-command `npx vibe-kanban`, native desktop binary (Tauri/Rust backend + React frontend), or Docker Compose deployment (`crates/remote/docker-compose.yml` with Caddy SSL termination and GitHub/Google OAuth).
- License: Apache-2.0
- Activity: 28,376+ GitHub stars; active maintenance and releases through October 2026 (v0.1.45), though the hosted cloud service was sunset in favor of community self-hosted and cleanroom implementations (e.g. `p-wegner/agentic-kanban`).
- Pros: Broadest out-of-the-box coding agent compatibility (Claude Code, Codex, Gemini, Cursor, OpenCode, Amp); First-class kanban task lifecycle tightly bound to git branches and interactive browser diff viewers; Built-in web preview browser with devtools, inspect mode, and device viewport emulation; Fast and lightweight local launch via `npx vibe-kanban` or Docker Compose; Large developer community with 28k+ GitHub stars.
- Cons: Hosted Vibe Kanban Cloud service is shutting down, shifting all operational burden to self-hosting; Mobile native experience is experimental Tauri shell rather than dedicated App Store/Play Store apps; High disk usage when maintaining multiple parallel branch worktrees and dev containers; Lacks native Model Context Protocol (MCP) server endpoints for bidirectional task manipulation by autonomous agents.
- Sources: https://github.com/BloopAI/vibe-kanban, https://vibekanban.com, https://github.com/p-wegner/agentic-kanban
- Confidence: high (verified via BloopAI/vibe-kanban repository tree, Docker compose manifests, tauri configs, and cloud shutdown banners).

### Agent Kanban (saltbo)
- URL: https://github.com/saltbo/agent-kanban (live site: https://agent-kanban.dev)
- Kind: agent-native
- What: An agent-first mission-control board designed to take humans out of the execution loop while preserving supervisory review, where autonomous agents discover and operate tasks via OpenAPI contracts and authenticated tools.
- Agent integration: Agent-native REST API with OpenAPI specification and Realmroot Toolbox / Enbor Session bindings; agents claim tasks (`POST /api/tasks/{id}/claim`), stream notes/progress, and submit review packages (`submit review`) while humans retain unilateral gate approvals.
- Mobile: Fully responsive web application optimized for mobile viewports, featuring mobile-specific board status tabs verified via automated Playwright mobile test suites (`tests/e2e/public/public-board-mobile.spec.ts`).
- Pricing: Free self-hosted / open source core under Functional Source License (FSL-1.1-ALv2); cloud hosted demo/service at agent-kanban.dev with optional enterprise multi-tenancy.
- Self-host: Yes; deployed as a modern TypeScript application using Cloudflare Workers + D1 database, or run locally via Node.js/pnpm (`pnpm db:migrate && pnpm dev`).
- License: FSL-1.1-ALv2 (Functional Source License, automatically converts to Apache-2.0 after 2 years).
- Activity: ~492 GitHub stars; active commits in October 2026; continuous development on multi-tenant architecture and Realmroot OIDC federation.
- Pros: Truly designed around autonomous agents as first-class actors rather than humans typing into a UI; Strict architectural separation of duty (assignee agent cannot approve its own work); Cloudflare Workers + D1 serverless architecture provides near-instant cold starts and zero idle compute costs; Robust dependency graph support with acyclic dependency validation; Mobile-tested public board sharing for external stakeholders.
- Cons: Tightly coupled to Realmroot OIDC and Enbor execution platform for runtime session execution; FSL license is business-source rather than traditional permissive OSI open source (converts to Apache 2.0 after 2 years); Does not package a native Docker container runtime or local terminal emulator in core; Lacks pre-built native iOS/Android binary apps (relies on responsive web).
- Sources: https://github.com/saltbo/agent-kanban, https://agent-kanban.dev
- Confidence: high (verified via repository structure, Playwright mobile tests, package.json, and architecture documentation).

### Automagik Forge
- URL: https://github.com/automagik-dev/forge (official site: https://forge.automag.ik)
- Kind: agent-native
- What: The "Vibe Coding++" multi-agent kanban and orchestration platform allowing developers to structure work into kanban boards, dispatch tasks to isolated Git worktrees, and run multiple AI providers/agents concurrently.
- Agent integration: Native Model Context Protocol (MCP) server support via `mcp_task_server` and `@automagik/genie` (`mcp__forge__get_task`, `mcp__genie__view`), alongside CLI runner (`automagik-forge`) and REST endpoints; supports Claude Code, Gemini CLI, and custom Genie agents.
- Mobile: Dedicated mobile native Android app specification (`.genie/wishes/mobile-native-app/` with Capacitor/native Android APK architecture, 10-week specification, bottom navigation, and mobile UI inventory) alongside responsive web UI on http://localhost:8887.
- Pricing: 100% Free and open source (`npm install -g @automagik/forge` or `npx @automagik/forge`); no mandatory commercial subscription fees.
- Self-host: Yes; run locally via Node.js/pnpm/npx (`npx @automagik/forge`), runs local Rust/TypeScript backend with SQLite storage and Git worktree management on localhost.
- License: Apache-2.0
- Activity: ~92 GitHub stars; active maintenance with updates in October 2026; active Discord community and roadmap tracking on GitHub Projects.
- Pros: Native MCP server integration allows agents like Claude Code to read, create, and update kanban tasks directly; Isolated multi-attempt execution in separate Git worktrees per task; Built-in Genie AI planning assistant to decompose complex feature requests; Human-in-the-loop review interface with diff inspection before merging; Active design documentation for native mobile expansion.
- Cons: Smaller community footprint compared to Vibe Kanban or Bloop (~92 stars); Mobile native application is in specification/Capacitor build stage rather than App Store release; Requires local Git environment and pnpm/Node.js toolchain; Heavy frontend bundle with 160+ TSX components.
- Sources: https://github.com/automagik-dev/forge, https://forge.automag.ik
- Confidence: high (verified via .mcp.json, architecture docs, mobile spec tree, and package.json).

### 5dive
- URL: https://github.com/5dive-ai/5dive (official site: https://5dive.ai)
- Kind: agent-native
- What: A lightweight, container/systemd-isolated multi-agent operations platform and kanban task board that coordinates autonomous teams of Claude, Codex, and other coding CLI agents on a self-hosted server with Telegram mobile bot/app control.
- Agent integration: Native bash/CLI task queue (`5dive task`, `5dive board`), SQLite task store, systemd sandboxing per agent Unix user, and bidirectional Telegram bridge; Claude Code and OpenAI Codex CLI run headless and invoke `5dive` CLI commands or MCP plugins to pick up tasks and request human approval.
- Mobile: Native mobile interaction via Telegram Bot and Telegram Mini App (`5dive telegram-app link` opening the `my.5dive.ai` Mini App directly inside Telegram on iOS and Android); provides tap-to-approve buttons for spend, publishing, and shell escalations.
- Pricing: Open-source core is 100% Free (MIT license); 5dive.ai offers an optional managed cloud VM starting with free trial, with paid managed instances available (~$15/month for cloud-hosted agent VMs).
- Self-host: Yes; self-hosted on any modern Linux system using systemd and bash (`curl -fsSL https://install.5dive.ai | sudo bash`), with zero external cloud broker required.
- License: MIT
- Activity: ~71 GitHub stars; active daily commits in October 2026; production releases published continuously; maintainers run their own company on the tool ("dogfooding").
- Pros: Pragmatic Unix-native design using standard OS features (systemd, journald, Unix user accounts) instead of heavy custom daemon bloat; Superb mobile operational flow via Telegram Bot and Mini App with interactive action buttons; Hard human gates preventing agents from unauthorized monetary spend or destructive actions without mobile approval; Clean decoupled JSON contract for kanban board rendering (`5dive board --json`); Completely MIT-licensed with zero proprietary open-core restrictions.
- Cons: Linux-centric architecture requires systemd and root installation (`sudo 5dive init`), making native macOS/Windows execution dependent on VMs or Docker containers; Web kanban UI is moving to an external plugin (`5dive-ui`) rather than being monolithic; Smaller developer community (~71 stars) compared to older devtools; Requires Telegram account for out-of-the-box mobile push notifications.
- Sources: https://github.com/5dive-ai/5dive, https://5dive.ai, https://t.me/FiveDiveBot
- Confidence: high (verified via repository inspection, board contract specs, install.sh, and Telegram mini app integration scripts).

### Agent Dashboard (dustinblack)
- URL: https://github.com/dustinblack/agent-dashboard
- Kind: agent-native
- What: A multi-host, containerized orchestration dashboard for deploying, monitoring, and interacting with AI coding agents (Claude Code, Pi, Codex, OpenCode, Antigravity) across multiple machines using rootless Podman/Docker containers.
- Agent integration: Spawns and manages agents inside tmux sessions across distributed hosts; streams interactive pseudo-terminals (xterm-256color) over WebSocket/Socket.IO; ingests live OpenTelemetry (OTLP) telemetry directly from agent CLI processes for token usage and cost accounting.
- Mobile: Responsive web dashboard (HTML5/Tailwind/React) supporting mobile and tablet touch scrolling; includes an experimental Python Textual TUI for SSH terminal access from mobile SSH clients (Termius, Blink).
- Pricing: 100% Free and open-source (no paid tiers or licenses).
- Self-host: Yes; deployable via `podman-compose up` or `docker compose` with rootless containerized host daemons and FastAPI backend hub.
- License: Apache-2.0
- Activity: ~17 GitHub stars; updated October 2026; clean documentation, active container build workflows.
- Pros: Distributed multi-host architecture allows controlling agents running on remote servers or local workstations from one pane of glass; Native OpenTelemetry receiver provides exact token count and dollar cost tracking per agent session; True bidirectional interactive PTY terminal emulation (xterm-256color); Containerized security with rootless Podman/Docker daemons; Extensible YAML profile configuration for adding new agents without code modification.
- Cons: Focuses more on terminal session multiplexing and resource telemetry than structured drag-and-drop kanban columns; No dedicated native mobile apps (responsive browser or TUI over SSH only); Smaller community adoption (~17 stars); Multi-node setup requires deploying daemons on each target machine.
- Sources: https://github.com/dustinblack/agent-dashboard, https://github.com/dustinblack/agent-dashboard/blob/main/docs/agent-profiles.md
- Confidence: high (verified via compose.yml, architecture docs, agent profiles, and CI workflow files).

### Agent Kanban PM (kronos3610)
- URL: https://github.com/kronos3610/agent-kanban-pm (docs: https://raman369ai.github.io/agent-kanban-pm/)
- Kind: agent-native
- What: A local-first, single-user Kanban project management server for coordinating human developers and headless AI coding CLI agents through isolated worktrees and review stages.
- Agent integration: Built-in Model Context Protocol (MCP) server endpoints and REST/WebSocket APIs; an orchestrator CLI agent can programmatically create cards, assign sub-agents to specific roles, inspect diffs, and transition tasks through approval stages.
- Mobile: Responsive browser UI accessible over local network (via FastAPI web server); no dedicated native mobile app or push notification infrastructure.
- Pricing: Free forever and open source under MIT; no paid tiers or subscription fees.
- Self-host: Yes; self-hosted local Python package (`pip install agent-kanban-pm` / PyPI v0.8.0) backed by SQLite; runs on Linux or macOS.
- License: MIT
- Activity: ~8 GitHub stars; release 0.8.0 published September/October 2026; stable single-developer tool.
- Pros: Native MCP server enables orchestrator agents (Claude, Codex) to manipulate the board programmatically; Strict local-first privacy with SQLite database and no external cloud telemetry; Integrated stage transitions ensuring execution, diff review, and handoff integrity; Minimal resource overhead compared to heavy Docker stacks.
- Cons: Designed strictly for single-operator use with token authentication (unsafe to expose directly to public internet without reverse proxy); Lacks container sandboxing out of the box (runs local processes in tmux); No native mobile app or push notifications.
- Sources: https://github.com/kronos3610/agent-kanban-pm, https://pypi.org/project/agent-kanban-pm/
- Confidence: high (verified via repository ARCHITECTURE.md, PyPI release data, and README).

### Plane
- URL: https://plane.so (repo: https://github.com/makeplane/plane)
- Kind: general-kanban-with-agent-integration
- What: An open-source enterprise project management and issue tracking platform (Linear/Jira alternative) offering comprehensive kanban boards, sprint cycles, issue triaging, and AI agent integration.
- Agent integration: Robust REST and GraphQL APIs, webhooks, official/community MCP server integrations (allowing Claude Code and Cursor to query and mutate issues), and Plane AI agent extensions for automated backlog refinement and issue triage.
- Mobile: Official native mobile applications available on Apple App Store (iOS) and Google Play Store (Android), complemented by a responsive web dashboard.
- Pricing: Open-source Community edition is Free; Plane Cloud offers Free tier (up to 10 users), Pro plan ($8/user/month), and Enterprise plan ($15/user/month billed annually).
- Self-host: Yes; officially supported self-hosting via Docker Compose, Kubernetes Helm charts, and single-node enterprise installers.
- License: GNU Affero General Public License v3.0 (AGPL-3.0) for Community edition; commercial license for Enterprise edition.
- Activity: 60,900+ GitHub stars; extremely active enterprise project with daily commits in October 2026 and regular release cadences.
- Pros: Mature enterprise project management with native iOS and Android mobile apps; Rich REST/GraphQL APIs and MCP connectors for agent automation; Full self-hosting capabilities via Docker and Kubernetes; Comprehensive permissions, cycle management, and analytics beyond toy kanban tools; Massive community support with 60k+ GitHub stars.
- Cons: General project management tool not inherently built around agent sandboxing or Git worktree isolation; Requires external glue code/MCP adapters for coding agents to execute CLI commands in local directories; Significantly heavier infrastructure footprint (PostgreSQL, Redis, MinIO, RabbitMQ); AGPL-3.0 license requires careful compliance evaluation for commercial modification.
- Sources: https://plane.so, https://github.com/makeplane/plane, https://plane.so/pricing
- Confidence: high (verified via official Plane repository, pricing documentation, and mobile app store releases).

## EXPAND
- LEAD: OpenClaw Agent Dashboard (karem505/openclaw-agent-dashboard) — WHY: Emerging glassmorphic and pixel-art ops dashboard for OpenClaw autonomous coding agents with mobile-ready web UI — ANGLE: gh repo view karem505/openclaw-agent-dashboard
- LEAD: Pier / Docker-based agent sandboxes (wandb/hivemind, wandb/background-agents) — WHY: Weights & Biases appears to have shifted agent visibility tooling from standalone catnip into `wandb/hivemind` and `wandb/background-agents` in late 2026 — ANGLE: gh repo view wandb/hivemind --json description,readme
- LEAD: Local-first MCP task coordinators (Get-Concord-AI/concord-mcp, OktoLabsAI/okto-nexus) — WHY: Shared state and file-claim collision detection for multi-agent workflows without full kanban UI — ANGLE: gh repo view Get-Concord-AI/concord-mcp
