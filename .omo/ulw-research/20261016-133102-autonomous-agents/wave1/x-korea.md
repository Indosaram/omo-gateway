<!-- task=st_01a1450c -->
st_01a1450c [completed] model mahoquot/claude-sonnet-5-5
Ask: Korea research on autonomous personal agents (Pickle, Kakao, Naver, Wrtn, SKT A.), covering messenger/payment integrations and regulation. Wanted: a sourced report.
For you: Kakao, Naver, SKT and Wrtn are identified and sourced. "Pickle" is not resolved. web_search was not available here, so I used curl on official pages, the Seoul Economic Daily site and Google News RSS. Most press items are headline-only because I could not open the article bodies.
Now: report complete. Next: none.

## Identity
- **Kakao / PlayMCP**: Kakao Corp, https://playmcp.kakao.com. It is an MCP tool platform and playground. The beta opened 2025-08-13 per Platum. Confidence: high.
- **Kakao Kanana / KakaoTalk assistant**: the Kakao assistant is called "Ryan" in English press, 라이언 in Korean. "Kanana Space" and "Kanvas" were shown at if(kakao)26 on 2026-10-13/14. "AI for Everyone" is a consortium service launching in December 2026. Confidence: high on existence, medium on product names, since only headlines and one full article were read.
- **Naver**: NAVER Corp. Its agent surfaces are "AI탭" (AI tab), the Whale browser agent and the Place agent. Confidence: high.
- **Wrtn (뤼튼)**: Wrtn Technologies. The enterprise arm is "뤼튼AX". Confidence: medium-high.
- **SKT A. (에이닷)**: SK Telecom's assistant. Confidence: high.
- **"Pickle"**: UNRESOLVED. The only Korean hit was "피클플러스·에이아이브 분산 AI 에이전트 컨소시엄" (Pickle Plus), dated 2026-08-14. I found no evidence of a consumer "Pickle" agent. Confidence that it matches the intended entity: low.

