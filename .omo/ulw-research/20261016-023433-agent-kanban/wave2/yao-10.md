### Multica
- URL: https://multica.ai/
- Kind: agent-native
- What: Multica is an open-source, source-available AI agent orchestration platform that organizes human engineers and AI coding agents as teammates on a Linear-style task board; KANBAN/board view of agent tasks: yes.
- Agent integration: Local daemon auto-detects 26 coding agent CLIs (Claude Code, OpenAI Codex, GitHub Copilot CLI, Cursor Agent, OpenCode, Kimi CLI, Grok, Trae CLI, Qwen Code, etc.), HTTP REST API, CLI tool (`multica issue`, `multica agent`), WebSocket real-time progress streaming, Multica CLI skill for Claude Code/Codex/Cursor, and channel triggers (Slack, Feishu/Lark, DingTalk, WeCom, Telegram).
- Mobile: Native iOS app built from source via Expo/React Native (not yet on App Store as of October 2026; builds to iPhone/iPad), no native Android app, responsive web dashboard available in mobile browsers (cite: https://multica.ai/docs/mobile-app).
- Pricing: Free / open-source self-hosted for internal organization use (unlimited workspaces and agents); Multica Cloud offers a free trial/tier with paid upgrades (commercial cloud pricing contact sales/unlisted custom pricing as of 2026-10-16).
- Self-host: yes, Docker Compose, single binary, or Helm/Kubernetes via `multica setup self-host` (requires Docker, Go backend, Next.js, and PostgreSQL 17); free for internal team use, commercial license required only for offering Multica as a hosted service to third parties.
- License: Multica License (source-available: Apache License 2.0 with additional commercial hosted-service, commercial embedding, and branding conditions).
- Activity: 52,949 GitHub stars, latest release v0.6.2 (released 2026-10-13), active daily weekday release cadence.
- Pros: True Linear-style collaborative Kanban board with agents assigned like human colleagues; Native support for 26 agent CLIs with auto-detection; Built-in reusable skills library and multi-machine daemon runtimes; Transparent task lifecycles with proactive blocker alerts and execution replay.
- Cons: Mobile iOS app must be compiled from source with Xcode (not distributed on App Store, no Android app); Source-available license restricts multi-tenant or commercial hosting for third parties; Requires hosting PostgreSQL 17 and Node.js/Go daemon infrastructure when self-hosting.
- Sources: https://multica.ai/, https://yetanotherorchestrator.app/apps/multica/, https://multica.ai/licensing, https://multica.ai/docs/mobile-app, https://multica.ai/docs/desktop-app, https://multica.ai/docs/cli, https://multica.ai/docs/cloud-quickstart, https://github.com/multica-ai/multica, https://github.com/multica-ai/multica/releases
- Confidence: high (all 13 fields verified directly from official website, documentation, GitHub repository, and licensing pages as of 2026-10-16)

### Sortie
- URL: https://sortie-ai.com/ (Docs: https://docs.sortie-ai.com/)
- Kind: agent-native
- What: Sortie is an open-source orchestration daemon that pulls tickets from issue trackers (Linear, Jira, GitHub Issues, GitLab, Gitea) and dispatches autonomous coding agents in isolated workspaces; KANBAN/board view of agent tasks: no (it relies on your existing tracker's Kanban board such as Linear or Jira for the board view, while its built-in web dashboard provides summary metric cards, accordion session tables, retry queues, and run history).
- Agent integration: Pluggable agent adapters for Claude Code, GitHub Copilot CLI, OpenAI Codex CLI, OpenCode, Kiro CLI, and Gemini CLI (via Agent Client Protocol / ACP); issue trackers via Linear, Jira, GitHub Issues, GitLab, and Gitea APIs; embedded HTTP JSON API (`GET /api/v1/state`, `GET /api/v1/{identifier}`, `POST /api/v1/refresh`), Prometheus `/metrics`, and MCP configuration injection (`.sortie/mcp.json`).
- Mobile: Responsive web dashboard accessible via any mobile browser over LAN/VPN/reverse-proxy (no native iOS app, no native Android app, no PWA install declared; cite: https://docs.sortie-ai.com/reference/dashboard/).
- Pricing: Free and 100% open-source under Apache-2.0 ($0 / free tier limits: none; no paid plans or per-user subscription fees as of 2026-10-16).
- Self-host: yes, single Go static binary with embedded SQLite database, systemd/launchctl service, or Docker container (`ghcr.io/sortie-ai/sortie`) and Kubernetes Helm deployment; zero external database dependencies.
- License: Apache License 2.0 (permissive open-source).
- Activity: 203 GitHub stars, latest release v1.27.0 (released 2026-10-11), active maintenance.
- Pros: Zero-dependency single Go binary with embedded SQLite persistence; Seamless synchronization with production trackers (Linear, Jira, GitHub) without duplicating ticketing; Automatic exponential backoff retries, stall detection, and workspace git isolation; Granular token budget ceilings and real-time cost estimation per ticket.
- Cons: No built-in native Kanban column board in its own UI (delegates board visualization to Linear/Jira/GitHub); No native mobile apps or PWA install mode; HTTP server lacks built-in authentication by default (binds to localhost; requires reverse proxy for remote).
- Sources: https://docs.sortie-ai.com/, https://yetanotherorchestrator.app/apps/sortie/, https://docs.sortie-ai.com/reference/dashboard/, https://docs.sortie-ai.com/reference/http-api/, https://docs.sortie-ai.com/guides/monitor-with-logs/, https://github.com/sortie-ai/sortie, https://github.com/sortie-ai/sortie/releases
- Confidence: high (all 13 fields verified directly from official docs, architecture reference, and GitHub releases as of 2026-10-16)

### Agent of Empires
- URL: https://www.agent-of-empires.com/
- Kind: agent-native
- What: Agent of Empires (aoe) is an open-source Rust-based session manager and orchestrator for running parallel AI coding agents in tmux sessions across git worktrees and container sandboxes; KANBAN/board view of agent tasks: no (it provides a categorized list and matrix dashboard sorted by attention/status/repo rather than a Kanban column board, though sessions can be grouped, prioritized, and triaged).
- Agent integration: Direct CLI process and tmux multiplexing supporting Claude Code, OpenAI Codex, OpenCode, Gemini CLI, Cursor Agent, GitHub Copilot CLI, Pi, Mistral Vibe, Factory Droid, Hermes, Qwen Code, and Kiro CLI; Agent Client Protocol (ACP) adapter; HTTP REST API (`GET /api/sessions`, `POST /api/sessions/{id}/send`, `GET /api/sessions/{id}/output`); Webhook dispatch callbacks (`callback_url`); Agent Skills package discovery and synchronization (`SKILL.md`).
- Mobile: Installable PWA (Progressive Web App on iOS Safari and Android Chrome) and fully responsive mobile web dashboard with touch controls and mobile-optimized font scaling (no native iOS or Android App Store app; remote access via Tailscale Funnel or Cloudflare Tunnel with QR pairing; cite: https://www.agent-of-empires.com/guides/web-dashboard/ and https://www.agent-of-empires.com/guides/web/dashboard/).
- Pricing: Free and 100% open-source under MIT ($0 / completely free; experimental team control plane "CityHall" is also open-source; no paid tiers as of 2026-10-16).
- Self-host: yes, local Rust binary via Homebrew (`brew install aoe`) or shell script, local tmux backend, companion web server (`aoe serve`), and self-hostable team control plane server (`CityHall`); all session data stays local or on your servers.
- License: MIT License (permissive open-source).
- Activity: 3,356 GitHub stars, latest release v1.20.0 (released 2026-10-15), active development supported by Mozilla.ai.
- Pros: Extremely low resource footprint with keyboard-driven Rust TUI and tmux persistence; First-class mobile support via installable PWA with Tailscale Funnel / Cloudflare remote tunneling; Strict isolation via git worktrees and container sandboxing (Docker, Podman, Apple Containers); Direct subagent steerability via HTTP API and callback URLs without hijacking human focus.
- Cons: Lacks a traditional drag-and-drop Kanban column board view (relies on sidebar lists, triage buckets, and status glyphs); Windows support is restricted to WSL2 due to POSIX tmux dependency; Does not automatically create PRs or synchronize directly with external issue trackers like Jira/Linear out of the box (requires CityHall or external orchestrators).
- Sources: https://www.agent-of-empires.com/, https://yetanotherorchestrator.app/apps/agent-of-empires/, https://www.agent-of-empires.com/docs/quick-start/, https://www.agent-of-empires.com/guides/web-dashboard/, https://www.agent-of-empires.com/guides/web/dashboard/, https://www.agent-of-empires.com/docs/api/, https://github.com/agent-of-empires/agent-of-empires, https://github.com/agent-of-empires/agent-of-empires/releases
- Confidence: high (all 13 fields verified directly from official site, web guides, API documentation, and GitHub repository as of 2026-10-16)

## EXPAND
- LEAD: Vibe Kanban (https://yetanotherorchestrator.app/apps/vibe-kanban/) — WHY: Specifically branded as a dedicated kanban board for Claude Code and Codex coding agents with multi-platform desktop and web support — ANGLE: Search GitHub and web for vibe-kanban repository, architecture, and agent dispatch mechanism
