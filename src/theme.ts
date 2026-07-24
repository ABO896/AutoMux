// Theme preference persistence + resolution.
// Mirrors keymap.ts's convention: exported pure functions, no class, no
// default export, no side effects at module scope.

export type ThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "automux.theme_preference";

/**
 * Read the stored theme preference from localStorage.
 *
 * T-10-03: validates the stored value is exactly one of "system" / "light" /
 * "dark" — any unrecognized or absent value falls back to "system" rather
 * than being trusted as-is.
 */
export function getStoredPreference(): ThemePreference {
  const value = localStorage.getItem(STORAGE_KEY);
  return value === "light" || value === "dark" || value === "system" ? value : "system";
}

/**
 * Resolve a ThemePreference to a concrete "light" | "dark" value, following
 * the OS `prefers-color-scheme` media query when the preference is "system".
 */
export function resolveTheme(pref: ThemePreference): "light" | "dark" {
  if (pref !== "system") return pref;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/**
 * Apply a theme preference: sets the `data-theme` attribute on <html> to the
 * resolved value AND persists the preference (not the resolved value) to
 * localStorage so a "system" preference keeps following the OS on next boot.
 */
export function applyTheme(pref: ThemePreference): void {
  document.documentElement.setAttribute("data-theme", resolveTheme(pref));
  localStorage.setItem(STORAGE_KEY, pref);
}
