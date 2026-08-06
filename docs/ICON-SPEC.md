# AutoMux App Icon — ChatGPT Generation Spec

AutoMux currently ships with the **stock default Tauri icon** (the orange/teal
figure-8 mark in `src-tauri/icons/icon.png`) — it carries no relationship to
the app's actual identity or UI. This document is a self-contained brief you
can hand to ChatGPT to generate a replacement that actually looks like
AutoMux.

## 1. App identity brief

- **Name:** AutoMux — "auto" (automation) + "mux" (multiplexer: many inputs
  routed through one controller).
- **What it does:** A cross-platform (macOS + Windows) desktop automation
  tool. Users define multi-step macros — clicks, keypresses, timed intervals,
  optional per-app targeting — and fire them system-wide or scoped to one
  application.
- **Audience:** Gamers, power users, and anyone automating repetitive
  mouse/keyboard input. Values reliability and precision, not novelty.
- **Category:** Desktop **Utility** — sits in the Dock / taskbar next to
  serious tools, not consumer/social apps. The icon should read as
  precise and technical, not playful or cartoonish.
- **Core value prop echoed in copy:** "sub-millisecond precision" — speed
  and exactness are the brand's emotional core, not "fun."

## 2. Current design language (extracted from the live UI)

Source: `src/App.css` (Tailwind v4 theme tokens).

| Token | Dark (default) | Light |
|---|---|---|
| Background | `#0a0a0f` (near-black) | `#f4f4f8` |
| Surface / card | `#12121a` | `#ffffff` |
| Border | `#2a2a3a` | `#e1e1ea` |
| Text | `#e8e8f0` | `#1c1c26` |
| **Accent (primary)** | `#6366f1` (indigo) | `#4f46e5` |
| Accent glow | `#818cf8` at ~50% alpha | `#6366f1` at ~30% alpha |
| Success | `#22c55e` (green) | `#16a34a` |
| Danger | `#ef4444` (red) | `#dc2626` |
| Warning | `#f59e0b` (amber) | `#d97706` |

**Shape and effect language:**
- 12px rounded corners on all cards/panels — soft-geometric, not sharp, not
  fully round.
- Circular **status-pulse** dot with a soft glow (`box-shadow: 0 0 12px
  var(--accent-glow)`) that breathes via a 2s opacity animation — this is
  the app's existing "macro is active" motif.
- Dark-first design; glow/luminance conveys state rather than heavy color
  blocking. Flat fills, no skeuomorphism, no gradients in the UI itself.
- Existing visual shorthand already used in-app: 🖱 (cursor/click) for input
  events, ⚡ (lightning bolt) for speed/empty-state emphasis.

**Net aesthetic:** modern, dark-mode-first developer/power-user tool —
closer to a monitoring dashboard or a synth/audio utility than a mobile app.
Indigo-violet glow on near-black is the single most distinctive visual
signature; reuse it rather than inventing a new palette.

## 3. Icon design constraints (non-negotiable)

These come from how icons actually get used, not aesthetic preference:

- **One concept, not a collage.** A single bold silhouette. No combining
  cursor + keyboard + clock + gear in one mark — pick one metaphor.
- **2–3 colors max.** Indigo/violet accent as the hero color against the
  dark background token (or transparent); avoid rainbow palettes.
- **No text/wordmark.** App icons at 16–32px cannot render legible text —
  this must be a pictorial mark only.
- **Legible at 32×32 and 16×16.** Thin strokes, fine detail, or small
  enclosed shapes disappear at menu-bar/taskbar size. Test mentally: would
  this still read as a distinct shape as a postage-stamp-sized icon?
- **Platform-safe silhouette.** macOS applies a rounded-square mask and
  standard drop shadow/perspective to `.icns` icons; Windows shows it
  unmasked in `.ico`. Keep the主 subject centered with安全 margin — don't
  bleed critical detail to the edges (macOS crops slightly at the corners).
- **Flat/vector treatment, not photorealistic.** Matches the app's flat,
  glow-driven UI — no 3D bevels, no photographic textures, no drop-shadow
  layering baked into the art itself (the OS adds its own shadow).

## 4. Suggested visual concepts

Pick one direction (or give ChatGPT several to riff on):

1. **Convergent Cursor** — a stylized mouse-cursor/pointer arrow with a
   circular pulse ring around its tip, echoing the app's own status-pulse
   dot. Reads as "click, automated."
2. **Mux Node** — two or three converging line-paths (signal traces) meeting
   at a single glowing node — literalizes "multiplexer" (many inputs, one
   controlled output) without being abstract-generic.
3. **Pulse Bracket** — a rounded-square bracket/frame (echoing the UI's
   12px-radius cards) with a single glowing dot pulsing at its center —
   most literal reuse of the existing in-app motif.

Direction 1 or 3 will scale down more reliably than direction 2 (thin
converging lines risk disappearing at 16px).

## 5. How to prompt ChatGPT's image generator (researched August 2026)

