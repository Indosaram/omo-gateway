# Evidence Coverage Audit Report: U01-U85 Parity Units (2026-09-12)

## VERDICT

PARTIAL

- **Total Parity Units**: 85 units (U01-U71 base units, U72-U85 upstream units)
- **EVIDENCED**: 82 units (96.5%) have concrete proof artifacts in `.omo/evidence/hermes-parity-20260905/` (1,000 total unit-prefixed files).
- **EVIDENCE-GAP**: 3 units (3.5%) have zero artifact files in the parity corpus:
  - **U36**: Live durable outbound obligations (`CR.C01`, `S.F13`)
  - **U37**: Scheduler-owned agent and gate results (`CR.C02`, `CR.C05`)
  - **U73**: Recover owned transport failures while running (`UP.IO-02`)

---

## EVIDENCE

### 1. Parity Corpus Manifest Sources

- **Base Units Manifest (U01-U71)**:
  - `.omo/evidence/hermes-parity-20260905/implementation-units.json:2-6`
  ```json
    "units": [
      {
        "id": "U01",
        "title": "Detector executable semantics",
        "findings": [
  ```

- **Upstream Units Manifest (U72-U85)**:
  - `.omo/evidence/hermes-parity-20260905/upstream-implementation-units.json:2-6`
  ```json
    "units": [
      {
        "id": "U72",
        "title": "Bound final response message count",
        "findings": [
  ```

### 2. Evidence-Gap Units Verification in Matrix and Current Code Tree

- **Unit U36 (Live durable outbound obligations)**:
  - Matrix registration: `.omo/evidence/hermes-parity-20260905/matrix.md:337`
    ```markdown
    | U36 | Live durable outbound obligations | CR.C01, S.F13 |
    ```
  - Current tree production symbol: `src/ledger/service.rs:374-376`
    ```rust
        /// Records an outbound delivery obligation as 'pending'.
        pub async fn record_obligation(
            &self,
    ```

- **Unit U37 (Scheduler-owned agent and gate results)**:
  - Matrix registration: `.omo/evidence/hermes-parity-20260905/matrix.md:338`
    ```markdown
    | U37 | Scheduler-owned agent and gate results | CR.C02, CR.C05 |
    ```
  - Current tree production symbol: `src/cron/executor.rs:19-21`
    ```rust
    pub struct AgentCronExecutor {
        pub backend: Arc<dyn AgentBackend>,
        pub workspace_root: PathBuf,
    ```

- **Unit U73 (UP.IO-02 Recover owned transport failures while running)**:
  - Matrix registration: `.omo/evidence/hermes-parity-20260905/matrix.md:391`
    ```markdown
    | U73 | UP.IO-02 | FIX | `src/ledger/service.rs:291-314`; `src/main.rs:301-365,1297`; `src/discord/adapter.rs:1040-1063` | `plugins/platforms/discord/adapter.py:217-249`; `gateway/delivery_ledger.py:274-320`; `gateway/platforms/base.py:3644-3665` | Extend U36 with bot/owner-instance runtime failed-send claims and reconnect-before-failure-record catch-up, without agent replay. |
    ```
  - Current tree production symbol: `src/discord/adapter.rs:2458-2460`
    ```rust
        /// Replays owned failed obligations for `bot_id` after a transport reconnection.
        pub async fn replay_failed_transport_obligations(
            &self,
    ```

### 3. Exit File Provenance Samples (Raw Cat Outputs)

For units having `-red` and `-green` exit files, exit code `101` indicates RED assertion failure / panic captured, and `0` indicates GREEN regression passed.

- **Sample 1: Unit U01 (Detector executable semantics)**:
  - Command: `cat .omo/evidence/hermes-parity-20260905/U01-red.exit`
    ```
    101
    ```
  - Command: `cat .omo/evidence/hermes-parity-20260905/U01-green.exit`
    ```
    0
    ```

- **Sample 2: Unit U14 (Absolute interactive deadline)**:
  - Command: `cat .omo/evidence/hermes-parity-20260905/U14-red.exit`
    ```
    101
    ```
  - Command: `cat .omo/evidence/hermes-parity-20260905/U14-green.exit`
    ```
    0
    ```

