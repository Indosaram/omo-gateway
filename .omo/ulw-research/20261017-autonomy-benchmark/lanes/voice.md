# 자율 에이전트 벤치마크 테마 분석: 음성 (Voice / Phone)

## 판정

**보류 (Later)**

omon-gateway는 단일 소유자(1인 오너) 중심의 macOS 데스크톱 상주 데몬으로, 비동기 텍스트 메신저(Discord, Slack)와 이벤트 기반 크론/인박스 작업 관리가 주 사용 목적입니다. 오너의 핵심 가치는 무인 자가 치유 크론, 비동기 인박스 수집(메일/카카오/iMessage), 검증된 작업 레인 및 방해 없는 침묵(KST 0~8시 발송 금지, 불필요한 폴링/토큰 낭비 배제)에 있습니다.
현재 오너 환경에서는 Discord 음성 첨부파일 전사(STT) 기능이 이미 운영 중이며 오너의 일상 업무를 충분히 소화하고 있습니다. 반면 전이중(Full-duplex) 실시간 스트리밍 음성 대화나 외부 전화(Twilio/Telnyx) 발신 기능은 macOS 상주 백그라운드 데몬이라는 특성상 상시 마이크 대기 리소스 낭비, WebRTC/SIP 세션 관리 복잡도, 통신망 비용 발생 대비 실질적 1인 오너 효용이 낮습니다. 따라서 P0(기반·보안) 및 이벤트 큐/승인 게이트웨이가 안정화될 때까지 실시간 음성/전화 기능 확장은 **보류**합니다.

---

## 벤치마킹 원천

| 플랫폼 | 주요 음성/전화 기능 패턴 | 출처 및 신뢰도 태그 |
|---|---|---|
| **Pickle** | • iOS 앱(`com.pickle.client`) 중심의 음성 대화 및 양방향 통화<br>• 사용자를 대행하여 고객센터, 요금 할인, 환불, 예약 등 외부 업체에 실제 전화 발신(Telnyx 기반)<br>• 통화 실시간 모니터링 및 완료 후 통화 요약 제공<br>• 음성 원본 오디오는 보존하지 않고 텍스트 요약/일기만 저장 | [1차] Y Combinator, App Store listing (id6795790129), privacy policy<br>[1차] `pickle-com/credential-enclave` README |
| **Instinct** | • "No new interfaces" 기조 하에 별도 UI 없이 전용 전화번호 및 SMS, WhatsApp, iMessage, 통화로 상호작용<br>• 지속형 클라우드 컴퓨터와 자체 전화번호로 외부 통화(Concierge calls) 수행<br>• 선제적 전화/문자 발신 기능 | [1차] instinct.com 공식 홈페이지<br>[2차] TechCrunch (2026-09-17, 2026-09-28 보도)<br>[2차/미검증] Vellum AI breakdown |
| **Hark** | • 선제적 음성 하드웨어 및 Handoff 인터페이스 표방 | [미검증] (선행 조사에서 구체적인 기업 정보, URL, 하드웨어 출시 사실 확인 불가) |

---

## 현재 코드

omon-gateway의 음성 관련 코드는 비동기 음성 메시지 수신/전사, Discord 음성 노트 송신, 그리고 연결되지 않은 단절된 실시간 음성 파이프라인 모듈로 구성되어 있습니다.

1. **단절된 실시간 음성 파이프라인 (미연동)**
   - `src/lib.rs:23`: `pub mod voice;`가 모듈로 등록되어 있습니다.
   - `src/voice/mod.rs:1-37`: `SongbirdAudioEventListener`가 구현되어 Songbird의 `EventContext::VoiceTick` 및 `EventContext::RtpPacket` 이벤트를 수신하여 내부 채널(`mpsc::Sender<AudioFrame>`)로 포워딩하는 구조를 갖추고 있습니다.
   - `src/voice/pipeline.rs:276-308`: STT(`SpeechToText`), LLM(`VoiceLanguageModel`), TTS(`TextToSpeech`) 트레이트를 조합하는 `SpeechPipeline` 구조체가 정의되어 있습니다.
   - `src/voice/pipeline.rs:163-255`: OpenAI Whisper 엔드포인트를 호출하는 `OpenAiSpeechToText`가 구현되어 있습니다.
   - `src/voice/pipeline.rs:311-332`: `AudioFrame`을 비동기 송수신하기 위한 `VoiceAudioPipeline` 채널 래퍼가 구현되어 있습니다.
   - **실제 사용 현황**: `src/main.rs` 및 `src/discord/adapter.rs` 등 실제 런타임 진입점 어디에서도 `SongbirdAudioEventListener`, `SpeechPipeline`, `VoiceAudioPipeline`을 인스턴스화하거나 Discord 음성 채널(Voice Channel)에 조인시키는 코드가 전무합니다(참조 0건).

