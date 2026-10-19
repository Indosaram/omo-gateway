### Pane
- URL: https://runpane.com
- Kind: agent-native
- What: A keyboard-first desktop app and CLI orchestrator for running multiple AI coding agents in parallel, giving each task its own isolated git worktree; it does NOT have a Kanban/board view (work is managed via Panes, Sessions, and terminal split-views).
- Agent integration: Official stdio MCP server (`runpane mcp` / `npx runpane@latest mcp`), comprehensive REST/CLI contract with 77 commands (`runpane` npm/pip package), `runpane watch` daemon event stream cursor, and native support for Claude Code (`claude --dangerously-skip-permissions`), OpenAI Codex (`codex --yolo`), Cursor (`cursor-agent --force --trust`), OpenCode (`opencode --auto`), Aider, Goose, or any arbitrary CLI tool.
- Mobile: Responsive web app at `runpane.com/app` / self-hosted Remote Pane browser interface reachable over Tailscale or SSH relay; native iOS and Android apps are listed as "coming soon" (https://runpane.com, https://runpane.com/compare/vibe-kanban).
- Pricing: Free and open-source; no paid tiers, no seat fees, and no hosted cloud subscription (users supply their own agent LLM subscriptions or API keys, as of 2026-08-23 / 2026-10-16).
- Self-host: Yes; entirely local desktop application (macOS DMG, Linux AppImage/deb, Windows x64/ARM64) with an optional self-hosted headless Remote Pane daemon deployed on any remote server/VM over Tailscale Serve or SSH tunnels.
- License: AGPL-3.0 (source on GitHub at `greenfield-inc/Pane`).
- Activity: Active development; releases shipped almost daily (v2.4.x, latest doc revisions August-October 2026; GitHub repo greenfield-inc/Pane).
- Pros: True local-first architecture with zero mandatory account registration; deep git worktree automation isolating parallel agents; native integration with Claude Code, Codex, and Cursor bypassing approval friction; rich CLI/MCP integration and cursor-based event watching (`runpane watch`); cross-platform support covering macOS, Linux, Windows, and WSL.
- Cons: Lacks a Kanban/board interface for task tracking; desktop client memory footprint can be heavy (~780 MB idle); telemetry via PostHog enabled by default (though opt-out exists); native mobile apps not yet released in app stores.
- Sources: https://runpane.com, https://runpane.com/pricing.md, https://runpane.com/agents.md, https://runpane.com/docs/runpane-cli, https://runpane.com/.well-known/mcp/server-card.json, https://runpane.com/docs/security, https://runpane.com/compare/vibe-kanban, https://runpane.com/compare/superset, https://github.com/greenfield-inc/Pane
- Confidence: high

### Maude
- URL: https://maude.pilotdev.fr
- Kind: agent-native
- What: A mobile-first 24/7 AI coding agent supervisor and remote control that connects to user-owned VPS/servers over SSH/relay; it does NOT have a Kanban/board view (tasks are tracked through an inbox of permission prompts/questions, session lists, diff views, and file trees).
- Agent integration: Direct daemon-level execution on the remote host driving CLIs headlessly or interactively for 5 agents: Claude Code, OpenAI Codex, OpenCode, xAI Grok Build, and Google Antigravity CLI (`agy`), with multi-agent context handoff and support for permission modes (manual, accept edits, plan, auto, bypass).
- Mobile: Native iOS/iPadOS app on Apple App Store (requires iOS 15.0+, iPadOS 15.0+, macOS 12.0+ with M1), native Android app on Google Play Store (`com.pilotdev.maude`), plus tablet-optimized split view (https://apps.apple.com/app/id6759869262, https://play.google.com/store/apps/details?id=com.pilotdev.maude).
- Pricing: Paid proprietary app with free trial: $2.99/week (3-day free trial) or $29.99/year (7-day free trial), plus a newly introduced Lifetime plan ($2.1.0); users bring their own server ($4-$8/mo VPS) and existing agent subscriptions (checked 2026-10-01 / 2026-10-16).
- Self-host: Partial (hybrid); the execution daemon is self-hosted on the user's Linux/macOS SSH server (installed automatically by Maude), while app licensing, push notifications, and E2E encrypted connection relays pass through Pilot Dev's infrastructure.
- License: Proprietary (built by independent developer Pilot Dev / Lucas Bastien).
- Activity: Active; Maude 2.0 released late 2026 with recent v2.1.0 update in mid-October 2026 on the iOS App Store.
- Pros: True 24/7 background agent execution without keeping a laptop open; native iOS/Android push notifications with 1-tap Allow/Deny buttons; zero manual server setup (auto-provisions daemon and agent CLIs via SSH); support for switching/handing off tasks between 5 major agent ecosystems; end-to-end encrypted relay.
- Cons: No Kanban or visual card-based task board; proprietary subscription required ($29.99/year or $2.99/week); requires bringing your own VPS/remote server; does not expose a public MCP/REST API for third-party orchestration.
- Sources: https://maude.pilotdev.fr, https://apps.apple.com/app/id6759869262, https://play.google.com/store/apps/details?id=com.pilotdev.maude, https://maude.pilotdev.fr/compare/claude-code-vs-codex-vs-opencode/, https://maude.pilotdev.fr/compare/maude-vs-claude-code-remote-control/
- Confidence: high

### CodeAgent Mobile
- URL: https://www.codeagent-mobile.com
- Kind: agent-native
- What: A mobile supervision platform and remote control companion for AI coding agents that turns tickets or chats into agent runs; it includes a shared task graph panel called "Beads" that tracks agent issues (created, in progress, closed) in team spaces, but does NOT offer a traditional visual Kanban column board.
- Agent integration: IDE extensions for VS Code, Cursor, Windsurf, JetBrains (IntelliJ, Android Studio), and a terminal CLI (`codeam-cli` supporting ACP - Agent Client Protocol); integrates directly with Claude Code, OpenAI Codex, GitHub Copilot, Cursor, Windsurf (Cascade), JetBrains AI Assistant, Aider, CodeRabbit, and Kimi Code; connects to Jira, Linear, Azure DevOps, GitHub PRs, and Slack.
- Mobile: Native iOS and Android mobile apps (App Store and Google Play via companion links), along with a browser-based web dashboard at `codeagent-mobile.com/dashboard`.
- Pricing: Freemium; Free tier at $0/month (3 agent tasks/day, 1 active cloud workspace, 2 paired devices, 1 linked agent); Pro tier at $19.99/month or $191.99/year (~$16/month, unlimited tasks, up to 5 devices, 5 collaborators, self-hosted deploys, Jira/Confluence toolkit, audit logs); optional pay-as-you-go credits for managed cloud models starting at $10 packs (DeepSeek V4, Qwen3 Coder, Codex) (as of 2026-10-16).
- Self-host: Partial; Free tier runs on local machines or managed GitHub Codespaces; Pro tier unlocks the "Self-hosted plane" allowing deployment to user-owned servers, VMs, or Docker containers via `codeam-cli`; central backend relay and auth remain proprietary SaaS.
- License: Proprietary core service; companion CLI (`codeam-cli`) and IDE plugins are distributed as open-source client tools on GitHub/Marketplace.
- Activity: Active commercial product with 700+ developers, recent extension updates in October 2026 (VS Code extension updated 2026-10-03; JetBrains plugin v2.75.43; active Discord and GitHub issues roadmap).
- Pros: Seamless pairing via QR code to existing IDEs (VS Code/Cursor/JetBrains) or CLI; supports launching work directly from Jira/Linear tickets or Slack threads; PR Command Center allows reviewing full diffs and merging from mobile; team collaboration spaces with multi-agent turn handoff; optional voice supervision ("Talk" / "Compose").
- Cons: Task tracking uses a task graph/list ("Beads") rather than a traditional drag-and-drop Kanban board; free tier is heavily restricted (3 tasks/day); full self-hosting of the relay/platform is not possible; Pro tier is relatively expensive ($19.99/month).
- Sources: https://www.codeagent-mobile.com, https://www.codeagent-mobile.com/pricing, https://www.codeagent-mobile.com/faq, https://www.codeagent-mobile.com/terms, https://marketplace.visualstudio.com/items?itemName=CodeAgentMobile.codeagent-mobile, https://plugins.jetbrains.com/plugin/30697-codeagent-mobile/versions/stable/1181091
- Confidence: high

## EXPAND
- LEAD: Vibe Kanban (https://github.com/BloopAI/vibe-kanban) — WHY: Mentioned in Pane's competitive analysis as an actual Kanban board tool explicitly designed for agent worktrees and Claude Code/Codex, but Bloop sunsetted in April 2026 leaving it community-maintained. — ANGLE: Search "BloopAI/vibe-kanban" GitHub releases, forks, and modern community board status.
