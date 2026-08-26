/**
 * Home — what Airlock is, and why. Reached from the wordmark, and where a
 * fresh visit lands.
 *
 * Every number here comes from README.md. Nothing is rounded up for effect and
 * nothing is invented — the limitations section is as prominent as the pitch,
 * because volunteering the false-positive rate is the cheapest credibility
 * available and the demo's fifth beat depends on it.
 */

/**
 * The concept, drawn. Two doors that are never open at once, and a chamber
 * between them where a transfer waits out its sixty seconds.
 *
 * Inline SVG rather than an asset: it recolours with the theme through CSS
 * variables, scales without a second file, and costs no request.
 */
const DIAGRAM = `
<svg class="airlock-diagram" viewBox="0 0 400 508" role="img"
     aria-label="An inbound scam message reaches the outer door, which seals. The transfer waits in the chamber for sixty seconds. The inner door stays closed until it does.">
  <defs>
    <pattern id="chamber-dots" width="13" height="13" patternUnits="userSpaceOnUse">
      <circle cx="1.4" cy="1.4" r="1.15" class="dg-dot" />
    </pattern>
    <marker id="dg-head" markerWidth="9" markerHeight="9" refX="4.2" refY="4.5" orient="auto">
      <path d="M1 1.6 L5.6 4.5 L1 7.4" class="dg-head" />
    </marker>
  </defs>

  <text x="4" y="11" class="dg-label">Outside &middot; untrusted</text>
  <rect x="4" y="22" width="392" height="78" rx="13" class="dg-card" />
  <text x="24" y="52" class="dg-msg">&ldquo;Your account will be suspended.</text>
  <text x="24" y="76" class="dg-msg">Pay N150,000 now to reactivate.&rdquo;</text>

  <path d="M200 108 V132" class="dg-arrow" marker-end="url(#dg-head)" />

  <rect x="4" y="142" width="392" height="11" rx="5.5" class="dg-door-sealed" />
  <text x="200" y="170" class="dg-door-label" text-anchor="middle">outer door &middot; sealed behind it</text>

  <rect x="4" y="184" width="392" height="196" rx="15" class="dg-chamber" />
  <rect x="5" y="185" width="390" height="194" rx="14" fill="url(#chamber-dots)" />
  <text x="22" y="209" class="dg-label">The chamber</text>

  <circle cx="200" cy="282" r="45" class="dg-ring-track" />
  <circle cx="200" cy="282" r="45" class="dg-ring" transform="rotate(-90 200 282)" />
  <text x="200" y="290" class="dg-count" text-anchor="middle">60s</text>

  <text x="200" y="352" class="dg-chip" text-anchor="middle">&#8358;150,000.00 &rarr; *******567</text>

  <rect x="4" y="396" width="392" height="11" rx="5.5" class="dg-door-shut" />
  <text x="200" y="424" class="dg-door-label" text-anchor="middle">inner door &middot; closed until it clears</text>

  <path d="M200 436 V458" class="dg-arrow-waiting" marker-end="url(#dg-head)" />

  <rect x="4" y="466" width="392" height="40" rx="12" class="dg-card dg-card-quiet" />
  <text x="24" y="491" class="dg-msg dg-msg-quiet">Your balance</text>
  <text x="376" y="491" class="dg-chip" text-anchor="end">&#8358;180,000.00</text>
</svg>`;