- **Sample 3: Unit U13 (Interrupt actual remote work)**:
  - Command: `cat .omo/evidence/hermes-parity-20260905/U13-red-stop_interrupts_remote_turn.exit`
    ```
    101
    ```
  - Command: `cat .omo/evidence/hermes-parity-20260905/U13-green-stop_interrupts_remote_turn.exit`
    ```
    0
    ```

---

## EVIDENCE COVERAGE TABLE (U01 - U85)

| unit | title | category | artifact_count | red_captured(y/n) | green_captured(y/n) | verdict |
| --- | --- | --- | --- | --- | --- | --- |
| U01 | Detector executable semantics | quick | 24 | y | y | EVIDENCED |
| U02 | Namespaced multi-tool approval scopes | quick | 10 | y | y | EVIDENCED |
| U03 | Cancellation-safe approval lifetime | quick | 12 | y | y | EVIDENCED |
| U04 | Redact approval display copies | quick | 12 | n | y | EVIDENCED |
| U05 | Faithful Tirith outcomes | quick | 45 | y | y | EVIDENCED |
| U06 | Confine staged skill writes | quick | 12 | y | y | EVIDENCED |
| U07 | Atomic exact-destination replay | quick | 4 | n | y | EVIDENCED |
| U08 | Session-scoped full pending review | quick | 5 | y | y | EVIDENCED |
| U09 | Fail-closed shared admission | quick | 5 | y | y | EVIDENCED |
| U10 | Pairing notification and lockout state | quick | 37 | y | y | EVIDENCED |
| U11 | Coherent yolo lifecycle | quick | 13 | y | y | EVIDENCED |
| U12 | Correlate remote turn frames | quick | 55 | y | y | EVIDENCED |
| U13 | Interrupt actual remote work | quick | 25 | y | y | EVIDENCED |
| U14 | Absolute interactive deadline | quick | 15 | y | y | EVIDENCED |
| U15 | No accepted-turn resubmission | quick | 19 | y | y | EVIDENCED |
| U16 | Durable remote conversation binding | quick | 24 | y | y | EVIDENCED |
| U17 | Canonical recovery bot identity | quick | 22 | y | y | EVIDENCED |
| U18 | Fail before undurable side effects | quick | 28 | y | y | EVIDENCED |
| U19 | Durable accepted queue and replay | deep | 20 | y | y | EVIDENCED |
| U20 | Permanent per-bot conversation lanes | quick | 12 | y | y | EVIDENCED |
| U21 | Parent-aware profile routing | quick | 13 | y | y | EVIDENCED |
| U22 | Durable split-message constituent dedup | quick | 12 | y | y | EVIDENCED |
| U23 | Recover source-time historical input | quick | 18 | y | y | EVIDENCED |
| U24 | Hydrate referenced attachments | quick | 8 | y | y | EVIDENCED |
| U25 | Immediate typing and terminal reactions | quick | 7 | n | y | EVIDENCED |
| U26 | Authoritative serialized slash controls | deep | 7 | y | y | EVIDENCED |
| U27 | Unicode-safe chunked slash replies | quick | 8 | y | y | EVIDENCED |
| U28 | Final reasoning and silence filter | quick | 8 | y | y | EVIDENCED |
| U29 | Bot-scoped recoverable dead targets | quick | 8 | y | y | EVIDENCED |
| U30 | Reference triggering message on final | quick | 8 | y | y | EVIDENCED |
| U31 | Safe final MEDIA uploads | quick | 8 | y | y | EVIDENCED |
| U32 | Valid native voice-note transport | quick | 8 | y | y | EVIDENCED |
| U33 | Configured encoded-audio STT | quick | 8 | y | y | EVIDENCED |
| U34 | Lossless fenced and wide PNG tables | quick | 14 | y | y | EVIDENCED |
| U35 | Forum and batched PNG delivery | quick | 8 | y | y | EVIDENCED |
| U36 | Live durable outbound obligations | deep | 0 | n | n | EVIDENCE-GAP |
| U37 | Scheduler-owned agent and gate results | deep | 0 | n | n | EVIDENCE-GAP |
| U38 | Atomic finite occurrence budget | quick | 8 | y | y | EVIDENCED |
| U39 | Profile-scoped persistent cron results | quick | 8 | y | y | EVIDENCED |
| U40 | Skill-only jobs and slash bundles | quick | 8 | y | y | EVIDENCED |
| U41 | Shared lifecycle guard on resolved scripts | quick | 8 | y | y | EVIDENCED |
| U42 | Two-tier assembled cron scanning | quick | 8 | y | y | EVIDENCED |
| U43 | Canonical Discord home and fanout targets | quick | 8 | y | y | EVIDENCED |
| U44 | Aggregate delivery receipts before ACK | quick | 8 | y | y | EVIDENCED |
| U45 | Exact reminder and mirror identity | quick | 6 | n | y | EVIDENCED |
| U46 | Validated timezone-aware schedules | quick | 9 | y | y | EVIDENCED |
| U47 | Bad-job isolation and explicit rearming | quick | 7 | y | y | EVIDENCED |
| U48 | Truthful complete CronTool mutations | quick | 8 | y | y | EVIDENCED |
| U49 | Cron runs and status tool surface | quick | 8 | y | y | EVIDENCED |
| U50 | Private atomic unique migration writes | quick | 27 | y | y | EVIDENCED |
| U51 | Verified safe launchd retirement | quick | 40 | y | y | EVIDENCED |
| U52 | Durable cutover job ownership | quick | 2 | n | n | EVIDENCED |
| U53 | Quiesced receipt-verified all-store cutover | quick | 3 | n | n | EVIDENCED |
| U54 | Dry-run shares full import validation | quick | 3 | n | n | EVIDENCED |
| U55 | Dotenv value fidelity | quick | 18 | y | y | EVIDENCED |
| U56 | Effective config import and policy reporting | quick | 7 | y | y | EVIDENCED |
| U57 | Per-bot named profile policy | deep | 7 | y | y | EVIDENCED |
| U58 | Real per-agent policy and provider bridge | deep | 7 | y | y | EVIDENCED |
| U59 | Live scoped daemon consent bridge | deep | 7 | y | y | EVIDENCED |
| U60 | Unified effective default model | quick | 7 | y | y | EVIDENCED |
| U61 | Wire final footer and truthful config docs | quick | 7 | y | y | EVIDENCED |
| U62 | Bound daemon restarts and readiness | quick | 46 | y | y | EVIDENCED |
| U63 | Cross-process drain epoch | quick | 7 | y | y | EVIDENCED |
| U64 | Reversible admission and bounded shutdown | deep | 7 | y | y | EVIDENCED |
| U65 | Dashboard Host Origin and auth boundary | quick | 14 | y | y | EVIDENCED |
| U66 | One multiplexed daemon for all work | deep | 7 | y | y | EVIDENCED |
| U67 | Attach dashboard to actual live runtime | deep | 7 | y | y | EVIDENCED |
| U68 | Live backend and connection readiness | quick | 7 | y | y | EVIDENCED |
| U69 | Truthful bot profile CRUD | quick | 14 | y | y | EVIDENCED |
| U70 | Cross-process runtime ownership lock | quick | 7 | y | y | EVIDENCED |
| U71 | Deterministic causal test fixtures | quick | 5 | n | y | EVIDENCED |
| U72 | Bound final response message count | unspecified-low | 8 | y | y | EVIDENCED |
| U73 | Recover owned transport failures while running | unspecified-low | 0 | n | n | EVIDENCE-GAP |
| U74 | Break slow repeated restart cycles | unspecified-low | 5 | y | y | EVIDENCED |
| U75 | Expire stale drain requests by age | unspecified-low | 9 | y | y | EVIDENCED |
| U76 | Retire overdue unclaimed one shot jobs | unspecified-low | 12 | y | y | EVIDENCED |
| U77 | Terminate owned cron script descendant trees | unspecified-low | 7 | y | y | EVIDENCED |
| U78 | Persist change monitor gates per job | unspecified-low | 7 | y | y | EVIDENCED |
| U79 | Route cron failures to configured lane | unspecified-low | 7 | y | y | EVIDENCED |
| U80 | Suppress acknowledged repeated cron failure incidents | unspecified-low | 7 | y | y | EVIDENCED |
| U81 | Persist profile scoped cron cursor notes | unspecified-low | 7 | y | y | EVIDENCED |
| U82 | Preflight configured cron delivery transports | unspecified-low | 7 | y | y | EVIDENCED |
| U83 | Recheck current authorization before startup replay | unspecified-low | 7 | y | y | EVIDENCED |
| U84 | Classify disk pressure using absolute headroom | unspecified-low | 18 | y | y | EVIDENCED |
| U85 | Confirm destructive scoped Discord session commands | unspecified-low | 7 | y | y | EVIDENCED |

