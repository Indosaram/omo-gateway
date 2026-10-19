### 1Code
- URL: https://1code.dev
- Kind: agent-native
- What: 1Code is an open-source, Cursor-like desktop/web client developed by 21st.dev for running Claude Code and OpenAI Codex coding agents in isolated git worktrees; it includes a Kanban board view of agent sessions (Kanban: yes).
- Agent integration: Direct native harness execution and process supervision for Claude Code CLI and Codex CLI; programmatic task creation and status tracking via official REST API (`POST /api/v1/tasks`, `GET /api/v1/tasks`); full MCP client server lifecycle management with a built-in plugin marketplace; webhooks/automations triggered via Linear, GitHub, Slack (@1code triggers) or git push/PR events.
- Mobile: PWA (Progressive Web App) companion for starting and monitoring cloud-sandbox background agents from mobile phones; no native iOS or Android app stores listings.
- Pricing: Free open-source core when building from source; subscription tier Pro at $20/month for pre-built binaries, automations, and cloud sandbox background agents (as of 2026-10-16).
- Self-host: yes; local desktop app (macOS, Windows, Linux via Bun/Electron build) with local-first file execution and SQLite storage (`better-sqlite3`), or running cloud sandboxes for background tasks.
- License: Apache License 2.0 (Apache-2.0)
- Activity: GitHub 5,574 stars; latest tagged release v0.0.84 on 2026-03-06 (repo pushed_at 2026-03-06, repository archived on GitHub as of 2026-10-16).
- Pros: Isolated git worktree per agent session preventing collisions; real-time diff previews, hunk rollback, and built-in git client; PWA support to supervise long-running background cloud agents from mobile; built-in MCP server manager and plugin marketplace.
- Cons: Repository is officially archived on GitHub (project inactive/dormant); limited out-of-the-box coding agent harnesses (focused primarily on Claude Code and Codex); pre-built binaries and cloud execution locked behind $20/mo subscription.
- Sources: https://1code.dev, https://yetanotherorchestrator.app/apps/1code/, https://github.com/21st-dev/1code, https://api.github.com/repos/21st-dev/1code, https://raw.githubusercontent.com/21st-dev/1code/main/package.json, https://api.github.com/repos/21st-dev/1code/releases/tag/v0.0.84
- Confidence: high

### Agent Orchestrator
- URL: https://ao-agents.com
- Kind: agent-native
- What: Agent Orchestrator (Orchestrator.inc, `ao`) is an open-source local orchestration platform that plans, runs, and supervises fleets of parallel AI coding agents across isolated git worktrees; it features a real-time, status-derived Kanban board grouping worker sessions into 'Working', 'Needs you', 'In review', and 'Ready to merge' (Kanban: yes).
- Agent integration: Rich multi-agent CLI/daemon architecture via `ao` Go/Cobra CLI and loopback REST API (`POST /api/v1/sessions`, `GET /api/v1/sessions`, `POST /api/v1/reports`, `/api/v1/browser/commands`); supports 25+ agent harnesses including Claude Code, Codex, Cursor, OpenCode, Aider, Goose, Cline, Amp, Kiro, Gemini CLI, DeepSeek, and custom ACP (Agent Client Protocol) Chat drivers; agents push status and progress reports via `ao report` CLI or daemon endpoints.
- Mobile: Native iOS and Android companion apps that pair over LAN or Tailscale/Cloudflare tunnels to monitor sessions, view terminal logs, and receive notifications while code execution stays on host.
- Pricing: Free and open-source (Apache-2.0); optional hosted AO Cloud features (cloud sandboxes and team sync) offer paid subscriptions via Stripe.
- Self-host: yes; local desktop app (macOS Apple Silicon/Intel, Windows, Linux AppImage/deb/rpm) with local Go daemon (`ao daemon`), SQLite DB, or headless self-hosted remote VM host via bootstrap script (`ao remote-host enable` systemd user service over LAN/Cloudflare tunnel).
- License: Apache License 2.0 (Apache-2.0)
- Activity: GitHub 13,217 stars; very active development with multiple daily releases; latest stable v0.14.2 / nightly v0.14.3-nightly.202610152038 on 2026-10-15 (latest push 2026-10-16).
- Pros: Comprehensive harness support across 25+ coding agents; autonomous feedback loops redirecting CI failures and PR review comments directly back to responsible agent sessions; native iOS and Android companion apps with remote daemon pairing; robust isolated browser control and artifact inspection per session.
- Cons: Complex dual-process architecture (Electron frontend + Go daemon backend) requires careful environment coordination; remote-host pairing over public networks requires Cloudflare tunnels or custom VPNs; high issue volume (880+ open issues) due to rapid multi-platform release velocity.
- Sources: https://ao-agents.com, https://yetanotherorchestrator.app/apps/agent-orchestrator/, https://github.com/OrchestratorInc/agent-orchestrator, https://api.github.com/repos/OrchestratorInc/agent-orchestrator, https://github.com/OrchestratorInc/agent-orchestrator/blob/main/docs/cli/README.md, https://github.com/OrchestratorInc/agent-orchestrator/releases, https://github.com/OrchestratorInc/agent-orchestrator/blob/main/docs/self-hosted-remote.md
- Confidence: high

