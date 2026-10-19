# omon-gateway 자율형 에이전트 플랫폼화 구현 계획서

- 작성일: 2026-10-16
- 범위: Instinct, Pickle, OpenAI Dots, Meta Muse, xAI Grok Bot, Hark Pro/Handoff, 그리고 유사 플랫폼(OpenClaw, Hermes, Poke, ChatGPT Pulse/Cowork, Manus 등)을 조사해, omon-gateway를 같은 부류의 "상시 가동 자율 개인 에이전트"로 만드는 데 필요한 것을 정리했습니다.
- 근거 자료: `.omo/ulw-research/20261016-133102-autonomous-agents/` 아래에 원본 리포트가 있습니다. 1차 조사 46개, 확장 조사 14개, 축약 4개입니다. 모든 워커는 `claude-sonnet-5-5`(리서치)와 `claude-opus-5-5`(코드 매핑·축약)로 실행했습니다.
- 표기:
  - **[1차]** 공식 문서, 약관, 소스를 직접 읽은 것
  - **[2차]** 언론, HN, 블로그
  - **[미검증]** 단일 일화이거나 출처가 약한 것
  - **[V]** 이 저장소 소스를 직접 열어 확인한 file:line

---

## 1. 요약

**공통점.** 여섯 플랫폼은 거의 같은 골격을 갖고 있습니다.

1. **사용자마다 전용 실행환경**을 둡니다. Dots는 cloud PC, Muse는 Linux VM, Grok Bot은 Firecracker microVM, Hark는 요청마다 가상 컴퓨터, Instinct는 지속형 cloud computer와 자체 전화번호, Pickle은 Kernel 브라우저와 격리 workspace를 씁니다.
2. **선제적으로 움직입니다.** 스케줄, 이벤트, watcher, 아침 브리핑으로 사용자가 없을 때도 일하고, 알릴 가치가 있을 때만 말을 겁니다.
3. **커넥터와 자격증명은 모델 밖에 둡니다.** Muse는 surrogate token, Pickle은 Nitro Enclave의 `forward`, Dots는 Secure sign-in, Hark는 tokenized vault를 씁니다.
4. **승인 게이트가 있습니다.** 송금, 발송, 비밀번호 같은 비가역 동작은 반드시 사람이 승인합니다. 승인은 대화가 아니라 out-of-band 카드나 버튼으로 받습니다.
5. **메모리는 편집 가능한 Markdown과 인덱스**입니다. Muse의 `MEMORY.md`, `SOUL.md`와 야간 dream 작업, Grok Bot의 4계층 메모리가 예입니다.

**omon-gateway의 현재 위치.** 이미 강한 것은 장시간 작업 lane(검증된 완료 판정), cron과 자가 수리, inbox 수집·분류, judge(jev), 원격 omo 세션 제어입니다. 자율 플랫폼으로 가는 데 막히는 갭은 일곱 가지입니다.

- 이벤트 트리거가 없습니다.
- kibitzer는 제안만 하고 실행하지 않습니다.
- 전용 sandbox가 없습니다.
- 커넥터와 자격증명 계층이 없습니다.
- agent turn 승인이 사실상 꺼져 있고, 승인 큐가 지속되지 않습니다.
- 메모리가 세션 단위이고 lexical 검색뿐입니다.
- 양방향 채널은 Discord 하나입니다.

**권장 순서.** 안전장치를 먼저 깔고 자율성을 나중에 켭니다. 근거는 조사한 모든 실제 사고입니다. Grok Bot의 Slack 은행정보 유출, Muse의 Marketplace 무승인 수락, Instinct의 prompt injection 이메일이 모두 결정론적 경계가 없는 곳에서 났습니다.

1. **P0 안전 기반**: effect 분류, 승인 원장, 목적지 ID 고정, outbox, taint.
2. **P1 선제 엔진**: 이벤트 트리거, heartbeat, 알림 envelope, kibitzer 실행 연결.
3. **P2 자격증명·커넥터**: egress 프록시, Keychain, MCP.
4. **P3 메모리**: owner scope, bi-temporal facts, dream.
5. **P4 실행 격리**: Sandbox trait, Apple container, macOS VM.
6. **P5 멀티채널·음성**.
7. **P6 결제·에이전트 간 통신**.

---

## 2. 조사 대상 플랫폼 프로필

각 항목의 출처는 해당 bullet 끝에 붙였습니다. 상세 원본은 `reduce/r-platforms-a.md`, `reduce/r-platforms-b.md`와 `wave1/<platform>-*.md`, `wave2/*.md`에 있습니다.

### 2.1 OpenAI Dots

- **정체.** ChatGPT 안의 always-on 에이전트이고, 하나하나를 "dot"이라 부릅니다.
  - GPT-6 Astra로 동작합니다. 2026-09-29 DevDay에서 발표됐습니다 [1차] https://learn.chatgpt.com/docs/whats-new/devday-2026.md
  - "4,000개 앱 연결"은 1차 문서에서 찾지 못했습니다 [미검증].
- **채널.** ChatGPT, Slack, Teams를 씁니다. 미국 Pro 사용자는 SMS를 beta로 씁니다 [1차] https://help.openai.com/articles/20001530
- **트리거.** 저장된 스케줄, 지원되는 이벤트 모니터링, dot이 스스로 정한 follow-up이 있습니다 [2차] https://myclaw.ai/blog/dots-ai
  - 사용자가 없을 때 도는 "Proactive research"는 read-only 도구만 쓰고, Custom Rules로도 이 제한을 풀 수 없습니다 [1차] https://help.openai.com/articles/20001529
- **실행환경.** dot마다 cloud Linux와 Chrome이 있습니다 [2차].
  - 로컬 PC 접근은 opt-in입니다.
  - Activity view의 "Take over"와 "Return control"로 사람이 넘겨받습니다 [1차] https://help.openai.com/articles/20001529
- **자격증명.** Secure sign-in은 모델을 멈춘 상태에서 브라우저에 직접 값을 넣으므로, 자격증명이 context에 남지 않습니다 [1차] 같은 URL.
- **메모리.** ChatGPT memory와 dot의 자체 노트를 씁니다. 개별 기억은 지울 수 없고, dot 전체를 지워야 사라집니다 [1차] 같은 URL.
- **권한.** 네 가지 동작이 있습니다(묻지 않고 실행, 사전 승인된 경우 실행, 실행 전 확인, 사용자에게 넘김) [1차] https://help.openai.com/articles/20001530
  - 비밀번호 변경과 송금은 반드시 사용자가 넘겨받아 직접 합니다.
  - 초안 작성 요청은 발송 허가가 아닙니다.
  - Auto-review(별도 reviewer)는 결정론적 보장이 아니라고 문서가 스스로 밝힙니다 [1차] https://learn.chatgpt.com/docs/sandboxing/auto-review
- **가격.** Pro와 Business Premium에 포함됩니다. Pro 500은 $500/월입니다 [1차] devday-2026.md
  - 프롬프트 없이 하루 약 307k 토큰을 쓴 사례가 있습니다 [2차] https://www.theregister.com/ai-and-ml/2026/10/13/openais-dot-assistant-burned-300k-tokens-in-a-day-without-a-single-prompt/5302913
- **실패.** cloud 머신이 죽거나, 밤새 typing 표시만 띄우고 응답이 없었다는 보고가 있습니다 [미검증] https://news.ycombinator.com/item?id=49920558
  - 스케줄이 등록되지 않은 경우도 있었습니다 [2차] https://amplifying.ai/research/openai-dots-choose

### 2.2 Meta Muse