2. **비동기 음성 첨부 전사 (운영 중)**
   - `src/discord/adapter.rs:1688-1711`: 인바운드 Discord 메시지에 첨부된 파일이 오디오인 경우(`voice_transcription::is_audio_attachment`), `voice_transcription::transcribe_audio(local_path)`를 호출하여 전사 결과를 메시지 내용 앞에 `🎤 [음성 전사]: ...` 형태로 첨부합니다. 실패 시 `⚠️ 음성 전사 실패` 경고를 붙입니다.
   - `src/voice_transcription.rs:3-17`: 확장자(`.ogg`, `.mp3`, `.m4a`, `.wav`, `.opus`, `.aac`, `.flac`) 및 MIME 타입을 검사합니다.
   - `src/voice_transcription.rs:19-35`: 시스템 `ffmpeg`, `whisper-cli`, 로컬 모델(`OMON_WHISPER_MODEL` 또는 `~/.omon/models/ggml-large-v3-turbo.bin`) 존재 여부를 확인합니다.
   - `src/voice_transcription.rs:70-137`: 로컬 프로세스 실행(`ffmpeg`로 16kHz WAV 변환 후 `whisper-cli` 실행)을 타임아웃 제한(`run_bounded`) 하에 수행합니다.

3. **Discord 음성 노트(Voice Message) 아웃바운드 발송 (운영 중)**
   - `src/discord/adapter.rs:2209-2252`: Discord 네이티브 음성 메시지 플래그(`DISCORD_VOICE_MESSAGE_FLAG = 8192`)와 파형 샘플링(`build_voice_metadata`) 로직이 구현되어 있습니다.
   - `src/discord/adapter.rs:2257-2274`: 첨부 파일명이나 Content-Type이 음성 메시지 형식인지 판별하는 `is_voice_audio_file` 함수가 구현되어 있습니다.
   - `src/discord/adapter.rs:2488-2503`: 음성 파일 발송 시 음성 노트 플래그를 붙여 Discord API로 전송하며, 실패 시 일반 파일 발송으로 폴백합니다.

4. **선행 계획서의 불일치 및 정정**
   - 선행 연구 계획서(`docs/research/autonomous-agent-platform-plan.md:230, 239, 416-418`)에서는 "음성 메시지 STT를 하나로 정리(adapter와 attachments 이중 경로 통합)"를 1단계 과제로 기술하였습니다. 그러나 현재 코드를 정밀 확인한 결과 이중 경로는 존재하지 않으며, `src/discord/adapter.rs:1688`에서 `voice_transcription::transcribe_audio`를 호출하는 단일 경로로 이미 깔끔하게 통합되어 있습니다.
   - README 및 문서에서 언급되는 Songbird 기반 음성 대화 기능은 코드상 골격(`src/voice/mod.rs`)만 존재할 뿐 실제 Serenity 클라이언트에 Voice Gateway 핸들러 등록 및 연결 코드가 누락된 완전 단절 상태입니다.

---

## 구현 단계

보류 상태이지만, 향후 오너가 음성 기능 확장을 결정할 경우 단계별 안전한 도입 절차는 다음과 같습니다.

### 단계 1: 비동기 음성 노트 TTS 생성 및 Outbox 연동
- **대상 파일**:
  - `src/voice/tts.rs` (신규: 로컬 macOS `say` 명령 또는 OpenAI TTS 클라이언트)
  - `src/discord/adapter.rs` (기존 `send_voice_file` 발송 로직 연계)
