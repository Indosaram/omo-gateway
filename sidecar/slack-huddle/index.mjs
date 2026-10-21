#!/usr/bin/env node
// Slack huddle sidecar for omo-gateway.
//
// Drives the normal Slack web UI in a dedicated Chrome profile over the local
// DevTools protocol. The operator signs that profile in. This process does not
// read the profile's cookie jar, does not call Slack web methods, and does not
// open meeting-media sockets of its own.
//
// The join, leave, caption, and device controls follow the same web-client
// behavior as the OpenClaw slack-huddles plugin (MIT, OpenClaw Foundation,
// https://github.com/openclaw/openclaw/tree/main/extensions/slack-huddles).
// This file is an independent implementation, not a copy of that plugin.
//
// Protocol: one JSON object per line on stdin (commands) and stdout (replies
// and events). Diagnostics go to stderr.

import { spawn, execFileSync } from "node:child_process";
import { createInterface } from "node:readline";
import { mkdir, readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { setTimeout as delay } from "node:timers/promises";

const JOIN_TIMEOUT_MS = Number(process.env.OMO_HUDDLE_JOIN_TIMEOUT_MS || 30000);
const MIC_LABEL = process.env.OMO_HUDDLE_MIC_LABEL || "";

let chrome;
let launchedChrome = false;
let cdp;
let sessionId;
let inCall = false;
let currentUrl = "";
let writing = Promise.resolve();

function emit(value) {
  const line = `${JSON.stringify(value)}\n`;
  writing = writing
    .then(
      () =>
        new Promise((resolve) => {
          process.stdout.write(line, () => resolve());
        }),
    )
    .catch(() => {});
}

function log(message) {
  process.stderr.write(`${message}\n`);
}

function which(bin) {
  try {
    return execFileSync("which", [bin], { encoding: "utf8" }).trim();
  } catch {
    return "";
  }
}

function chromeBin() {
  if (process.env.OMO_HUDDLE_CHROME) return process.env.OMO_HUDDLE_CHROME;
  for (const candidate of [
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
  ]) {
    const found = which(candidate);
    if (found) return found;
  }
  return "google-chrome";
}

function normalizeTarget(input) {
  const value = String(input || "").trim();
  const channelOnly = value.replace(/^(?:slack:)?channel:/i, "");
  const id = /^[CGD][A-Z0-9]{8,}$/;
  const teamId = /^[TE][A-Z0-9]{8,}$/;
  if (id.test(channelOnly)) return `https://app.slack.com/huddle/${channelOnly}`;
  const qualified = value.match(/^(?:slack:)?team:([^:]+):channel:([^:]+)$/i);
  if (qualified && teamId.test(qualified[1]) && id.test(qualified[2])) {
    return `https://app.slack.com/huddle/${qualified[1]}/${qualified[2]}`;
  }
  let url;
  try {
    url = new URL(value);
  } catch {
    return "";
  }
  if (url.protocol !== "https:" || url.port || url.username || url.password) return "";
  if (!/^[a-z0-9-]+\.slack\.com$/i.test(url.hostname)) return "";
  const match = url.pathname.match(/^\/huddle\/(?:([TE][A-Z0-9]{8,})\/)?([CGD][A-Z0-9]{8,})\/?$/);
  if (!match) return "";
  return `https://app.slack.com/huddle/${match[1] ? `${match[1]}/` : ""}${match[2]}`;
}

async function launchChrome() {
  const port = process.env.OMO_HUDDLE_CDP_PORT;
  if (port) {
    const version = await fetch(`http://127.0.0.1:${port}/json/version`).then((response) =>
      response.json(),
    );
    return version.webSocketDebuggerUrl;
  }
  const profile = process.env.OMO_HUDDLE_PROFILE;
  if (!profile) throw new Error("OMO_HUDDLE_PROFILE is not set");
  await mkdir(profile, { recursive: true });
  const bin = chromeBin();
  chrome = spawn(
    bin,
    [
      `--user-data-dir=${profile}`,
      "--remote-debugging-port=0",
      "--remote-allow-origins=*",
      "--no-first-run",
      "--no-default-browser-check",
      "--autoplay-policy=no-user-gesture-required",
      "--disable-features=Translate",
      "--window-size=1280,800",
      "about:blank",
    ],
    { stdio: "ignore" },
  );
  launchedChrome = true;
  chrome.on("exit", (code) => log(`chrome exited ${code}`));
  const marker = `${profile}/DevToolsActivePort`;
  const started = Date.now();
  while (Date.now() - started < 15000) {
    if (existsSync(marker)) {
      const text = await readFile(marker, "utf8");
      const [debugPort, path] = text.split("\n");
      if (debugPort && path) return `ws://127.0.0.1:${debugPort.trim()}${path.trim()}`;
    }
    await delay(200);
  }
  throw new Error(`Chrome did not open a DevTools port. Is ${bin} installed and is a display available?`);
}

function connectCdp(url) {
  const ws = new WebSocket(url);
  const pending = new Map();
  let next = 0;
  const ready = new Promise((resolve, reject) => {
    ws.addEventListener("open", () => resolve());
    ws.addEventListener("error", () => reject(new Error("DevTools socket failed")));
  });
  ws.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const waiter = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) waiter.reject(new Error(message.error.message || "cdp error"));
      else waiter.resolve(message.result || {});
      return;
    }
    if (message.method === "Runtime.bindingCalled") {
      const payload = message.params?.payload;
      if (typeof payload === "string") {
        try {
          const body = JSON.parse(payload);
          if (body.kind === "audio" || body.kind === "silence") emit({ event: body.kind, ...body });
        } catch {
          // Ignore a malformed page sample. The next buffer will retry.
        }
      }
    }
  });
  return {
    ready,
    send(method, params, session) {
      const id = ++next;
      const body = { id, method, params: params || {} };
      if (session) body.sessionId = session;
      ws.send(JSON.stringify(body));
      return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
    },
    close() {
      ws.close();
    },
  };
}