- **정체.** Meta Superintelligence Labs가 2026-09-08 미국에서 출시했습니다. 모델은 Muse Spark 1.3입니다 [1차] https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/
- **채널.** Muse 앱(iOS, Android, web)과 WhatsApp을 쓰고, Mac 앱도 있습니다 [1차] https://ai.meta.com/muse/
- **트리거.** 목표를 정하면 스케줄과 이벤트에 따라 계속 일하고, 의미 있는 변화나 승인이 필요할 때만 알립니다. 선제성 정도는 말로 조절합니다 [1차] https://introducing.muse.ai
  - "Upcoming" 화면에서 예약 작업을 보고, 작업마다 권한을 줍니다 [1차] https://www.meta.com/help/artificial-intelligence/1385290430137537/
- **실행환경.** 사용자마다 cloud Linux VM이 있고 `hatch` daemon이 `systemd-nspawn` 안에서 돕니다 [1차] http://security.muse.ai/
  - 컨테이너 밖에는 `hatch-authd`, Sentinel, `privsep` 프로세스가 따로 돌며, `SO_PEERCRED` Unix socket으로 통신합니다.
  - 브라우저는 accessibility tree를 보고, CDP는 바깥의 broker가 소유합니다.
- **자격증명.** 에이전트는 surrogate token만 봅니다. Sentinel이 승인하면 네트워크 경계에서 진짜 토큰으로 바꿉니다.
  - 이메일에서 OTP, 비밀번호 재설정 링크, magic link를 걸러냅니다.
  - 결제는 Stripe Link 일회용 카드로 하고, 가맹점·금액·만료가 묶입니다 [1차] http://security.muse.ai/
- **메모리.** 홈 디렉터리에 `SOUL.md`, `IDENTITY.md`, `USER.md`, `MEMORY.md`, `AGENTS.md`가 있습니다 [1차, 연구자 덤프] https://mouse.dev/blog/muse-runtime-export
  - `memory.claims`에 evidence, confidence, `supersedes_claim_id`를 둡니다.
  - 매시간 claim을 원본과 대조하고, 매일 밤 "dream" 작업을 돌립니다.
  - forget은 claim retraction과 인덱스 재빌드로 처리합니다.
- **권한.** 승인은 채팅이 아니라 클라이언트 UI의 결정론적 카드로 받으므로 injection으로 위조할 수 없습니다 [1차] https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse
  - 범위는 once, session, task, 기간 제한, perpetual 다섯 가지입니다.
  - 사용자 데이터를 읽은 프로세스는 자동 허용을 잃습니다(tainted egress) [1차] http://security.muse.ai/
- **가격.** 무료이고 사용량 한도와 유료 tier가 있습니다. tier 가격은 미확인입니다 [1차] https://ai.meta.com/muse/
  - 한 리뷰어는 월 $80를 냈습니다 [2차] https://sigh.dev/posts/metas-muse-is-fantastic-for-web-scraping/
- **사고.**
  - 로컬 앱이 transcription endpoint를 바꿔 계정 토큰을 탈취할 수 있는 0-day가 있었고, 12시간 만에 고쳤습니다 [2차] https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/
  - Marketplace 거래를 승인 없이 수락하고 주소를 넘겼다는 보고가 있습니다 [미검증] https://futurism.com/artificial-intelligence/metas-muse-ai-giving-users-home-addresses.md
  - 권한을 거부했는데도 Messages를 읽었다는 보고가 있습니다 [미검증, 기자 1인] https://appleinsider.com/articles/26/09/28/metas-new-ai-agent-blatantly-ignores-users-permissions
  - Amazon이 "unauthorized AI agent"로 차단했습니다 [2차].

### 2.3 xAI Grok Bot

- **정체.** "your team of always-on agents"를 내걸고 2026-08-11 beta로 나왔습니다. desktop과 iOS를 지원합니다 [1차] https://x.ai/news/introducing-grok-bot
- **채널.** desktop과 iOS가 같은 thread를 씁니다. 봇끼리 메시지를 주고받고 그룹 채팅에도 들어가며, Slack도 씁니다 [2차].
  - 메일함 name@mail.grokbot.com이 있지만, 메일이 와도 봇이 깨어나지 않습니다 [2차] https://shipmail.to/blog/grok-bot-email-on-your-own-domain
- **트리거.** 스케줄과 이벤트(Slack thread, GitHub PR, 다른 봇)로 움직입니다. 사용자가 한 번 하는 걸 보면 routine으로 저장합니다 [1차] https://x.ai/bot/guides/grok-bot-101
  - webhook은 `whsec_` secret, Standard Webhooks HMAC-SHA256, 5분 허용, 1MiB 상한, `202` 비동기 응답을 씁니다 [1차] https://docs.x.ai/grok/automations/webhooks
- **아키텍처.** Lee Robinson의 Stanford 강연 transcript 기준입니다 [1차] https://www.peterp.me/transcripts/how-grok-bot-works/
  - 오너 메시지만 실행 중인 턴을 끊고, 그룹·Slack·routine 메시지는 큐 뒤에 섭니다.
  - 트리거 router가 중복을 제거하고, 사용자를 방해할지 판단합니다.
  - Temporal durable workflow로, 크래시가 나면 마지막 단계부터 재개합니다.
  - 별도 모델이 모든 shell 명령을 사전 검토하는 autoreview가 있습니다. untrusted 입력은 지시로 취급하지 않습니다.
  - 비용이 싼 순서로 시도합니다: 대화, plugin/MCP/API, 웹 검색, 브라우저, 데스크톱 순입니다.
  - Firecracker microVM은 snapshot, sleep, resume을 지원합니다. agent loop는 VM이 아니라 서버에서 돕니다.
  - 캐시가 아직 따뜻할 때 미리 compaction합니다. 메모리는 4계층(핵심 사실, 일자 로그, 30~60분 scratchpad, grep 가능한 파일)입니다.
- **자격증명.** 앱에 한 번 로그인해 두고 씁니다. CAPTCHA와 2FA는 데스크톱을 사람에게 넘겨 해결하고, 비밀은 secure form으로 받습니다.
  - 봇 하나가 로그인하면 모든 봇이 그 세션을 공유합니다 [1차] grok-bot-101.
- **가격.** Premium $30, Super $100, Ultra $200 [1차] https://x.ai/bot. 흔히 인용되는 "$120"는 HN 댓글 하나뿐이라 미검증입니다.
  - API 크레딧 $50가 약 10분 만에 사라졌다는 보고가 있습니다 [2차] https://forum.cursor.com/t/anyone-used-grokbot-on-the-api-very-high-costs/169551.json
- **사고.** CFO 봇이 이름이 같은 Slack 채널에 개인 은행 보고서를 올렸고, xAI는 채널 사이 정보 이동에 허가를 받도록 수정했습니다 [2차] https://africa.businessinsider.com/news/my-personal-ai-agent-posted-my-bank-details-on-company-slack-its-made-me-rethink-how/hw9t2xn
  - `SAND_*` 토큰이 노출돼 Web Bot Auth 서명을 위조할 수 있었다는 분석이 있습니다 [미검증] https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/

### 2.4 Instinct (Spear Street Technology)

- **정체.** Spear Street Technology d/b/a Instinct이고, 창업자는 전 Sierra 연구원 Noah Shinn입니다. 2026년 8월 초대제로 출시해 바이럴이 됐습니다 [1차] https://instinct.com/privacy-policy, [2차] https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/
  - 펀딩 금액은 출처마다 다릅니다($250M, $350M, $1B Series C).
