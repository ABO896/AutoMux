---
quick_id: 260806-jx4
status: complete
---

# Summary: AutoMux app icon ChatGPT generation spec

Wrote `docs/ICON-SPEC.md` — a self-contained brief for generating a
replacement AutoMux app icon via ChatGPT, to replace the current stock
default Tauri icon.

**Contents:** app identity brief; design tokens extracted from
`src/App.css` (dark `#0a0a0f` background, indigo `#6366f1` accent with
`#818cf8` glow, 12px rounded corners, circular pulse-glow status motif);
icon design constraints (single concept, 2-3 colors, legible at 16-32px,
platform mask safety); three grounded visual concept directions
(Convergent Cursor, Mux Node, Pulse Bracket); a prompting guide for
ChatGPT's current image model (GPT Image 2 / "ChatGPT Images 2.0", current
as of 2026-08-06 — GPT-4o image gen and DALL·E are both superseded/being
retired); two ready-to-use prompts; and the implementation plan (`npx tauri
icon <source.png>` regenerates every platform size from one 1024x1024
transparent PNG, matching filenames already referenced in
`tauri.conf.json`).

**Deferred:** actual icon file swap — waiting on the user to generate and
provide the image via ChatGPT, per the task's explicit two-step framing.

**Correction during research:** first pass cited GPT-4o's native image
generation as ChatGPT's current model; user caught this as stale (dated
2026-08-06). Re-searched and confirmed GPT Image 2 ("ChatGPT Images 2.0")
replaced it in April 2026, with legacy DALL·E fully retiring from ChatGPT
2026-08-30. Corrected the spec's prompting section before finalizing.