## Findings
- PlayMCP is Kakao's MCP platform. A user's toolbox holds up to 10 MCP servers. A single OAuth2 "mcp-gateway" endpoint at `https://playmcp.kakao.com/mcp` proxies them. [https://playmcp.kakao.com/llms.txt]
- PlayMCP ships an agent-onboarding flow aimed at OpenClaw-style agents. The user clicks "OpenClaw와 연결" to get a one-time token (OTT). The agent runs `mcporter config add mcp-gateway https://playmcp.kakao.com/mcp --auth oauth --scope home`. It then POSTs the OTT to `/api/v1/auths/otts:exchange` for access and refresh tokens, and writes them to `~/.mcporter/credentials.json`. [https://playmcp.kakao.com/llms/mcp-connection-guide.md]
- Claude and ChatGPT connect to the same gateway through documented user-facing guides. [https://playmcp.kakao.com/llms.txt]
- Kakao says about 500 tools are registered on PlayMCP. Selected tools will be offered inside the new service after review. Long term, it wants usage to turn into revenue. [https://en.sedaily.com/technology/2026/10/14/kakao-to-launch-ai-for-everyone-service-in-december]
- "AI for Everyone" runs inside KakaoTalk or on the web. It is built by 34 organisations. LG Uplus supplies model and GPU operations, and LG AI Research supplies EXAONE. The target is a December 2026 launch and 5M monthly active users. A "one agent per person" feature is planned in stages from 2027. [https://en.sedaily.com/technology/2026/10/14/kakao-to-launch-ai-for-everyone-service-in-december]
- The service re-confirms the conditions and the amount with the user before a booking step. [https://en.sedaily.com/technology/2026/10/14/kakao-to-launch-ai-for-everyone-service-in-december]
- Headline only (MK, 2026-10-15): the KakaoTalk AI introduces a "personal-information level" scheme, where users share more data to get more help. [https://news.google.com/rss/search?q=카톡+AI+비서+라이언+개인정보+레벨+카나나+스페이스&hl=ko&gl=KR&ceid=KR:ko]
- Headline only: Kakao published "SPIRIT" autonomous-agent ethics principles on 2026-10-15, with a checklist to be applied within the year. Coverage describes five risk areas. [https://news.google.com/rss/search?q=AI+기본법+에이전트+개인정보위원회+에이전트+when:180d&hl=ko&gl=KR&ceid=KR:ko]
- Headline only: Chosun reports AI agents are good at shopping and poor at reservations, in a Muse vs Naver vs Kakao comparison (2026-10-13). [https://news.google.com/rss/search?q=Kakao+ChatGPT+KakaoTalk+agent+Kanana+when:180d&hl=en-US&gl=US&ceid=US:en]
- Naver's AI탭 passed 10M users 3 weeks after launch, per July 2026 coverage. Its framing is "search to execution", covering purchase and booking. [https://news.google.com/rss/search?q=네이버+에이전트+쇼핑+예약+네이버페이+when:120d&hl=ko&gl=KR&ceid=KR:ko]
- Headline only: Naver is withholding its shopping data from ChatGPT and Gemini (Business Post, 2026-10-05). A Whale-browser agent and a Place agent also appeared in that feed. [https://news.google.com/rss/search?q=네이버+에이전트+쇼핑+예약+네이버페이+when:120d&hl=ko&gl=KR&ceid=KR:ko]
- Headline only: CBC's "AI 결제 시대" piece says Naver and Meta are racing on AI payments, and consumers want the "final approval by my own hand" (2026-10-10). Yonhap asks whether you should hand an AI your card (2026-10-09). [https://news.google.com/rss/search?q=네이버+에이전트+쇼핑+예약+네이버페이+when:120d&hl=ko&gl=KR&ceid=KR:ko]
- Headline only (MK, 2026-09-11): the FSC (financial regulator) will ease the Credit Information Act and network-separation rules in stages for AI-agent payments. This is regulatory direction, not enacted text. [https://news.google.com/rss/search?q=AI+기본법+에이전트+개인정보위원회+에이전트+when:180d&hl=ko&gl=KR&ceid=KR:ko]
- Headline only (MK, 2026-10-15): the privacy regulator (PIPC) plans personal-information standards for agentic AI. Separately, the government plans data-handling standards for agents before the free "모두의 AI" service launches in December. [https://news.google.com/rss/search?q=AI+기본법+에이전트+개인정보위원회+에이전트+when:180d&hl=ko&gl=KR&ceid=KR:ko]
- The AI Basic Act took effect in January 2026. Bills to amend it numbered 17 to 19 by August 2026. [https://news.google.com/rss/search?q=AI+기본법+에이전트+개인정보위원회+에이전트+when:180d&hl=ko&gl=KR&ceid=KR:ko]
- A KAIST professor said at a 2026-10-15 seminar that prompt injection is the biggest agent security threat and is hard to block today. His example is a text message that hijacks an agent managing a phone. He recommended activity logging and accountability tracing, while keeping data minimisation. [https://en.sedaily.com/technology/2026/10/15/ai-agents-that-cannot-tell-commands-from-data-are]
- The same seminar scored Korea's personal-data "regulatory friction" at 70.4/100, level with China and above the EU's 63. In a survey, 78.8% of AI firms called consent compliance a heavy burden. [https://en.sedaily.com/technology/2026/10/15/ai-agents-that-cannot-tell-commands-from-data-are]
- SKT: "A. Biz Cowork" beta launched 2026-05-28. A. gained call-wait and calendar agent features (2026-06-30). The "1인 1 에이전트" ("one agent per person") strategy is stated. [https://news.google.com/rss/search?q=SK텔레콤+에이닷+에이전트+개인+when:365d&hl=ko&gl=KR&ceid=KR:ko]
- SKT discontinued "Ester", its global personal agent, in November 2025. Its AI-platform legal entity was reportedly liquidated in August 2026. [https://news.google.com/rss/search?q=SK텔레콤+에이닷+에이전트+개인+when:365d&hl=ko&gl=KR&ceid=KR:ko]
- Wrtn is pivoting to B2B. It reached a roughly 1 trillion won valuation (about 1.4 trillion won per Seoul Economic Daily). It signed an agent-development partnership with BC Card in August 2026 and picked IPO underwriters in September. Its AX CEO predicts computer-use agents in 2027. [https://news.google.com/rss/search?q=뤼튼+에이전트+when:120d&hl=ko&gl=KR&ceid=KR:ko]
- Not found: a public, official KakaoTalk API for a bot to read or write a user's personal chats. Treat the Kakao integration surface as PlayMCP, the Kakao Channel/chatbot business APIs, and "send to me" messaging. These are from my background knowledge, not verified this session. [https://developers.kakao.com]

## Implications for a self-hosted Rust agent gateway
1. **PlayMCP as an MCP upstream.**
   - Implement the OTT-exchange and OAuth refresh flow natively. Store tokens in the macOS Keychain instead of `~/.mcporter` plain JSON. This gives the daemon Kakao services (Melon, maps, and so on) without scraping.
2. **KakaoTalk is a delivery channel, not an inbox.**
   - Since no personal-chat read API was found, route the agent to the owner via a Kakao Channel bot or "send to me". Treat KakaoTalk ingestion as unsupported.
   - Ingest via Discord or iMessage instead, plus any user-exported chat files.
3. **Two-phase commit for spend and bookings.**
   - Copy Kakao's confirm-conditions-and-amount step. Any payment, booking or send action becomes a pending item that needs an owner tap on Discord or iMessage.
   - Korean consumer sentiment ("final approval by my hand") backs this.
4. **Prompt-injection lanes.**
   - Split lanes by trust level. Inbound messages, web pages and email are untrusted data lanes with no tool-write permissions. Owner commands run in a separate lane.
   - Log every tool call with its provenance, which matches the logging and accountability the KAIST talk recommended.
5. **Privacy "levels" for memory.**
   - Add tiered memory scopes (public, personal, financial) per tool. Reveal tier N only when the owner has granted it, similar to Kakao's level scheme.
6. **Naver surfaces.**
   - Naver is closing its shopping data to outside agents. Plan for an owner-session browser lane (headless Chrome logged in as the owner) rather than an API for Naver search and Naver Pay. Keep Naver Pay behind rule 3.
7. **Cron + inbox digests for Korean life admin.**
   - Examples are an SKT-style subscription and benefits calendar, delivery tracking and bill reminders. Parse them from SMS/email ingestion.
8. **Locale-aware model routing.**
   - Add a Korean-language eval and routing profile. Korean models such as EXAONE or Kanana can serve as cheap fallbacks, but I did not verify their API availability.

## CLAIMS
- CLAIM: PlayMCP exposes a Bearer/OAuth2 MCP gateway usable by external agents through a one-time-token exchange — RISK: normal — SOURCES: playmcp.kakao.com — PRIMARY: https://playmcp.kakao.com/llms/mcp-connection-guide.md
- CLAIM: About 500 tools are registered on PlayMCP, and Kakao plans a revenue model on top of them — RISK: normal — SOURCES: en.sedaily.com — PRIMARY: https://en.sedaily.com/technology/2026/10/14/kakao-to-launch-ai-for-everyone-service-in-december
- CLAIM: Kakao's "AI for Everyone" launches in December 2026 with a 5M MAU target, with a 34-organisation consortium including LG Uplus and EXAONE — RISK: normal — SOURCES: en.sedaily.com — PRIMARY: https://en.sedaily.com/technology/2026/10/14/kakao-to-launch-ai-for-everyone-service-in-december
- CLAIM: KakaoTalk AI introduces a personal-information level scheme — RISK: high — SOURCES: mk.co.kr, headline only — PRIMARY: none
- CLAIM: The FSC will relax Credit Information Act and network-separation rules in stages for AI-agent payments — RISK: high — SOURCES: mk.co.kr, headline only — PRIMARY: none
- CLAIM: The PIPC is drafting privacy standards for agentic AI — RISK: high — SOURCES: mk.co.kr, headline only — PRIMARY: none
- CLAIM: Naver will not share shopping data with ChatGPT or Gemini — RISK: high — SOURCES: businesspost.co.kr, headline only — PRIMARY: none
- CLAIM: SKT cancelled the global agent "Ester" in November 2025 — RISK: normal — SOURCES: etnews.com, mt.co.kr, headlines — PRIMARY: none
- CLAIM: The Korean AI Basic Act has been in force since January 2026 — RISK: normal — SOURCES: inews24.com, headline — PRIMARY: none
- CLAIM: No official public KakaoTalk personal-chat read API exists — RISK: high — SOURCES: none, prior knowledge and absence of evidence — PRIMARY: none
- CLAIM: "Pickle" is a Korean consumer agent — RISK: high — SOURCES: none found — PRIMARY: none

## EXPAND
- LEAD: Identify "Pickle". — WHY: not found, and the entity is named in the brief. — ANGLE: ask the requester for the URL or company. Otherwise search Pickle Plus (피클플러스), Pickle Robot, and the "Pickle" app-store listing.
- LEAD: Read the MK article on the KakaoTalk AI privacy "level" scheme and the Kakao SPIRIT ethics text. — WHY: these define what data the agent may access and by what consent. — ANGLE: fetch the kakaocorp.com press release and mk.co.kr/news; look at https://privacy.kakao.com.
- LEAD: Kakao Developers terms on talk messaging and chatbots for personal accounts. — WHY: settles the KakaoTalk channel claim. — ANGLE: https://developers.kakao.com docs on Kakao Talk Message API and Kakao i Open Builder.
- LEAD: The FSC and PIPC primary documents, from fsc.go.kr and pipc.go.kr. — WHY: the payment and agent-data rules are known only from headlines. — ANGLE: search by Korean title; confirm whether any rule is enacted or only proposed.
- LEAD: Naver Whale and Place agent docs, and the Naver Pay agent-payment scheme. — WHY: determines whether a browser lane or an API path exists. — ANGLE: navercorp.com newsroom and the Naver developer docs.