---

## EVIDENCE ANALYSIS & ARTIFACT PAIR STATUS

### 1. Verification of Red / Green Pair Status

Across the 85 units:
- **Units with Dual `.exit` Pairs (61 units)**:
  `U01`, `U05`, `U10`, `U11`, `U12`, `U13`, `U14`, `U15`, `U16`, `U17`, `U18`, `U19`, `U20`, `U21`, `U22`, `U23`, `U24`, `U26`, `U27`, `U28`, `U29`, `U30`, `U31`, `U32`, `U33`, `U35`, `U38`, `U39`, `U40`, `U41`, `U42`, `U43`, `U44`, `U48`, `U49`, `U50`, `U51`, `U56`, `U57`, `U58`, `U59`, `U60`, `U61`, `U62`, `U64`, `U65`, `U66`, `U67`, `U68`, `U69`, `U70`, `U72`, `U77`, `U78`, `U79`, `U80`, `U81`, `U82`, `U83`, `U84`, `U85`.
  - In all 61 units, `cat` on `-red*.exit` yields `101` (assertion failure) and `cat` on `-green*.exit` yields `0` (pass).
- **Units with Dual Log / Text Capture Pairs (12 units)**:
  `U02`, `U06`, `U08`, `U09`, `U34`, `U46`, `U47`, `U63`, `U74`, `U75`, `U76` (and `U03`/`U55` with red `.exit` + green `.log`).
  - These units captured behavioral RED test failure (exit 101) and GREEN passing test (exit 0) in `.txt` or `.log` files rather than standalone `.exit` files.
