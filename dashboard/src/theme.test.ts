import { describe, expect, it, vi } from "vitest";

import { applyTheme } from "./theme";

function colorSchemeQuery(matches: boolean) {
  let listener: (() => void) | undefined;
  return {
    addEventListener: vi.fn((_event: "change", nextListener: () => void) => {
      listener = nextListener;
    }),
    matches,
    notify: () => listener?.(),
    removeEventListener: vi.fn(),
  };
}

describe("applyTheme", () => {
  it("tracks system preference changes without changing the configured theme", () => {
    const root = document.createElement("div");
    const query = colorSchemeQuery(false);
    const cleanup = applyTheme("system", root, query);
    expect(root.dataset.theme).toBe("light");

    query.matches = true;
    query.notify();
    expect(root.dataset.theme).toBe("dark");

    cleanup();
    expect(query.removeEventListener).toHaveBeenCalledOnce();
  });

  it("applies an explicit theme without subscribing to system changes", () => {
    const root = document.createElement("div");
    const query = colorSchemeQuery(false);

    applyTheme("dark", root, query);

    expect(root.dataset.theme).toBe("dark");
    expect(query.addEventListener).not.toHaveBeenCalled();
  });
});
