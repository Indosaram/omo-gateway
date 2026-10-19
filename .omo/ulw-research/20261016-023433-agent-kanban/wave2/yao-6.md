### Nimbalyst
- URL: https://nimbalyst.com/
- Kind: agent-native
- What: Open-source visual desktop and mobile workspace for orchestrating AI coding agents in parallel git worktrees, featuring WYSIWYG editors for markdown, mockups, diagrams, spreadsheets, and code alongside inline diff reviews. Has a KANBAN/board view of agent tasks: yes (includes both a visual Session Kanban board tracking parallel agent sessions across phases like backlog, planning, implementing, and complete, as well as an integrated collaborative task board).
- Agent integration: Native agent harnesses for Claude Code (via Claude Agent SDK) and OpenAI Codex CLI/SDK; alpha support for OpenCode, GitHub Copilot, and Gemini CLI; built-in MCP (Model Context Protocol) client and servers rendering tool outputs visually; custom extension SDK (`EditorHost` contract); plain-text files and task trackers kept directly in git repos.
- Mobile: Native iOS companion app (written in SwiftUI, MIT-licensed, tested via TestFlight/App Store); responsive web console (console.nimbalyst.com) and local wiki mobile web view; Android mobile support is also actively released as an Android client.
- Pricing: Free for individuals (core app is completely free and open source with no feature limits or trial periods); Nimbalyst Teams multiplayer cloud collaboration is $20 per user per month (free during beta as of 2026-10-16). Users supply their own API keys or subscriptions for Claude Code / Codex.
- Self-host: yes (desktop/mobile app operates 100% locally on files and git worktrees with zero cloud dependency; real-time collaboration sync server runs on Cloudflare Workers / Durable Objects with wire-format types open-sourced in `packages/collab-protocol/`).
- License: MIT License (desktop app and iOS app repository licensed under MIT).
- Activity: Active GitHub repo (`Nimbalyst/nimbalyst`); latest releases in October 2026 include v0.81.6 (2026-10), v0.81.5, v0.81.0, and v0.80.6 (2026-10-09).
- Pros: True visual WYSIWYG workspace with first-class markdown, Excalidraw, and Mermaid editors; Dedicated session and task kanban boards showing real-time agent progression and status; Native iOS companion app for checking session status, reviewing diffs, and responding to approval prompts on the go; Local-first, open-format architecture keeping code, specs, and tasks as plain files in git worktrees; Full multiplayer team support with real-time cursors and document collaboration.
- Cons: Focuses heavily on Claude Code and Codex, with OpenCode/Copilot still in alpha; Requires users to manage their own provider API keys and subscriptions; Multi-repo projects have complex setup when syncing worktrees across disparate services; Team collaboration server backend requires Cloudflare Workers infrastructure.
- Sources: https://yetanotherorchestrator.app/apps/nimbalyst/, https://nimbalyst.com/, https://nimbalyst.com/open-source/, https://nimbalyst.com/features/session-management/, https://nimbalyst.com/features/task-management/, https://nimbalyst.com/teams/, https://github.com/Nimbalyst/nimbalyst, https://github.com/Nimbalyst/nimbalyst/releases
- Confidence: high

### Parallel Code
- URL: https://parallelcode.app/
- Kind: agent-native
- What: Free, open-source desktop application for orchestrating multiple AI coding agents in parallel, automatically isolating each task inside its own git branch and worktree with built-in diff reviews, inline commentary, mind maps, and change tours. Has a KANBAN/board view of agent tasks: no (uses a tiled multi-panel horizontal layout and single-task focus mode rather than a column-based kanban board; tracks tasks in a grouped list with Super Productivity integration).
- Agent integration: Deep built-in MCP server (`@modelcontextprotocol/sdk`) over stdio and HTTP exposing tools (`create_task`, `list_tasks`, `get_task_status`, `send_prompt`, `wait_for_idle`, `get_task_diff`, `merge_task`, `close_task`, `wait_for_signal_done`, `signal_done`, `land_self`, `mindmap_update`, `canvas_open`, `tour_publish`, `confidence_publish`, `submit_evidence`); supports CLI and native agents including Claude Code, OpenAI Codex CLI, Gemini CLI, GitHub Copilot CLI, and Google Antigravity (`agy`); bidirectional Super Productivity Local REST API integration.
- Mobile: Native Android app (`parallel-code-phone-*.apk` distributed via GitHub Releases and Obtainium); responsive mobile web view paired via QR code or connection token over local Wi-Fi or Tailscale Serve (supporting Web Push notifications on iOS 16.4+ via Add to Home Screen PWA).
- Pricing: Free and open-source (100% free under MIT license, no platform fees, zero telemetry/traffic proxying; users bring their own LLM API keys or local CLI credentials).
- License: MIT License.
- Self-host: yes (pure local desktop application distributed as macOS DMG, Linux AppImage/DEB, and source; runs entirely on the developer machine with optional local Docker sandboxing via project `.parallel-code/Dockerfile`).
- License: MIT License.
- Activity: Active GitHub repo (`johannesjo/parallel-code`) created by Johannes Millan; recent major release v4.0.0 (and v3.1.0) with active commits and releases in 2026.
- Pros: 100% free and open-source with no hosted subscription requirement; Comprehensive built-in MCP server enabling autonomous subagent creation, status reporting, and completion signals; Automatic git worktree isolation with symlinking of node_modules and Docker sandbox support; Excellent mobile pairing via native Android APK and Tailscale-compatible mobile web PWA with push notifications; Interactive change tours, mind maps, and AI Arena mode for benchmarking agents head-to-head.
- Cons: Lacks a kanban column board view (relies strictly on a horizontal tiled strip and sidebar list); Desktop app is currently limited to macOS and Linux (no native Windows release); Multi-agent coordination requires managing local compute and memory resources when running 5+ concurrent agents.
- Sources: https://yetanotherorchestrator.app/apps/parallel-code/, https://parallelcode.app/, https://github.com/johannesjo/parallel-code, https://github.com/johannesjo/parallel-code/releases, https://raw.githubusercontent.com/johannesjo/parallel-code/main/electron/mcp/server.ts, https://raw.githubusercontent.com/johannesjo/parallel-code/main/src/components/TilingLayout.tsx
- Confidence: high

