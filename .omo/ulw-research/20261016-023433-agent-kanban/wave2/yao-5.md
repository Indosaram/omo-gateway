### Dorothy
- URL: https://dorothyai.app/
- Kind: agent-native
- What: Dorothy is a free, open-source desktop orchestrator for running and coordinating 10+ AI coding agents in parallel across projects; it includes a built-in KANBAN board view (yes) featuring backlog, planned, ongoing, and done stages with automated skill-based agent task assignment.
- Agent integration: Claude Code CLI, Codex CLI, Gemini CLI, Grok CLI, Ollama; official built-in MCP servers (mcp-orchestrator with 26+ tools for agent lifecycle, mcp-kanban with 8 tools for task CRUD/assignment/progress, mcp-vault, mcp-telegram, mcp-socialdata); REST API server running inside Electron app; GitHub CLI (gh) poller; JIRA REST API v3 poller; Slack and Telegram bot bridges.
- Mobile: none (desktop app for macOS/Linux; remote monitoring and control supported via Telegram and Slack bot integrations, but no native iOS/Android app or PWA).
- Pricing: free / open-source free ($0; no paid tiers or license fees, as of 2026-10-16).
- Self-host: yes (local desktop app via Electron DMG or built from source with Node.js/Electron; local state stored in `~/.dorothy/` SQLite DB and JSON files; no enterprise-only restrictions).
- License: MIT License.
- Activity: 351 GitHub stars, last release 1.2.9 on 2026-07-07, last commit 2026-07-07 (active open-source desktop app repository at Charlie85270/Dorothy).
- Pros: Dedicated built-in mcp-kanban server with 8 programmatic tools for agents to push and update card status directly; Self-managing task pipeline where agents automatically pick up backlog tasks matching required skills; Integrated Telegram and Slack remote bots for receiving alerts and delegating tasks to agents from a phone; Multi-agent coordination with Super Agent orchestrator and shared persistent SQLite Vault for cross-agent memory.
- Cons: Desktop application limited to macOS builds (or building from source for Linux); No hosted web app or mobile native/PWA UI for viewing the board directly without a laptop; Last commit and release occurred in July 2026 with no recent autumn 2026 updates.
- Sources: https://dorothyai.app/, https://yetanotherorchestrator.app/apps/dorothy/, https://github.com/Charlie85270/Dorothy, https://raw.githubusercontent.com/Charlie85270/Dorothy/main/README.md
- Confidence: high