- **채널.** "no new interfaces"를 내걸고 SMS, WhatsApp, iMessage, 전화로만 씁니다. 대화는 하나의 연속 스레드입니다 [1차] https://instinct.com/ [2차] TechCrunch.
- **트리거.** 선제적인 전화·문자, 반복 루프, 매진 표 watcher가 있다고 합니다 [2차/미검증] https://www.vellum.ai/blog/official-instinct-breakdown, https://www.usecarly.com/blog/what-is-instinct-ai/
- **실행환경.** 자체 전화번호와 지속형 cloud computer를 씁니다. 다만 사용자 쿠키와 주거용 IP가 없어 CAPTCHA와 2FA 벽에 걸립니다 [2차].
- **자격증명.** Google Workspace는 OAuth로 연결합니다. 제3자 서비스는 사용자가 username과 password를 직접 맡깁니다 [1차] privacy-policy.
- **권한.** 체계적인 승인 UI가 없습니다. 약관상 사용자 대신 구속력 있는 계약을 맺을 수 있습니다 [2차] TechCrunch.
- **가격.** 비공개 액세스 기간이라 무료이고 가격은 미공개입니다. "$200-500/월"은 근거 없는 추정입니다.
- **사고.** 아래는 모두 TechCrunch 기사(위 URL)에 실린 사례입니다.
  - 확인 없이 이메일을 보냈습니다.
  - 이메일에 심은 prompt injection을 그대로 따랐습니다.
  - Google 연결을 끊은 뒤에도 받은편지함 요약이 남아 있었습니다.
  - Resy를 과도하게 폴링해 계정이 정지됐습니다 [2차].

### 2.5 Pickle (YC, "AI를 키우는 앱")

- **정체.** Pickle, Inc.이고 YC W25입니다. iOS 앱 `com.pickle.client`를 2026-08-10에 출시했으며 무료이고 EN/JA/KO를 지원합니다 [1차] https://www.ycombinator.com/companies/pickle-2, https://itunes.apple.com/lookup?id=6795790129&country=us
- **채널.** iPhone 앱 하나로 메시지, 음성, 전화를 다룹니다. 3D 캐릭터가 있고, 통화와 브라우저를 실시간으로 지켜보다 넘겨받을 수 있습니다.
- **트리거.** 메일과 캘린더를 보고 작업을 **제안**합니다(분실 택배, 항공 환불 등). watcher가 취소표를 잡고, 밤사이 탐색한 결과를 아침에 보고합니다 [1차] KR App Store, https://pickle.com/assets/privacy-policy-BtYR0eAZ.js
- **실행환경.** 격리 구역이 셋입니다 [1차] https://pickle.com/assets/security-n1i5zLW2.js
  - Nitro Enclave vault
  - 작업마다 닫히는 사용자별 Kernel 브라우저
  - vault에 닿을 수 없는 파일 workspace
  - 전화는 Telnyx로 겁니다.
- **자격증명.** 오픈소스 `credential-enclave`(Rust, Apache-2.0)를 씁니다 [1차] https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md
  - 마스터 키는 iPhone이 만들고, 최대 30일짜리 grant로 위임합니다.
  - `forward`는 allow-list에 있는 host와 path에만 토큰을 주입하고, 응답에 토큰이 비치면 `response_withheld`로 막습니다.
  - TOTP는 6자리 코드만 내보냅니다.
  - 감사 로그는 사용 **전에** S3 Object Lock(365일)에 hash-chain으로 기록하고, 기록이 안 되면 사용하지 않습니다(backlog 64건이 넘으면 `log_backlog`).
- **메모리.** 일기, 사용자 노트, 주제 노트, 요약이 있고 사용자가 편집·삭제할 수 있습니다. 음성 원본은 저장하지 않습니다.
- **권한.** 결제 전에 승인 카드(주문 내역, 총액, 버튼)를 띄우고, 금액이 오르면 다시 묻습니다. 카드 접근은 마지막으로 앱을 연 뒤 30일까지만 유효합니다 [1차] https://pickle.com/assets/faq-pFbILNx9.js
  - Pickle끼리 대화할 때 외부 상대를 응대하는 에이전트는 읽기와 대화 도구만 갖고, never-share 목록이 적용됩니다.
- **가격.** 무료와 Pickle Plus가 있고, 사용량은 compute 기반 "energy"로 계량합니다. 가격 숫자는 미공개입니다.
- **사고.** 앱 자체 사고 보고는 없습니다. 회사는 과거 "Pickle 1" AR 안경 사기 의혹을 받았습니다 [2차] https://daringfireball.net/linked/2026/01/03/dowd-pickle-expose

### 2.6 Hark (Hark Pro / Handoff)

- **정체.** Hark Labs이고 Figure AI의 Brett Adcock가 세웠습니다.
  - 2026-05-20 $700M Series A(밸류 $6B) [1차] https://hark.com/articles/hark-announces-usd700-million-fundraising-round
  - 2026-08-05 Handoff preview, 2026-10-06 Pro 출시 [1차] https://hark.com/articles/introducing-hark-pro
  - 하드웨어는 2027년 예정입니다.
- **채널.** iOS, Android, web만 지원합니다. 화면은 Home(Action Cards와 Panels), Chat, Projects입니다 [1차] https://hark.com/using-hark
- **트리거.** "proactive thinking"이 핵심입니다. 영화표를 산 뒤 주차를 제안하고 적절한 시점에 QR을 보내는 식입니다. 급식카드 잔액이 부족하면 감지해 충전합니다.
  - 자연어로 Scheduled Tasks를 만들고, Specialties로 워크플로를 학습합니다.
- **실행환경.** 요청마다 가상 컴퓨터(브라우저, 파일시스템, 터미널)를 띄우고 브라우저는 최대 6개입니다. DOM 대신 좌표로 조작하며 단계마다 약 10초 지연이 있습니다 [1차] https://hark.com/articles/introducing-hark-handoff
- **자격증명.** "Secured by Hark" vault가 토큰화를 한다고 주장합니다(vendor 주장). MCP는 client로만 지원합니다.
- **권한.** 약관상 기본값은 돈을 쓰거나 메시지를 보내기 전에 확인하는 것이고, 범주별 standing permission을 줄 수 있습니다 [1차] https://hark.com/terms
  - "default to consent"에서 시작해 사용자의 선택을 학습합니다 [1차] https://hark.com/security
- **가격.** Free(사용량 상한), Pro² $20, Pro³ $100입니다 [1차] https://hark.com/pricing
- **사고.** 출시 10일째라 독립 리뷰가 없습니다. 벤치마크 1위 주장은 독립 검증되지 않았습니다 [2차] https://www.rockingrobots.com/hark-unveils-handoff-an-ai-agent-that-completes-tasks-directly-in-a-browser/

### 2.7 비교표

| 항목 | Dots | Muse | Grok Bot | Instinct | Pickle | Hark |
|---|---|---|---|---|---|---|
| 실행환경 | cloud Linux+Chrome | 사용자별 Linux VM(nspawn) | Firecracker microVM | cloud PC+전화번호 | Kernel 브라우저+enclave | 요청별 가상 컴퓨터 |
| 채널 | ChatGPT·Slack·Teams·SMS(beta) | 앱·WhatsApp·Mac | desktop·iOS·Slack·메일 | SMS·WhatsApp·iMessage·전화 | iOS 앱(음성·전화) | iOS·Android·web |
| 선제성 | 스케줄·이벤트·read-only research | Goals·salience gate | routine·webhook·봇 트리거 | watcher·follow-up [2차] | 제안 우선·watcher | proactive thinking·단계별 전달 |
| 자격증명 | Secure sign-in | surrogate token | secure form+공유 로그인 | 비밀번호 위탁 | Nitro enclave `forward` | tokenized vault [주장] |
| 승인 | 4상태+Auto-review | Sentinel+5단계 범위 | 자연어 규칙+review agent | 사실상 없음 | 결제 카드·재확인 | ask/standing |
| 메모리 | 지울 수 없는 노트 | Markdown+claims+dream | 4계층 | 비공개 | 일기·노트 | portrait·forget |
| 가격 | Pro $100~500 | 무료+유료 | $30/$100/$200 | 무료(비공개) | 무료+Plus | 무료/$20/$100 |

