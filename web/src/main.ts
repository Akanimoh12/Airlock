import "./styles.css";

import { icons } from "./lib/dom";
import { store } from "./lib/store";
import { applyTheme, currentTheme, initTheme, nextTheme } from "./lib/theme";
import { renderWallet } from "./views/wallet";
import { renderSender } from "./views/sender";
import { renderPipeline } from "./views/pipeline";
import { renderHome } from "./views/home";

type Route = "home" | "wallet" | "sender" | "pipeline";

/**
 * The tab strip, in demo order: the pipeline is the frame, the message is
 * what a judge sends, the wallet is where it lands. Home is not in it — the
 * wordmark is the way back.
 */
const TABS: { id: Route; label: string }[] = [
  { id: "pipeline", label: "Pipeline" },
  { id: "sender", label: "Send a message" },
  { id: "wallet", label: "Wallet" },
];

function routeFromHash(): Route {
  const id = location.hash.replace(/^#\/?/, "") as Route;
  return TABS.some((t) => t.id === id) ? id : "home";
}

const app = document.getElementById("app")!;
app.innerHTML = `
  <header class="topbar">
    <a class="brand" href="#/" id="brand" title="What Airlock is">
      ${icons.airlock}<span>Airlock</span>
    </a>
    <div class="spacer spacer-lead"></div>
    <nav class="nav" id="nav">
      <span class="nav-pill" id="nav-pill" aria-hidden="true"></span>
      ${TABS.map((t) => `<a href="#/${t.id}" data-route="${t.id}">${t.label}</a>`).join("")}
    </nav>
    <div class="spacer"></div>
    <div class="topbar-end">
      <span class="pill" id="conn-pill"><i class="dot"></i><span>connecting</span></span>
      <button class="icon-btn" id="theme" title="Light or dark" aria-label="Toggle theme"></button>
    </div>
  </header>
  <main id="view"></main>
`;

const view = document.getElementById("view")!;
const nav = document.getElementById("nav")!;
const navPill = document.getElementById("nav-pill")!;
const brand = document.getElementById("brand")!;
const connPill = document.getElementById("conn-pill")!;
const themeBtn = document.getElementById("theme") as HTMLButtonElement;

// -- theme ------------------------------------------------------------------

initTheme();
paintThemeButton();

themeBtn.addEventListener("click", () => {
  applyTheme(nextTheme());
  paintThemeButton();
});

function paintThemeButton() {
  const t = currentTheme();
  const dark =
    t === "dark" ||
    (t === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  themeBtn.innerHTML = dark ? icons.sun : icons.moon;
}

window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", paintThemeButton);

// -- status indicator -------------------------------------------------------
// Real server state: the SSE connection itself. The Reader's health is shown
// on the pipeline instead of here — in global chrome it reads as a permanent
// disclaimer about the build; on the pipeline it reads as component status,
// which is what it is.

store.subscribe((s) => {
  const cls =
    s.connection === "live" ? "live" : s.connection === "down" ? "down" : "busy";
  connPill.innerHTML = `<i class="dot ${cls}"></i><span>${s.connection}</span>`;
});

// -- routing ----------------------------------------------------------------

let dispose: (() => void) | null = null;

/**
 * Slide the marker under the active tab. Measured from layout rather than
 * computed from the label text, so a font swap or a resize cannot leave it
 * sitting under the wrong word. On `home` there is no active tab, so it fades
 * out instead of parking somewhere arbitrary.
 */
function moveNavPill(route: Route) {
  const active = nav.querySelector<HTMLElement>(`a[data-route="${route}"]`);
  if (!active) {
    navPill.classList.remove("shown");
    return;
  }

  const left = active.offsetLeft;
  const width = active.offsetWidth;
  navPill.style.transform = `translateX(${left}px)`;
  navPill.style.width = `${width}px`;
  navPill.classList.add("shown");

  // Enable the transition only after the first placement, so it does not
  // animate in from the left edge on load.
  if (!navPill.classList.contains("ready")) {
    requestAnimationFrame(() => navPill.classList.add("ready"));
  }
}

window.addEventListener("resize", () => moveNavPill(routeFromHash()));
// Webfonts change tab widths when they swap in.
document.fonts?.ready.then(() => moveNavPill(routeFromHash()));

function show() {
  const route = routeFromHash();
  nav.querySelectorAll("a").forEach((a) => {
    if (a.dataset.route === route) a.setAttribute("aria-current", "page");
    else a.removeAttribute("aria-current");
  });
  if (route === "home") brand.setAttribute("aria-current", "page");
  else brand.removeAttribute("aria-current");

  moveNavPill(route);

  dispose?.();
  view.innerHTML = "";

  // Home runs its own staggered entrance; the rest get one plain fade-up so
  // arriving at any page feels the same.
  view.classList.remove("entered");
  view.classList.toggle("plain-enter", route !== "home");

  if (route === "wallet") dispose = renderWallet(view);
  else if (route === "sender") dispose = renderSender(view);
  else if (route === "pipeline") dispose = renderPipeline(view);
  else dispose = renderHome(view);

  // A fresh route starts at the top rather than inheriting the last scroll.
  window.scrollTo(0, 0);

  // Same two-frame reason as the hero: the start state has to be observed
  // before the transition will run.
  requestAnimationFrame(() =>
    requestAnimationFrame(() => view.classList.add("entered")),
  );
}

window.addEventListener("hashchange", show);
show();
store.start();