- **Units with Green Verified, Missing Standalone Red Run File (5 units)**:
  `U04`, `U07`, `U25`, `U45`, `U71`.
  - Production fixes and passing regressions (`-green.exit` / `-green.log`) are verified, but standalone pre-patch RED test run files are absent from the directory (though pre-patch failures were recorded in session monitors / evidence descriptions).
- **Units with Evidence / Patches but No Test Run Files (3 units)**:
  `U52`, `U53`, `U54`.
  - Contain `evidence.md`, `commands.json`, and `proof-diff.patch`, but lack direct pre/post cargo test logs. Verified separately in `w1-ambiguous.md` as CLOSED-BY-EVIDENCE.
- **Evidence Gap Units (3 units)**:
  `U36`, `U37`, `U73`.
  - Exactly 0 files in `.omo/evidence/hermes-parity-20260905/`.

### 2. Breakdown by Category

| Category | Total Units | Evidenced | Evidence Gap | Units with Gap |
| --- | --- | --- | --- | --- |
| quick | 61 | 61 | 0 | None |
| deep | 10 | 8 | 2 | `U36`, `U37` |
| unspecified-low | 14 | 13 | 1 | `U73` |
| **Total** | **85** | **82** | **3** | `U36`, `U37`, `U73` |

---

## RESIDUAL

The remaining evidence gap consists of exactly three units having 0 proof artifacts in the parity corpus:
1. **U36 (CR.C01, S.F13 - Live durable outbound obligations)**:
   - Residual state: `record_obligation` existed with test-only callers; failed Discord dispatch previously ignored in `omo_backend.rs`; boot sweep lacked process start-stamp guard. Investigated in parallel by task `w36`.
2. **U37 (CR.C02, CR.C05 - Scheduler-owned agent and gate results)**:
   - Residual state: Agent cron results previously bypassed scheduler save/filter/deliver pipeline; literal `[SILENT]`/`NO_REPLY` could leak to Discord. Investigated in parallel by task `w37`.
3. **U73 (UP.IO-02 - Recover owned transport failures while running)**:
   - Residual state: Runtime failed-send claims and transport reconnection catch-up without agent resubmission. Method `replay_failed_transport_obligations` exists in `src/discord/adapter.rs:2458` but remains unwired to Discord `FullEvent::Resume`/`Ready` lifecycles (documented in `w1-u73.md`).

---

## CLAIM

REMAINING: 3 evidence-gap units (U36, U37, U73) out of 85 total units; 82 units evidenced across 1,000 artifacts with 61 dual red/green exit pairs verified.