/** The two questions, side by side. Same transfer, different check. */
const FIG_QUESTIONS = `
<svg class="fig" viewBox="0 0 680 182" role="img"
     aria-label="Every existing control asks whether it is really you, and the answer is yes, so the money leaves. Airlock asks whose idea it was, and holds when the answer is someone else's.">
  <defs>
    <marker id="fq-head" markerWidth="9" markerHeight="9" refX="4.2" refY="4.5" orient="auto">
      <path d="M1 1.6 L5.6 4.5 L1 7.4" class="dg-head" />
    </marker>
  </defs>

  <text x="1" y="10" class="dg-label">Every existing control</text>
  <rect x="0" y="20" width="384" height="54" rx="11" class="dg-card" />
  <text x="22" y="53" class="fig-q">&ldquo;Is this really you?&rdquo;</text>
  <path d="M394 47 H448" class="dg-arrow" marker-end="url(#fq-head)" />
  <rect x="462" y="20" width="218" height="54" rx="11" class="dg-card dg-card-quiet" />
  <text x="483" y="53" class="fig-out">Yes &mdash; money leaves</text>

  <text x="1" y="108" class="dg-label">Airlock</text>
  <rect x="0" y="118" width="384" height="54" rx="11" class="dg-card fig-live" />
  <text x="22" y="151" class="fig-q fig-q-live">&ldquo;Whose idea was this?&rdquo;</text>
  <path d="M394 145 H448" class="dg-arrow" marker-end="url(#fq-head)" />
  <rect x="462" y="118" width="218" height="54" rx="11" class="fig-hold-card" />
  <text x="483" y="151" class="fig-out fig-out-hold">Theirs &mdash; held 60s</text>
</svg>`;

/** What each agent is allowed to see. Neither can finish the attack alone. */
const FIG_BOUNDARIES = `
<svg class="fig" viewBox="0 0 680 206" role="img"
     aria-label="The Reader sees raw message text but has no account access and cannot move money. The Linker sees account facts but never raw text, and also cannot move money.">
  <rect x="0" y="0" width="330" height="200" rx="13" class="dg-card" />
  <text x="22" y="30" class="dg-label">Agent</text>
  <text x="22" y="55" class="fig-name">Reader</text>
  <g class="fig-yes"><path d="M24 88 l7 7 l13 -14" /></g>
  <text x="56" y="93" class="fig-item">Raw message text</text>
  <g class="fig-no"><path d="M24 124 l16 16 M40 124 l-16 16" /></g>
  <text x="56" y="137" class="fig-item fig-item-off">Account access</text>
  <g class="fig-no"><path d="M24 164 l16 16 M40 164 l-16 16" /></g>
  <text x="56" y="177" class="fig-item fig-item-off">Can move money</text>

  <rect x="350" y="0" width="330" height="200" rx="13" class="dg-card" />
  <text x="372" y="30" class="dg-label">Agent</text>
  <text x="372" y="55" class="fig-name">Linker</text>
  <g class="fig-no"><path d="M374 124 l16 16 M390 124 l-16 16" transform="translate(0,-36)" /></g>
  <text x="406" y="93" class="fig-item fig-item-off">Raw message text</text>
  <g class="fig-yes"><path d="M374 132 l7 7 l13 -14" /></g>
  <text x="406" y="137" class="fig-item">Account facts</text>
  <g class="fig-no"><path d="M374 164 l16 16 M390 164 l-16 16" /></g>
  <text x="406" y="177" class="fig-item fig-item-off">Can move money</text>
</svg>`;

/** When screening dies, only one of the two exits stays open. */
const FIG_FAILCLOSED = `
<svg class="fig" viewBox="0 0 680 176" role="img"
     aria-label="When screening is unavailable, the route to cleared is closed and the transfer is held instead.">
  <defs>
    <marker id="fc-head" markerWidth="9" markerHeight="9" refX="4.2" refY="4.5" orient="auto">
      <path d="M1 1.6 L5.6 4.5 L1 7.4" class="dg-head" />
    </marker>
    <marker id="fc-head-dead" markerWidth="9" markerHeight="9" refX="4.2" refY="4.5" orient="auto">
      <path d="M1 1.6 L5.6 4.5 L1 7.4" class="dg-head-dead" />
    </marker>
  </defs>

  <rect x="0" y="58" width="236" height="60" rx="12" class="fig-dead-card" />
  <text x="22" y="83" class="dg-label">Screening</text>
  <text x="22" y="105" class="fig-item">unavailable</text>

  <path d="M246 78 C296 78, 296 44, 344 44" class="dg-arrow-dead" marker-end="url(#fc-head-dead)" />
  <path d="M246 98 C296 98, 296 132, 344 132" class="dg-arrow" marker-end="url(#fc-head)" />

  <rect x="356" y="20" width="230" height="48" rx="11" class="dg-card dg-card-quiet" />
  <text x="377" y="50" class="fig-out fig-out-off">Cleared</text>
  <path d="M368 44 H574" class="fig-strike" />

  <rect x="356" y="108" width="230" height="48" rx="11" class="fig-hold-card" />
  <text x="377" y="138" class="fig-out fig-out-hold">Held</text>
</svg>`;

