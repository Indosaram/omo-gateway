### Agent Deck
- URL: https://github.com/asheshgoplani/agent-deck
- Kind: agent-native
- What: A high-performance terminal session manager and control plane built in Go (Bubble Tea + tmux) for orchestrating multiple concurrent AI coding agent sessions across git worktrees and docker sandboxes.
- Agent integration: Direct CLI process supervisor for Claude Code, Codex CLI, OpenCode, Gemini CLI, Pi; features MCP server toggle manager, automated state detection (idle, busy, waiting for input), interactive prompt injection (`agent-deck session send`), session forking with conversation history, and an autonomous Conductor loop.
- Mobile: responsive web (via builtin web server `agent-deck web` with mobile-accessible browser terminal UI, authentication tokens, and reverse proxy / Tailscale Serve support); also supports mobile notification and two-way steering via official Telegram and Slack bridge daemons; no native iOS/Android binary app (cite: README `agent-deck web` and Conductor Telegram bridge docs).
- Pricing: free / open-source free ($0, 100% free under MIT license, includes local token cost monitoring with user-configurable per-model rate cards and daily/weekly budget alerts; as of 2026-10-16).
- Self-host: yes (local CLI binary installed via Homebrew, Go install, or GitHub release binaries; runs locally with tmux and sqlite/file state under `~/.local/share/agent-deck`; optional local web daemon `agent-deck web --listen 0.0.0.0:8420`; no enterprise paywall).
- License: MIT
- Activity: 1,092 GitHub stars; active development with latest commit on 2026-10-14 and tag v1.9.73 (active release series).
- Pros: Rich Bubble Tea TUI with mouse drag, split views, and zero-latency session switching; Native Git worktree support with `.worktreeinclude` andMonorepo sparse-checkout; Autonomous 'Conductor' pattern for supervising worker agents and auto-escalating to mobile Telegram/Slack; Built-in MCP manager and model cost calculation dashboard.
- Cons: Requires tmux installed as an underlying prerequisite; Web UI requires manual daemon launch and firewall/Tailscale configuration for remote mobile access; Advanced features (sparse monorepos, Conductor setup) carry a steep configuration curve; No native cloud syncing across different developer workstations.
- Sources: https://github.com/asheshgoplani/agent-deck, https://raw.githubusercontent.com/asheshgoplani/agent-deck/main/README.md
- Confidence: high

### CCManager
- URL: https://github.com/kbwo/ccmanager
- Kind: agent-native
- What: A lightweight terminal UI session manager built with TypeScript and Ink (React in terminal) to supervise and switch between parallel AI coding assistants across Git worktrees without requiring tmux.
- Agent integration: CLI supervisor supporting Claude Code, Gemini CLI, Codex CLI, Cursor Agent (`cursor-agent`), Copilot CLI, Cline CLI, OpenCode, and Kimi CLI; detects states (idle, busy, waiting for input) via terminal output parsing; features configurable status-change hooks (`CCMANAGER_NEW_STATE` shell triggers) for webhooks/Slack/noti and experimental AI auto-approval.
- Mobile: none (desktop/terminal CLI only; no native iOS/Android app, no PWA, and no embedded web server; mobile status alerts are achievable only via user-configured external status change shell hooks pushing to Slack/Telegram/Pushover; cite: package.json and docs/status-hooks.md).
- Pricing: free / open-source free ($0, MIT licensed; as of 2026-10-16).
- Self-host: yes (local npm/bun package or precompiled standalone binaries for macOS, Linux, and Windows; completely local-first with state in `~/.config/ccmanager/config.json` and `.ccmanager.json`).
- License: MIT
- Activity: 1,264 GitHub stars; very active development with latest push on 2026-10-15 and release tag v4.4.7.
- Pros: Zero dependency on tmux (uses headless xterm serialization for completely self-contained terminal emulation); Native support for 8+ coding agents with agent-specific state detection strategies; Seamless Git worktree creation with background loading and `.worktreeinclude` support; Session data persistence and context transfer between branches; Easy extensibility via status lifecycle hooks.
- Cons: No built-in web or remote interface (strictly terminal local machine); No native mobile app or companion view; Lacks an MCP server management layer; Limited orchestrator capabilities (no multi-agent DAG or autonomous coordinator agent).
- Sources: https://github.com/kbwo/ccmanager, https://raw.githubusercontent.com/kbwo/ccmanager/main/README.md, https://raw.githubusercontent.com/kbwo/ccmanager/main/docs/status-hooks.md, https://raw.githubusercontent.com/kbwo/ccmanager/main/package.json
- Confidence: high

