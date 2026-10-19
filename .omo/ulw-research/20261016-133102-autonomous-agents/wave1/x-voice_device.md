<!-- task=st_01a1450b -->
st_01a1450b [completed] model mahoquot/claude-sonnet-5-5
Ask: research voice and device-native agents (Hark, Rabbit, Humane, Limitless, Sesame, OpenAI device, Siri/App Intents, macOS accessibility agents) - wanted: a sourced report. For you: **Hark could not be verified, and Sesame has no 2026 evidence.** `web_search` was not available in this session, so I used HN Algolia (search and item endpoints) and a few direct page fetches. I read no company-primary pages for Rabbit, Humane or Limitless. Now: done. Next: none.

## Identity

| Entity | What I could establish | Confidence |
|---|---|---|
| **Hark (hardware plan)** | Five Algolia queries returned nothing relevant: "Hark hardware", "hark.ai", "Hark Gerstner", and title-only "Hark" over 2025-10 to now. I found no company, URL, or launch date. | **Unverified.** I won't guess an identity. |
| **Rabbit (R1)** | Rabbit Inc.'s R1 handheld. 2026 coverage is community hacks, not company news. | Medium (names only). |
| **Humane (AI Pin)** | Discontinued. The Pin's software lineage reportedly became HP's enterprise Copilot. Hobbyists run the hardware as a standalone Android device. | Medium. |
| **Limitless pendant** | Limitless (formerly Rewind) was acquired by Meta in Dec 2025. Per a user report, the Pendant is no longer sold and existing devices get one year of support. | Medium. The acquisition is reported by the FT. The support terms come from a user post. |
| **Sesame** | The only hit is a Feb 2025 Verge piece on the voice assistant and glasses (Brendan Iribe). It is not 2026 evidence. Other "Sesame" hits are unrelated, such as a password manager. | Low. |
| **OpenAI device** | Bloomberg reports the first device is a screenless, movable "companion" speaker. Apple sued OpenAI over trade secrets in July 2026. | Medium. Reporting is secondhand and no launch date is confirmed. |
| **Apple Siri / App Intents** | The iOS 27 App Intents material comes from a third-party developer post, not Apple's docs. Apple also announced the $299 "HomeView" smart display (Oct 13, 2026). | Medium. |
| **macOS agent controls** | Apple is tightening Full Disk Access because of AI agents (Oct 2, 2026). I did not read the article body. | Medium. |

## Findings