const OWNERSHIP: [string, string, boolean][] = [
  ["Understanding what a message is pressuring you to do", "Agent", false],
  ["Judging whether a transfer answers that pressure", "Agent", false],
  ["Whether to hold", "Deterministic policy", true],
  ["How long the hold lasts", "Deterministic policy", true],
  ["Transaction state transitions", "Deterministic policy", true],
  ["Releasing a held transfer", "User only", true],
  ["Moving money", "Deterministic policy", true],
];

const NOT: [string, string][] = [
  [
    "Not spam detection",
    "The message being a scam is not the finding. The payment being caused by it is.",
  ],
  [
    "Not a block",
    "Every hold is releasable by the account holder. A minute of inconvenience against a drained wallet.",
  ],
  [
    "Not a replacement for authentication",
    "It runs after authentication succeeds — which is exactly when this fraud happens.",
  ],
  [
    "Not a claim to catch everything",
    "It targets one specific, enormous, currently-unaddressed class: fraud the victim authorises.",
  ],
];

const LIMITS: string[] = [
  "It holds legitimate first-time transfers. Measured, reported, and mitigated with a one-tap release rather than hidden.",
  "It requires visibility into inbound messages — a control a wallet, bank or telco can run, not one a third party can bolt on.",
  "A patient attacker who waits out the correlation window defeats the recency signal. Novelty and responsiveness still apply; the window is defence in depth, not the whole defence.",
];

