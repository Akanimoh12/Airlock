/**
 * Light and dark, the way the viewer asked for it. An explicit choice is
 * stamped on <html> and remembered; "system" stamps nothing and lets
 * prefers-color-scheme decide.
 */

export type Theme = "light" | "dark" | "system";

const KEY = "airlock-theme";

export function currentTheme(): Theme {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === "light" || saved === "dark") return saved;
  } catch {
    /* private window, blocked storage — system is a fine answer */
  }
  return "system";
}

export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);

  try {
    if (theme === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, theme);
  } catch {
    /* the page still looks right; it just will not be remembered */
  }
}

/** What a click on the toggle means: whatever is showing now, invert it. */
export function nextTheme(): Theme {
  const now = currentTheme();
  if (now === "system") {
    const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    return dark ? "light" : "dark";
  }
  return now === "dark" ? "light" : "dark";
}

export function initTheme() {
  applyTheme(currentTheme());
}