async function ensureBrowser() {
  if (cdp && sessionId) return;
  const url = await launchChrome();
  cdp = connectCdp(url);
  await cdp.ready;
  const { targetId } = await cdp.send("Target.createTarget", { url: "about:blank" });
  const attached = await cdp.send("Target.attachToTarget", { targetId, flatten: true });
  sessionId = attached.sessionId;
  await cdp.send("Page.enable", {}, sessionId);
  await cdp.send("Runtime.enable", {}, sessionId);
  await cdp.send("Runtime.addBinding", { name: "omoHuddleAudio" }, sessionId);
  await cdp.send(
    "Page.addScriptToEvaluateOnNewDocument",
    { source: PAGE_AUDIO_SOURCE },
    sessionId,
  );
}

async function evaluate(expression) {
  const result = await cdp.send(
    "Runtime.evaluate",
    { expression, awaitPromise: true, returnByValue: true },
    sessionId,
  );
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.text || "page script failed");
  }
  return result.result?.value;
}

const PAGE_AUDIO_SOURCE = `(() => {
  if (window.__omoHuddleAudio) return;
  window.__omoHuddleAudio = true;
  const tapped = new Set();
  const push = (body) => {
    if (typeof window.omoHuddleAudio === "function") window.omoHuddleAudio(JSON.stringify(body));
  };
  const speakerName = () => {
    const labels = [...document.querySelectorAll("[data-qa^='huddle_peer_tile_userId_']")]
      .map((node) => (node.getAttribute("aria-label") || "").trim())
      .filter(Boolean);
    return labels.length === 1 ? labels[0] : "participant";
  };
  const tap = (track) => {
    if (!track || track.kind !== "audio" || tapped.has(track.id)) return;
    tapped.add(track.id);
    const context = new AudioContext();
    const source = context.createMediaStreamSource(new MediaStream([track]));
    const processor = context.createScriptProcessor(2048, 1, 1);
    const silent = context.createGain();
    silent.gain.value = 0;
    source.connect(processor);
    processor.connect(silent);
    silent.connect(context.destination);
    const ratio = context.sampleRate / 16000;
    let bucket = 0;
    let count = 0;
    let pending = [];
    processor.onaudioprocess = (event) => {
      const input = event.inputBuffer.getChannelData(0);
      let peak = 0;
      for (let index = 0; index < input.length; index += 1) {
        const sample = input[index];
        peak = Math.max(peak, Math.abs(sample));
        bucket += sample;
        count += 1;
        if (count >= ratio) {
          const mixed = bucket / count;
          const clipped = Math.max(-1, Math.min(1, mixed));
          pending.push(clipped < 0 ? clipped * 0x8000 : clipped * 0x7fff);
          bucket = 0;
          count = 0;
        }
      }
      if (!pending.length) return;
      const quiet = peak < 0.02;
      const samples = pending;
      pending = [];
      if (quiet) {
        push({ kind: "silence", speaker: speakerName(), ms: Math.round((samples.length / 16000) * 1000) });
        return;
      }
      const pcm = new Int16Array(samples);
      const bytes = new Uint8Array(pcm.buffer);
      let binary = "";
      for (let index = 0; index < bytes.length; index += 1) binary += String.fromCharCode(bytes[index]);
      push({ kind: "audio", speaker: speakerName(), pcm: btoa(binary) });
    };
  };
  const Original = window.RTCPeerConnection;
  if (!Original) return;
  function ObservedPeerConnection(...args) {
    const peer = new Original(...args);
    peer.addEventListener("track", (event) => tap(event.track));
    return peer;
  }
  ObservedPeerConnection.prototype = Original.prototype;
  window.RTCPeerConnection = ObservedPeerConnection;
})();`;

