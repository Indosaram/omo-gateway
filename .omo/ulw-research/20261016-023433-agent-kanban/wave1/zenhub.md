### ZenHub
- URL: https://www.zenhub.com
- Kind: general-kanban-with-agent-integration
- What: Agile project management and kanban platform built natively into GitHub workflows, providing multi-level project hierarchy, sprint planning, roadmaps, and automated reporting.
- Agent integration: Official public GraphQL API (https://api.zenhub.com/public/graphql) and deprecated REST API; community MCP servers (derekbar90/zenhub-mcp covering 54 GraphQL operations, zenhub-cli bundled FastMCP server); CLI tools (daniel-pittman/zenhub-cli `zh` with bundled Claude Code agent `agents/zenhub.md`, cou929/zhb, hiredgun/zenhub-cli); incoming/outgoing webhooks for real-time status updates across Claude Code, Cursor, Codex, and CI pipelines.
- Mobile: responsive web only (legacy standalone native iOS/Android mobile apps were discontinued and delisted from Apple App Store and Google Play Store).
- Pricing: Free tier ($0 for up to 50 users, 1 workspace, 2 connected repos, 250 issues); Teams tier ($8.33 to $12.50 per user/month paid annually/monthly with 14-day trial, up to 10 workspaces, 50 repos, 5,000 issues, AI labels/acceptance criteria/sprint reviews); Enterprise tier (custom quote with 60-day trial, unlimited workspaces/repos/issues, self-hosted on-prem option, SSO). Checked 2026-10-16.
- Self-host: yes (Enterprise plan only supports self-hosted on-premises deployment alongside GitHub Enterprise, in addition to cloud-hosted SaaS).
- License: proprietary (commercial SaaS and on-prem enterprise; open-source client libraries/CLIs under MIT).
- Activity: Active commercial SaaS (operated by Axiom Labs, Inc. DBA Zenhub; active developer GraphQL platform at developers.zenhub.com; legacy REST API deprecated in 2021).
- Pros: Deep bidirectional sync with GitHub issues and pull requests; Native 5-level planning hierarchy (Initiative, Project, Epic, Bug/Feature/Task, Sub-task); Automated AI sprint planning, label suggestions, and acceptance criteria; First-class support by community MCP servers and CLI tooling with Claude Code agents; Multi-repo kanban workspaces with real-time dependency tracking.
- Cons: Mobile experience lacks a dedicated native app; Self-hosting is locked strictly behind high-cost Enterprise tier; Dependent on GitHub authentication and OAuth sync tokens which can lapse; In-app ZenHub-only cards without underlying GitHub issues cannot be manipulated cleanly via standard GitHub-backed CLIs; Closed parent ticket handling can silently orphan sub-issues without health-checking tooling like `zh doctor`.
- Sources: https://www.zenhub.com, https://www.zenhub.com/pricing, https://developers.zenhub.com, https://developers.zenhub.com/graphql-api-docs/getting-started, https://github.com/ZenHubIO/API, https://www.zenhub.com/terms-of-service, https://github.com/daniel-pittman/zenhub-cli, https://github.com/derekbar90/zenhub-mcp
- Confidence: high (all pricing tiers, API specifications, MCP tools, and self-hosting constraints verified directly from official sites and GitHub repos; mobile app delisting verified via App Store/Play Store 404 responses).

### zenhub-cli (`zh`) & Bundled MCP Server
- URL: https://github.com/daniel-pittman/zenhub-cli
- Kind: agent-native
- What: A specialized CLI and Python FastMCP server tailored for AI coding assistants (Claude Code, Cursor, Codex) to inspect, manage, and automate ZenHub boards, sprints, and sub-issue hierarchies.
- Agent integration: Ships a Python FastMCP server (`mcp_server.py`) exposing ~68 native tools over stdio for Claude Code, Cursor, and Codex; bundles a drop-in Claude Code persona agent (`agents/zenhub.md`); drives ZenHub's GraphQL API directly for sub-issues/sprints while wrapping bash CLI commands; integrates sentence-transformers (`all-MiniLM-L6-v2`) semantic similarity search to prevent duplicate ticket generation by AI agents.
- Mobile: none (terminal CLI and local MCP daemon intended for developer workstations and agent runtimes).
- Pricing: free / open-source ($0, MIT licensed).
- Self-host: yes (local CLI script and local Python MCP server executed on host machine; requires bash 4.0+, git, gh CLI, jq, python 3.10+).
- License: MIT License.
- Activity: 7 GitHub stars, actively maintained with releases through v2.4.0 (2026-08-23; latest push 2026-09-09).
- Pros: Native FastMCP server with 68 tools purpose-built for agent backlogs; Local sentence-embedding duplicate detection prevents agent ticket bloat; Strict safety controls preventing silent orphan sub-issues on closed parents (`blocked_closed_parent=True`); Direct GraphQL integration for untruncated sprint and sub-issue data; Asymmetric verification cross-checking ZenHub issue counts against real GitHub repository counts.
- Cons: Does not address ZenHub-only cards (requires cards backed by real GitHub issues); Requires authenticated GitHub CLI (`gh`) and both GraphQL and REST ZenHub tokens; Initial cold-start embedding model download requires ~500MB disk space and Python 3.10+; Requires local clone or PATH installation.
- Sources: https://github.com/daniel-pittman/zenhub-cli, https://raw.githubusercontent.com/daniel-pittman/zenhub-cli/develop/README.md, https://raw.githubusercontent.com/daniel-pittman/zenhub-cli/develop/CLAUDE.md, https://api.github.com/repos/daniel-pittman/zenhub-cli/releases
- Confidence: high (verified repository code, releases up to v2.4.0, CLAUDE.md agent protocol, and test suite).

### zenhub-mcp (TypeScript)
- URL: https://github.com/derekbar90/zenhub-mcp
- Kind: agent-native
- What: A TypeScript-based Model Context Protocol (MCP) server providing standard MCP access to ZenHub's GraphQL API for tools like Claude Desktop and Cursor.
- Agent integration: Provides 54 MCP tools covering issue creation, movement across pipeline columns, assignees, labels, estimations, workspace management, and sprint creation; executes raw and structured GraphQL operations via POST to https://api.zenhub.com/public/graphql; compatible with Claude Desktop and Cursor via stdio configuration.
- Mobile: none (Node.js daemon designed for local workstation agent execution).
- Pricing: free / open-source ($0).
- Self-host: yes (runs locally as an npm/npx package or Node.js process).
- License: proprietary (no explicit open-source license file committed to repo, public GitHub repository).
- Activity: 1 GitHub star, last pushed 2025-07-01 (created 2025-06-16).
- Pros: Covers 54 of 176 ZenHub GraphQL operations out of the box; Direct npx and Claude Desktop configuration support; Full arbitrary GraphQL query tool (`zenhub_query`) available as escape hatch; Simple TypeScript modular tool design.
- Cons: Lacks formal open-source license declaration; No built-in semantic duplicate detection or agent safety guards; Stale maintenance (last push July 2025); Does not include high-level hierarchy health checks like closed-parent orphan verification.
- Sources: https://github.com/derekbar90/zenhub-mcp, https://raw.githubusercontent.com/derekbar90/zenhub-mcp/main/README.md, https://api.github.com/repos/derekbar90/zenhub-mcp
- Confidence: high (verified source code, tool manifest, and commit history).

## EXPAND
- LEAD: GitHub Projects (v2 / Project Boards) with GitHub CLI & GitHub MCP — WHY: ZenHub is tightly bound to GitHub repositories; GitHub Projects v2 serves as the direct alternative with official GitHub MCP servers and built-in CLI support — ANGLE: search for "GitHub Projects MCP server" and compare latency/features against ZenHub GraphQL.
- LEAD: Linear + Linear MCP — WHY: Linear is frequently evaluated alongside ZenHub for fast-moving AI engineering teams and has mature first-party MCP tools — ANGLE: examine Linear MCP tools for sub-issues and sprint board automation.