---

## 3. 공통 설계 패턴 (우리에게 이식할 것)

상세 옵션 비교와 출처는 `reduce/r-cross.md`에 있습니다.

| 테마 | 채택할 패턴 | 출처 |
|---|---|---|
| Heartbeat | heartbeat를 파일이 아니라 cron row로 둡니다. 모델 호출 전에 skip 사유를 판정합니다(disabled, outside-hours, empty-scratch, busy, no-route, channel-not-ready). 할 일이 없으면 `NO_REPLY`로 끝냅니다. 발화는 먼저 기록하고(Hermes), 러너가 시작되지 않으면 되돌립니다. 이벤트 wake는 30초 간격, 분당 5회로 제한합니다. | [1차] https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/heartbeat.md, https://github.com/NousResearch/hermes-agent/blob/e6899c8090da3bd48c829893e4103ce989d3482d/hermes_cli/heartbeat.py |
| 큐·interrupt | 오너 메시지만 interrupt하고, cron·inbox·helper 결과는 비중단 큐로 보냅니다. 단계 로그를 append-only로 남기고 크래시가 나면 그 단계부터 재개합니다. | [1차] Grok Bot 강연 transcript |
| 트리거 | Standard Webhooks(HMAC, 5분, 1MiB, 202)를 받고, 트리거 router에서 dedupe합니다. | [1차] https://docs.x.ai/grok/automations/webhooks |
| 승인 | Effect 클래스를 `Read`, `Draft`, `Write`, `SendExternal`, `Spend`, `Irreversible`로 나눕니다. 결정은 `Allow`, `Ask`, `HandOff`, `Deny` 네 가지입니다. 승인은 action payload hash에 바인딩하고, 범위는 Once, Task, Until, Standing입니다. 응답이 없으면 deny하고, 승인은 out-of-band 카드로 받습니다. | [1차] Dots help 20001530, security.muse.ai, https://hark.com/terms |
| 목적지 | 불변 ID에 바인딩합니다. 이름이 여러 대상과 겹치면 fail-closed하고, 내보내는 동작은 outbox에 draft로 넣었다가 승인 후 보냅니다. | [2차] Grok Bot Slack 사고 |
| Taint | inbox, 웹, 이메일에서 읽은 lane은 자동 허용을 잃습니다. Rule of Two에 따라 untrusted, 민감, 상태 변경 세 가지를 한 세션에서 모두 갖지 못하게 합니다. | [1차] security.muse.ai, https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/ |
| 자격증명 | 에이전트에는 handle이나 surrogate만 줍니다. egress 프록시가 host와 path를 매칭해 진짜 값을 주입하고, 응답에 토큰이 비치면 차단합니다. 사용 전에 hash-chain 감사 로그를 씁니다. | [1차] credential-enclave, security.muse.ai |
| 메모리 | Markdown(`USER.md`, `MEMORY.md`, 일자 로그)을 두고, `facts` 테이블은 bi-temporal로 invalidate합니다(Graphiti). FTS5와 sqlite-vec를 쓰고, dream은 diff 제안으로만 반영합니다. | [1차] mouse.dev muse-runtime-export, `wave2/memory-impl.md` |
| 알림 | `notify`, `question`, `review` 타입 envelope을 쓰고, 선제성 다이얼과 salience gate를 둡니다. 억제한 알림은 로그로 남깁니다. | [1차] https://github.com/langchain-ai/agent-inbox, https://introducing.muse.ai |
| 실행격리 | `Sandbox` trait을 두고, 코딩은 Apple `container`, GUI는 Tart/Lume과 Cua로 합니다. seatbelt와 데몬 소유 egress 프록시를 씁니다. | [1차] https://github.com/apple/container, https://github.com/anthropics/sandbox-runtime |
| 결제 | 가맹점·금액·만료에 묶인 단일 사용 lease를 씁니다. 건당, 일, 30일 상한과 spend ledger를 두고, 금액이 오르면 다시 묻습니다. | [1차] Stripe agentic commerce (`wave2/payments-creds.md`), Pickle FAQ |

---

## 4. 게이트웨이 현재 역량 맵

`reduce/r-gateway.md`가 file:line 22개를 소스와 대조했습니다(22개 중 오류 2건). 아래 [V] 표시는 그중 확인된 근거입니다.

| 영역 | 현재 역량 | 근거 | 성숙도 |
|---|---|---|---|
| 장시간 작업 lane | 8상태 CAS state machine, CLI start/list/send/claim-done/close | `src/lanes/state.rs:9-77`, `src/lanes/cli.rs:24-76` [V] | 운영 |
| lane 완료 검증 | done-signal(merged/file/url/command)과 judge diff 검토, settle과 queued retry 연결 | `src/lanes/runner.rs:1033,1086,1093` [V] | 운영 |
| lane 자원 guard | 최대 6 lane, heavy 슬롯 1 | `src/lanes/guard.rs:10-15` [V] | 운영 |
| lane 턴 상한 | 기본 24, env `OMON_LANE_MAX_TURNS`로 조정 | `src/lanes/runner.rs:30,1005` [V] | 운영 |
| 선제 제안 (kibitzer) | inbox·lane 이벤트로 트리거, 30분 간격, 도구 없음, 제안만 함 | `src/lanes/kibitzer.rs:21-24,94,300` [V] | 부분 |
| cron | cron/interval/once, lease, catch-up, monitor, 스크립트, 침묵 토큰 | `src/cron/scheduler.rs:233` [V], `src/cron/executor.rs` | 운영 |
| cron 자가 수리 | 문제 분류, LLM 원인 진단, fix lane, owner 승인 토큰 | `src/cron/remediate.rs:299-395` | 운영 |
| inbox 수집 | mail, kakao, iMessage, discord 읽기 전용(SQL CHECK 4종) | `src/inbox/mod.rs:116`, `migrations/0034_inbox.sql:3` [V] | 운영(읽기만) |
| inbox 제안 | jev 분류, 제안을 DM "<n> 해"로 승인 | `src/inbox/proposals.rs:430-505` | 운영 |
| agent backend | omo app-server WS, steer, 데몬 자가 복구 | `src/agent/omo_backend.rs` | 운영 |
| 승인 (agent turn) | `APPROVAL_MODE != always`이면 자동 승인 | `src/agent/omo_backend.rs:1456-1470` [V] | **사실상 꺼짐** |
| 승인 (terminal) | hardline, deny glob, Tirith, Discord 버튼(120초) | `src/tools/terminal.rs:69` [V] | 운영 |
| 승인 allowlist | pattern_key, created_at만 있음(TTL·주체 없음) | `migrations/0007_approval_allowlist.sql:1-4` [V] | 빈약 |
| 승인 UI | Discord 구현 하나뿐 | `src/discord/approval.rs:177,239` [V] | Discord 한정 |
| 메모리 | 세션 단위 lexical 검색, agent용 쓰기 도구 없음 | `src/memory/store.rs:35,100,108-133` [V] | 부분 |
| 채널 | Discord만 양방향, cron 전송은 u64 ID만 | `src/dashboard.rs:557-565`, `src/cron/scheduler.rs:2357` [V] | 부분 |
| 실시간 음성 | listener 생성 코드 0건, 음성 메시지 STT만 | `src/discord/adapter.rs:1688-1693` [V] | 없음 |
| MCP | 클라이언트 목록 비어 있음 | `src/main.rs:1082` [V] | 없음 |
| 단일 owner | owner ID 하드코딩 | `src/inbox/proposals.rs:10`, `src/lanes/kibitzer.rs:21` [V] | 하드코딩 |
| 이벤트 유실 | lead가 바쁘면 시스템 이벤트를 버림 | `src/multiplexer/actor.rs:360-364` [V] | 결함 |
| 사용량 원장 | `model_usage` 테이블이 있지만 쓰는 코드 0건 | grep [V] | 없음 |
| dashboard | loopback 전용, 인증 없음 | `src/dashboard.rs:108-113` [V] | 로컬만 |

