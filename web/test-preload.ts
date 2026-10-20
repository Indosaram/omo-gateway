/**
 * Bun test preload (see ``bunfig.toml``).
 *
 * The dashboard suite is written against vitest and the gate runs it with
 * ``bun test``, whose ``vitest`` compatibility layer (``bun:test``)
 * implements ``vi.fn`` / ``vi.spyOn`` / the mock-restore helpers but not
 * ``vi.stubGlobal`` / ``vi.unstubAllGlobals``. Those two are used to swap
 * ``window`` per test, so they are installed here with vitest's semantics:
 * ``stubGlobal`` records the previous property descriptor and installs the
 * stub, ``unstubAllGlobals`` restores every stub in reverse order.
 *
 * Both directions use ``Object.defineProperty`` because some Bun globals
 * (``fetch``, and ``window`` once the test runner has touched it) are
 * accessor or non-writable properties, where plain assignment throws
 * "Attempted to assign to readonly property".
 */

import { vi } from "vitest";

interface GlobalStub {
  key: string;
  /** Original descriptor, or null when the global did not exist. */
  descriptor: PropertyDescriptor | null;
}

const stubs: GlobalStub[] = [];

function install(name: string, value: unknown): void {
  const target = globalThis as unknown as Record<string, unknown>;
  const existing = Object.getOwnPropertyDescriptor(target, name);
  stubs.push({ descriptor: existing ?? null, key: name });
  Object.defineProperty(target, name, {
    configurable: true,
    enumerable: existing?.enumerable ?? true,
    value,
    writable: true,
  });
}

function restoreAll(): void {
  const target = globalThis as unknown as Record<string, unknown>;
  for (let stub = stubs.pop(); stub; stub = stubs.pop()) {
    if (stub.descriptor) Object.defineProperty(target, stub.key, stub.descriptor);
    else delete target[stub.key];
  }
}

const patched = vi as unknown as {
  stubGlobal?: (name: string, value: unknown) => void;
  unstubAllGlobals?: () => void;
};

if (typeof patched.stubGlobal !== "function") patched.stubGlobal = install;
if (typeof patched.unstubAllGlobals !== "function") patched.unstubAllGlobals = restoreAll;