### Agent Teams
- URL: https://agentteams.live
- Kind: agent-native
- What: Agent Teams (Agent Teams AI) is a free, open-source desktop application that orchestrates multi-agent coding teams collaborating autonomously on tasks, communicating across squads, and reviewing code hunks; it features an interactive 5-column real-time Kanban board that updates itself as agents create, pick up, and complete tasks (Kanban: yes).
- Agent integration: Built-in `agent-teams-mcp` (Model Context Protocol) server package exposing tool endpoints (FastMCP/mcp-proxy based with app context binding `x-agent-teams-app-context`); integrated standalone HTTP API and loopback WebSocket/IPC controller; direct integration with Claude Code CLI (discovering `~/.claude/`), Codex, OpenCode, Cursor, and 200+ LLM providers via OpenRouter/custom keys; agents update cards autonomously through internal orchestration protocols.
- Mobile: Responsive web dashboard in active development; desktop app is primary; no native iOS or Android apps currently published.
- Pricing: 100% Free and open-source (AGPL-3.0); includes a zero-config built-in free model requiring no signup or API key; users can bring their own API keys or subscriptions (BYOK) for paid models.
- Self-host: yes; local desktop application (macOS DMG, Windows EXE, Linux AppImage/deb/rpm/pacman) or Docker/standalone HTTP server mode (`src/main/standalone.ts` + `docker/vite.standalone.config.ts`) using local SQLite database (`better-sqlite3`).
- License: GNU Affero General Public License v3.0 (AGPL-3.0) with Contributor License Agreement (CLA v2)
- Activity: GitHub 2,265 stars; active development; latest release v2.17.10 on 2026-10-09 (latest commit pushed on 2026-10-16).
- Pros: Complete zero-cost onboarding with built-in free model requiring no API keys; multi-agent peer collaboration with cross-team messaging and nested organization mapping; fine-grained hunk-by-hunk code review dialog; integrated local terminal workspace, resource monitoring, and budget alerts.
- Cons: Mobile interface is limited (native mobile apps not yet available, web interface still evolving); AGPL-3.0 copyleft license imposes strict redistribution constraints; heavy Electron footprint bundling full devtools, Monaco editor, and Terminal Platform SDKs.
- Sources: https://agentteams.live, https://yetanotherorchestrator.app/apps/agent-teams/, https://github.com/777genius/agent-teams-ai, https://api.github.com/repos/777genius/agent-teams-ai, https://raw.githubusercontent.com/777genius/agent-teams-ai/main/package.json, https://api.github.com/repos/777genius/agent-teams-ai/releases/tag/v2.17.10, https://raw.githubusercontent.com/777genius/agent-teams-ai/main/mcp-server/README.md
- Confidence: high

## EXPAND
- LEAD: Paperclip (https://github.com/paperclipai/paperclip) — WHY: Mentioned in Agent Teams' competitive matrix as a leading open-source 7-column drag-and-drop Kanban orchestrator for autonomous agent fleets with heartbeat protocols and board governance. — ANGLE: Search GitHub 'paperclipai/paperclip' and inspect Kanban board implementation, adapter protocol, and agent heartbeat telemetry.
- LEAD: Gas Town (https://github.com/gastownhall/gastown) — WHY: Featured in multi-agent orchestration benchmarks alongside Agent Teams for persistent role coordination, convoys, and automated PR review gates. — ANGLE: Research 'gastownhall/gastown' architecture, convoy templates, and task monitoring APIs.
- LEAD: OpenRig (https://github.com/mvschwarz/openrig) — WHY: An Apache-2.0 multi-agent rig coordinator that uses queue-based mission dispatch and tmux/cmux terminal telemetry instead of a visual Kanban board. — ANGLE: Survey 'mvschwarz/openrig' coordination primitives and contrast owned-work queue mechanisms with traditional Kanban boards.