문서와 코드 불일치(후속 수정 대상):
- `docs/AGENTS_DELEGATE.md`는 settle, retry, kibitzer가 없다고 쓰지만 실제로는 모두 연결돼 있습니다.
- README는 Songbird 음성과 MCP를 기능으로 내세우지만 실제로 연결된 코드가 없습니다.

---

## 5. 갭 분석

| # | 갭 | 플랫폼 기준 | 영향 | 확장 지점 | DB 변경 |
|---|---|---|---|---|---|
| G1 | 트리거가 시간뿐(webhook·이벤트 없음) | Grok Bot webhook, Dots 이벤트 | 외부 사건에 반응할 수 없음 | `src/cron/scheduler.rs` wake Notify, `src/multiplexer/router.rs` | `triggers`, `trigger_events` |
| G2 | kibitzer가 제안만 함 | Hark proactive, Muse Goals | 선제 행동이 없음 | `src/lanes/kibitzer.rs:340-361` → `lanes::cli::start` | `kibitzer_state`를 migration으로 이관 |
| G3 | 바쁘면 이벤트를 버림 | Grok Bot 비중단 큐 | cron·선제 결과 유실 | `src/multiplexer/actor.rs:360-364` | `pending_events` |
| G4 | agent turn 자동 승인 | 전 플랫폼 승인 게이트 | 무인 운영 시 위험 | `src/agent/omo_backend.rs:1456-1470` | `approval_requests`, `approval_grants` |
| G5 | 승인이 동기식이고 Discord 전용, lane 안에서는 승인 요청 자체를 거부 | Muse out-of-band 카드, Dots hand-off | 무인 작업이 막히거나 위험 | `src/discord/approval.rs:239`, `src/lanes/runner.rs:661` | 위와 같음 |
| G6 | effect 분류와 목적지 바인딩 없음 | Grok Slack 사고 | 오발송·유출 | `src/tools/mod.rs`, outbox 신설 | `outbox`, `destinations` |
| G7 | taint 추적 없음 | Muse tainted egress, Rule of Two | prompt injection | `src/security/`, lane 프로필 | `lanes.taint`, `lane_profiles` |
| G8 | 커넥터, 자격증명, MCP 없음 | Muse surrogate, Pickle `forward` | 외부 서비스를 쓸 수 없음 | `src/main.rs:1082`, `src/tools/mcp.rs` | `connectors`, `credentials`, `mcp_servers` |
| G9 | 메모리가 세션 단위·lexical, 주입 없음 | Muse claims+dream, Grok 4계층 | 맥락이 단절됨 | `src/memory/store.rs:111`, `src/multiplexer/actor.rs:1344` | `episodes`, `facts`, `fact_fts`, `fact_vec` |
| G10 | 전용 실행환경 없음 | 전 플랫폼 VM·sandbox | 호스트 Mac에 위험 | `src/agent/omo_backend.rs:481-533`, `src/tools/terminal.rs` | `sandboxes` |
| G11 | 양방향 채널이 Discord뿐 | Instinct SMS/iMessage, Dots Slack | 채널 고착 | `src/dashboard.rs:557`, `src/cron/scheduler.rs:2357` | `identities`, 대상 테이블 |
| G12 | 실시간 음성 없음 | Pickle, Instinct 전화 | 음성 UX 불가 | `src/voice/pipeline.rs`, `src/voice/mod.rs` | (선택) `voice_sessions` |
| G13 | 비용·감사 원장 없음 | Dots 307k 토큰, Grok 비용 사고 | 폭주를 감지할 수 없음 | 백엔드 turn 완료 지점 | `model_usage` writer, `action_audit` |
| G14 | heartbeat 없음(work_heartbeat는 stale 감지 전용) | OpenClaw/Hermes heartbeat | 상시 자기점검 없음 | `src/cron/` 신규 job kind | `heartbeats` |
| G15 | 결제 경로 없음 | Muse Link, Pickle 카드, Hark | 구매 대행 불가 | 신규 `src/spend/` | `spend_leases`, `spend_events` |

---

## 6. 단계별 구현 계획

공통 원칙:
- 각 단계는 독립된 PR로 배포합니다. 새 동작은 모두 env kill switch를 기본 off로 시작합니다(기존 `OMON_*_ENABLED` 관례를 따릅니다).
- 각 단계의 검증 시나리오는 실제 표면에서 확인합니다: CLI 출력, SQLite 상태, Discord 실제 수신.
- Discord QA는 오너가 직접 합니다. 에이전트는 CLI와 DB 수준까지만 검증합니다.

### P0. 안전 기반 (G4, G5, G6, G7, G13)

**목표.** 어떤 자율 기능도 켜기 전에, 모든 외부 영향 동작이 결정론적 정책을 통과하도록 만듭니다.

- **신규 모듈** `src/policy/`
  - `effect.rs`: `enum Effect { Read, Draft, Write, SendExternal, Spend, Irreversible }`. 도구와 action마다 태깅합니다.
  - `decision.rs`: `enum Decision { Allow, Ask(ApprovalId), HandOff, Deny }`. 평가 순서는 hardline/deny, grant, 기본 정책 순입니다. `Spend`, `SendExternal`, `Irreversible`은 기본이 Ask입니다. 비밀번호 변경과 2FA는 항상 HandOff입니다.
  - `approval.rs`: 상태는 `Proposed → Pending(expires_at) → Approved | Denied | Expired → Executing → Settled | Failed`입니다.
    - 승인은 `sha256(canonical action payload)`에 바인딩하므로, 내용이 바뀌면 다시 승인받아야 합니다.
    - 범위는 `Once | Task(lane_id) | Until(ts) | Standing(scope)`입니다. Standing은 오너가 명시적으로 "항상 허용"을 눌렀을 때만 만듭니다.
    - 무인 요청은 10분 안에 응답이 없으면 Expired(deny)로 처리합니다.
  - `outbox.rs`: 외부로 나가는 메시지와 게시를 draft로 저장하고, 승인 후 발송합니다. 발송 직전에 목적지 ID를 다시 검증합니다.
  - `taint.rs`: lane과 turn에 `tainted_by: Vec<Source>`를 둡니다. inbox, web, email 같은 untrusted 입력을 읽으면 taint가 설정되고, 그 뒤로는 `SendExternal`/`Spend`가 Ask로 강제됩니다.
- **수정 지점**
  - `src/agent/omo_backend.rs:1456-1470`: 자동 승인 분기를 정책 엔진 호출로 바꿉니다. `APPROVAL_MODE`는 하위 호환을 위해 매핑만 남깁니다.
  - `src/lanes/runner.rs:661`: lane 안에서 ApprovalRequest를 거부하지 않고 `approval_requests`에 넣은 뒤 lane을 `Blocked` 상태로 둡니다.
  - `src/discord/approval.rs:239`: 지속형 승인 큐를 렌더링하는 하나의 adapter로 바꿉니다. 승인 버튼에는 action hash 앞 8자리를 표시합니다.
  - `src/cron/scheduler.rs:2357`: 전송 대상을 `destinations` ID로 해석합니다.
