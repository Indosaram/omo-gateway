### Agent Kanban
- URL: https://agent-kanban.dev/
- Kind: agent-native
- What: Agent Kanban is an agent-native task board and mission-control platform designed for autonomous software agents to coordinate durable work across repositories. It features a full KANBAN/board view of agent tasks (yes), supporting task lifecycle states (Todo, In Progress, In Review, Done, Cancelled) with human review gates.
- Agent integration: Realmroot Toolbox CLI / REST OpenAPI contract with OIDC and DPoP authentication; integrates with Enbor/Agency runtime sessions, GitHub App, and packaged agent role skills (`ak-worker`, `ak-maintainer`, `ak-task`, `ak-plan`) for agents like Claude Code, Codex, and other ACP/autonomous agents.
- Mobile: responsive web only (accessible via mobile browsers; no native iOS/Android app or official PWA installation found).
- Pricing: open-source free (free self-hosted and source-available; no paid per-seat tiers as of October 2026).
- Self-host: yes via Cloudflare Workers + D1 database (`pnpm db:migrate` and `pnpm deploy` via Wrangler) or local development server (`pnpm dev`, port 6265). Requires external Realmroot and Enbor services for full session dispatch.
- License: Functional Source License 1.1 with future conversion to Apache 2.0 (FSL-1.1-ALv2; converts to Apache-2.0 after 2 years).
- Activity: GitHub 492 stars (repository `saltbo/agent-kanban`), last committed around 2026-09-13; active development in v2 boundary.
- Pros: True agent-first model with first-class agent identity and execution provenance; Built-in human review gates where assignees cannot approve their own PRs/submissions; Native dependency-aware planning with cycle rejection; Clean serverless Cloudflare Workers and D1 deployment.
- Cons: Tightly coupled to Realmroot and Enbor ecosystems for auth and runtime execution; FSL license restricts commercial competing uses for the first 2 years; No dedicated native mobile app; Complex setup requiring multiple service credentials.
- Sources: https://agent-kanban.dev/, https://yetanotherorchestrator.app/apps/agent-kanban/, https://github.com/saltbo/agent-kanban, https://raw.githubusercontent.com/saltbo/agent-kanban/main/README.md, https://raw.githubusercontent.com/saltbo/agent-kanban/main/LICENSE, https://raw.githubusercontent.com/saltbo/agent-kanban/main/docs/architecture/system-overview.md
- Confidence: high

### AI Maestro
- URL: https://ai-maestro.23blocks.com/
- Kind: agent-native
- What: AI Maestro is an open-source operating system and orchestration platform for AI-first organizations managing fleets of terminal-based coding agents across multiple machines. It includes a drag-and-drop KANBAN/board view of agent tasks (yes) within its "Team Meetings & Kanban" war room, featuring 5 status columns and task dependencies.
- Agent integration: Native support for Claude Code, Codex, Grok Build, Aider, Cursor, and any terminal agent; inter-agent messaging protocol (AMP), Agent Identity (AID), Agent Actions Protocol (AAP), Agent Files Protocol (AFP), Claude Code inbox/secrets mods, and 13 Agent Skills (Agent Skills Standard) alongside a CLI (`aimaestro-agent.sh`, `aim-secret`).
- Mobile: responsive web (optimized for touch screens with auto-collapsing sidebar and compact header; accessed remotely via Tailscale VPN without port forwarding; no native app in App Store/Play Store).
- Pricing: free and open-source forever ($0, MIT licensed; no paid per-seat tier, no hosted SaaS tier as of October 2026).
- Self-host: yes via local Node.js + tmux (`yarn dev`), Docker containers, or cloud deployments via Terraform onto AWS EC2 or AWS ECS Fargate.
- License: MIT License (permissive open-source).
- Activity: GitHub 822 stars (repository `23blocks-OS/ai-maestro`), version v0.81.6, active commits up to 2026-10-15.
- Pros: Completely free and open-source (MIT) with no telemetry or upsell; Decentralized peer mesh network allows orchestrating agents across different machines (Mac, Linux, EC2) without a central server; Rich protocol ecosystem (AMP, AID, AAP, AFP) and 13 modular skills; Includes persistent memory (CozoDB) and code graph indexing.
- Cons: No native mobile application (relies on browser over Tailscale); Setup requires Node.js 18+ and tmux 3.0+; Can be complex to configure for multi-machine mesh and cloud Terraform setups.
- Sources: https://ai-maestro.23blocks.com/, https://yetanotherorchestrator.app/apps/ai-maestro/, https://github.com/23blocks-OS/ai-maestro, https://ai-maestro.23blocks.com/mobile.html
- Confidence: high