const PROBE = `(() => {
  const one = (selector) => document.querySelector(selector);
  const text = (selector) => (one(selector)?.textContent || "").trim();
  return {
    href: location.href,
    signIn: Boolean(one("input[name=email], form[action*='signin']")),
    preview: Boolean(one("[data-qa='huddle_join_preview_modal']")),
    join: Boolean(one("[data-qa='huddle_join_preview_modal_go']")),
    confirmation: Boolean(one("[data-qa='huddle_join_modal'], [data-qa='huddle_in_thread_speed_bump_modal']")),
    multiDevice: Boolean(one("[data-qa='huddle_multi_device_modal_switch_device'], [data-qa='huddle_multi_device_modal_use_both_device']")),
    inCall: Boolean(one("[data-qa='huddle_toolbar__leave_button'], [data-qa='huddle_mini_player_leave_button'], [data-qa='huddle_sidebar_footer']")),
    member: Boolean(one(".p-huddle_channel_header_button--in_huddle")),
    muted: Boolean(one("[data-qa='huddle_mic_icon_mute']")),
    micLabel: text("#microphone-info"),
    captioning: Boolean(one(".p-huddle_closed_captions, [data-qa='huddle_transcribe_event']")),
    participants: document.querySelectorAll("[data-qa^='huddle_peer_tile_userId_'], [data-qa='huddle_avatar_stack__member']").length,
  };
})()`;

async function probe() {
  return (await evaluate(PROBE)) || {};
}

async function click(selector) {
  return Boolean(
    await evaluate(`(() => {
      const node = document.querySelector(${JSON.stringify(selector)});
      if (!node) return false;
      node.click();
      return true;
    })()`),
  );
}

function statusFrom(page, extras = {}) {
  const speechReady = Boolean(page.member && page.inCall && !page.muted && extras.micReady);
  return {
    phase: page.member && page.inCall ? "in_call" : "idle",
    url: currentUrl,
    muted: Boolean(page.muted),
    microphone: page.micLabel || "",
    speech_ready: speechReady,
    captioning: Boolean(page.captioning),
    participants: Number(page.participants || 0),
    manual_reason: extras.manualReason || null,
    manual_message: extras.manualMessage || null,
  };
}