- **DB 마이그레이션**: 없음 (기존 파일 발송 인프라 활용)
- **킬 스위치**: `OMON_VOICE_TTS_ENABLED=0` (기본값 off)
- **기계 검증 시나리오**:
  ```bash
  # CLI 또는 단위 테스트를 통해 텍스트로부터 voice.ogg 파일 생성 검증
  OMON_VOICE_TTS_ENABLED=1 cargo test --test test_voice_transcription tts_generation_test -- --exact
  # 로그 라인 검증: "Generated outbound voice note file path=..."
  ```

### 단계 2: Tailscale 기반 비동기 음성 세션 WS/HTTP 엔드포인트
- **대상 파일**:
  - `src/voice/endpoint.rs` (신규: 로컬/Tailscale 네트워크 전용 HTTP/WS 수신부)
  - `src/dashboard.rs` (라우터 연동)
- **DB 마이그레이션**: `migrations/0051_voice_sessions.sql`
  ```sql
  CREATE TABLE voice_sessions (
      id TEXT PRIMARY KEY,
      device_id TEXT NOT NULL,
      status TEXT NOT NULL CHECK (status IN ('active', 'completed', 'failed')),
      transcript TEXT,
      created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
  );
  ```
- **킬 스위치**: `OMON_VOICE_ENDPOINT_ENABLED=0` (기본값 off)
- **기계 검증 시나리오**:
  ```bash
  # loopback 엔드포인트로 더미 오디오 프레임 전송 후 SQLite 세션 레코드 확인
  curl -s -X POST http://127.0.0.1:19741/api/voice/session -F "audio=@tests/fixtures/sample.wav"
  sqlite3 ~/.omon/state/gateway.db "SELECT status, transcript FROM voice_sessions ORDER BY created_at DESC LIMIT 1;"
  ```

### 단계 3: Discord Songbird 실시간 리스너 및 음성 채널 바인딩
- **대상 파일**:
  - `src/discord/adapter.rs` (Serenity VoiceGateway Intents 및 Songbird 클라이언트 초기화)
  - `src/voice/mod.rs` (`SongbirdAudioEventListener` 실시간 등록)
- **DB 마이그레이션**: 없음
- **킬 스위치**: `OMON_VOICE_SONGBIRD_ENABLED=0` (기본값 off)
- **기계 검증 시나리오**:
  ```bash
  # 게이트웨이 기동 로그에서 Songbird 드라이버 활성화 여부 확인
  grep "Songbird audio voice listener attached" ~/.omon/logs/gateway.log
  ```

---

## 위험·의존성

### 타 테마 의존성
- **`outbox-destination`**: 생성된 음성 노트(TTS 오디오)를 안전하게 디스코드나 외부 대상에 비동기 전달하기 위한 전송 큐 의존성.
- **`usage-ledger`**: OpenAI Whisper/TTS API 또는 로컬 GPU 연산 토큰/비용 집계 원장 의존성.
- **`event-queue`**: 실시간 오디오 패킷 수신 시 multiplexer의 이벤트 유실 방지를 위한 백프레셔 큐 의존성.
- **`approval-effects`**: 음성 대화 중 외부 통화나 명령 실행 시 오너 승인 우회 방지.

### 주요 리스크
1. **오너 방해 및 토큰/비용 낭비**: 실시간 음성 스트리밍 파이프라인 상시 가동 시 마이크 감지 오작동(VAD)으로 인한 불필요한 LLM 호출 폭주.
2. **프로세스 안정성 및 자원 점유**: 음성 디코딩(Opus/PCM) 및 STT(`whisper-cli`) 고부하 작업이 게이트웨이 메인 이벤트 루프를 지연시킬 위험(현재 `run_bounded` 타임아웃 격리로 부분 방어 중).
3. **보안 및 TCC 권한**: macOS 데스크톱 환경에서 마이크 및 음성 하드웨어 접근 시 시스템 TCC(시스템 정책) 팝업 발생 및 권한 거부 리스크.
