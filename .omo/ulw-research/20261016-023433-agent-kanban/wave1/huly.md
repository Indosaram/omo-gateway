### Huly
- URL: https://huly.io
- Kind: general-kanban-with-agent-integration
- What: Huly is an open-source, all-in-one process, project, and knowledge management platform designed as a fast alternative to Jira, Linear, Slack, and Notion, featuring customizable kanban boards, real-time collaboration, and issue tracking.
- Agent integration: Community MCP server and CLI (@firfi/huly-mcp, @firfi/huly-cli supporting Claude Code, Codex, Cursor, Windsurf, OpenCode via stdio/HTTP transport), official TypeScript/Node.js REST and WebSocket API client (@hcengineering/api-client), webhooks, and an in-repo OpenAI-based conversational bot service (services/ai-bot, Hulia/MetaBrain assistant).
- Mobile: Responsive web (no native iOS app, no native Android app, no installable PWA manifest; official downloads cite only macOS, Windows, Linux desktop apps alongside web access at https://huly.io/download and docs).
- Pricing: Open-source self-hosted free ($0); cloud SaaS previously listed Common ($0/mo), Rare ($19.99/mo), Epic ($99.99/mo), Legendary ($399.99/mo) with resource-based compute/storage billing, but official hosted Huly cloud shut down due to funding discontinuation.
- License: Eclipse Public License 2.0 (EPL-2.0).
- Activity: ~17k+ GitHub stars on original repo hcengineering/platform (official upstream frozen/archived following hosted cloud shutdown; community fork actively maintained under Platform-Collective/platform with commits as recent as 2026-10-15; last release tag v0.7.423).
- Pros: Fast reactive UI built on Svelte with LexoRank-based smooth kanban reordering; comprehensive all-in-one suite unifying kanban, documents, chat, video (LiveKit), and calendar; rich ecosystem of ready-to-use AI agent tooling including feature-complete MCP (@firfi/huly-mcp) and CLI for automated workflows.
- Cons: Official upstream repository (hcengineering/platform) and hosted SaaS cloud shut down due to funding; heavy and complex self-hosting architecture requiring 15+ containers (MongoDB, Elasticsearch, CockroachDB/Postgres, Redpanda/Kafka, MinIO, LiveKit); lack of native mobile applications or mobile-optimized offline experience.
- Sources: https://github.com/hcengineering/platform, https://github.com/Platform-Collective/platform, https://huly.io, https://huly.io/pricing, https://huly.io/download, https://docs.huly.io/getting-started/self-host, https://github.com/hcengineering/huly-selfhost, https://github.com/dearlordylord/huly-mcp, https://github.com/hcengineering/huly-examples
- Confidence: high (all 13 fields corroborated directly against official GitHub repositories, documentation, and live source code; mobile app store absence verified via official download channels and responsive web UI analysis)

## EXPAND
- LEAD: Platform-Collective fork governance and release automation — WHY: hcengineering/platform officially declared itself frozen and redirected ongoing development to Platform-Collective/platform, where active commits continue through October 2026 — ANGLE: Monitor Platform-Collective/platform release tags and self-hosting compose artifacts for long-term project viability.