As of this writing, ChatGPT's image generation runs on **GPT Image 2**
(surfaced in the product as "ChatGPT Images 2.0"), which replaced GPT-4o's
native image generation back in April 2026. The legacy DALL·E interface is
being fully retired from ChatGPT on August 30, 2026, so there's no "which
tool" choice to make anymore — whatever ChatGPT gives you when you ask for
an image today is GPT Image 2. It reasons over the prompt before rendering,
supports higher-resolution output (up to ~2K), and reads plain natural
language rather than tag/keyword lists.

**Prompt framework — five components, in this order (GPT Image 2 weighs
earlier words more heavily, so keep the subject first):**

1. **Subject** — the one concept, stated plainly and first (e.g. "a mouse
   cursor with a glowing pulse ring around its tip").
2. **Important details** — geometry, style, composition (e.g. "flat vector,
   geometric, minimal, centered, no gradients besides the glow").
3. **Color** — exact hex codes, not color names, so the model doesn't
   improvise a different shade of "purple."
4. **Use case** — say what it's for: "app icon" / "logo mark" — this
   measurably changes composition/margin choices.
5. **Constraints** — format and technical requirements: transparent PNG
   background, 1024x1024, 1:1 aspect ratio, no text.

**Other things that measurably improve results with this model:**
- Write like you're briefing a designer, not stacking tags — "solid flat
  colors, clean geometric shapes, no gradients, no shadows, minimal
  strokes" beats vague adjectives like "modern and clean." Keyword-spam
  suffixes ("8k, masterpiece, trending on artstation") do nothing for this
  model and can be dropped entirely.
- **Transparency requires PNG or WebP** — say "transparent PNG background"
  explicitly. If ChatGPT ever hands back a JPEG, transparency silently
  fails; ask it to re-render as PNG.
- State explicitly what to avoid — "no text, no watermark, no
  photorealism, no drop shadow baked into the art, no clutter, no
  additional icons."
- Ask for **multiple variants** in one turn ("give me 3 variations of this
  concept").
- **One revision per turn when refining.** GPT Image 2 keeps style/color
  memory across a chat thread, but stacking several changes in one edit
  ("make the ring thinner AND shift the color AND recenter it") degrades
  results — adjust one thing (color, then geometry, then detail), look,
  then adjust the next.
- If a refinement thread drifts far from the original intent, start a new
  chat rather than continuing to patch — accumulated edits compound style
  drift.
- Request a plain **solid near-black background version too** (same
  prompt, swap the background line) — useful to sanity-check contrast and
  legibility against the app's actual dark theme before finalizing.

## 6. Ready-to-use prompts

Paste one of these directly into ChatGPT.

**Primary (Convergent Cursor direction):**

> Create a minimalist flat-vector app icon: a stylized mouse cursor/pointer
> arrow, with a thin glowing ring pulsing outward from its tip like a
> ripple. Use only two colors: the cursor and ring in `#6366f1` (indigo)
> with a soft outer glow in `#818cf8`, on a fully transparent PNG
> background. Geometric, clean, no gradients beyond the soft glow, no
> bevels, no 3D, no drop shadow, no text, no watermark, no extra icons or
> clutter. Bold enough silhouette to stay recognizable at 16x16px. Square
> canvas, 1024x1024, 1:1 aspect ratio, centered subject with margin from
> the edges for a rounded-square icon mask. Give me 3 variations.

**Alternative (Pulse Bracket direction):**

> Create a minimalist flat-vector app icon: a rounded-square outline
> bracket (like a soft app-card frame, corner radius roughly 20% of the
> icon width) in `#6366f1` (indigo), with a single glowing dot pulsing at
> the exact center in `#818cf8`. Fully transparent PNG background. Flat,
> geometric, no gradients beyond the dot's soft glow, no bevels, no 3D, no
> drop shadow, no text, no watermark, no clutter. Must stay legible as a
> distinct shape at 16x16px. Square canvas, 1024x1024, 1:1 aspect ratio,
> centered with safe margin from the edges. Give me 3 variations.

**Dark-background preview variant** (swap into either prompt above to
sanity-check on-brand contrast):

> ...on a solid `#0a0a0f` near-black background instead of transparent...

## 7. What to bring back

At minimum:
- One **1024×1024 PNG, transparent background** — this is the master
  source used to regenerate every platform size.

Nice to have (not required, I can derive them):
- The same mark on a solid `#0a0a0f` background, for visual comparison
  against the live app.

## 8. Implementation plan (once you provide the image)

1. Save the chosen 1024×1024 PNG into the repo (e.g.
   `src-tauri/icons/source-icon.png`).
2. Run `npx tauri icon src-tauri/icons/source-icon.png` — the Tauri CLI
   (already a devDependency, `@tauri-apps/cli`) regenerates every required
   platform size (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`,
   `icon.ico`, plus the Android/iOS variants already present) directly into
   `src-tauri/icons/`, matching the filenames already referenced in
   `src-tauri/tauri.conf.json`'s `bundle.icon` list — no config changes
   needed.
3. Rebuild (`npm run tauri dev` or a full bundle) and visually confirm the
   new icon in the Dock/taskbar and window title bar at actual size.
4. Commit the regenerated `src-tauri/icons/` assets.