export function renderHome(root: HTMLElement): () => void {
  root.innerHTML = `
    <div class="prose-wrap dotgrid dotgrid-capped">

      <header class="hero">
        <div class="hero-inner">
          <div class="hero-copy">
            <span class="eyebrow">Authorised push payment fraud</span>
            <h1>Your bank checks that it&rsquo;s really you.<em>Airlock checks whether it was really your idea.</em></h1>
            <p class="lede">
              A transaction guard for mobile money and digital wallets that stops the scams
              where the victim sends the money themselves.
            </p>
            <p class="tagline">Nothing passes straight from outside to inside. It waits in the chamber first.</p>
          </div>
          <figure class="hero-art">
            ${DIAGRAM}
            <figcaption>
              Two doors, never open at once. Sixty seconds is all it takes to break the hurry
              the scam depends on.
            </figcaption>
          </figure>
        </div>
      </header>

      <article class="prose">

        <section>
          <h2>The problem</h2>
          <blockquote>
            MTN Alert: your account will be suspended today.<br />Call this number to reactivate.
          </blockquote>
          <p>
            You call. A polite person walks you through some steps. At the end, <strong>you</strong>
            send the money. Your phone, your PIN, your fingerprint.
          </p>
          <p>
            Every fraud control in the stack is asking one question &mdash; <em>is this really you?</em>
            &mdash; and the answer is yes. So every check passes correctly, and the money is gone.
          </p>

          <div class="stats">
            <div class="stat">
              <b>$4bn+</b><span>lost to mobile money fraud in Africa each year</span>
            </div>
            <div class="stat">
              <b>$192M &rarr; $484M</b><span>continental cybercrime losses, 2024 to 2025</span>
            </div>
            <div class="stat">
              <b>300%</b><span>rise in confirmed Nigerian SIM-swap cases, 2022&ndash;2024</span>
            </div>
            <div class="stat">
              <b>97%</b><span>of countries INTERPOL surveyed call it a major threat</span>
            </div>
          </div>

          <p class="kicker">The locks all work. The person holding the key was tricked.</p>
        </section>

        <section>
          <h2>The insight</h2>
          <p>
            Airlock asks a different question. Not <em>&ldquo;is this really you?&rdquo;</em>
            but <em>&ldquo;whose idea was this?&rdquo;</em>
          </p>
          <p>
            Paying your landlord, buying airtime, sending money to family the way you do every
            month &mdash; your idea. Straight through, no interruption.
          </p>
          <p>
            But a first-time recipient, four minutes after an unsolicited message arrived, in an
            amount that message named &mdash; that wasn&rsquo;t your idea. Someone put it there.
          </p>
          <p>
            So Airlock holds it in the chamber for sixty seconds. These scams run on manufactured
            urgency; the caller is <em>hurrying you</em>. Remove the hurry and most of them collapse.
          </p>
          ${FIG_QUESTIONS}
          <p class="kicker">
            A hold is not a block. You can always release it.
          </p>
        </section>

        <section>
          <h2>Agents propose. Code controls.</h2>
          <p>
            A language model reads and judges. It never decides. No model output moves money,
            sets a hold duration, or opens the chamber.
          </p>
          <div class="own">
            ${OWNERSHIP.map(
              ([concern, owner, hard]) =>
                `<div class="own-row">
                   <span>${concern}</span>
                   <span class="badge ${hard ? "busy" : ""}">${owner}</span>
                 </div>`,
            ).join("")}
          </div>
          <p class="small muted">
            See it run on the <a href="#/pipeline">pipeline</a>.
          </p>
        </section>

        <section>
          <h2>Two agents, neither of which can complete the attack</h2>
          <p>
            The <strong>Reader</strong> handles untrusted content. It is good at language and has
            zero access to accounts or funds. A message crafted to manipulate it is talking to
            something with no power.
          </p>
          <p>
            The <strong>Linker</strong> has account context but never receives raw message text
            &mdash; only a typed, schema-validated signal. It cannot be prompt-injected because it
            is never shown attacker-controlled prose.
          </p>
          ${FIG_BOUNDARIES}
          <p>
            The reason you are reading is a fixed enum, not model output. There is no path by
            which text an attacker wrote reaches this screen.
          </p>
        </section>

        <section>
          <h2>Fail closed</h2>
          <p>
            If screening crashes, times out, or returns malformed output, a transfer to a
            first-time recipient defaults to <em>held</em> &mdash; never to approved. Component
            death degrades toward safety, never toward silent approval.
          </p>
          ${FIG_FAILCLOSED}
          <p class="kicker">An airlock that loses power seals. It does not open both doors.</p>
        </section>

        <section>
          <h2>What Airlock is not</h2>
          <div class="notlist">
            ${NOT.map(
              ([t, d]) => `<div class="notitem"><b>${t}</b><span>${d}</span></div>`,
            ).join("")}
          </div>
        </section>

        <section>
          <h2>Known limitations</h2>
          <p class="small muted" style="margin-top:-4px">
            Stated up front, because a fraud control that hides its false positives is not one
            you should trust.
          </p>
          <ul class="limits">
            ${LIMITS.map((l) => `<li>${l}</li>`).join("")}
          </ul>
        </section>

        <footer class="prose-foot">
          <a class="btn primary" href="#/sender">Try it &mdash; send a message</a>
          <a class="btn" href="#/wallet">Open the wallet</a>
        </footer>

      </article>
    </div>
  `;

  // Two frames, not one. A single rAF can land before the browser has
  // computed the starting `opacity: 0`, and a transition with no observed
  // start state does not run — the hero would just snap in. The second frame
  // guarantees the initial style has been flushed, so the entrance replays on
  // every arrival rather than only on a cold load.
  //
  // CSS gates the whole thing behind prefers-reduced-motion.
  requestAnimationFrame(() =>
    requestAnimationFrame(() =>
      root.querySelector(".hero")?.classList.add("in"),
    ),
  );

  return () => {};
}