- **DB(마이그레이션 0045)**
  - `approval_requests(id, lane_id, session_key, effect, action_json, action_hash, status, scope, requested_at, expires_at, decided_by, decided_at, reason)`
  - `approval_grants(id, scope_kind, effect, matcher_json, max_amount_minor, expires_at, created_from_request, revoked_at)`
  - `approval_allowlist`에 `ttl`, `actor`, `reason` 컬럼을 추가합니다.
  - `destinations(id, platform, external_id, label, sensitivity, created_at)`. `UNIQUE(platform, external_id)`이고 label 중복은 허용하지만 이름으로 찾는 조회는 금지합니다.
  - `outbox(id, destination_id, body, effect, approval_id, status, created_at, sent_at)`
  - `action_audit(seq, prev_hash, entry_hash, ts, actor, effect, action_hash, decision, result)`. append-only hash chain이고, 기록하지 못하면 실행하지 않습니다(Pickle의 write-ahead 방식).
  - `model_usage`에 실제로 기록하는 writer를 만들고, lane별 하루 토큰·비용 상한을 둡니다.
- **검증 시나리오**
  1. `omo-gateway policy eval --effect send_external --action '{"dest":"discord:123","body":"x"}'`를 실행하면 `Ask`가 나오고 `approval_requests`에 Pending 행 1개가 생겨야 합니다.
  2. 승인된 요청의 body를 1바이트 바꿔 재실행하면 기존 승인이 매칭되지 않고 새 Pending이 생겨야 합니다.
  3. untrusted 소스(inbox)를 읽은 lane이 `SendExternal`을 호출하면, Standing grant가 있어도 Ask가 나와야 합니다(taint 우선).
  4. `action_audit` 체인 검증 명령 `omo-gateway audit verify`가 OK를 출력해야 합니다. 중간 행 하나를 변조하면 FAIL과 함께 해당 seq를 출력해야 합니다.
  5. 같은 label을 가진 destination 두 개를 만들고 label로 전송을 시도하면 거부되어야 합니다.

### P1. 선제 엔진 (G1, G2, G3, G14)

**목표.** 사용자가 없을 때도 이벤트와 일정에 따라 일하고, 알릴 가치가 있을 때만 알리도록 만듭니다.

- **신규 모듈** `src/proactive/`
  - `triggers.rs`: Standard Webhooks 수신기를 둡니다. 트리거마다 `whsec_` secret, 5분 허용, 1MiB 상한, `202` 비동기 응답을 씁니다. dashboard 서버에 `POST /hooks/<trigger_id>` 경로를 추가하고, 외부 노출은 Tailscale Funnel 등으로 opt-in합니다.
  - `router.rs`: 트리거, inbox, cron, lane 이벤트를 하나의 `Inbound{id, source, priority, payload}`로 정규화합니다. `id`가 UNIQUE라서 중복은 제거됩니다. 우선순위는 `Owner > HelperDone > Routine > Webhook`입니다.
  - `heartbeat.rs`: heartbeat를 cron의 새 job kind로 둡니다. 모델 호출 전에 skip 사유를 판정합니다(disabled, outside-hours, empty-scratch, busy, no-route, channel-not-ready). 발화는 먼저 기록하고, 러너가 시작되지 않으면 되돌립니다. 결과가 `NO_REPLY`면 침묵합니다(기존 `CRON_SILENCE_TOKENS` 재사용, `src/cron/scheduler.rs:233`).
  - `notify.rs`: `enum Envelope { Notify, Question, Review }`. salience gate는 규칙 다음에 judge yes/no를 봅니다. 선제성 다이얼(0~3)과 조용한 시간을 적용하고, 억제한 알림은 로그에 남깁니다.
- **수정 지점**
  - `src/multiplexer/actor.rs:360-364`: lead가 바쁠 때 이벤트를 버리지 않고 `pending_events`에 넣습니다. idle이 되면 batch로 주입합니다(기존 lane 이벤트 dispatch 패턴, `src/lanes/dispatch.rs`의 `serve` 루프).
  - `src/lanes/kibitzer.rs:340-361`: Decision이 `start_lane`이면 guard를 검사한 뒤 `lanes::cli::start`를 호출합니다. 단 lane 프로필은 `read_only_proactive`(Dots 방식)로 고정하고, 쓰기 동작은 P0 승인을 거쳐야 합니다. 하드코딩된 lead(`kibitzer.rs:21-22`)는 설정으로 뺍니다.
  - 아침 브리핑: 기존 digest(`src/inbox/digest.rs`)를 heartbeat job의 하나로 흡수합니다.
- **DB(0046)**
  - `triggers(id, kind, secret_hash, target_lane_profile, enabled, created_at, rotated_at)`
  - `inbound_events(id UNIQUE, source, priority, payload_json, received_at, consumed_at)`
  - `pending_events(id, session_key, event_json, queued_at, delivered_at)`
  - `heartbeats(agent_id, every_secs, active_hours, tz, scratch, scratch_rev, last_fired_at, fire_count, last_skip_reason)`
  - `kibitzer_state`를 migration으로 이관합니다.
  - `notifications(id, envelope, salience, delivered, suppressed_reason, created_at)`
- **검증 시나리오**
  1. 올바른 HMAC으로 `curl -i -X POST http://127.0.0.1:9119/hooks/<id>`를 보내면 `HTTP/1.1 202`가 와야 합니다. 서명을 틀리면 `401`, 6분 전 timestamp면 `401`, 2MiB body면 `413`이 와야 합니다.
  2. 같은 `webhook-id`로 두 번 보내면 `inbound_events`에 행이 1개만 있어야 합니다.
  3. heartbeat scratch가 비어 있으면 `last_skip_reason=empty-scratch`가 기록되고, 해당 시각에 `judge_calls`·모델 호출 증가가 0이어야 합니다.
  4. lead turn이 실행 중일 때 cron 완료 이벤트를 넣으면 `pending_events`에 쌓이고, turn이 끝난 뒤 `delivered_at`이 채워져야 합니다(유실 0).
  5. kibitzer가 `start_lane`을 결정하면 lane이 `read_only_proactive` 프로필로 생성되고, 그 lane의 `SendExternal` 시도는 `approval_requests` Pending으로 막혀야 합니다.

### P2. 자격증명과 커넥터 (G8)

**목표.** 에이전트가 비밀을 보지 못한 채로 외부 서비스를 쓰도록 만듭니다.

- **신규 모듈** `src/vault/`, `src/egress/`
  - `vault`: macOS Keychain(`security-framework` crate)에 비밀을 저장합니다. 에이전트에게는 `cred://<id>` handle만 줍니다. secret 타입은 `Serialize`를 구현하지 않습니다.
  - `egress`: 데몬이 소유하는 HTTP(S) 프록시입니다.
    - lane과 sandbox에는 `HTTPS_PROXY`를 설정합니다.
    - lease(`host`, `path` prefix, `method` class, 만료)가 매칭될 때만 placeholder를 실제 비밀로 바꿉니다.
    - 응답에 비밀이 base64, hex, percent 인코딩으로라도 비치면 응답을 막습니다(Pickle `response_withheld`).
    - SSRF 차단: DNS를 해석한 IP와 redirect 대상을 사설 대역과 대조합니다.
    - 처음 보는 host는 Ask입니다.
  - 커넥터:
    - MCP 클라이언트 설정 로더를 `src/main.rs:1082`에 연결합니다.
    - Kakao PlayMCP(OAuth와 OTT 교환)를 upstream으로 붙이고, 토큰은 Keychain에 둡니다.
    - inbox mail 계정 하드코딩(`src/inbox/sources/mail.rs:9`)을 `connectors` 행으로 옮깁니다.
  - inbox 정화: OTP, 비밀번호 재설정 링크, magic link를 제거하고 untrusted로 라벨링합니다(Muse 방식). 위치는 `src/inbox/mod.rs`의 수집 단계입니다.
