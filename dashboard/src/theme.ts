import type { Theme } from "./generated/transport";

type ColorSchemeQuery = Pick<
  MediaQueryList,
  "addEventListener" | "matches" | "removeEventListener"
>;

export function applyTheme(
  theme: Theme,
  root: HTMLElement = document.documentElement,
  colorSchemeQuery: ColorSchemeQuery = window.matchMedia(
    "(prefers-color-scheme: dark)",
  ),
): () => void {
  const apply = () => {
    const resolvedTheme =
      theme === "system"
        ? colorSchemeQuery.matches
          ? "dark"
          : "light"
        : theme;
    root.dataset.theme = resolvedTheme;
    root.style.colorScheme = resolvedTheme;
  };
  apply();
  if (theme !== "system") {
    return () => undefined;
  }

  colorSchemeQuery.addEventListener("change", apply);
  return () => colorSchemeQuery.removeEventListener("change", apply);
}
