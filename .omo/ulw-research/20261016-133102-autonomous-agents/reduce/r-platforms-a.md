<!-- task=st_01a14525 -->
st_01a14525 [completed] model mahoquot/claude-opus-5-5
I read this as a reduction task: read all 12 wave1 files in full and turn them into profiles for 3 platforms with 10 sections each. I'll stop once the reduction below is delivered.

Ask: Instinct·Pickle·Hark wave1 리서치 12개 파일을 플랫폼별 프로필로 축약 - wanted: 출처가 붙은 10개 섹션 프로필. For you: 12개 파일을 모두 읽고 3개 프로필로 축약했습니다. 파일 간 충돌은 **[충돌]**, 2차 출처나 미검증 주장은 **[SECONDARY]** 또는 **[미검증]**으로 표시했고, 출처가 없는 사실은 뺐습니다. Now: 없음(완료). Next: 없음.

---

# 1. Instinct

**정체**
- 회사: Spear Street Technology, Inc. d/b/a Instinct(San Francisco). https://instinct.com/privacy-policy
- 창업자: 전 Sierra 연구원 Noah Shinn. https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/
- 법인 등록: 2026년 4월. Forbes 보도를 Vellum이 인용한 것입니다 **[SECONDARY]**. https://www.vellum.ai/blog/official-instinct-breakdown
- URL: canonical은 instinct.com이고 앱은 app.instinct.com입니다. instinct.co도 같은 사이트를 보여 준다고 하는데(https://www.instinct.co/), critique 파일은 두 도메인이 같은 사이트인지 확인하지 못했다고 적었습니다 **[약한 충돌]**.
- 출시: 2026년 8월 초대제 비공개 액세스로 나왔고 8월 20~24일에 바이럴이 됐습니다(TechCrunch 상동).
- 펀딩 **[충돌]**:
  - $250M Series B, 밸류 $2.5B (TechCrunch, Vellum)
  - $350M (Spinnable, HN 제목). https://news.ycombinator.com/item?id=49457217
  - 9월 28일 $1B Series C, 밸류 $10B (TC API). https://techcrunch.com/wp-json/wp/v2/posts?slug=viral-ai-agent-instinct-raises-1b-series-c-at-a-10b-valuation
- 확신도: 실체는 높음(약 95%). UX와 아키텍처는 중간인데, 공식 문서 없이 경쟁사 블로그(Vellum, Spinnable, Carly)에 의존하기 때문입니다.

**사용자 채널·UX**
- 공식 표현은 "no new interfaces"이고, 문자나 전화로 씁니다. https://instinct.com/
- 채널은 SMS, WhatsApp, iMessage, 전화입니다. https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/ https://www.usecarly.com/blog/what-is-instinct-ai/
- Slack, Discord, email 채널을 지원한다는 증거는 없습니다.
- privacy policy에 "Mac OS and mobile applications"가 언급됩니다. https://instinct.com/privacy-policy
- **[충돌]** TC 9/28 기사는 그 시점에 모바일 앱이 없었다고 썼고(위 TC API URL), privacy policy는 모바일 앱을 언급합니다.
- 모든 대화가 단일 연속 스레드입니다. 작업별 스레드 분리 요구가 가장 큰 UX 불만입니다. https://www.vellum.ai/md/blog/official-instinct-breakdown
- 진행 보고는 일반 채팅입니다. Amazon 주문 하나에 메시지 13개가 오갔다는 사례가 있습니다. https://www.usecarly.com/blog/what-is-instinct-ai/

**트리거·선제성**
- 다음 주장은 모두 **[SECONDARY/미검증]**이고 공식 문서가 없습니다.
  - 먼저 전화나 문자를 하고 끊긴 스레드를 follow-up 한다. https://www.vellum.ai/md/blog/official-instinct-breakdown
  - 투자자 Acharya의 표현으로 "recurring loops"와 "top-level agent that monitors and dispatches". https://www.usecarly.com/blog/what-is-instinct-ai/
- watcher 실사용 사례로, 매진된 IMAX 취소표를 하루 안에 찾았습니다(Carly 상동).
- daily brief, cron, watcher 설정 UI는 문서화된 것이 없습니다.

**실행환경**
- 자체 전화번호와 지속형 cloud computer를 씁니다(TC 9/28 API URL).
- 브라우저와 cached credentials를 쓴다는 것은 Vellum의 서술입니다 **[SECONDARY]**.
- "vision-language model"을 쓴다는 Spinnable 주장은 근거가 없습니다 **[미검증]**. https://spinnable.ai/blog/what-is-instinct-ai-guide
- 클라우드에는 사용자 쿠키나 주거용 IP가 없어 CAPTCHA, 2FA 벽, 크래시를 겪었습니다. https://www.usecarly.com/blog/what-is-instinct-ai/

**커넥터·인증·자격증명**
- Google Workspace(Calendar, Gmail, Drive, Docs, Sheets, Slides, Tasks)를 OAuth로 연결합니다. 이 데이터는 학습에서 제외됩니다. https://instinct.com/privacy-policy
- 사용자가 제3자 서비스 username·password와 결제정보를 직접 맡깁니다(상동).
- 에이전트가 비밀번호를 스스로 재설정해 구매를 마친 사례가 있습니다(Acharya). https://www.vellum.ai/blog/official-instinct-breakdown
- 연결 해제와 삭제는 따로 이뤄집니다. 인덱싱된 데이터는 /workspace에서 별도로 지워야 합니다. https://instinct.com/privacy-policy
- "Vault" 기능이 있다고 하는데 Terms 본문에서 확인하지 못했습니다 **[미검증]**(Vellum).

**메모리**
- 단일 스레드와 외부 소스 인덱싱 "Workspace"가 있습니다. 상호작용 맥락을 기반으로 개인화합니다. https://instinct.com/privacy-policy
- 메모리 스키마는 공개되지 않았습니다.

**권한·승인 게이트**
- 체계적인 승인 UI가 있다는 증거는 없습니다.
- 공식 문구는 "We encourage you to review actions"로, 사후 검토만 권합니다. https://instinct.com/privacy-policy
- Terms는 사용자 대신 구속력 있는 계약을 맺을 수 있다고 규정합니다(TechCrunch 8/24).
- 책임은 사용자에게 있다는 해석은 Vellum의 것이고 원문으로 확인하지 못했습니다 **[SECONDARY]**.
- "no approval gates"라는 주장은 경쟁사 Spinnable만 했고 근거가 없습니다 **[미검증]**.

**가격·제한**
- 비공개 액세스 기간에는 무료이고 가격은 미공개입니다. 구독과 광고 모두 검토 중이라고 합니다(Vellum이 Forbes 인용).
- Spinnable의 "$200–500/월"은 근거 없는 추정입니다 **[미검증]**. https://www.spinnable.ai/blog/what-is-instinct-ai-guide
- waitlist 또는 초대제이고, 제약은 compute라고 합니다.

**사고·실패 사례** (TechCrunch 8/24 URL 기준)
- Katie Jacobs Stanton: 확인 없이 이메일이 발송됐습니다.
- Alex Cohen: 자기 이메일에 넣은 prompt injection 지시를 에이전트가 따랐습니다.
- Claire Vo: Google 연결을 끊은 뒤에도 받은편지함 요약을 받았고 평문 저장이 드러났습니다. 이후 삭제 도구가 추가됐습니다. Peter Yang도 같은 문제를 겪었습니다.
- Resy 가입 코드를 받은편지함에서 임의로 꺼내 쓴 사례가 있습니다.
- Terms의 "perpetual and irrevocable" 라이선스와 화면·키 입력 캡처 조항이 문제가 됐습니다.
- 8/26 개정으로 perpetual 조항이 삭제됐다는 보도는 Carly만 했습니다 **[SECONDARY]**. https://www.usecarly.com/blog/instinct-ai/
- CNN 보도로, 시간당 수백 회 폴링한 뒤 Resy 계정이 정지됐습니다 **[SECONDARY, Carly 경유]**.
- TheStreet 보도로, 9/21~22 장애 동안 상태 페이지가 없었습니다 **[SECONDARY]**.
- Amazon 차단 주장은 출처가 없습니다(critique 파일 확인 결과).
- Spinnable이 든 실패 사례와 조항 번호는 출처가 없어 날조로 추정됩니다.

**우리 게이트웨이에 차용할 패턴** (반면교사)
1. 작업마다 Discord thread나 lane을 하나씩 두고, 메인 채널에는 요약만 올립니다.
2. inbox ingestion 내용은 tainted로 표시합니다. 이 lane에는 send나 spend 도구를 주지 않고, 이메일에서 파생된 tool call은 반드시 owner 확인을 거칩니다(Cohen 사례 대응).
3. `revoke`를 실행하면 토큰과 파생 인덱스·요약을 원자적으로 삭제하고 삭제 내역을 보고합니다(Vo 사례 대응).
4. watcher를 일급 객체로 둡니다(target, interval, backoff, expiry). list, pause, kill이 가능해야 하고 도메인별 rate budget을 둡니다(Resy 사례 대응).
5. `/health`, lane 상태 명령, heartbeat, 발송 기록과 대기 목록을 갖춥니다(9/21 장애 대응).
6. 비밀번호 재설정이나 OTP 조회는 승인된 흐름에서만 허용합니다.
7. 로컬 Mac의 실제 브라우저 프로필과 주거용 IP를 쓰는 것을 차별점으로 삼습니다(Springwater 비판의 역이용).

---

# 2. Pickle (피클)

**정체**
- 회사: Pickle, Inc.(Hillsborough, CA). https://pickle.com/privacy-policy
- YC: pickle-2, W25, 6명, SF. https://www.ycombinator.com/companies/pickle-2
- 창업 시점 **[충돌]**: product·safety 파일은 YC 기준 "founded 2024"라고 적었고, GitHub org는 2023-11-21에 생성됐습니다. https://github.com/pickle-com/credential-enclave
- 창업자: Daniel Park(CEO), Sanio Jung, Emmett Kim, Hojin Yu("Sam Yu, CTO").
- iOS 앱: id6795790129, `com.pickle.client`. 2026-08-10 첫 출시, 현재 v1.1.12(2026-10-13), 무료, 17+. https://itunes.apple.com/lookup?id=6795790129&country=us
- 평점: US 4.59(41개), KR 5.0(3개). 표본이 작습니다.
- 슬로건 "Raise your AI"는 그 문구 그대로는 찾지 못했습니다. 비슷한 표현은 GitHub org의 "A personal AI you raise"와 privacy policy의 "AI를 키우는 앱"입니다. https://pickle.com/assets/privacy-policy-BtYR0eAZ.js
- 피벗 이력:
  - product 파일은 2025년 화상회의 "클론" 제품과 같은 회사인지 미확인으로 남겼습니다.
  - arch 파일은 YC launch post의 "AI body double for video calls"로 피벗을 확인했습니다.
  - critique 파일은 2026년 1월 "Pickle 1" AR 안경을 판 회사와 같다고 중상 정도의 확신으로 봅니다. 근거는 Daring Fireball이 같은 pickle-2 페이지와 CEO를 링크한 점입니다.
  - 직원 수는 Gruber 기사의 15명에서 현재 6명입니다 **[충돌/변화]**.
- 확신도: 실체는 높음. 반대로 "피클플러스"는 별개 회사입니다.

**사용자 채널·UX**
- iPhone 앱 하나로, 메시지·음성·전화를 지원합니다. https://itunes.apple.com/lookup?id=6795790129&country=us
- SMS, iMessage, Slack을 대화 채널로 쓴다는 증거는 없습니다. Slack 등은 credential provider로만 나옵니다. https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md
- 3D 캐릭터를 쓰고 EN/JA/KO를 지원합니다.
- 결제 전에 채팅에 주문 내역, 총액, 결제 버튼이 담긴 approval card가 뜹니다. https://pickle.com/assets/security-n1i5zLW2.js
- 통화와 브라우저를 실시간으로 볼 수 있고, 사람이 직접 넘겨받을 수 있습니다(human takeover)(US listing).

**트리거·선제성**
- 메일과 캘린더를 보고 작업을 **제안**합니다. 예: 분실 택배, 주차 딱지, 항공 환불. 자동 실행이 아니라 suggestion-first입니다. https://itunes.apple.com/lookup?id=6795790129&country=kr
- watcher: 매진 좌석 취소표가 나오면 알리고, 요청하면 바로 예약합니다(상동).
- 모닝 브리핑과 예약 작업 **[충돌]**: product 파일은 증거가 없다고 했고, arch 파일은 YC 페이지가 morning briefing과 scheduled tasks를 언급한다고 했습니다. https://www.ycombinator.com/companies/pickle-2
- 밤사이 커뮤니티 게시판을 자율로 탐색하고 아침에 보고합니다. https://pickle.com/assets/privacy-policy-BtYR0eAZ.js

**실행환경**
- cloud에서 에이전트 자신의 컴퓨터와 브라우저로 일합니다. 브라우저는 Kernel이 제공하고 사용자별 browser profile이 유지됩니다. https://pickle.com/privacy-policy
- 격리 구역은 셋입니다. https://pickle.com/assets/security-n1i5zLW2.js
  - Nitro Enclave vault
  - 작업마다 닫히는 사용자별 가상 브라우저(로그인 상태만 유지)
  - vault나 내부망에 닿을 수 없는 격리 파일 workspace
- 전화는 Telnyx로 걸고, 한국 번호 파트너 이름은 파일마다 다릅니다 **[충돌: arch는 "Flover", safety는 "Plover"]**.
- 인프라는 AWS US-West와 LiveKit, 인증은 Stytch와 Sign in with Apple입니다.
- 로컬 접근은 iPhone 권한(연락처, 캘린더, 사진, 위치 "Always")에 한정됩니다. Mac daemon은 없습니다.
- product 파일은 사용자별 전용 cloud가 미확인이라고 했지만, arch·safety 파일이 사용자별 브라우저와 workspace를 확인했으므로 부분 해소됐습니다.

**커넥터·인증·자격증명**
- 연결 서비스: Google, Microsoft(Teams 포함), Slack, Notion, X, Granola, Mercury(MCP+DCR), GitHub, Link(Stripe), Plaid(읽기 전용). https://pickle.com/privacy-policy
- NAVER Mail은 앱 비밀번호와 고정 IMAP/SMTP만 씁니다. https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/egress-policy.md
- KR listing 연결 대상: Naver, KakaoTalk, Gov24, Coupang, KTX 등. US listing 연결 대상: Amazon, DoorDash 등. 시장마다 다릅니다.
- **credential-enclave**(Rust, Apache-2.0, 2026-10-01 생성, v1.5.2): https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/README.md
  - 마스터 키는 iPhone에서 생성해 iCloud Keychain에 둡니다. attestation을 검증한 뒤 최대 30일 동안 사용자 키를 enclave 메모리에 위임합니다.
  - enclave 안에서 OAuth와 PKCE를 처리합니다. `forward`는 allow-list에 있는 host와 path에만 토큰을 주입하고, 토큰 자체는 반환하지 않습니다. https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/docs/enclave.md
  - 응답에 토큰이 base64, hex, percent 인코딩 형태로라도 반사되면 `response_withheld` 처리합니다(egress-policy URL).
  - vault 값은 `release`로 필드 단위로만 operator worker에게 나갑니다. 그 이후에는 보호되지 않는다고 README가 자인합니다. TOTP seed는 밖으로 나가지 않고 6자리 코드만 나갑니다.
  - 모델은 실제 값을 보지 못하고 "filled" 표시만 봅니다. https://pickle.com/assets/security-n1i5zLW2.js
  - README가 밝힌 한계: iOS 앱은 비공개, Slack·Notion은 PKCE 없음, operator가 예고 없이 새 버전을 배포할 수 있음.

**메모리**
- 일기("memory bubbles"), 사용자 노트, 주제 노트, 긴 대화 요약, 검색 인덱스가 있습니다. 사용자가 편집하고 삭제할 수 있습니다. https://pickle.com/privacy-policy
- 음성 원본은 저장하지 않고 transcript만 남깁니다. 카메라 프레임은 몇 초마다 샘플링합니다.
- 보존 기간: 백그라운드 작업 기록 30일, 서버 로그 90일, 위치 7일. https://pickle.com/assets/privacy-policy-BtYR0eAZ.js

**권한·승인 게이트**
- 돈을 쓰거나 되돌릴 수 없는 작업은 승인을 기다립니다(US listing).
- 금액이 오르면 다시 묻습니다. https://pickle.com/assets/faq-pFbILNx9.js
- Link 결제는 Link 쪽에서 별도로 승인하고 Link의 한도가 적용됩니다. https://pickle.com/assets/terms-of-service-BQeY61qP.js
- 카드와 연결 계정 접근은 앱을 마지막으로 연 뒤 30일(또는 더 짧게) 유효한 time-box입니다(ToS 상동).
- Pickle끼리 대화할 때는 외부 상대용 에이전트가 읽기와 대화만 하고 예약·결제·파일 도구가 없습니다. 민감 범주는 기본으로 공유하지 않고 never-share 목록이 있습니다(security JS).
- 감사 로그는 사용 **전에** 기록합니다. S3 Object Lock(365일)에 사용자만 열 수 있게 봉인하며, 기록에 실패하면 credential을 사용하지 않습니다(README).
- prompt injection 방어를 다룬 문서는 없습니다. 구조적 완화책만 있습니다.

**가격·제한**
- 무료이고 "Pickle Plus" 구독(Apple 결제)이 있습니다.
- 사용량은 compute 비용 기반의 "energy"로 계량하고, 바닥나면 일시정지됩니다. 가격 숫자는 미공개입니다(ToS JS).
- 지역 **[충돌]**: 영문 ToS는 미국과 18세 이상으로 한정하는데, KR listing이 운영 중이고 YC는 "한국은 다음"이라고 합니다.
- privacy policy 시행일이 2026-10-22로 오늘보다 미래입니다(pending 버전).

**사고·실패 사례**
- 에이전트 앱 자체의 실사용 리뷰나 사고는 찾지 못했습니다(HN 0건). https://hn.algolia.com/api/v1/search?query=pickle%20ai%20agent%20korea
- 회사 수준 평판: 2026년 1월 "Pickle 1 AR Glasses May Be Fraudulent" 글이 HN에서 110점을 받았고, Gruber가 "sham"이라고 썼습니다 **[RISK high]**. https://hn.algolia.com/api/v1/items/46483429 https://daringfireball.net/linked/2026/01/03/dowd-pickle-expose
- 앱 listing 스스로 "부정확하거나 예상치 못한 행동"을 할 수 있다고 고지합니다.
- 출시 후에도 Naver IMAP 문제(UIDVALIDITY 0)와 메일 파서 경화 작업이 이어졌습니다. https://github.com/pickle-com/credential-enclave/issues?state=all
- SmartThings provider에서는 잠금 해제, 열기, 난방 명령을 거부합니다. https://github.com/pickle-com/credential-enclave/pull/6

**우리 게이트웨이에 차용할 패턴**
1. Rust credential broker를 `forward` 방식으로 만듭니다. agent에게는 handle만 주고, allow-list에 있는 host·path에만 header를 주입하며, 응답에 토큰이 반사되는지 검사합니다. Nitro 대신 macOS Keychain/Secure Enclave를 씁니다.
2. write-ahead 감사 로그: 서명하고 hash-chain으로 묶은 append-only 기록을 먼저 쓰고, ack를 받은 뒤에만 실행합니다("no record, no use"). owner는 `/log`로 열람합니다.
3. lane마다 N일 capability lease를 주고 만료되면 KakaoTalk이나 Discord에서 "renew"를 받아야 합니다.
4. 승인 카드를 typed 이벤트로 만듭니다(action, payee, amount). 금액이 바뀌면 다시 묻고, 타임아웃이 나면 거부로 처리합니다.
5. inbox ingestion은 실행이 아니라 proposal queue로 보냅니다. 무인 실행은 규칙별 opt-in으로만 허용합니다.
6. 외부 입력을 다루는 lane은 읽기와 대화 도구만 받습니다(구조적 injection 방어).
7. secret을 serialize할 수 없는 Rust 타입을 둡니다. 폼 채우기는 LLM이 아닌 filler가 맡고 모델은 "filled"만 봅니다.
8. IMAP ingestion: 파서 크기와 메모리를 제한하고, admission queue를 분리하고, Naver의 이상 동작을 테스트합니다.
9. 물리 기기 명령은 default-deny로 두고 scene 같은 우회 매크로는 제외합니다.

---

# 3. Hark (Hark Pro / Handoff)

**정체**
- 회사: Hark Labs, Inc.(San Jose). Figure AI의 Brett Adcock가 세웠습니다. https://hark.com/terms https://hark.com/articles/hark-announces-usd700-million-fundraising-round
- URL: hark.com. Hark Pro는 iOS/Android/web 앱이고, Handoff는 그 안의 computer-use 모델 겸 에이전트입니다. https://hark.com/using-hark
- 타임라인:
  - 2025년 말 Adcock가 자기 돈 $100M으로 설립했고 2026년 3월경 stealth를 벗어났습니다 **[SECONDARY]**. https://thenextweb.com/news/hark-series-a-funding-adcock-ai-hardware
  - 2026-05-20 $700M Series A, 밸류 $6B.
  - 2026-08-05 Handoff preview. https://hark.com/articles/introducing-hark-handoff
  - 2026-10-06 Pro 출시. https://hark.com/articles/introducing-hark-pro
  - 하드웨어는 2027년 예정.
- 옛 hark.com 오디오 클립 사이트와는 다릅니다. "Hark decision intelligence"는 세 파일 모두 찾거나 검증하지 못했습니다.
- 확신도: 실체는 높음(약 95%). 아키텍처는 vendor 마케팅 수준입니다.

**사용자 채널·UX**
- iOS, Android, web만 지원하고 SMS, iMessage, 음성, Slack 채팅 채널은 없습니다. Slack은 추적 소스일 뿐입니다. https://hark.com/using-hark https://hark.com/pricing
- 하나의 지속 스레드와 세 개 화면으로 구성됩니다. https://hark.com/using-hark
  - Home: Action Cards와 Panels(사용자가 요청한 미니앱)
  - Chat
  - Projects: 주제별 스레드와 파일
- 짧게 나눠 답하고, "not a chatbot/assistant/agent"를 표방합니다. https://hark.com/articles/introducing-hark-pro

**트리거·선제성** (가장 큰 차별점)
- "proactive thinking"이 세 기둥 중 하나입니다. 예를 들어 영화표를 산 뒤 주차를 제안하고, 적절한 시점에 QR을 보내는 식으로 단계를 나눠 전달합니다. https://hark.com/articles/introducing-hark-pro
- 학생 급식카드 잔액이 부족하면 감지해서 알리고, 허용("if you allow it")하면 직접 충전합니다(상동).
- 자연어로 Scheduled Tasks를 만들 수 있습니다(매일 커피, 주간 알림).
- Action Buttons("Pay", "Cancel")는 메일과 대화에서 생성됩니다.
- Specialties는 워크플로를 기억했다가 자동으로 만들어 줍니다. https://hark.com/using-hark
- 주문 추적에 몇 분 지연이 있습니다. polling으로 추정되지만 추론입니다.
- daily brief나 watcher 설정 기능은 문서에 없습니다.

**실행환경**
- 요청마다 전용 가상 컴퓨터를 띄웁니다(브라우저, 파일시스템, 터미널). https://hark.com/articles/introducing-hark-handoff
- 동시에 최대 6개 브라우저를 씁니다. TNW가 처음 36개로 썼다가 6개로 정정했습니다 **[정정된 충돌]**. https://thenextweb.com/news/hark-pro-agent-hardware-europe
- DOM을 쓰지 않고 x/y 좌표 기반 커서와 키보드로 조작합니다. 단계마다 브라우저 지연이 약 10초입니다.
- 웹 사이트들의 bot-blocking과 팝업을 주요 장애물로 스스로 인정합니다(Handoff 글).
- 로컬 기기에는 접근하지 않습니다.

**커넥터·인증·자격증명**
- 직접 API로 연결되는 "Services"는 Google Workspace와 Microsoft 365이고, 그 밖의 사이트는 Handoff가 처리합니다. MCP는 client로만 지원합니다. https://hark.com/using-hark https://hark.com/articles/introducing-hark-pro
- "Secured by Hark" vault는 카드와 비밀번호를 토큰화하고, 목적지 페이지에서만 de-tokenize한다고 주장합니다 **[vendor 주장, 미검증]**.
- trust.hark.com은 열람 요청을 해야 볼 수 있어 확인하지 못했습니다. https://trust.hark.com/
- Terms상 세션 쿠키와 토큰도 저장되며, 그로 인한 책임은 대부분 사용자에게 있습니다. https://hark.com/terms
- OAuth scope는 문서화되지 않았고, 개발자 API나 SDK도 없습니다. https://hark.com/sitemap.xml

**메모리**
- 선호를 쌓아 "portrait"를 만들고 모든 상호작용에 적용합니다. 대화로 "forget"하라고 하면 지웁니다. https://hark.com/articles/introducing-hark-pro
- 데이터는 계정이 있는 동안 보존하고 백업은 90일 안에 지워집니다. 학습에 쓰지 않고, 제3자 모델은 보존 0입니다. https://hark.com/privacy-policy https://hark.com/security
- 실행 로그(shell 명령, 출력, 코드)는 디버깅과 보안 목적으로 보관합니다.

**권한·승인 게이트 [충돌]**
- product 파일은 승인 흐름이 미문서화라고 결론냈습니다.
- safety 파일은 Terms에서 "Unless you tell us otherwise" 문구로 돈을 쓰거나 메시지를 보내기 전 확인이 기본값이고, 범주별 standing permission이 있음을 확인했습니다. https://hark.com/terms
- **Terms가 1차 출처이므로 safety 쪽이 맞습니다.**
- 예약 작업과 선제적 행동은 부여된 권한 안에서만 실행되고, 일시정지·취소할 수 있습니다. Hark도 위험한 행동은 스스로 거부할 수 있습니다(Terms).
- 실행을 실시간으로 지켜볼 수 있고 항상 취소 버튼이 있습니다. "default to consent"로 시작해 원하는 행동을 학습합니다. https://hark.com/security
- 사용자용 감사 로그와 injection 방어는 공개되지 않았습니다. Terms에 사용자의 prompt injection을 금지하는 조항만 있습니다.

**가격·제한**
- Free(모든 기능, 사용량 상한), Pro² $20/월(사용량 2배), Pro³ $100/월(사용량 10배). https://hark.com/pricing
- 상한 수치는 미공개이고 크레딧은 환불되지 않습니다. auto top-up에는 사용자가 정하는 ceiling이 있습니다. https://hark.com/using-hark
- 책임 한도는 12개월 요금 또는 $100(EU는 6개월 또는 €100)입니다. merchant of record가 아닙니다(Terms).
- EU 출시 여부는 미정입니다. PSD2 SCA에는 예외가 없다는 지적이 있습니다(TNW).
- Meta Muse가 같은 가격대라는 내용은 **[SECONDARY]**입니다.

**사고·실패 사례**
- 출시 약 10일째라 독립 리뷰나 사고 보고가 없습니다. HN 글 2건은 댓글 0개입니다. https://hn.algolia.com/api/v1/items/49980710
- 벤치마크 **[충돌]**:
  - Hark는 Online-Mind2Web 1위이고 "independently verified"라고 주장하며, 비교 대상을 GPT 5.4와 Opus 4.8로 듭니다.
  - RockingRobots는 독립 검증이 안 됐다고 보고, 비교 대상을 GPT-5.5로 적었습니다. https://www.rockingrobots.com/hark-unveils-handoff-an-ai-agent-that-completes-tasks-directly-in-a-browser/
  - 일부 비교 수치는 OpenAI 자체 보고이거나 Browserbase 보고입니다.
- 업계 차원 위험으로, Resy가 에이전트 사용 금지를 공지했는데 Resy는 Hark 데모 사이트입니다 **[SECONDARY, TNW가 Axios 인용]**. https://thenextweb.com/news/meta-muse-personal-ai-agent-launch

**우리 게이트웨이에 차용할 패턴**
1. 행동 범주별 3상태 정책 `ask | standing | deny`를 두고 기본값은 ask로 합니다. standing에는 건당·일일 지출 ceiling을 둡니다. Hark의 standing에는 상한이 보이지 않는데, 이를 보완하는 것입니다.
2. 같은 범주를 N번 명시 승인하면 standing으로 승격하는 consent ledger를 둡니다("default to consent, learn").
3. Action Button: inbox에서 파생된 제안을 Discord 버튼이나 iMessage·KakaoTalk 응답 키워드로 보내 원탭 승인을 받습니다. 승인은 action hash에 묶어 재사용(replay)을 막습니다.
4. 후속 단계 페이싱: plan을 시간 지정 단계로 저장하고 cron lane이 적절한 시점에 하나씩 전달합니다.
5. Specialties: 성공한 omo 실행을 이름과 버전이 있는 playbook으로 승격합니다.
6. 모든 lane에 실시간 상태와 `/cancel`(프로세스 트리 종료)을 두고, 전역 `/stop`을 둡니다.
7. 단계 예산과 wall-clock 타임아웃, "still working" heartbeat를 둡니다(약 10초/단계 지연 대응).
8. CAPTCHA나 bot-wall을 감지하면 해당 사이트를 "needs human"으로 표시하고 owner에게 세션을 넘깁니다. API와 MCP를 먼저 쓰고 픽셀 조작은 마지막 수단으로 씁니다.
9. Projects처럼 주제별 thread, 파일, 메모리 scope를 두고, `forget`과 그 감사 기록을 남깁니다.

---

## 세 플랫폼 공통으로 비어 있는 부분
- 세 곳 모두 prompt injection 방어를 공개하지 않았습니다.
- 사용자가 볼 수 있는 감사 로그는 Pickle만 있습니다.
- 모닝 브리핑은 Pickle만 YC 페이지에서 언급합니다(product 파일과 충돌).
- 이 공백을 메우는 것(taint lane, write-ahead ledger, cron brief)이 우리 게이트웨이의 차별점입니다.

<results>
<files>
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/instinct-product.md - Instinct의 채널, UX, 선제성, 승인
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/instinct-arch.md - Instinct의 실행환경, 커넥터, 메모리, 모델
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/instinct-safety.md - Instinct의 승인, injection, 보존, 펀딩 충돌
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/instinct-critique.md - Instinct 사고 사례와 출처 신뢰도
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/pickle-product.md - Pickle 정체, App Store 기반 UX
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/pickle-arch.md - Pickle 인프라, credential-enclave, 메모리, 모델
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/pickle-safety.md - Pickle 승인 카드, 감사 로그, 가격
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/pickle-critique.md - Pickle AR 안경 평판, issue tracker
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/hark-product.md - Hark UX, 선제성, Action Buttons
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/hark-arch.md - Hark Handoff 실행환경, 모델
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/hark-safety.md - Hark Terms 기반 승인, 보존, 책임
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/hark-critique.md - Hark 증거 공백, 벤치마크 충돌
</files>
<answer>위 본문이 플랫폼별 축약 프로필입니다. 핵심 충돌은 다섯 가지입니다. Instinct 펀딩($250M/$350M/$1B), Pickle 모닝 브리핑 유무, Pickle 한국 통화 파트너 이름(Flover/Plover), Pickle 지역(US 한정 ToS와 KR 출시), Hark 승인 게이트(product는 미문서화, Terms 1차 출처는 기본 확인)입니다.</answer>
<next_steps>wave2에서 확인할 것: Instinct Terms 원문, Pickle help-center 승인 페이지, trust.hark.com, Hark App Store 권한 목록.</next_steps>
</results>