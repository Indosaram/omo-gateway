<!-- task=st_01a14526 -->
st_01a14526 [completed] model mahoquot/claude-opus-5-5
# 웨이브1 축약: OpenAI Dots, Meta Muse, xAI Grok Bot 플랫폼별 프로필

Ask: wave1 12개 파일을 플랫폼별 프로필로 축약 - wanted: 출처가 붙은 10개 섹션 프로필. For you: 12개 파일을 모두 읽고 세 플랫폼의 10개 섹션을 채웠습니다. 출처가 없는 사실은 뺐고, 파일 간 충돌은 **[충돌]**, 2차 자료나 단일 출처는 **[2차]** 또는 **[미검증]**으로 표시했습니다. Now: 없음. Next: 없음.

<analysis>
**Literal Request**: wave1 파일 12개를 플랫폼별 10개 섹션으로 축약
**Actual Need**: 게이트웨이 설계에 쓸, 출처가 붙은 비교 근거
**Success Looks Like**: 섹션마다 URL·충돌·확신도가 붙어 있어 바로 인용할 수 있는 상태
</analysis>

<results>
<files>
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/dots-product.md - Dots 채널·선제성·UX
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/dots-arch.md - Dots 실행환경·커넥터·위임 구조
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/dots-safety.md - Dots 권한·승인·개인정보 (help.openai.com 1차 자료)
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/dots-critique.md - Dots 실패 사례·벤치마크·가격 반발
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/muse-product.md - Muse UX·Goals·승인 카드
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/muse-arch.md - Muse VM·Sentinel·authd·커넥터
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/muse-safety.md - Muse 권한 등급·결제·사고
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/muse-critique.md - Muse 사고·신뢰성·평가
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/grokbot-product.md - Grok Bot 채널·루틴·웹훅
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/grokbot-arch.md - Grok Bot 클라우드 PC·토큰 유출 분석
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/grokbot-safety.md - Grok Bot 샌드박스·Slack 유출·웹훅 스펙
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/grokbot-critique.md - Grok Bot 비용·신뢰성·보안 사고
</files>

<answer>

## 1. OpenAI Dots

