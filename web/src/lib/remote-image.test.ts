import { describe, expect, it } from "bun:test";

import { isAutoLoadableImageSrc } from "./remote-image";

const ORIGIN = "http://127.0.0.1:9119";

describe("isAutoLoadableImageSrc", () => {
  it("loads inline and same-origin images", () => {
    expect(isAutoLoadableImageSrc("data:image/png;base64,AAAA", ORIGIN)).toBe(true);
    expect(isAutoLoadableImageSrc("blob:http://127.0.0.1:9119/abc", ORIGIN)).toBe(true);
    expect(isAutoLoadableImageSrc("/favicon-64.png", ORIGIN)).toBe(true);
    expect(isAutoLoadableImageSrc("./a.png", ORIGIN)).toBe(true);
    expect(isAutoLoadableImageSrc("http://127.0.0.1:9119/x.png", ORIGIN)).toBe(true);
  });

  it("never auto-loads remote images", () => {
    expect(isAutoLoadableImageSrc("https://example.com/track.png", ORIGIN)).toBe(false);
    expect(isAutoLoadableImageSrc("http://127.0.0.1:9120/x.png", ORIGIN)).toBe(false);
    expect(isAutoLoadableImageSrc("//example.com/x.png", ORIGIN)).toBe(false);
    expect(isAutoLoadableImageSrc("ftp://example.com/x.png", ORIGIN)).toBe(false);
    expect(isAutoLoadableImageSrc("/\\evil.com/x.png", ORIGIN)).toBe(false);
    expect(isAutoLoadableImageSrc("\\\\evil.com/x.png", ORIGIN)).toBe(false);
    expect(isAutoLoadableImageSrc("/\t/evil.com/x.png", ORIGIN)).toBe(false);
  });

  it("rejects empty sources", () => {
    expect(isAutoLoadableImageSrc("", ORIGIN)).toBe(false);
    expect(isAutoLoadableImageSrc(undefined, ORIGIN)).toBe(false);
  });
});