- OpenAI's first hardware device is reportedly a screenless speaker with "mechanical elements that can move on their own", designed as a "companion". TechCrunch is relaying Bloomberg. [https://techcrunch.com/2026/07/14/openais-first-hardware-device-is-reportedly-a-screenless-speaker-that-can-move/]
- A newsletter described the device as battery-powered. HN commenters mocked the form factor as a Tamagotchi. Treat the details as unconfirmed. [https://www.machinesociety.ai/p/open-ais-first-hardware-device-will]
- Apple sued OpenAI for trade-secret theft around July 10, 2026, and OpenAI called the suit "rotten to its core" in a dismissal request. Coverage frames the case as part of the OpenAI hardware fight. [https://www.latimes.com/business/story/2026-07-10/apple-accuses-openai-of-stealing-trade-secrets-in-blockbuster-silicon-valley-lawsuit] [https://www.theverge.com/tech/976042/openai-apple-trade-secrets-lawsuit-dismissal-request]
- Meta bought Limitless in Dec 2025 (FT headline: "to expand hardware push"). [https://www.ft.com/content/a1a7adab-506e-4623-8f7a-0b7c94c8d6b4]
- A Limitless customer says Meta will stop selling the Pendant and support existing units for about a year. This is one user's claim, not Meta's statement. [HN thread: https://hn.algolia.com/api/v1/items/46169414]
- The Limitless pendant was reportedly banned in the EU, leaving users 30 days to export their data. The author's fix was a local Claude Code workflow. It turned transcripts into a knowledge base and a portable `CLAUDE.md`, because LLM context windows can't hold millions of tokens. [https://thoughts.jock.pl/p/limitless-pendant-eu-ban-what-i-did-with-voice-data]
- Humane's AI Pin became "HP's Copilot", an enterprise laptop chatbot. [https://gizmodo.com/this-is-what-has-become-of-the-humane-ai-pin-an-enterprise-laptop-chatbot-2000737668]
- Hackers turned the discontinued Humane Pin into a standalone Android-powered gadget. [https://liliputing.com/humane-ai-pin-hacks-turn-the-discontinued-gadget-into-a-standalone-android-powered-gadget/]
- Rabbit R1 is now a hacker platform. Pomi is a standalone voice agent on a rooted R1 (Android 13) built on Hermes (Nous Research). The agent gateway starts at boot, keeps state locally, and is reachable only over Tailscale. It uses an offline "Hey Pomi" wake word. [https://github.com/sidmohan0/pomi]
- A second R1 project uses a LiveKit skill to run the R1 with "whatever model you want". [https://github.com/ShayneP/rabbit-r1-livekit-skill]
- Apple's own pendant was rumored in Feb 2026, per Gizmodo's title only (speculation). [https://gizmodo.com/apples-ai-pendant-sounds-like-a-watered-down-humane-ai-pin-2000723004]
- Apple's new Siri is reportedly backed by a distilled Gemini model that must fit on the iPhone (Ars, via HN title). [https://arstechnica.com/ai/2026/05/apple-reportedly-trying-to-distill-googles-multi-trillion-parameter-gemini-ai-to-run-on-iphone/]
- Apple announced HomeView, a $299 smart display "to push Siri AI into homes", which won't launch in the EU. [https://www.cnn.com/2026/10/13/tech/apple-homeview-siri-ai] [https://www.macrumors.com/2026/10/13/apple-homeview-wont-be-available-in-eu/]
- HN mood on Siri is poor. A Reddit post titled "Siri is so bad at everything" has 126 points and about 100 comments. [https://www.reddit.com/r/apple/comments/1x6gz83/siri_is_so_bad_at_everything/]
- The iOS 27 App Intents surface, as shown in a dev walkthrough:
  - `@AppIntent(schema: .system.open)` lets Siri treat "open this" as the system open action.
  - `IndexedEntityQuery` lets Spotlight ask the app to reindex.
  - `AppShortcutsProvider` phrases must interpolate `\(.applicationName)`.
  - `updateAppShortcutParameters()` refreshes the phrase cache.
  
  [https://emredegirmenci.substack.com/p/ios-27-app-intents-journey]
- Apple says it is tightening macOS Full Disk Access controls because of AI-agent risks (Oct 2, 2026). HN reaction was split:
  - Complaints that TCC prompts are already unmanageable.
  - Anecdotes of agents causing damage.
  - A comment that Apple would restrict third-party LLMs but keep its own access level (speculation).
  
  [https://techcrunch.com/2026/10/02/apple-says-its-tightening-macos-full-disk-access-controls-due-to-new-risks-from-ai-agents/] [https://hn.algolia.com/api/v1/items/49937239]
- macOS 27.x is already shipping, so the FDA changes may apply to this Mac soon. [https://hn.algolia.com/api/v1/search_by_date?query=Siri&tags=story] (item 50118035)
- The macOS agent ecosystem is moving toward sandboxing and supervision. Examples are SideKernel (MicroVM sandbox for coding agents), Augur (sandboxed macOS VMs with Xcode), Hall Monitor (multi-machine agent status on the Notch) and AgentiLoop (a GUI agent with an `/auto` loop and a "Stop All" kill switch). [https://github.com/minoansecurity/sidekernel] [https://github.com/h1d3mun3/augur] [https://github.com/hiteshbandhu/hallmonitor] [https://agentiloop.ai/]
- On-device inference with cloud fallback is productized. Cactus v2 runs a Gemma 4 E2B-class model on an M5 Max and escalates hard turns to a configurable cloud endpoint. [https://github.com/cactus-compute/cactus]

## Implications for a self-hosted agent gateway

1. **Treat voice and devices as just more lanes.**
   - R1 (Pomi/Hermes), LiveKit and Tailscale show that a device can be a thin client to a home agent.
   - Add a transport-agnostic `voice` lane: an HTTP/WebSocket endpoint over Tailscale that takes STT text and returns TTS-ready text, with owner identity pinned to the device key.
2. **Capture ingestion with a portable export format.**
   - The Limitless EU ban and Meta acquisition show vendor memory is fragile.
   - Make the inbox ingestor accept pendant, Omi and Plaud transcript dumps. Chunk and summarize them on cron into a markdown knowledge base (the `CLAUDE.md` pattern) that omo agents can load.
3. **Siri bridge via App Intents.**
   - Ship a tiny companion app or Shortcut exposing "Ask gateway" and "Run lane X" intents that post to the daemon.
   - Use schema-based intents and `\(.applicationName)` phrases (not just freeform text). Siri is reported to be weak, so keep Shortcuts and the iMessage lane as primary.
4. **Plan for Full Disk Access tightening.**
   - Run the daemon under a dedicated launchd user or helper with the narrowest TCC grants.
   - Prefer app-specific automation (AppleScript, Shortcuts, App Intents) over a blanket FDA grant for every spawned coding agent.
   - Add a `doctor` command that probes the FDA, Accessibility and Automation grants and tells the owner what is missing.
5. **Gate GUI control behind supervision.**
   - Gate any accessibility or GUI-automation lane behind a per-task owner approval message on Discord or iMessage.
   - Add a global kill switch (like AgentiLoop's "Stop All"), an audit log, and a cap on actions per cycle.
6. **Sandbox spawned coding agents.** Offer an optional VM or sandbox per omo run (SideKernel/Augur style) so a rogue agent cannot reach the whole disk.
7. **Local-first fallback.** Add a model-router option for short voice turns to a local Apple-silicon model with cloud escalation (Cactus-style confidence routing). That cuts voice latency and cost, and keeps private audio on the box.
8. **Ambient status.** Expose lane and agent status as a read-only feed (menu bar, Notch or Discord status) that a future companion device can consume.

## CLAIMS
- CLAIM: OpenAI's first device is reportedly a screenless, movable companion speaker (per Bloomberg, unconfirmed) — RISK: normal — SOURCES: techcrunch.com, machinesociety.ai — PRIMARY: https://techcrunch.com/2026/07/14/openais-first-hardware-device-is-reportedly-a-screenless-speaker-that-can-move/
- CLAIM: Apple sued OpenAI for trade-secret theft in July 2026 — RISK: normal — SOURCES: latimes.com, cbc.ca, theverge.com — PRIMARY: https://www.latimes.com/business/story/2026-07-10/apple-accuses-openai-of-stealing-trade-secrets-in-blockbuster-silicon-valley-lawsuit
- CLAIM: Meta acquired Limitless in Dec 2025 and the Pendant is being retired with about a year of support — RISK: high (the retirement terms are from a user post, not an official source) — SOURCES: ft.com, hn.algolia.com — PRIMARY: none (the Meta announcement was not read)
- CLAIM: The Limitless pendant was banned in the EU, with a 30-day export window — RISK: high — SOURCES: thoughts.jock.pl — PRIMARY: none (a single blog post)
- CLAIM: Humane's Pin became HP's Copilot, and the hardware has been hacked into a standalone Android device — RISK: normal — SOURCES: gizmodo.com, liliputing.com — PRIMARY: none
- CLAIM: Rabbit R1 can run a standalone Hermes-based voice agent on a rooted Android 13 device — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/sidmohan0/pomi
- CLAIM: Apple announced HomeView, a $299 smart display for Siri AI, and it is not available in the EU — RISK: normal — SOURCES: cnn.com, macrumors.com — PRIMARY: none
- CLAIM: iOS 27 adds a `system.open` App Intent schema and `IndexedEntityQuery` — RISK: normal (developer blog, and I did not open the Apple docs pages it links) — SOURCES: emredegirmenci.substack.com — PRIMARY: https://developer.apple.com/documentation/appintents/appschema/systemintent/open
- CLAIM: Apple is tightening macOS Full Disk Access because of AI agents — RISK: high for the gateway (the headline is confirmed but I did not read the article or Apple's statement) — SOURCES: techcrunch.com, 9to5mac.com, hn.algolia.com — PRIMARY: https://techcrunch.com/2026/10/02/apple-says-its-tightening-macos-full-disk-access-controls-due-to-new-risks-from-ai-agents/
- CLAIM: Siri's new backend is a distilled Gemini model (Ars via HN title only) — RISK: normal — SOURCES: arstechnica.com — PRIMARY: none
- CLAIM: Hark has an identifiable hardware plan — RISK: high — SOURCES: none found — PRIMARY: none (unverified)

## EXPAND
- LEAD: Hark identity (company, founders, hardware plan) — it is the headline scope item and I found nothing — ANGLE: Ask the requester for the canonical URL or a source article. Otherwise try x_search or web_search for "Hark" with "AI device" or "voice computer" after 2026-06 and check YC/Crunchbase.
- LEAD: Apple's exact FDA/TCC policy change — it directly affects the daemon's entitlements — ANGLE: Read the TechCrunch article body, Apple's developer news and the macOS 27 release notes. Test a spawned agent under macOS 27.
- LEAD: The Apple docs pages for `system.open`, `TargetContentProvidingIntent` and `IndexedEntityQuery` — I only saw them second-hand — ANGLE: Fetch them from developer.apple.com.
- LEAD: Sesame's 2026 status (voice model, glasses) — only a Feb 2025 article was found — ANGLE: sesame.com and its blog, plus GitHub for open-sourced CSM models.
- LEAD: Official Rabbit and Meta/Limitless statements — only secondhand coverage was read — ANGLE: rabbit.tech and limitless.ai help-center and blog pages.