### Granular
- URL: https://granular.build/
- Kind: agent-native
- What: Granular is a local-first desktop workspace and agentic terminal designed for non-developers and visionary builders to orchestrate teams of AI agents; it includes a KANBAN board view (yes), called the Roadmap board in Brain, featuring 6 columns (Idea bank, Backlog, In progress, Testing, Done, Parked) stored in a living `roadmap.md` file that agents read and move cards across.
- Agent integration: Claude Code CLI login, Codex CLI login, Gemini CLI login, Granular Hosted AI models (29 models across 9 providers), custom OpenAI/Anthropic-compatible providers (Ollama, LM Studio, OpenRouter); built-in MCP connector library (1,400+ apps via MCP including GitHub, Notion, Linear, Slack, Stripe, Vercel) and custom MCP servers; Telegram remote bot notification bridge; agents read/update the roadmap directly by reading and modifying markdown notes/cards in the project brain (`roadmap.md`).
- Mobile: none (native desktop app on macOS and Windows beta; remote notifications and status pings provided via Telegram bot bridge, but no native iOS/Android app or mobile web/PWA interface).
- Pricing: Free tier available (includes boss & worker agents, 1 team of 1 boss + 2 workers across 2 projects and 4 sessions per project, BYO API keys/logins); Pro tier at $12.99/user/month, $129/user/year, or $259 one-time lifetime license (with 40% launch promo GRANULAR40 offering $77/yr or $155 lifetime; as of 2026-10-16); optional Granular Hosted AI Models add-on starting at $20/month.
- Self-host: yes (local desktop app for macOS Apple Silicon/Intel and Windows 10/11 beta; all files and `roadmap.md` stored locally in the user's project directory; encrypted local Vault for secrets; no enterprise restrictions, licensed per user on up to 2 computers).
- License: Proprietary (closed-source commercial desktop software).
- Activity: Active commercial product (version v0.3.30 desktop release with macOS signed/notarized and Windows beta builds; documentation updated as recently as 2026-10-14).
- Pros: Native Roadmap kanban board integrated into project Brain with bi-directional human/agent sync via plain `roadmap.md`; Boss and worker architecture running parallel isolated git worktree branch sessions without stepping on files; Strong privacy and zero-markup model support (BYO Claude/Codex logins, API keys, or optional hosted credits); Telegram bot bridge provides mobile push alerts when tasks finish or need approvals.
- Cons: Closed-source proprietary software requiring paid subscription or lifetime license for unlimited projects/agents; No native mobile app or web board view (requires desktop app running); Windows version is still in beta (manual updates and SmartScreen warning).
- Sources: https://granular.build/, https://yetanotherorchestrator.app/apps/granular/, https://granular.build/pricing/, https://granular.build/deal/, https://granular.build/download/, https://granular.build/docs/, https://granular.build/docs/how-the-roadmap-board-works/
- Confidence: high

### Jean
- URL: https://jean.build/
- Kind: agent-native
- What: Jean (by coolLabs) is an opinionated, open-source AI agent dev environment and workspace that runs multiple coding agents in parallel across isolated git worktrees; it does NOT feature a traditional column-based KANBAN board view (no), instead utilizing a "Project Canvas" card dashboard grouped by worktrees, session activity states (planning, active execution, waiting for approval, completed), PRs, and Linear/GitHub ticket integrations.
- Agent integration: Claude CLI, Codex CLI, Cursor Agent, OpenCode, Pi, Command Code, Grok, Kimi Code, and Antigravity; native integration with GitHub Issues/PRs, Linear tickets, Sentry, and CodeRabbit; MCP server support per project; automated git worktree commands, magic prompts, and autonomous issue worker Mr. Robot (beta); REST/WebSocket API endpoints (`/healthz`, `/readyz`, `/api/init`, `/api/auth`, `/ws`) with bearer token authentication on `jean-server`.
- Mobile: responsive web (accessible on phone/tablet browsers via built-in Web Access HTTP/WebSocket server on port 3456, optimized for Tailscale/LAN with mobile swipe gestures and token authentication; no native iOS or Android app store binary).
- Pricing: open-source free (100% free under Apache-2.0 with no paid tiers, no feature gates, and no usage limits; optional sponsorship; as of 2026-10-16).
- Self-host: yes (local native desktop app on macOS/Windows/Linux built with Tauri v2/Rust, standalone headless `jean-server` Linux amd64/arm64 binary with systemd service, and official Docker container `ghcr.io/coollabsio/jean-server`; full multi-server remote management over SSH or Web Access URL; no enterprise restrictions).
- License: Apache-2.0 License.
- Activity: 1,333 GitHub stars, last release v1.0.11 on 2026-10-14, last commit on 2026-10-15 (highly active open-source project by coolLabs/Andras Bacsai).
- Pros: 100% free and open-source under Apache-2.0 with zero feature paywalls; First-class headless server daemon (`jean-server` / Docker) and embedded Web Access enabling phone/browser usage across Tailscale or reverse proxies; Native parallel git worktrees with automated branch management and conflict resolution; Deep issue tracker integration pulling tasks directly from GitHub Issues and Linear into isolated agent sessions.
- Cons: Does not provide a classic multi-column Kanban board (uses a status-grouped Project Canvas card view instead); Desktop builds for Windows and Linux are community-tested and less tested than macOS; No native iOS/Android mobile app in App Store / Google Play (relies on Web Access via mobile browser).
- Sources: https://jean.build/, https://yetanotherorchestrator.app/apps/jean/, https://github.com/coollabsio/jean, https://raw.githubusercontent.com/coollabsio/jean/main/README.md, https://jean.build/docs, https://jean.build/docs/workflows/project-canvas-and-chat, https://jean.build/docs/core-concepts/projects-worktrees-and-sessions, https://jean.build/docs/manage/web-access-and-headless
- Confidence: high

## EXPAND
- LEAD: Nimbalyst (https://yetanotherorchestrator.app/apps/nimbalyst/) — WHY: Listed alongside Jean and Granular in the directory as an open-source visual workspace for Codex and Claude Code with workspace canvas concepts — ANGLE: Search for "Nimbalyst" GitHub repo, task management features, and whether it implements a formal Kanban board.
