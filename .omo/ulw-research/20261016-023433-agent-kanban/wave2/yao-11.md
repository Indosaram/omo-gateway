### AgentBox
- URL: https://agent-box.sh/
- Kind: agent-native
- What: AgentBox is an orchestrator that provisions and teleports AI coding agents into isolated, checkpointed Docker sandboxes or cloud microVMs with dedicated browsers, VNC, and full dev containers; it does not have a native kanban-column board view (has a dashboard TUI, native macOS menu bar app, and web hub with grouped project/box cards, queues, and task management).
- Agent integration: CLI (`agentbox claude`, `agentbox codex`, `agentbox opencode`, `agentbox pi`, `agentbox openclaw`), REST API (`/api/v1` on localhost:8787 or control box), in-box agent skill (`/agentbox` for Codex/Claude Code), host-tool proxy shims for CLI agents, SSE streams for agent events/approvals; agents include Claude Code, OpenAI Codex, OpenCode, Pi, and OpenClaw daemons.
- Mobile: responsive web only (accessible via web browser on port 8787 / remote control box / phone SSH keys via `agentbox connect --add-key` and `agentbox inbound open`; no native iOS or Android app).
- Pricing: free and open-source (Apache/MIT; local Docker is 100% free with no account/bill; cloud providers like Hetzner, Daytona, Vercel, DigitalOcean, and E2B use the user's own cloud provider API keys and usage-based billing).
- Self-host: yes (local Docker via Docker Desktop/OrbStack on macOS/Linux, remote SSH Docker server via `agentbox remote-docker`, or self-hosted control-plane hub on a Linux VPS / Vercel).
- License: MIT License (repo: madarco/agentbox).
- Activity: Active OSS project by Marco D'Alia (@madarco); npm package `@madarco/agentbox` (v0.0.0 monorepo root / active development in 2026); GitHub stars ~hundreds (active releases and nightly channel).
- Pros: True container/microVM sandbox isolation protects host filesystem and keys; Sub-second warm checkpoint restores and auto-pausing to conserve CPU/RAM; Supports both local hardware and multi-cloud providers (Hetzner, Daytona, Vercel, E2B, DigitalOcean); Dedicated GUI access via noVNC web streaming, portless local tunnels, and VS Code/Cursor Dev Containers; Safe git architecture keeping git credentials on host with explicit approval prompts.
- Cons: No visual drag-and-drop kanban board column view (uses list/card dashboard); Requires Docker Desktop or OrbStack running locally; Heavy initial image build (~1GB base image); Multi-machine setup and remote control box require networking/token configuration.
- Sources: https://agent-box.sh/, https://agent-box.sh/docs, https://agent-box.sh/docs/cli, https://agent-box.sh/docs/hub, https://agent-box.sh/docs/api, https://agent-box.sh/docs/core-concepts, https://github.com/madarco/agentbox, https://yetanotherorchestrator.app/apps/agentbox/
- Confidence: high

### dmux
- URL: https://dmux.ai/
- Kind: agent-native
- What: dmux is an agent multiplexer and terminal UI developed by FormKit that runs multiple AI coding agents and shell panes in parallel using tmux and isolated git worktrees; it does not have a kanban board view (it organizes tasks through a terminal sidebar list, tmux split panes, and worktree branches).
- Agent integration: CLI agent orchestration (spawns agent CLI processes directly inside dedicated tmux panes and git worktrees); supports 12+ CLI agents including Claude Code, Codex, Grok Build, OpenCode, Cline CLI, Gemini CLI, Qwen CLI, Amp CLI, pi CLI, Cursor CLI, GitHub Copilot CLI, and Crush CLI; inference providers configured for automated branch/commit/terminal naming via API keys or ChatGPT/Grok subscription logins; lifecycle hooks (`.dmux/hooks`).
- Mobile: none (terminal TUI inside tmux on macOS and Linux; documentation website at dmux.ai is responsive, but dmux tool itself runs strictly inside local terminal/tmux).
- Pricing: free and open-source (100% free under MIT License; users provide their own LLM API keys or Claude/Codex/Grok subscriptions for inference automation).
- Self-host: yes (local CLI tool installed via `npm install -g dmux` on macOS or Linux; requires Node.js >= 18, tmux >= 3.0, and git).
- License: MIT License (repo: standardagents/dmux / formkit/dmux).
- Activity: Active open-source project by Justin Schroeder, Boyd, and FormKit, Inc.; version 5.11.1 (latest 2026 releases); repository `standardagents/dmux` (formerly `formkit/dmux`).
- Pros: Complete git worktree isolation prevents branch and working copy conflicts across concurrent agents; Broad agent ecosystem support (12 CLI agents including Claude Code, Codex, Grok Build); Native macOS notification integration alerting when background agents need human input; Built-in read-only terminal file and diff browser (`f`); Two-phase automatic merge workflow with AI conflict resolution and conventional commit generation.
- Cons: No kanban column task board (relies on sidebar pane lists and tmux windows); Terminal-only workflow requiring familiarity with tmux and keyboard shortcuts; Limited Windows support (native Windows best as CLI only, requires WSL for full tmux); No native mobile companion or web dashboard interface.
- Sources: https://dmux.ai/, https://github.com/formkit/dmux, https://github.com/standardagents/dmux, https://yetanotherorchestrator.app/apps/dmux/
- Confidence: high

### Gas Town
- URL: https://github.com/gastownhall/gastown
- Kind: agent-native
- What: Gas Town is a multi-agent workspace manager and industrial coding factory created by Steve Yegge that coordinates fleets of 20-30+ concurrent AI coding agents with git-backed worktree hooks and Beads issue tracking; it does not feature a traditional drag-and-drop kanban board view (it utilizes a multi-panel terminal activity dashboard `gt feed`, a web dashboard via `gt dashboard`, and git-backed dependency issue graphs/convoys).
- Agent integration: CLI orchestration (`gt sling`, `gt mayor`, `gt prime`), Beads issue tracking integration (`bd` CLI with Dolt SQL server database backend), OpenTelemetry log/metrics emission (VictoriaMetrics/VictoriaLogs), and Claude Code hooks (`.claude/settings.json`, SessionStart, PreToolUse, PreCompact, Stop hooks; planned Claude Code Agent Teams integration); supports Claude Code, GitHub Copilot, Codex, Gemini CLI, Cursor, Kiro, Auggie, Amp, OpenCode, Pi, and OMP.
- Mobile: responsive web dashboard only (via `gt dashboard` served locally on default port 8080 using htmx; no native iOS or Android apps).
- Pricing: free and open-source (MIT License; self-hosted open-source software; users supply their own LLM seats/API subscriptions for Claude/Codex/etc.; commercial Beads Team Server / Gasworks enterprise tiers exist via gascity.com for VPC/managed deployment).
- Self-host: yes (local installation via Homebrew `brew install gastown`, Go binary, or Docker Compose using Dolt database server, tmux, and Beads).
- License: MIT License (repository: gastownhall/gastown).
- Activity: Highly active open-source project created by Steve Yegge under the `gastownhall` organization; rapid version development in 2026 with an active community and Discord (gastownhall.com / gascity.com).
- Pros: Industrialized multi-agent architecture with specialized roles (Mayor coordinator, Deacon supervisor, Witness monitors, Polecat workers, Refinery merge queue); Git worktree hooks ensure complete state persistence and crash recovery across agent restarts; Git-backed Beads issue graph and convoy system prevents context loss across agent sessions; Three-tier watchdog system detecting and triaging stuck/zombie agents at scale; Automated Bors-style bisecting merge queue (`the Refinery`) prevents broken main builds.
- Cons: No visual kanban column board (tracks work as convoys, beads graphs, and event feeds); High conceptual complexity and steep learning curve (Mayor, rigs, hooks, convoys, molecules, beads, dogs); Heavy prerequisite stack (Dolt SQL server, ICU4C headers, Beads, tmux, Go); Significant token consumption when running persistent coordinator loops.
- Sources: https://github.com/gastownhall/gastown, https://gastownhall.com, https://gascity.com, https://yetanotherorchestrator.app/apps/gastown/, https://steve-yegge.medium.com/welcome-to-gas-town-4f25ee16dd04
- Confidence: high

## EXPAND
- LEAD: OpenKanban (https://yetanotherorchestrator.app/apps/openkanban/ / https://github.com/openkanban) — WHY: While agentbox, dmux, and Gas Town are agent orchestrators without kanban column views, OpenKanban specifically implements a dedicated TUI kanban board for managing AI coding agents (Claude Code, OpenCode, Codex, Gemini, Aider) in git worktrees. — ANGLE: Search for OpenKanban GitHub repo, AGPL-3.0 features, and kanban card workflow for coding agents.