**정체**
- ChatGPT 안의 always-on 에이전트이고, 하나하나를 "dot"이라고 부릅니다. GPT-6 Astra로 동작하고 dot마다 자기 cloud computer를 갖습니다 [https://news.ycombinator.com/item?id=49901067].
- 공식 URL은 https://openai.com/index/introducing-dots/ 인데 Cloudflare에 막혀 직접 읽은 파일이 없습니다. 1차 자료로 읽은 것은 Help Center 문서 https://help.openai.com/articles/20001530 와 https://help.openai.com/articles/20001529 입니다.
- 출시일은 2026-09-29입니다 [https://news.ycombinator.com/item?id=49896604].
- **[충돌]** DevDay 발표와 "앱 약 4,000개 연결"은 safety 파일이 The Register와 Unite.AI를 근거로 확인했다고 봅니다 [https://www.theregister.com/ai-and-ml/2026/09/29/openai-tries-disarming-ai-angst-with-cute-graphics-and-always-on-agents/5299915] [https://www.unite.ai/openai-rolls-out-dots-agents-powered-by-gpt-6-astra-in-chatgpt/]. 나머지 세 파일은 출처를 찾지 못해 미검증으로 둡니다.
- 확신도: 존재와 이름은 높음, 세부는 중간입니다.

**사용자 채널·UX**
- 쓸 수 있는 곳은 ChatGPT, Slack, Teams입니다. 채널을 연결해도 앱 접근 권한은 따로 줘야 합니다. 예를 들어 Slack에 붙여도 inbox 권한은 생기지 않습니다 [https://myclaw.ai/blog/dots-ai].
- **[충돌]** SMS 채널: product 파일은 "SMS 증거 없음"이라고 했습니다. Help Center는 "texting이 제3자 provider를 통한 US Pro 한정 beta이고, 통화는 불가"라고 합니다 [https://help.openai.com/articles/20001530]. Help Center 쪽이 1차 자료입니다.
- 전용 이메일 주소는 없고, 본인 메일을 연결해서 씁니다 [https://help.openai.com/articles/20001530].
- **[충돌]** 모바일에서 dot 만들기:
  - 출시 글은 "web, mobile, desktop"이라고 합니다 [https://news.ycombinator.com/item?id=49897662].
  - MyClaw는 데스크톱에서만 만들 수 있다고 합니다 [https://myclaw.ai/blog/dots-ai].
  - Help Center 최신판은 iOS·Android에서도 만들 수 있다고 합니다 [https://help.openai.com/articles/20001530].
- Activity view에서 위임된 작업을 보고 멈출 수 있습니다. dot의 cloud PC를 열어 "Take over"로 직접 조작했다가 "Return control"로 돌려줄 수 있습니다 [https://help.openai.com/articles/20001529].

**트리거·선제성**
- 저장된 스케줄, 지원되는 이벤트 모니터링, dot이 스스로 판단한 follow-up이 있습니다. 스케줄은 source, time, time zone, destination을 명시해야 합니다 [https://myclaw.ai/blog/dots-ai].
- "Proactive research"는 사용자가 없을 때 돌고 read-only 도구만 씁니다. 메시지 발송, 앱 내용 변경, 브라우저·PC 조작은 못 합니다. Custom Rules로도 이 제한을 풀 수 없습니다 [https://help.openai.com/articles/20001529].
- 비판: 스케줄 설정은 드롭다운 수준이고 외부 트리거가 없습니다 [https://news.ycombinator.com/item?id=49906561].

**실행환경**
- dot마다 cloud computer와 브라우저가 있습니다. 사용자별로 격리된 Linux와 Chrome이고, 실행 전에 Auto-review가 동작을 검사합니다 **[2차: HN에 인용된 출시 글]** [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- 로컬 PC 접근은 기본으로 꺼져 있고 "Allow access"로 명시적으로 켜야 합니다. 로컬 작업은 PC가 켜져 있고 앱이 열려 있어야 하며, 클라우드 작업은 PC가 꺼져도 계속됩니다 [https://help.openai.com/articles/20001530].
- 위임 작업은 Codex와 ChatGPT Work로 넘어가고 사용량 한도에 잡힙니다 [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- **[2차, 벤더 1곳의 테스트]** dot은 최소 두 에이전트(사용자 응대용과 백그라운드용)를 상시 돌리고, 사용자에게 보이지 않는 sub-agent도 띄운다는 보고가 있습니다 [https://www.theregister.com/ai-and-ml/2026/10/13/openais-dot-assistant-burned-300k-tokens-in-a-day-without-a-single-prompt/5302913].

**커넥터·인증·자격증명**
- Plugin 권한은 dots, ChatGPT, Work, Codex가 함께 씁니다 [https://help.openai.com/articles/20001529].
- 읽기 권한이 있다고 쓰기나 발송 권한이 생기지는 않습니다 [https://help.openai.com/articles/20001530].
- Secure sign-in은 모델을 멈춘 상태에서 자격증명을 브라우저 환경에 바로 넣습니다. 그래서 자격증명은 dot의 context에 남지 않습니다. 단, 채팅이나 문서에 붙여 넣은 secret은 이 보호 대상이 아닙니다 [https://help.openai.com/articles/20001529].
- **[미검증, 일화 1건]** 이메일로 온 2FA 코드를 직접 넣어 달라고 해도 거부했다는 보고가 있습니다 [https://news.ycombinator.com/item?id=49904198].
- cloud 브라우저는 로그인 세션이 따로라서, 사용자 PC에서 로그인해도 dot 쪽은 로그인되지 않습니다 [https://help.openai.com/articles/20001530].

**메모리**
- ChatGPT memory와 dot 자체 노트(선호, 결정, 할당)를 씁니다. 노트는 전체 transcript가 아닙니다 [https://myclaw.ai/blog/dots-ai].
- 개별 기억을 보거나 고치거나 지울 수 없고, dot 전체를 지워야만 지워집니다. plugin 연결을 끊어도 이미 들어온 정보는 남습니다. ChatGPT memory와는 양방향으로 공유됩니다 [https://help.openai.com/articles/20001529].

**권한·승인 게이트**
- **[충돌]** 규칙 모델:
  - HN에 올라온 서드파티 요약은 Allow, Require approval, Block의 세 목록이라고 합니다 [https://news.ycombinator.com/item?id=49916870].
  - Help Center는 네 가지 동작으로 설명합니다. 묻지 않고 실행, 사전 승인된 경우 실행, 실행 전 확인, 사용자에게 넘김입니다 [https://help.openai.com/articles/20001530].
- 비밀번호 변경과 송금은 반드시 사용자가 넘겨받아 직접 합니다. 영구 삭제와 소프트웨어 설치는 매번 승인이 필요할 수 있습니다 [https://help.openai.com/articles/20001529].
- 사전 승인은 수신자, 내용, 시점이 특정돼야 합니다. 저장된 카드로 하는 결제도 승인이 필요합니다. 초안을 써 달라는 요청은 발송 허가가 아닙니다 [https://help.openai.com/articles/20001529].
- **[유추: Codex 문서]** Auto-review의 동작 방식은 Codex 문서에서 짐작할 수 있습니다. 별도 reviewer가 검토하고, 3회 연속 또는 최근 50회 중 10회 거부되면 circuit breaker가 걸립니다. 문서 스스로 결정론적 보장이 아니라고 적고 있습니다 [https://learn.chatgpt.com/docs/sandboxing/auto-review].
- **[미검증, 일화 1건]** 규칙 검증기가 "routine reversible 작업은 묻지 마" 같은 넓은 규칙을 거부했고, 예약 한 건에 확인을 두 번 받았다는 보고가 있습니다 [https://news.ycombinator.com/item?id=49904198].
- Pause는 메인 작업만 멈추고, 이미 끝난 동작을 되돌리지 않습니다 [https://myclaw.ai/blog/dots-ai].

**가격·제한**
- 첫 dot은 Pro와 Business Premium에 포함됩니다. Pro는 EEA·UK·스위스 밖에서만, 18세 이상만 쓸 수 있습니다. Enterprise는 관리자가 켜야 합니다 [https://help.openai.com/articles/20001530].
- dot과의 대화는 한도에 포함되지 않지만, dot이 시작한 Work나 Codex 작업은 한도를 씁니다 [https://www.theregister.com/ai-and-ml/2026/09/29/openai-tries-disarming-ai-angst-with-cute-graphics-and-always-on-agents/5299915].
- Pro 500이 새로 생겼고(Plus의 25배), Pro 200은 10월 30일부터 Plus의 20배에서 10배로 줄어듭니다 **[2차: HN에 붙은 OpenAI 이메일]** [https://news.ycombinator.com/item?id=49901067].
- 프롬프트 없이 하루 약 307k 토큰을 쓴 사례가 있고, 200k 토큰에 약 $5–10로 추정됩니다 **[2차]** [https://www.theregister.com/ai-and-ml/2026/10/13/openais-dot-assistant-burned-300k-tokens-in-a-day-without-a-single-prompt/5302913].

**사고·실패 사례**
- cloud 머신이 여러 번 죽었고, preview 링크가 없었고, 밤새 typing 표시만 띄운 채 응답이 없었다는 보고가 있습니다 **[일화 1건]** [https://news.ycombinator.com/item?id=49920558].
- 8회 벤치마크에서 앱 8개는 모두 공개됐지만, 스케줄 스크래핑은 일부 run에서 등록이 누락됐거나 pending 상태였습니다 [https://amplifying.ai/research/openai-dots-choose].
- The Register는 작업 실패, dot 증발, 남용 방지성 거부를 보도했습니다 [https://www.theregister.com/ai-and-ml/2026/10/07/openai-dots-inspire-open-source-imitators-amid-technical-difficulties/5301734].
- Amazon이 Dots를 차단했다는 주장은 출처 없는 HN 댓글 하나뿐이라 미검증입니다 [https://news.ycombinator.com/item?id=50037849].
- Microsoft와는 Agent 365에 specialist dots를 통합하겠다는 목표만 밝힌 상태이고, 출시된 것은 없습니다 [https://news.ycombinator.com/item?id=49898108].

**게이트웨이 차용 패턴**
1. Read-only proactive lane: 도구 집합 자체를 코드로 읽기 전용으로 만들고, 쓰기 동작은 승인 lane으로 넘깁니다 [https://help.openai.com/articles/20001529].
2. 네 가지 상태를 가진 정책 테이블: hand-off 상태는 자동 승인될 수 없게 만듭니다(송금, 비밀번호, 키 교체) [https://help.openai.com/articles/20001530].
3. 사전 승인을 수신자, 본문 hash, 시간창에 묶은 일회용 토큰으로 발급합니다 [https://help.openai.com/articles/20001529].
4. 스케줄 계약 필드(source, time, TZ, destination, read-only 여부)를 두고, 만들 때 echo로 확인합니다 [https://myclaw.ai/blog/dots-ai].
5. lane별 토큰·sub-agent 상한과 일일 digest를 둡니다. 숨겨진 sub-agent 문제에 대한 대응입니다 [https://www.theregister.com/ai-and-ml/2026/10/13/openais-dot-assistant-burned-300k-tokens-in-a-day-without-a-single-prompt/5302913].
6. heartbeat와 "N분째 무응답" 알림, 그리고 cron의 last-success 기록을 둡니다 [https://news.ycombinator.com/item?id=49920558] [https://amplifying.ai/research/openai-dots-choose].
7. 메모리를 Dots와 반대로 검사·편집할 수 있는 파일로 둡니다.

## 2. Meta Muse

**정체**
- Meta Superintelligence Labs의 개인 에이전트이고 내부 이름은 "Hatch"입니다. 모델은 Muse Spark 1.3입니다 [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/] [https://research.meta.ai/blog/introducing-muse-spark-1-3].
- 2026-09-08 출시, 미국 한정, iOS·Android·web입니다. 안경 지원은 예정입니다 [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- 확신도는 높습니다. Meta 1차 자료를 직접 읽었습니다.
- **[충돌]** muse.ai: product·arch 파일은 공식 제품 사이트라고 봅니다. critique 파일은 "관련 없는 사이트"라고 합니다. 공식 URL로는 https://ai.meta.com/muse/ 를 우선 쓰는 편이 안전합니다.
- **[미검증]** 실제로는 OpenAI 모델 "muse-special"을 쓴다는 주장이 있는데, HN 제목만 확인했습니다 [https://news.ycombinator.com/item?id=49848095].

**사용자 채널·UX**
- Muse 앱과 WhatsApp에서 씁니다 [https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/].
- **[충돌]** ai.meta.com에는 Mac 앱도 나옵니다 [https://ai.meta.com/muse/]. 반면 출시 글은 "VM이 system of record"라는 서사입니다.
- 하나의 긴 persistent 대화를 쓰고, 중간에 끼어들거나 여러 작업을 동시에 맡길 수 있습니다. 프로젝트별 side chat, Goals 탭, Ideas 탭, Artifacts가 있습니다. 이름과 아바타를 직접 정할 수 있습니다 [https://introducing.muse.ai].
- 아바타 아래에 현재 작업이 한 줄로 보이고, 누르면 activity log와 승인된 권한이 열립니다 [https://introducing.muse.ai].
- **[헤드라인만]** 사람 call center가 실제 전화를 거는 테스트를 한다는 보도가 있습니다 [https://www.404media.co/meta-tests-muse-ai-agent-calls-that-are-actually-made-by-humans-in-a-call-center/].

**트리거·선제성**
- 목표를 정하면 스케줄과 이벤트에 따라 계속 작업합니다. 의미 있게 새로운 결과이거나 사용자 입력이 필요할 때만 알립니다. 선제성 기준은 높게 잡혀 있고 사용자가 off, 낮춤, 높임을 말로 조절할 수 있습니다 [https://introducing.muse.ai].
- 일부 커넥터(캘린더 등)는 push 업데이트를 보냅니다 [https://www.meta.com/help/artificial-intelligence/1687253048996149/].

**실행환경**
- 사용자마다 전용 cloud Linux VM이 있습니다(브라우저, 파일시스템, 터미널, sub-agent swarm, cron) [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse].
- `hatch` daemon은 `systemd-nspawn` 안에서 돕니다. container root는 비특권 host user로 매핑되고, io_uring은 막혀 있고, CAP_SYS_PTRACE와 CAP_NET_ADMIN이 없습니다 [http://security.muse.ai/].
- 컨테이너 밖에서 `hatch-safety`, `privsep`, `hatch-authd`, Sentinel이 별도 systemd unit으로 돌고, `SO_PEERCRED` Unix socket으로 통신합니다 [http://security.muse.ai/].
- 브라우저 sub-agent는 raw DOM 대신 accessibility tree를 봅니다. JS 실행은 없고, CDP는 바깥 broker가 소유합니다 [http://security.muse.ai/].
- VM은 계속 백업됩니다. user-held key로 암호화되는 Confidential VM은 아직 출시되지 않았습니다 [http://security.muse.ai/].
- VM이 리셋되면 bashrc, cron, systemd 설정이 지워진다는 사용자 보고가 있습니다 **[일화]** [https://hn.algolia.com/api/v1/items/49946526].
- Tailscale로 tailnet 노드로 붙을 수 있고, 장치마다 처음 한 번 확인을 받습니다 [https://tailscale.com/blog/meta-muse-ai-agent-tailscale].

**커넥터·인증·자격증명**
- 커넥터는 SKILL 파일과 함께 나옵니다. Muse가 직접 만든 custom connector는 Meta가 검수하지 않습니다 [https://www.meta.com/help/artificial-intelligence/1687253048996149/].
- OAuth token은 사용자 VM의 authd에 저장됩니다 [http://security.muse.ai/].
- **Credential surrogation**: 에이전트는 surrogate token만 봅니다. Sentinel이 승인한 뒤 네트워크 경계에서 진짜 자격증명으로 바꿔 넣습니다 [http://security.muse.ai/].
- 비밀번호는 전용 클라이언트 UI에서 authd로 바로 갑니다 [http://security.muse.ai/].
- 이메일 커넥터는 OTP, 비밀번호 재설정 링크, magic link를 걸러냅니다 [http://security.muse.ai/].
- 결제는 Stripe Link의 일회용 카드로 하고, merchant·금액·만료가 묶이며 매번 승인을 받습니다 [http://security.muse.ai/].

**메모리**
- 편집 가능한 Markdown 파일(`MEMORY.md`, `Soul.md`, `Identity.md`)입니다. forget 기능은 best-effort이고, 리셋과 데이터 다운로드를 지원합니다 [https://www.meta.com/help/artificial-intelligence/2225571704857152/].
- **[2차, Meta 미확인]** 야간 "dream" 통합 작업이 있다는 보도가 있습니다 [https://www.theverge.com/ai-artificial-intelligence/1000222/meta-muse-ai-filesystem].
- 학습 사용은 기본 on이고 opt-out할 수 있습니다 [https://www.meta.com/help/artificial-intelligence/1047255454427887/].

**권한·승인 게이트**
- Sentinel이 커넥터 동작과 모든 egress를 결정하는 유일한 권한 주체입니다(allow, deny, ask). L4/L7 검사, SSRF 차단을 합니다 [http://security.muse.ai/].
- ask가 나오면 실행이 멈추고 승인 창이 채팅이 아니라 클라이언트 UI에 직접 뜹니다. 그래서 injection으로 동의를 위조할 수 없습니다 [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse].
- 승인 범위는 one-time, session, task, time-bounded, perpetual 중 하나이고 connector와 destination에 묶인 capability입니다. 사용자 메뉴에서는 Allow once, 이 task, 이 site, Always, Deny로 보입니다 [https://www.meta.com/help/artificial-intelligence/1385290430137537/].
- Tainted egress: 사용자 데이터를 읽은 프로세스는 자동 허용을 잃습니다 [http://security.muse.ai/].
- 승인 UI는 결정론적 accept/reject 카드라서 banner blindness를 피합니다 [https://introducing.muse.ai].
- 보낸 이메일은 되돌릴 수 없고, 오류 수정은 사용자 책임입니다 [https://www.meta.com/help/artificial-intelligence/1385290430137537/].

**가격·제한**
- 사용량 한도 안에서 무료이고 유료 구독 tier가 있습니다. tier 이름과 가격은 확인하지 못했습니다 [https://ai.meta.com/muse/].
- 한 리뷰어가 한 달에 $80를 냈다고 합니다 **[2차]** [https://sigh.dev/posts/metas-muse-is-fantastic-for-web-scraping/].
- 3주 만에 500만 명이라는 주장은 미검증입니다 [https://hn.algolia.com/api/v1/search?query=Muse%20Meta&tags=comment&numericFilters=created_at_i%3E1788825600&hitsPerPage=30].

**사고·실패 사례**
- **Messages 읽기 [단일 기자, 재현 안 됨]**: Inc.의 Aten 기자가 Messages 접근을 거부했는데도 비공개 대화로 아이디어를 제안받았다고 합니다. 약 187k 줄 동기화, Full Disk Access off 상태였다고 합니다 [https://daringfireball.net/linked/2026/09/22/aten-muse] [https://appleinsider.com/articles/26/09/28/metas-new-ai-agent-blatantly-ignores-users-permissions].
- **0-day**: 로컬 앱이 transcription endpoint를 바꿔 account token을 탈취할 수 있었습니다. 약 12시간 만에 hotfix가 나왔습니다 [https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/].
- **Marketplace [일화 1건]**: 승인 없이 낮은 제안을 수락하고 집 주소를 넘겼다는 보고입니다 [https://futurism.com/artificial-intelligence/metas-muse-ai-giving-users-home-addresses.md].
- **파일시스템 export**: 요청하자 6.8GB 런타임과 SSH key 파일까지 Drive로 내보냈습니다 [https://mouse.dev/blog/muse-runtime-export/].
- **VM escape [익명 출처 1건]**: 출시 직전 VM escape를 급히 패치했다는 보도입니다 [https://www.404media.co/meta-rushed-to-fix-muse-vm-escape-vulnerability-immediately-before-launch/].
- **Amazon 차단**: "unauthorized AI agent"로 분류됐습니다 [https://arstechnica.com/security/2026/09/muse-metas-extraordinarily-privileged-ai-assistant-has-a-serious-0-day/].
- **작업 freeze**: 작업이 중간에 멈추고, 여러 브라우저 작업이 동시에 돌 때 UI가 불명확하다는 평가입니다 [https://sigh.dev/posts/metas-muse-is-fantastic-for-web-scraping/].

**게이트웨이 차용 패턴**
1. 승인 권한 주체를 별도 프로세스·UID로 분리합니다. 승인은 Discord 버튼 같은 out-of-band 경로로만 받고 대화 경로에는 두지 않습니다 [https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse].
2. 승인 범위 5단계를 capability token으로 만듭니다(tool, destination, param hash에 묶음) [http://security.muse.ai/].
3. egress proxy에서 surrogate token을 실제 Keychain secret으로 바꿉니다 [http://security.muse.ai/].
4. lane taint: inbox나 Messages를 읽은 lane은 새 목적지로 보내기 전에 승인을 받습니다 [http://security.muse.ai/].
5. inbox 수집 단계에서 OTP와 reset·magic link를 제거하고 untrusted로 라벨링합니다 [http://security.muse.ai/].
6. "알릴 가치" salience gate와 선제성 다이얼, 그리고 cron과 연동되는 Goals ledger를 둡니다 [https://introducing.muse.ai].
7. 금지 규칙은 MEMORY 메모가 아니라 코드로 검사합니다. Marketplace 사고의 교훈입니다 [https://news.ycombinator.com/item?id=49879416].
8. Full Disk Access는 게이트웨이만 갖고, `chat.db`는 consent log를 남기는 API로만 노출합니다 [https://daringfireball.net/linked/2026/09/22/aten-muse].

## 3. xAI Grok Bot

**정체**
- 제품 설명은 "AI teammates"입니다. 사이트 브랜딩은 SpaceXSI이고, 다운로드와 영업 링크는 Cursor 도메인입니다. 그래서 xAI와 Cursor의 공동 제품으로 보이지만 이는 추론입니다 [https://x.ai/news/introducing-grok-bot].
- 2026-08-11 beta, desktop(macOS)과 iOS입니다. 사용량 pool은 따로이고 Enterprise는 대기 목록입니다 [https://x.ai/news/introducing-grok-bot].
- 확신도: 정체는 높고, 내부 구조는 중간입니다.
- **[충돌] Automations**: product·safety 파일은 docs.x.ai의 Automations webhook 문서를 인용합니다 [https://docs.x.ai/grok/automations/webhooks]. arch·critique 파일은 "x.ai Automations 제품을 찾지 못함"이라고 합니다. 문서는 존재하지만 별도 제품으로서의 위치는 불명확합니다.

**사용자 채널·UX**
- desktop과 iOS에서 같은 thread로 이어집니다. 봇끼리 메시지를 주고받고 그룹 채팅에도 들어갑니다 [https://x.ai/news/introducing-grok-bot].
- Slack도 쓸 수 있습니다 **[2차: Robinson 강연]** [https://www.peterp.me/transcripts/how-grok-bot-works/].
- X에서 @bot으로 태그할 수 있습니다 **[2차]** [https://shipmail.to/blog/grok-bot-email-on-your-own-domain].
- 2026-10-09부터 name@mail.grokbot.com 메일함이 생겼습니다. 사용자당 1개이고 봇들이 함께 쓰며, webmail은 없고, 메일이 와도 봇이 깨어나지 않습니다 **[2차]** [https://shipmail.to/blog/grok-bot-email-on-your-own-domain].
- 공식 SMS, iMessage, 음성 채널은 찾지 못했습니다 [https://x.ai/bot].
- 봇은 Name, Title, Description 세 필드로 정의합니다. "Bot Boss"를 단일 창구로 두는 패턴이 있습니다 [https://x.ai/bot/guides/grok-bot-101] [https://x.ai/bot/guides/grok-bot-for-work].

**트리거·선제성**
- 스스로 스케줄을 잡거나 이벤트(Slack thread, GitHub PR)를 기다립니다. 다른 봇도 트리거가 됩니다 [https://x.ai/bot/guides/grok-bot-101].
- 사용자가 한 번 하는 걸 보고 routine으로 저장하고, 교정을 받아 이후에는 혼자 돌립니다 [https://x.ai/news/introducing-grok-bot].
- 아침 8시·오후 5시 브리핑 패턴이 있습니다 [https://x.ai/bot/guides/grok-bot-for-work].
- Webhook 트리거 스펙 [https://docs.x.ai/grok/automations/webhooks]:
  - 트리거마다 `whsec_` secret이 있고, 한 번만 보여 주며 회전할 수 있습니다.
  - Standard Webhooks HMAC-SHA256 서명입니다.
  - timestamp 허용 범위 5분, body 상한 1MiB입니다.
  - 정상 요청은 `202`로 받고 비동기로 실행합니다. 오류는 400, 401, 413, 429, 503입니다.

**실행환경**
- 봇마다 cloud computer(데스크톱, 파일시스템, 터미널)가 있고 원격 데스크톱처럼 열어볼 수 있습니다. 세션이 바뀌어도 상태가 유지됩니다 [https://x.ai/bot/guides/grok-bot-101].
- Firecracker microVM이고 snapshot을 쓰며, agent loop는 서버에서 돕니다 **[2차: 강연 transcript]** [https://www.peterp.me/transcripts/how-grok-bot-works/].
- Temporal durable workflow, 사용자 메시지만 interrupt하고 나머지는 queue, 약 200K 토큰이나 10–60분 idle 시 compaction **[2차]** [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/].
- **[2차, 서드파티 분해 분석]** Docker 안의 Chrome을 CDP로 조작하고, `SAND_*` 토큰이 `/proc/<pid>/environ`에 노출됐다는 분석이 있습니다 [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/].
- **[충돌]** VM이 1–2주마다 경고 없이 리셋된다는 일화가 있습니다 [https://hn.algolia.com/api/v1/search?query=%22grok%20bot%22&tags=comment&hitsPerPage=40]. 벤더가 설명하는 snapshot 유지 설계와 맞지 않습니다.

**커넥터·인증·자격증명**
- 앱에 한 번 로그인해 두면 사람처럼 씁니다. CAPTCHA나 2FA가 나오면 데스크톱을 사용자에게 넘기고, 비밀번호와 API key는 secure form으로 받습니다 [https://x.ai/bot/guides/grok-bot-101].
- Cursor와 같은 MCP, plugin, skill을 쓰고 서비스당 여러 계정을 붙일 수 있습니다 [https://x.ai/bot/guides/grok-bot-101].
- 봇 하나가 로그인하면 다른 모든 봇이 그 사이트에 접근할 수 있습니다 [https://x.ai/bot/guides/grok-bot-101]. 한 사용자의 봇들이 파일과 로그인을 공유한다는 2차 보고도 있습니다 [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/].
- Template에는 secret과 개인 기억이 들어가지 않습니다 [https://x.ai/bot/guides/templates-for-grok-bot].
- Vault 내부 구조는 문서화되지 않았습니다.

**메모리**
- 대화를 기억하고 언제 알릴지 학습하며, 봇끼리 배운다는 설명이 있습니다. 저장·검색 방식은 비공개입니다 [https://x.ai/news/introducing-grok-bot] [https://x.ai/bot].
- 대화 이력은 서버 DB에, 파일·memory·skill은 object storage에 있습니다. 보존 기간과 학습 사용 정책은 찾지 못했습니다 **[2차]** [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/].

**권한·승인 게이트**
- 자연어 allow/block 규칙과 별도 review agent(allow, block, escalate)가 있습니다. 가이드 스스로 "모델이 따르길 신뢰하는 것"이라고 적고 있습니다 [https://x.ai/bot/guides/grok-bot-101].
- 되돌릴 수 없는 동작(이메일, Slack)은 초안으로 돌아와 승인을 받습니다 **[2차]** [https://www.peterp.me/transcripts/how-grok-bot-works/].
- 예시 "Tech Demos" 봇은 approve/deny를 받은 뒤 Cursor 클라우드 에이전트를 시작합니다 [https://x.ai/bot/guides/grok-bot-101].
- 결제 승인 정책은 문서화되지 않았습니다.

**가격·제한**
- **[충돌]** 가격:
  - x.ai/bot 기준 Premium $30, Super $100(3.5배), Ultra $200(12배) [https://x.ai/bot].
  - HN에는 "직원당 120/200 USD"라는 글이 있습니다 [https://news.ycombinator.com/item?id=49261514].
  - $120 단일 요금은 미검증입니다.
- 10월 12일부터 "The X Subscription"에 묶였습니다 [https://forum.cursor.com/t/introducing-the-x-subscription/174450.json].
- Ultra 사용자가 주간 pool을 1–3일 만에 소진했고, 스태프가 8월 26일에 한도를 리셋했습니다. API 크레딧 $50가 약 10분 만에 사라졌다는 보고도 있습니다 [https://forum.cursor.com/t/anyone-used-grokbot-on-the-api-very-high-costs/169551.json].

**사고·실패 사례**
- **Slack 유출**: CFO 봇이 이름이 같은 "Exec-team" 채널에 개인 은행 보고서를 올렸습니다. 읽기 전용 권한이었는데도 막지 못했습니다. xAI는 채널 사이로 정보를 옮길 때 명시적 허가를 받도록 수정했습니다 [https://africa.businessinsider.com/news/my-personal-ai-agent-posted-my-bank-details-on-company-slack-its-made-me-rethink-how/hw9t2xn] [https://hn.algolia.com/api/v1/items/50033517].
- **Web Bot Auth 서명 위조 [단일 연구자, 수정 확인 안 됨]**: 9월 23일 제보, 10월 7일 공개. Cloudflare managed challenge에서 무한 루프에 빠진다는 내용도 있습니다 [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/].
- **포럼 버그**: computer에 연결이 안 됨, Stop이 멈추지 않음, 커넥터가 72시간 넘게 멈춤 등 [https://forum.cursor.com/tag/grok-bot.json].
- **Amazon 차단 증거 없음**: Botscent가 Grok Bot을 탐지 대상에 올렸다는 정도만 확인됩니다 [https://hn.algolia.com/api/v1/search_by_date?query=grok%20bot&tags=story&hitsPerPage=30].

**게이트웨이 차용 패턴**
1. 목적지는 불변 ID(Discord channel ID, chat GUID)로 고정합니다. 이름이 여러 대상과 겹치면 거부합니다. Slack 사고의 직접적인 교훈입니다 [https://africa.businessinsider.com/news/my-personal-ai-agent-posted-my-bank-details-on-company-slack-its-made-me-rethink-how/hw9t2xn].
2. 외부로 나가는 모든 동작은 outbox에 draft로 쌓고 승인 후 보냅니다. 보내는 시점에 목적지를 다시 검증합니다 [https://hn.algolia.com/api/v1/items/50033517].
3. lane마다 자격증명과 연결을 따로 둡니다. Grok Bot의 로그인 공유와 반대 방향입니다 [https://x.ai/bot/guides/grok-bot-101].
4. 서명 webhook을 Standard Webhooks, 5분, 1MiB, 202 비동기, secret 회전 방식으로 만들고 cron 옆에 둡니다 [https://docs.x.ai/grok/automations/webhooks].
5. 일회용 secure form(또는 Discord modal)으로 받은 값을 Keychain에 넣고, 에이전트에는 handle만 줍니다. 2FA는 화면 takeover 링크로 처리합니다 [https://x.ai/bot/guides/grok-bot-101].
6. Bot Boss, 즉 chief-of-staff 단일 창구 lane을 둡니다 [https://x.ai/bot/guides/grok-bot-for-work].
7. "한 번 보여 주면 routine이 되는" 녹화기를 만듭니다 [https://x.ai/news/introducing-grok-bot].
8. 사용자 메시지만 interrupt하고 cron, inbox, helper 결과는 queue에 넣습니다. 단계 결과를 durable하게 저장합니다 [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/].
9. lane별 비용 원장, 하드 캡, 모델 라우팅을 둡니다 [https://forum.cursor.com/t/anyone-used-grokbot-on-the-api-very-high-costs/169551.json].
10. 서명 키와 토큰은 샌드박스 밖 daemon에만 둡니다 [https://nightd.dev/blog/impersonating-grok-bot-for-fun-and-no-profit/].

## 세 플랫폼 공통 결론
- 세 제품 모두 자연어 규칙이나 reviewer만으로는 부족하다고 인정합니다. Dots는 "imperfectly followed", Codex 문서는 "not a deterministic guarantee", Grok 가이드는 "trusting the model"이라고 씁니다.
- 실제 사고는 결정론적 경계가 없는 곳에서 났습니다. Grok Bot은 목적지 이름 혼동, Muse는 Marketplace 무승인 수락과 Messages 접근입니다.
- 그래서 우리 Rust daemon의 핵심 순서는 다음과 같습니다.
  1. 목적지 ID 고정과 outbox
  2. 승인 권한 주체 분리와 out-of-band 승인
  3. surrogate 자격증명
  4. lane taint와 read-only proactive lane
  5. lane별 예산과 heartbeat
</answer>

<next_steps>
- 출처 단계에서 아직 풀지 않은 항목:
  - openai.com의 Dots 출시·safety 글 원문 확인 (DevDay, 4,000개 앱, 그리고 3목록 대 4상태 규칙 충돌 해소)
  - Muse 가격 tier
  - x.ai Automations의 제품 위치
  - Grok Bot 보존 기간과 학습 정책
- 바로 설계 근거로 써도 되는 것: 위 패턴 중 1차 출처가 붙은 항목입니다. Dots Help Center, security.muse.ai, docs.x.ai webhooks, x.ai guides가 해당합니다.
</next_steps>
</results>