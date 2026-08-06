---
quick_id: 260806-jx4
status: complete
---

# Quick Task 260806-jx4: AutoMux app icon ChatGPT generation spec

AutoMux currently ships the stock default Tauri icon (unbranded orange/teal
figure-8), unrelated to the app's actual dark/indigo, glow-driven UI. Create
a self-contained spec document the user can hand to ChatGPT to generate a
replacement icon that matches AutoMux's real design language, then implement
the result once provided.

## Tasks

1. Extract the app's actual design language from source (`src/App.css`
   theme tokens, shape/motif conventions in `App.tsx`) rather than guessing.
2. Research current (August 2026) best practices for prompting ChatGPT's
   image generator specifically for icon/logo work — model identity,
   prompt structure, transparency handling, iteration strategy.
3. Write `docs/ICON-SPEC.md`: app identity brief, extracted design tokens,
   icon design constraints, visual concept directions grounded in existing
   UI motifs, prompting guide, ready-to-use prompts, and the implementation
   plan for swapping in the result (`npx tauri icon <source>`).
4. Do NOT implement the icon swap yet — the user will provide the
   ChatGPT-generated image in a follow-up turn.

## Notes

Mid-task correction: initial research cited GPT-4o's native image
generation as ChatGPT's current model. The user flagged this as outdated
(2026-08-06) — GPT-4o image gen was superseded by GPT Image 2 / "ChatGPT
Images 2.0" in April 2026, and DALL·E is being fully retired from ChatGPT
on 2026-08-30. Re-researched and corrected the spec's prompting-guide
section accordingly before finalizing.
