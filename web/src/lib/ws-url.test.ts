import { afterEach, describe, expect, it, vi } from "vitest";

import { buildHermesWebSocketUrl } from "./ws-url";

function stubLocation(protocol: string, host: string) {
  vi.stubGlobal("window", { location: { host, protocol } });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("buildHermesWebSocketUrl", () => {
  it("uses ws on http and wss on https", () => {
    stubLocation("http:", "127.0.0.1:9119");
    expect(buildHermesWebSocketUrl({ authParam: ["token", "abc"], path: "/api/ws" })).toBe(
      "ws://127.0.0.1:9119/api/ws?token=abc",
    );

    stubLocation("https:", "kanban.tilos.com");
    expect(buildHermesWebSocketUrl({ authParam: ["ticket", "t1"], path: "/api/ws" })).toBe(
      "wss://kanban.tilos.com/api/ws?ticket=t1",
    );
  });

  it("applies the dashboard base path prefix", () => {
    stubLocation("https:", "mission-control.tilos.com");
    expect(
      buildHermesWebSocketUrl({
        authParam: ["ticket", "t1"],
        basePath: "/hermes",
        path: "/api/ws",
      }),
    ).toBe("wss://mission-control.tilos.com/hermes/api/ws?ticket=t1");
  });

  it("merges extra params before the auth param and encodes them", () => {
    stubLocation("http:", "localhost:9119");
    expect(
      buildHermesWebSocketUrl({
        authParam: ["ticket", "a b"],
        basePath: "/",
        params: { channel: "chat 1", profile: "work" },
        path: "/api/events",
      }),
    ).toBe("ws://localhost:9119/api/events?channel=chat+1&profile=work&ticket=a+b");
  });

  it("omits an empty auth value instead of emitting a dangling key", () => {
    stubLocation("http:", "localhost:9119");
    expect(buildHermesWebSocketUrl({ authParam: ["token", ""], path: "/api/ws" })).toBe(
      "ws://localhost:9119/api/ws",
    );
  });
});
