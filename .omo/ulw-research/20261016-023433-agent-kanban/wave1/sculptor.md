### Sculptor (by Imbue)
- URL: https://imbue.com/product/sculptor
- Kind: agent-native
- What: An open-source desktop workspace for orchestrating and monitoring parallel coding agents in isolated Git worktrees and containers, featuring unified diff reviews, live pairing, and multi-agent lifecycle controls.
- Agent integration: Official CLI (`sculpt` with JSON schemas, `sculpt agent status`, `sculpt run`, `sculpt agent send`), local REST API (OpenAPI spec exposed at `http://localhost:5050` or custom port with auto-generated Python client), environment variables injected into agent shells (`SCULPT_AGENT_ID`, `SCULPT_WORKSPACE_ID`, `SCULPT_PROJECT_ID`, `SCULPT_API_PORT`), runtime JavaScript/TypeScript UI extension plugins, and custom MCP server integration on the roadmap; natively integrates with Claude Code, Pi agentic harness (pi.dev), OpenAI Codex, and any terminal-based agent process.
- Mobile: none (desktop-only Electron application supporting macOS Apple Silicon and Linux; official FAQ explicitly states "No Windows or mobile support currently", citing https://imbue.com/product/sculptor)
- Pricing: free and open-source (100% free with no subscription or token markups; users bring their own LLM API keys or Claude/Claude Max plan; verified 2026-10-16)
- Self-host: yes (runs locally on developer machine via Electron desktop app; supports containerized backend via Docker recipe in `container/recipes/docker/` or remote backend connected via custom backend command over SSH/network)
- License: MIT License (https://github.com/imbue-ai/sculptor/blob/main/LICENSE)
- Activity: 239 GitHub stars, last commit 2026-10-15 (pushed 2026-10-15T14:32:57Z), 30 releases (latest v0.48.0 on 2026-09-21); active research preview / public beta
- Pros: True local isolation per agent via Git worktrees or Docker containers; First-class multi-agent supervision with live diffs, tool call inspection cards, and turn status; Bring-your-own-model without vendor lock-in (Claude, Pi, Codex, any terminal CLI agent); Scriptable agent orchestration via dedicated `sculpt` CLI and REST API with JSON schemas; Extensible UI via modular JavaScript/TypeScript extensions and custom skills.
- Cons: No mobile app or responsive mobile web interface (desktop Electron only); Windows support is still pending/unreleased (macOS Apple Silicon and Linux only); Multi-agent collaboration currently shares workspace files without internal file locking; External contribution throughput is restricted by the team's internal bandwidth.
- Sources: https://imbue.com/product/sculptor, https://imbue.com/blog/sculptor-announce, https://imbue.com/blog/sculptor, https://github.com/imbue-ai/sculptor, https://raw.githubusercontent.com/imbue-ai/sculptor/main/README.md, https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/getting_started.md, https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/workspaces.md, https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/integrated_harnesses.md, https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/experimental/container_backend.md, https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/extensions.md, https://raw.githubusercontent.com/imbue-ai/sculptor/main/sculptor/sculptor-plugin/skills/sculpt-cli/SKILL.md, https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/development/cli.md
- Confidence: high

### mngr (by Imbue)
- URL: https://imbue.com/product/mngr
- Kind: agent-native
- What: Imbue's Unix-style CLI tool and agent manager ("git for agents") to launch, monitor status, snapshot, message, and scale dozens of coding agents across local machines, Docker containers, and cloud sandboxes.
- Agent integration: Native CLI (`mngr list`, `mngr create`, `mngr status`, `mngr message`, `mngr ask`, `mngr connect`), JSON output flags, direct SSH session orchestration, and extensible plugin architecture for custom agent providers (supports Claude Code, Codex, OpenCode, and arbitrary shell agents).
- Mobile: none (command-line CLI tool for Unix/macOS/Linux environments; accessible remotely only via SSH from mobile terminal apps like Termius, but has no mobile app, PWA, or web dashboard, citing https://imbue.com/product/mngr)
- Pricing: free and open-source (100% free MIT licensed tool; compute and model inference costs billed directly to user's cloud/LLM provider such as Modal, Docker host, or Anthropic/OpenAI; verified 2026-10-16)
- Self-host: yes (runs locally on developer machine or on self-hosted Docker containers, remote servers over SSH, and Modal sandboxes)
- License: MIT License (https://github.com/imbue-ai/mngr)
- Activity: 423 GitHub stars, last commit 2026-10-15 (pushed 2026-10-15T23:29:54Z); active development
- Pros: Lightweight CLI approach adhering to Unix philosophy with minimal overhead; Scales agents seamlessly across local, Docker, and remote cloud sandboxes (Modal); Direct programmatic status inspection (`mngr list`) and interactive message passing; Agent snapshotting, branching, and state forking modeled after Git; No vendor lock-in to proprietary cloud dashboards or subscription services.
- Cons: No graphical kanban board or visual drag-and-drop dashboard out of the box (terminal output and text listings only); Lacks native mobile web or app interface; Requires terminal proficiency and shell scripting for complex workflow pipelines; Visual diff and conflict resolution must be delegated to external git/editor tools.
- Sources: https://imbue.com/product/mngr, https://github.com/imbue-ai/mngr, https://raw.githubusercontent.com/imbue-ai/mngr/main/README.md
- Confidence: high

## EXPAND
- LEAD: Conductor vs Sculptor workflow differences — WHY: Sculptor's official FAQ specifically compares itself against Conductor for parallel agent workflows with CI Babysitter and TDD skills — ANGLE: compare Sculptor and Conductor on parallel worktree management and CI failure automated dispatch
- LEAD: Imbue Blueprint and Vet — WHY: Imbue offers companion tools in the same agent ecosystem (Blueprint for planning copilot and Vet for catching agent errors) that feed status into Sculptor — ANGLE: explore how Blueprint and Vet integrate with Sculptor and mngr for end-to-end task-board tracking