async function muteInCall() {
  const page = await probe();
  if (!page.muted) {
    await click("[data-qa='huddle_sidebar_footer_mute_button'], button[aria-label='Microphone']");
    await delay(300);
  }
}

async function selectMicrophone() {
  if (!MIC_LABEL) return false;
  await click("[data-qa='huddle-toolbar-mic-popover-button']");
  await delay(400);
  const clicked = await evaluate(`(() => {
    const label = ${JSON.stringify(MIC_LABEL)};
    const items = [...document.querySelectorAll("[data-qa^='av-microphone-device-menu-item_'], [role='menuitem'], [role='option']")];
    const item = items.find((node) => (node.textContent || "").includes(label));
    if (!item) return false;
    item.click();
    return true;
  })()`);
  await delay(400);
  return Boolean(clicked);
}

async function prepareTalkback() {
  await muteInCall();
  const selected = await selectMicrophone();
  const page = await probe();
  const micReady = Boolean(selected && page.micLabel && page.micLabel.includes(MIC_LABEL));
  if (!micReady) {
    return {
      micReady: false,
      manualReason: "slack-audio-choice-required",
      manualMessage: `Select the virtual microphone "${MIC_LABEL}" in the huddle audio menu. The microphone stays muted until Slack shows that device.`,
    };
  }
  if (page.muted) {
    await click("[data-qa='huddle_sidebar_footer_mute_button'], button[aria-label='Microphone']");
    await delay(300);
  }
  return { micReady: true };
}

function fail(id, reason, message) {
  emit({ id, ok: false, reason, message });
}

async function join(id, rawUrl) {
  const url = normalizeTarget(rawUrl);
  if (!url) {
    fail(
      id,
      "invalid-target",
      "Use a Slack Copy huddle link (https://app.slack.com/huddle/TEAM/CHANNEL) or a channel id such as C0123ABCD.",
    );
    return;
  }
  if (inCall && currentUrl && currentUrl !== url) {
    fail(
      id,
      "slack-session-conflict",
      "This account is already in a huddle. Leave it before joining another.",
    );
    return;
  }
  await ensureBrowser();
  currentUrl = url;
  await cdp.send("Page.navigate", { url }, sessionId);
  await evaluate(PAGE_AUDIO_SOURCE);
  const deadline = Date.now() + JOIN_TIMEOUT_MS;
  let clickedJoin = false;
  while (Date.now() < deadline) {
    const page = await probe();
    if (page.signIn || String(page.href || "").includes("/signin")) {
      fail(id, "slack-login-required", "Sign this Chrome profile into the dedicated Slack account, then retry.");
      return;
    }
    if (page.confirmation) {
      fail(
        id,
        "slack-confirmation-required",
        "Slack is asking for a confirmation. Complete it in the huddle window. This client does not confirm switching huddles.",
      );
      return;
    }
    if (page.multiDevice) {
      fail(
        id,
        "slack-session-conflict",
        "This account is already in a huddle on another device. Leave it, then retry.",
      );
      return;
    }
    if (page.inCall && !page.member) {
      fail(
        id,
        "slack-session-conflict",
        "This account is already in a different huddle. Leave it, then retry.",
      );
      return;
    }
    if (page.member && page.inCall) {
      const talk = await prepareTalkback();
      inCall = true;
      const after = await probe();
      emit({ id, ok: true, status: statusFrom(after, talk) });
      return;
    }
    if (page.preview && page.join && !clickedJoin) {
      const previewUnmuted = await evaluate(
        "Boolean(document.querySelector('[data-qa=huddle_join_preview_modal] [data-qa=huddle_mic_icon_unmute]'))",
      );
      if (previewUnmuted) await click("[data-qa='huddle_join_preview_mic_button']");
      clickedJoin = await click("[data-qa='huddle_join_preview_modal_go']");
    }
    await delay(500);
  }
  fail(
    id,
    "slack-huddle-not-active",
    "No active huddle was found. Start the huddle in Slack, then retry. This client only joins an existing huddle.",
  );
}