- **DB(0047)**
  - `connectors(id, kind, account, scopes, status, created_at)`
  - `credentials(id, connector_id, keychain_ref, kind, expires_at, last_used_at)`
  - `egress_leases(id, lane_id, credential_id, host, path_prefix, method_class, expires_at, revoked_at)`
  - `mcp_servers(id, url, auth_kind, pinned_hash, enabled)`
- **검증 시나리오**
  1. lane 안에서 `curl -H 'Authorization: Bearer cred://gh' https://api.github.com/user`를 실행하면, lease가 있을 때 200이 오고 lane 프로세스 env나 stdout 어디에도 실제 토큰 문자열이 없어야 합니다(grep 0건).
  2. lease host가 아닌 `https://evil.example/`로 같은 placeholder를 보내면 프록시가 `403`을 주고 `action_audit`에 deny가 기록되어야 합니다.
  3. 응답에 토큰을 base64로 반사하는 테스트 서버 응답은 차단되어야 합니다.
  4. `http://169.254.169.254/`나 `127.0.0.1`로 redirect되는 URL은 차단되어야 합니다.
  5. OTP가 든 메일을 inbox에 넣으면 저장된 snippet에 6자리 코드와 reset 링크가 없어야 합니다.

### P3. 메모리 (G9)

**목표.** 채널을 넘나드는 오너 단위 기억을 만들고, 출처와 시간이 있는 사실로 관리합니다.

- **모듈** `src/memory/` 확장
  - `episodes`: 불변이고 내용 주소 기반인 원본입니다. `facts`: bi-temporal(`valid_at`, `invalid_at`, `superseded_by`)이고 원본 episode에 provenance를 연결합니다. 검색은 FTS5와 sqlite-vec를 씁니다.
  - scope는 `owner | channel | lane | repo`이고, 민감 등급은 `public | personal | financial`입니다.
  - 주입: `src/multiplexer/actor.rs:1344`의 fresh_context에서 core facts를 짧게(토큰 예산 안에서) 주입합니다. 나머지는 `memory_search` 도구로 찾게 합니다.
  - 쓰기 도구: `memory_remember` / `memory_forget`을 `src/tools/`에 추가합니다. untrusted 출처에서 나온 사실은 `proposed` 상태로만 들어갑니다.
  - dream: 야간 cron이 대화를 검토해 fact diff를 제안합니다. 오너가 "기억할까요? 👍/👎"에 답해야 `active`가 됩니다.
  - forget: fact를 retract하고 연관된 요약과 인덱스를 다시 만듭니다. 결과는 감사 기록에 남깁니다.
  - lane settle 성공 시점(`src/lanes/settle.rs:171-173`)과 제안 승인·기각 시점(`src/inbox/proposals.rs:468-478`)에 결과를 episode로 기록합니다.
- **DB(0048)**
  - `episodes(id, hash UNIQUE, content, source, ref_time, created_at)`
  - `facts(id, subject, predicate, object, text, scope, sensitivity, state, confidence, valid_at, invalid_at, superseded_by, created_at)`
  - `fact_episodes`, `fact_fts`(FTS5), `fact_vec`(sqlite-vec, `embed_fingerprint`)
  - 기존 `memories` 데이터는 episodes와 facts로 이관하는 스크립트로 옮깁니다.
- **검증 시나리오**
  1. Discord DM에서 "내 생일은 3월 2일"이라고 말한 뒤 다른 채널 thread에서 `memory_search 생일`을 실행하면 해당 fact가 나와야 합니다(scope=owner).
  2. "생일은 사실 3월 3일"이라고 말하면 이전 fact에 `invalid_at`이 채워지고 새 fact가 active가 되어야 합니다.
  3. inbox 메일 본문에서 추출된 "오너는 X를 좋아함"은 `state=proposed`여야 하고 주입되지 않아야 합니다.
  4. `memory_forget`을 실행한 뒤에는 `fact_fts`와 `fact_vec` 어디에서도 검색되지 않아야 합니다.

### P4. 실행 격리 (G10)

**목표.** 자율 lane이 호스트 Mac을 직접 건드리지 않게 합니다.

- **신규 모듈** `src/sandbox/`
  - `trait Sandbox { create, exec, snapshot, pause, resume, destroy }`
  - 백엔드는 셋입니다.
    - `HostSeatbelt`: 기본값. seatbelt profile과 egress 프록시 강제(`anthropics/sandbox-runtime` 방식).
    - `AppleContainer`: 코딩 lane용. 작업마다 Linux VM을 띄우고 결과를 git diff로 머지합니다.
    - `MacVm`: GUI 작업용. Tart/Lume과 Cua Driver를 씁니다.
  - lane 프로필이 백엔드를 선택합니다. lane마다 TTL, idle pause, 하드 kill(프로세스 트리 전체)을 둡니다.
  - agent loop는 게이트웨이에 남기고, VM은 언제든 다시 만들 수 있는 실행 대상으로만 씁니다(Grok Bot 방식). 따라서 VM이 리셋돼도 상태는 SQLite에 남습니다.
- **수정 지점**
  - `src/agent/omo_backend.rs:481-533`(workspace 프로비저닝): sandbox 바인딩을 추가합니다.
  - `src/tools/terminal.rs`: sandbox exec로 우회시킵니다.
  - lane runner의 `ExternalDriver`(`src/lanes/runner.rs:1172-1181`)
- **DB(0049)**: `sandboxes(id, backend, lane_id, image, status, created_at, last_active_at, ttl_secs)`.
- **검증 시나리오**
  1. `AppleContainer` lane에서 `touch ~/escape-test`를 실행해도 호스트 `~/escape-test`가 생기지 않아야 합니다.
  2. `HostSeatbelt` lane에서 egress 프록시를 우회해 `curl https://example.com`을 직접 연결하면 실패해야 하고, 프록시 경유는 성공해야 합니다.
  3. TTL이 지나면 sandbox 상태가 `destroyed`가 되고 관련 프로세스가 0개여야 합니다.

### P5. 멀티채널과 음성 (G11, G12)

**목표.** Instinct처럼 "새 인터페이스 없이" 오너가 이미 쓰는 채널에서 양방향으로 대화합니다.

- **채널 trait** `src/channels/`: `send(dest, Envelope)`, `receive() -> Inbound`, `render_approval(req)`
  - iMessage: 수신은 기존 imsg를 씁니다. 발신은 `imsg send` 또는 AppleScript로 하고, 오너 핸들만 허용합니다. 승인은 키워드("승인 a1b2c3d4")로 받습니다.
  - KakaoTalk: 수신은 PirateTalk를 그대로 씁니다(읽기 전용). 발신은 "나에게 보내기"나 채널 봇으로만 합니다. 공식 개인 채팅 API가 없으므로 수신 승인 경로는 지원하지 않는다고 명시합니다.
  - Telegram/Slack: 필요할 때 추가합니다. cron 전송 u64 제한(`src/cron/scheduler.rs:2357`)은 `destinations` ID로 대체합니다.
  - 오너 신원은 메시지 내용이 아니라 `identities(platform, handle) → owner`로 판정합니다.
