/** Escape anything interpolated into markup. */
export function esc(s: unknown): string {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

export function on<K extends keyof HTMLElementEventMap>(
  root: ParentNode,
  selector: string,
  type: K,
  handler: (e: HTMLElementEventMap[K], el: HTMLElement) => void,
) {
  root.querySelectorAll<HTMLElement>(selector).forEach((el) => {
    el.addEventListener(type, (e) => handler(e as HTMLElementEventMap[K], el));
  });
}

/** Inline SVG only — no emoji, so icons scale and recolour. */
export const icons = {
  airlock: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none" aria-hidden="true">
    <circle cx="10" cy="10" r="8.25" stroke="currentColor" stroke-width="1.6"/>
    <circle cx="10" cy="10" r="3.25" stroke="currentColor" stroke-width="1.6"/>
  </svg>`,
  pause: `<svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true">
    <rect x="4.5" y="3" width="3.25" height="12" rx="1.6" fill="currentColor"/>
    <rect x="10.25" y="3" width="3.25" height="12" rx="1.6" fill="currentColor"/>
  </svg>`,
  sun: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
    <circle cx="8" cy="8" r="3.25" stroke="currentColor" stroke-width="1.5"/>
    <path d="M8 1v1.6M8 13.4V15M15 8h-1.6M2.6 8H1M12.95 3.05l-1.13 1.13M4.18 11.82l-1.13 1.13M12.95 12.95l-1.13-1.13M4.18 4.18L3.05 3.05"
      stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
  </svg>`,
  moon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
    <path d="M13.5 9.6A5.9 5.9 0 016.4 2.5a5.9 5.9 0 107.1 7.1z"
      stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
  </svg>`,
};
