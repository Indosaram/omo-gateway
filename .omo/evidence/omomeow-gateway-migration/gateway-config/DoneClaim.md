# DoneClaim: OmOMeow Gateway Configuration & Bot Profile

## Overview
Implemented and verified todo 9 (numbered 10 in plan text): gateway environment configuration template (`.env.omomeow.example`) and OmOMeow bot profile (`bot_profiles/omomeow.json`) porting core behavioral rules from `OMOMEOW-MODE.md` and `AGENTS.md`.

## Created Files
1. `.env.omomeow.example`:
   - `DISCORD_BOT_TOKENS=<OMOMEOW_DISCORD_TOKEN>` (token transferred from agent-messenger credentials specification)
   - `DISCORD_BOT_ID=1465631383862120451`
   - `OMON_DEFAULT_MODEL=inferhub/cb/deepseek-v4.1-flash`
   - `DEFAULT_MODEL=inferhub/cb/deepseek-v4.1-flash`
   - `OMON_AGENT_BACKEND=omo`
   - `OMON_OMO_APPSERVER_URL=ws://127.0.0.1:19742`
   - `OMON_OMO_CRON_APPSERVER_URL=ws://127.0.0.1:19742`
   - `OMON_BUTTON_ACTIONS_FILE=~/.omon/button-actions.json`
   - `APPROVAL_POLICY=auto` & `APPROVAL_MODE=auto`
   - `OMON_PER_AGENT_WORKSPACE=true`
   - Lane timeouts: `OMON_OMO_TURN_TIMEOUT_SECS=1800`, `OMON_OMO_TURN_TOTAL_TIMEOUT_SECS=1800`, `OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS=600`, `OMON_CRON_SCRIPT_TIMEOUT_SECS=1800`, `APPROVAL_TIMEOUT_SECS=900`
   - Dashboard port: `DASHBOARD_PORT=9119`

2. `bot_profiles/omomeow.json`:
   - `bot_id`: `"1465631383862120451"`
   - `name`: `"OmOMeow"`
   - `model`: `"inferhub/cb/deepseek-v4.1-flash"`
   - `enabled_toolsets`: `["terminal", "file", "mcp", "cron", "skills", "web", "message_context"]`
   - `system_prompt`: Ported core behavioral rules from `OMOMEOW-MODE.md` and `AGENTS.md`:
     - Friendly and clear tone (친절하고 명확한 어조, 친구처럼 다정하되 작업 보고는 철저하게 사실과 증거 기반)
     - Immediate reaction & transparent reporting (👀 즉각 반응, 1분 이내 답변/안내, 15분 주기/마일스톤 진행 보고)
     - Thread-based work management (스레드 집중 수행, 🔄 진행 중, ⏸ 대기 중, ✅ 완료 상태 관리)
     - Tool delegation rules (60~120초 이상 장시간 작업 또는 깊은 자율 구현 시 `omomeow-delegate` CLI 백그라운드 위임, 간단한 작업은 직접 수행)
     - Security & safety (비밀/토큰/개인정보 보호, 파괴적 명령 안전 처리)
   - `custom_settings`:
     - `display_name_ko`: `"오모냥"`
     - `mode`: `"omomeow-mode"`
     - `per_agent_workspace`: `true`
     - `delegate_command`: `"bin/omomeow-delegate"`
     - `heartbeat_interval_secs`: `900`

## Verification
- Validated JSON syntax of `bot_profiles/omomeow.json`: parsed cleanly, verified required fields (`bot_id`, `name`, `model`, `enabled_toolsets`, `system_prompt`, `custom_settings`).
- Validated `.env.omomeow.example` required keys presence:
  - `DISCORD_BOT_TOKENS`: present
  - `DISCORD_BOT_ID`: present
  - `OMON_DEFAULT_MODEL`: present
  - `OMON_BUTTON_ACTIONS_FILE`: present
  - `APPROVAL_POLICY`: present
  - `OMON_PER_AGENT_WORKSPACE`: present
- `cargo check`: exit code 0.
- `cargo test --test test_profile_routing`: exit code 0 (7 passed, 0 failed).
- `cargo test bot_profile`: exit code 0 (`bot_profile_delete_stays_deleted` ok).