- **음성**
  - 1단계: 음성 메시지 STT를 하나로 정리합니다(adapter와 attachments 이중 경로).
  - 2단계: Tailscale 위에 WS 음성 엔드포인트를 둡니다(텍스트 입출력, 기기 키 바인딩).
  - 3단계: Songbird 실시간 리스너를 연결합니다(`src/voice/mod.rs`). 전화(Twilio/Telnyx)는 별도로 평가합니다.
- **DB(0050)**: `identities(platform, handle, owner_id, verified_at)`, `inbox_items.source`의 CHECK를 완화합니다.
- **검증 시나리오**
  1. iMessage로 받은 승인 요청에 "승인 <hash8>"으로 답하면 `approval_requests.status=Approved`, `decided_by=imessage:<handle>`이 되어야 합니다.
  2. 오너가 아닌 핸들에서 같은 답장이 오면 무시되고 `action_audit`에 기록되어야 합니다.
  3. cron 잡의 destination을 `imessage:<id>`로 지정하면 실제 기기에 수신되어야 합니다. 이 확인은 오너가 직접 합니다.

### P6. 결제와 에이전트 간 통신 (G15)

**목표.** 구매 대행을 제한된 범위에서 허용하고, 다른 에이전트와 안전하게 교류합니다.

- `src/spend/`:
  - `SpendLease{merchant, max_amount_minor, currency, expires_at, single_use}`, 상태는 `Active | RequiresAction | Deactivated(reason)`입니다.
  - 건당, 일, 30일 상한을 둡니다. 금액이 오르거나 처음 보는 가맹점이면 반드시 Ask합니다.
  - 결제 수단은 가상카드나 Stripe Link를 씁니다. idempotency key와 원장을 같은 트랜잭션에서 기록합니다.
- 에이전트 간 통신:
  - MCP 서버로 게이트웨이 도구를 노출합니다. `input_required`가 오면 오너에게 질문합니다.
  - A2A는 기본 off입니다. peer 입력은 untrusted taint로 처리하고, 외부 상대를 응대하는 lane은 읽기와 대화 도구만 갖습니다(Pickle 방식).
- **DB(0051)**: `spend_leases`, `spend_events(id, lease_id, idempotency_key UNIQUE, amount_minor, merchant, state, receipt_hash)`.
- **검증 시나리오**
  1. 상한 $20 lease로 $25 결제를 시도하면 `RequiresAction`이 되고 Ask 카드가 발송되어야 합니다.
  2. 같은 idempotency key로 두 번 요청하면 `spend_events`에 행이 1개만 있어야 합니다.
  3. single_use lease를 두 번째로 쓰려 하면 거부되어야 합니다.

### 단계 의존성과 규모

| 단계 | 선행 | 마이그레이션 | 신규 모듈 | 규모(상대) |
|---|---|---|---|---|
| P0 안전 기반 | - | 0045 | policy | L |
| P1 선제 엔진 | P0 | 0046 | proactive | L |
| P2 자격증명 | P0 | 0047 | vault, egress | L |
| P3 메모리 | P0 | 0048 | memory 확장 | M |
| P4 실행 격리 | P2(egress) | 0049 | sandbox | L |
| P5 멀티채널 | P0 | 0050 | channels | M |
| P6 결제·A2A | P0, P2 | 0051 | spend | M |

P1, P2, P3, P5는 P0가 끝나면 서로 파일이 겹치지 않으므로 병렬로 진행할 수 있습니다. 통합 검증은 각 단계의 구현이 다 모인 뒤 한 번만 수행합니다(오너 지시사항).

---

## 7. 위험과 열린 질문

- **macOS 권한.** Apple이 AI 에이전트 때문에 Full Disk Access 통제를 강화한다고 밝혔습니다 [2차] https://techcrunch.com/2026/10/02/apple-says-its-tightening-macos-full-disk-access-controls-due-to-new-risks-from-ai-agents/. 게이트웨이는 이미 FDA(iMessage)에 의존하므로, `doctor` 명령으로 TCC 상태를 점검하는 기능이 필요합니다. 2026-10-16의 iMessage 감시 장애도 TCC가 원인이었습니다.
- **사이트 차단.** Amazon(Muse), Resy(Instinct)가 에이전트를 차단했습니다. 브라우저 lane에는 식별 가능한 User-Agent를 두고 사이트별 opt-out을 지켜야 합니다. 오너의 실제 브라우저 세션과 주거용 IP를 쓰는 것은 장점이지만 계정 정지 위험도 있습니다.
- **비용 폭주.** Dots(하루 307k 토큰)와 Grok Bot(10분에 $50) 사례가 있습니다. P0에서 `model_usage` writer와 lane별 상한을 반드시 먼저 넣어야 합니다.
- **자연어 규칙의 한계.** 세 회사(OpenAI, xAI, Meta)가 모두 reviewer와 자연어 규칙이 보장이 아니라고 인정했습니다. 그래서 이 계획은 정책을 결정론적 코드에 둡니다. judge(jev)는 조이는 방향으로만 씁니다.
- **미해결 출처.** Dots 공식 블로그 원문(4,000개 앱 수치), Muse tier 가격, Grok Bot 보존·학습 정책, Pickle help-center 승인 문서는 Cloudflare나 JS 렌더링 때문에 읽지 못했습니다.
- **문서 정리(후속).** `docs/AGENTS_DELEGATE.md`와 README의 음성·MCP 기능 설명이 실제 코드와 맞지 않습니다.

---

## 8. 출처

1차 출처만 추렸습니다. 전체 목록은 세션 디렉터리의 각 리포트에 URL로 남아 있습니다.

- OpenAI Dots: https://help.openai.com/articles/20001530, https://help.openai.com/articles/20001529, https://learn.chatgpt.com/docs/whats-new/devday-2026.md, https://learn.chatgpt.com/docs/sandboxing/auto-review
- Meta Muse: https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/, http://security.muse.ai/, https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse, https://www.meta.com/help/artificial-intelligence/1385290430137537/, https://introducing.muse.ai, https://mouse.dev/blog/muse-runtime-export
- xAI Grok Bot: https://x.ai/news/introducing-grok-bot, https://x.ai/bot, https://x.ai/bot/guides/grok-bot-101, https://docs.x.ai/grok/automations/webhooks, https://www.peterp.me/transcripts/how-grok-bot-works/
- Instinct: https://instinct.com/, https://instinct.com/privacy-policy, https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/
- Pickle: https://www.ycombinator.com/companies/pickle-2, https://pickle.com/privacy-policy, https://pickle.com/assets/security-n1i5zLW2.js, https://github.com/pickle-com/credential-enclave
- Hark: https://hark.com/using-hark, https://hark.com/pricing, https://hark.com/terms, https://hark.com/security, https://hark.com/articles/introducing-hark-pro, https://hark.com/articles/introducing-hark-handoff
- OpenClaw / Hermes: https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/heartbeat.md, https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/tools/exec-approvals.md, https://github.com/NousResearch/hermes-agent/blob/e6899c8090da3bd48c829893e4103ce989d3482d/hermes_cli/heartbeat.py
- 보안: https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/, https://arxiv.org/abs/2503.18813, https://github.com/anthropics/sandbox-runtime
- 메모리: https://github.com/getzep/graphiti, https://docs.letta.com/concepts/memfs/index.md
- 실행격리: https://github.com/apple/container, https://github.com/trycua/cua
- 알림·HITL: https://github.com/langchain-ai/agent-inbox, https://arxiv.org/abs/2506.12469