### Container-use
- URL: https://github.com/dagger/container-use
- Kind: agent-native
- What: An open-source MCP server and CLI tool powered by Dagger that provisions containerized, sandboxed development environments and git branches for coding agents working in parallel.
- Agent integration: Official MCP server protocol (`container-use stdio`) integrating with Claude Code (`claude mcp add container-use`), Cursor, Goose, VS Code, and any MCP-compatible coding agent; agents interact via MCP tool calls (`environment_run_cmd`, filesystem inspection) rather than host shell commands.
- Mobile: none (developer machine / CI daemon tool; no native mobile app, no PWA, no web dashboard; environments are inspected via host CLI `container-use log <id>` / `container-use checkout <id>`; cite: https://container-use.com and README).
- Pricing: free / open-source free ($0, Apache-2.0 licensed open-source project by Dagger; as of 2026-10-16).
- Self-host: yes (runs locally using Docker/Dagger container engine on macOS, Linux, and Windows; no SaaS dependency or enterprise feature tier).
- License: Apache-2.0
- Activity: 4,072 GitHub stars; active development with release v0.4.2 and commits in September/October 2026.
- Pros: Bulletproof container isolation preventing agents from damaging the host filesystem or modifying `.git` directly; First-class MCP server native architecture allowing agents to provision and manage their own ephemeral environments; Reproducible container stacks using Dagger engine; Seamless Git branch checkout to inspect or discard agent outcomes.
- Cons: Not a visual Kanban dashboard (CLI/MCP backend infrastructure rather than a status board UI); Requires a running Docker or container runtime with significant disk and memory overhead for parallel containers; Startup latency per agent environment compared to lightweight bare worktrees; Does not provide interactive terminal TUI multiplexing.
- Sources: https://github.com/dagger/container-use, https://raw.githubusercontent.com/dagger/container-use/main/README.md, https://raw.githubusercontent.com/dagger/container-use/main/rules/agent.md, https://container-use.com
- Confidence: high

### Uzi
- URL: https://github.com/devflowinc/uzi
- Kind: agent-native
- What: A high-throughput parallel AI coding agent CLI that orchestrates multiple autonomous agents across Git worktrees and tmux sessions with automated confirmation handling and dev-server port routing.
- Agent integration: CLI orchestrator supporting Claude, Codex, Aider, Cursor, and custom agent binaries; provides multi-agent prompt dispatch (`uzi prompt --agents claude:3,codex:2`), batch execution (`uzi run`), real-time status watch table (`uzi ls -w`), auto-approval daemon (`uzi auto`), and unified branch merge/checkpoint (`uzi checkpoint`).
- Mobile: none (terminal CLI and tmux only; no web UI, no PWA, no mobile app; cite: README and https://www.uzi.sh).
- Pricing: free / open-source free ($0, MIT licensed; as of 2026-10-16).
- Self-host: yes (local Go binary installed via `go install github.com/devflowinc/uzi@latest`; state stored in `~/.local/share/uzi`; requires local Git and Tmux).
- License: MIT
- Activity: 582 GitHub stars; stable/maintenance phase with latest tag v0.0.2 (initial release surge mid-2025; pushed_at 2025-06-04 / 2026 maintenance mirrors).
- Pros: Simple declarative parallel agent spawning across multiple agent flavors in one command; Built-in dev server port range allocation (`uzi.yaml` port management) for running live apps per agent; Automated prompt answering (`uzi auto`) to bypass manual confirmations; Integrated checkpoint and git rebase commands for harvesting agent PRs.
- Cons: Requires tmux preinstalled; Terminal-only tabular list (`uzi ls`) without a rich interactive TUI or GUI Kanban board; Development activity has slowed compared to newer tools like Agent Deck and CCManager; Lacks fine-grained MCP or token cost accounting.
- Sources: https://github.com/devflowinc/uzi, https://raw.githubusercontent.com/devflowinc/uzi/main/README.md, https://www.uzi.sh
- Confidence: high

## EXPAND
- LEAD: Claude Squad (https://github.com/smtg-ai/claude-squad) — WHY: Widely cited predecessor to CCManager and Agent Deck for tmux-based Claude Code multi-session orchestration — ANGLE: "claude-squad" github tmux session manager
- LEAD: AgentDeck by puritysb / darknecrocities / mantou132 — WHY: Alternate hardware/mobile-focused tools using the 'AgentDeck' name (Stream Deck integration, mobile ACP agent control from iOS/Android via private relay) — ANGLE: "mantou132/AgentDeck" OR "puritysb/AgentDeck"
- LEAD: Devflow / Trieve ecosystem around Uzi — WHY: Uzi was authored by Devflow/Trieve team for "asymmetric coding advantages", exploring how modern cloud agent workboards differ from local tmux launchers — ANGLE: "devflowinc" trieve uzi whitepaper
