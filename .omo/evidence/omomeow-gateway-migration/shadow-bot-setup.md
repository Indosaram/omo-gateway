# OmOMeow Gateway Phase 2 Shadow Bot Setup Guide

이 문서는 Discord Developer Portal에서 섀도우 검증용 테스트 봇을 생성하고, Gateway 환경 변수를 구성하여 Phase 2 검증을 진행하는 절차를 설명합니다.

---

## 1. Discord Developer Portal에서 Application 생성

1. [Discord Developer Portal](https://discord.com/developers/applications)에 접속하여 로그인합니다.
2. 우측 상단의 **New Application** 버튼을 클릭합니다.
3. 애플리케이션 이름으로 `OmOMeow-Shadow`를 입력하고 이용약관에 동의한 뒤 **Create**를 누릅니다.
4. **General Information** 페이지에서 Application ID(클라이언트 ID)를 복사해 둡니다. 이 값은 봇 ID(`DISCORD_BOT_ID`)로 쓰입니다.

---

## 2. Bot 탭 설정 및 Privileged Gateway Intents 활성화

1. 좌측 메뉴에서 **Bot** 탭으로 이동합니다.
2. **Build-A-Bot** 섹션에서 **Reset Token**을 클릭하고 확인 팝업을 거쳐 생성된 토큰을 복사합니다.
   - 복사한 토큰은 절대 외부에 노출되지 않도록 안전하게 보관합니다.
3. **Privileged Gateway Intents** 섹션으로 스크롤합니다.
4. 아래 필수 Intent 2종 스위치를 켭니다:
   - **Server Members Intent** (활성화)
   - **Message Content Intent** (활성화: 메시지 내용 및 명령어 수신 필수)
5. 페이지 하단의 **Save Changes** 버튼을 눌러 변경사항을 저장합니다.

---

## 3. OAuth2 URL 생성 및 테스트 서버 초대

1. 좌측 메뉴에서 **OAuth2** > **URL Generator**를 선택합니다.
2. **Scopes** 항목에서 다음 항목들을 체크합니다:
   - `bot`
   - `applications.commands`
3. 하단에 나타나는 **Bot Permissions** 섹션에서 필요한 권한을 선택합니다:
   - **Send Messages** (메시지 전송)
   - **Create Public Threads** (스레드 생성)
   - **Send Messages in Threads** (스레드 내 메시지 전송)
   - **Read Message History** (메시지 기록 읽기)
   - **Add Reactions** (이모지 반응 등록)
   - **Attach Files** (음성 파일 등 첨부 지원)
4. 페이지 최하단에 생성된 OAuth2 URL을 복사합니다.
5. 브라우저 새 탭에 해당 URL을 붙여넣고, 테스트용 전용 디스코드 서버를 선택하여 봇을 초대합니다.
6. 테스트 서버 내에 `#bot-testing` 전용 채널을 준비합니다.

---

## 4. Gateway Shadow 환경 변수 구성 (`.env.shadow`)

Gateway 루트 디렉토리에 섀도우 실행용 `.env.shadow` 파일을 생성합니다.

```bash
# .env.shadow 예시
DISCORD_BOT_TOKENS=<SHADOW_TOKEN>
DISCORD_BOT_ID=<SHADOW_BOT_ID>

# 기존 메인 설정에서 상속하거나 테스트용으로 지정할 항목들
NODE_ENV=development
LOG_LEVEL=debug
MONITORING_CHANNEL_ID=<TEST_MONITORING_CHANNEL_ID>
```

실행 시 환경 변수를 적용하는 방법:

```bash
# 섀도우 환경 파일로 실행
dotenv -e .env.shadow -- bun run src/index.ts
```

또는 인라인 환경 변수로 실행할 수 있습니다:

```bash
DISCORD_BOT_TOKENS="your_shadow_bot_token" DISCORD_BOT_ID="your_shadow_bot_id" bun run src/index.ts
```

---

## 5. Phase 2 섀도우 검증 항목 확인 절차

`#bot-testing` 채널에서 섀도우 봇을 대상으로 다음 6가지 핵심 항목을 순차적으로 검증합니다.

### 1) 👀 즉시 반응 (Instant Reaction)
- 사용자가 채널 또는 스레드에서 봇을 멘션(`@OmOMeow-Shadow`)하거나 메시지를 보냅니다.
- 봇이 즉시 눈 모양 이모지(`👀`) 반응을 등록하는지 확인합니다.

### 2) 첫 응답 3초 내 전달 (TTSR SLA)
- 질문 메시지를 보낸 시점부터 봇의 첫 스트리밍 청크 또는 안내 응답이 출력될 때까지 걸린 시간을 측정합니다.
- 목표치인 3초 이내에 첫 응답이 도착하는지 체크합니다.

### 3) 버튼 3종 컴포넌트 동작
- 생성된 응답 메시지 하단에 제공되는 3개 버튼을 순서대로 테스트합니다:
  - **재시도 (Retry)**: 직전 질의를 동일 컨텍스트로 다시 실행하는지 확인
  - **요약 (Summary)**: 현재 스레드 대화 내역 요약을 생성하는지 확인
  - **종료 (Close/Done)**: 활성 스레드 작업을 정리하고 완료 상태로 변경하는지 확인

### 4) 음성 전사 (Voice Transcription)
- `#bot-testing` 채널에 음성 메시지(OGG/MP3 등 오디오 파일)를 전송하거나 음성 메모를 보냅니다.
- 게이트웨이가 오디오를 수신하여 텍스트로 정상 전사하고, 전사된 텍스트 기반 답변을 반환하는지 확인합니다.

### 5) 스레드 수명주기 (Thread Lifecycle)
- 채널에서 질의를 시작할 때 새 스레드가 정상 생성되는지 확인합니다.
- 스레드 내 연속 질의응답이 컨텍스트를 유지하며 이어지는지 테스트합니다.
- 일정 시간 비활성 후 자동 보관(archive) 또는 종료 요청 시 스레드 락 및 정리가 정상 수행되는지 점검합니다.

### 6) SLA 감시 및 알림 (SLA Monitoring)
- 인위적으로 지연을 유발하거나 응답 임계치를 초과하는 시나리오를 구성합니다.
- 지연 경고 로그가 기록되고, 지정된 모니터링 채널 또는 로그에 SLA 초과 알림이 찍히는지 확인합니다.