### amux
- URL: https://amux.io/
- Kind: agent-native
- What: amux is an open-source control plane and multiplexer for running parallel AI coding agent sessions (Claude Code, Codex, Gemini CLI) coordinated from a unified dashboard. It has an integrated KANBAN/board view of agent tasks (yes) with SQLite compare-and-swap atomic claiming, dependency blocking (`blocked_on`), and strict status gates (`done` requires evidence, `verified` requires peer check).
- Agent integration: REST API (e.g., `POST /api/board/<task>/claim`, `GET /api/board/ready`, `POST /api/send`), native CLI (`amux-rs` and bash `amux`), structured AgentProtocol (`opencode`), and origin-stamped inter-agent messaging; supports Claude Code, OpenAI Codex CLI, Gemini CLI, OpenCode, and Ollama.
- Mobile: native iOS app (available on Apple App Store: "amux — agent multiplexer", free, requires iOS/iPadOS 16.0+, macOS 13.0+ Apple Silicon) and installable PWA with full steering, prompt approvals, and terminal output peeking.
- Pricing: open-source free self-hosted ($0 forever for core features); amux Cloud managed hosting starts at $20/user/month (or $200/year, billed annually at ~$16.67/mo) with a 7-day free trial (as of October 2026).
- Self-host: yes via single Rust binary (`amux-server-rs` on port 8824) with embedded SQLite store and bundled SPA dashboard; installable via `./install.sh` on macOS (launchd) or Linux (systemd user services); no Docker required.
- License: MIT License + Commons Clause (free to use, modify, and self-host for internal use; commercial resale prohibited without a commercial license).
- Activity: GitHub 534 stars (repository `mixpeek/amux`), iOS app v1.1.0 updated late September 2026, active repo commits up to 2026-10-15.
- Pros: Single lightweight Rust binary with zero required external daemons or node runtime; Native iOS app and PWA for monitoring and steering agents on mobile; Atomic task claiming prevents race conditions among autonomous agents; Robust self-healing watchdog with auto-compaction and crash replay.
- Cons: Commons Clause condition restricts commercial SaaS resale; Native iOS app currently iOS/iPadOS-only (no native Android app); Requires macOS or Linux with systemd / tmux 3.2+ for worker execution.
- Sources: https://amux.io/, https://yetanotherorchestrator.app/apps/amux/, https://github.com/mixpeek/amux, https://amux.io/pricing/, https://raw.githubusercontent.com/mixpeek/amux/main/LICENSE, https://apps.apple.com/us/app/amux-agent-multiplexer/id6760410435
- Confidence: high

## EXPAND
- LEAD: CliDeck, Fusion, and jat — WHY: Listed alongside amux and Agent Kanban on YetAnotherOrchestrator as agent orchestration dashboards with task management and terminal controls — ANGLE: web fetch yetanotherorchestrator.app/apps/clideck, /apps/fusion, and /apps/jat
- LEAD: agent-viewer (hallucinogen/agent-viewer) & kandev (kdlbs/kandev) — WHY: Highly starred GitHub alternatives dedicated to kanban boards for Claude Code agents in tmux — ANGLE: search GitHub repositories "agent-viewer" and "kandev"