### Proliferate
- URL: https://proliferate.com/
- Kind: agent-native
- What: Open-source (AGPL-3.0) AI agent IDE and control plane (YC S25) for managing parallel coding agents across isolated git worktrees locally or in cloud sandboxes with automated workflows and diff reviews. Has a KANBAN/board view of agent tasks: no (provides a recency-grouped Command/filter list view and sidebar tree for workspaces, alongside workflow execution builders and chat tabs, without a kanban column board).
- Agent integration: Built-in AnyHarness runtime (`@anyharness/sdk`) and Workspace MCP adapter; supports native agent harnesses for Claude Code, OpenAI Codex, OpenCode, Cursor, and Grok; REST/FastAPI control plane API with Celery background workers and LiteLLM model gateway proxy (`/llm`); automated workflows triggered by schedules or events.
- Mobile: Expo-based mobile app scaffold with React Native (`apps/mobile`), responsive mobile web interface, and native mobile auth helper (via ngrok and Expo Go/iOS simulator).
- Pricing: Free tier / open-source free (AGPL-3.0); paid upgrades and per-seat cloud plans for enterprise cloud orchestration and team billing.
- Self-host: yes (comprehensive self-hosting stack via Docker Compose with Caddy, PostgreSQL 16, LiteLLM gateway, and FastAPI control plane; automated one-click AWS CloudFormation stack on EC2; desktop app points to self-hosted control plane via `~/.proliferate/config.json`).
- License: AGPL-3.0 (GNU Affero General Public License v3.0).
- Activity: Active GitHub repo (`proliferate-ai/proliferate`) with daily release train (v0.4.29 on 2026-08-27, v0.4.30 in package manifests) and over 516 GitHub stars.
- Pros: Complete enterprise self-hostable control plane with Docker Compose and 1-click AWS CloudFormation templates; Bundled LiteLLM proxy gateway for central model credential management and cost control; Support for both local git worktrees and isolated cloud sandboxes (via E2B); Automated background workflows and scheduled agent sweeps; Native AnyHarness SDK standardizing agent operations across Claude Code, Codex, OpenCode, and Cursor.
- Cons: AGPL-3.0 copyleft license may restrict proprietary enterprise distribution without a commercial license; No kanban board view (workspace organization is limited to grouped command lists and sidebars); Full-stack self-hosting requires significant infrastructure (Postgres, Caddy, Python FastAPI, LiteLLM, Redis).
- Sources: https://yetanotherorchestrator.app/apps/proliferate/, https://proliferate.com/, https://github.com/proliferate-ai/proliferate, https://github.com/proliferate-ai/proliferate/releases, https://raw.githubusercontent.com/proliferate-ai/proliferate/main/guides/deploying/self-hosted-deploy.md, https://raw.githubusercontent.com/proliferate-ai/proliferate/main/apps/packages/product-client/src/pages/WorkspacesPage.tsx
- Confidence: high

## EXPAND
- LEAD: Supacode and Superset — WHY: Both are listed alongside Proliferate on yetanotherorchestrator.app as dedicated multi-agent desktop orchestrators that may have alternative board/kanban implementations for agent task tracking — ANGLE: fetch https://yetanotherorchestrator.app/apps/supacode/ and https://yetanotherorchestrator.app/apps/superset/