async function leave(id) {
  if (!cdp || !sessionId) {
    inCall = false;
    emit({ id, ok: true, status: { phase: "idle", speech_ready: false } });
    return;
  }
  const page = await probe();
  if (page.confirmation || page.multiDevice) {
    fail(id, "slack-confirmation-required", "A Slack prompt is open. Dismiss it in the window, then leave again.");
    return;
  }
  if (page.member) {
    const clicked = await click(
      "[data-qa='huddle_toolbar__leave_button'], [data-qa='huddle_mini_player_leave_button']",
    );
    if (!clicked) {
      fail(id, "not-in-call", "The Leave Huddle control is not on this channel. Open the huddle channel and retry.");
      return;
    }
  }
  inCall = false;
  emit({ id, ok: true, status: { phase: "left", speech_ready: false, url: currentUrl } });
}

async function captions(id) {
  if (!cdp || !sessionId) {
    emit({ id, ok: true, captions: [] });
    return;
  }
  const rows = await evaluate(`(() => {
    const nodes = [...document.querySelectorAll(".p-huddle_closed_caption_event__wrapper, [data-qa='huddle_transcribe_event']")];
    return nodes.slice(-20).map((node) => ({
      speaker: (node.querySelector(".p-huddle_closed_caption_event__member_name")?.textContent || "").trim(),
      text: (node.querySelector(".p-huddle_closed_caption_event__event_text, [data-qa='huddle_transcribe_event']")?.textContent || node.textContent || "").trim(),
    })).filter((row) => row.text);
  })()`);
  emit({ id, ok: true, captions: rows || [] });
}

async function status(id) {
  if (!cdp || !sessionId) {
    emit({ id, ok: true, status: { phase: "idle", speech_ready: false } });
    return;
  }
  const page = await probe();
  inCall = Boolean(page.member && page.inCall);
  const micReady = Boolean(MIC_LABEL && page.micLabel && page.micLabel.includes(MIC_LABEL) && !page.muted);
  emit({ id, ok: true, status: statusFrom(page, { micReady }) });
}

async function handle(line) {
  let request;
  try {
    request = JSON.parse(line);
  } catch {
    return;
  }
  const id = request.id;
  try {
    if (request.op === "join") await join(id, request.url);
    else if (request.op === "leave") await leave(id);
    else if (request.op === "status") await status(id);
    else if (request.op === "transcript") await captions(id);
    else fail(id, "unknown-op", `Unknown huddle op ${request.op}`);
  } catch (error) {
    fail(id, "browser-control-unavailable", error instanceof Error ? error.message : String(error));
  }
}

if (process.env.OMO_HUDDLE_MOCK === "1") {
  const rl = createInterface({ input: process.stdin });
  rl.on("line", (line) => {
    const request = JSON.parse(line);
    if (request.op === "join") {
      emit({
        id: request.id,
        ok: true,
        status: { phase: "in_call", speech_ready: true, microphone: MIC_LABEL, muted: false },
      });
      emit({ event: "audio", speaker: "Ada", pcm: Buffer.from(Int16Array.from([1000, -1000]).buffer).toString("base64") });
    } else if (request.op === "transcript") {
      emit({ id: request.id, ok: true, captions: [{ speaker: "Ada", text: "mock caption" }] });
    } else {
      emit({ id: request.id, ok: true, status: { phase: "idle", speech_ready: false } });
    }
  });
} else {
  const rl = createInterface({ input: process.stdin });
  rl.on("line", (line) => {
    if (line.trim()) void handle(line);
  });
  rl.on("close", () => {
    if (launchedChrome) chrome?.kill("SIGTERM");
    process.exit(0);
  });
  process.on("exit", () => {
    if (launchedChrome) chrome?.kill("SIGTERM");
  });
}
