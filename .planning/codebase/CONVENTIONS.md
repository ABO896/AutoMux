# Coding Conventions

**Analysis Date:** 2026-05-15

## Overview

This is a dual-language codebase: **Rust** (backend/Tauri) in `src-tauri/src/` and
**TypeScript + SolidJS** (frontend) in `src/`. Conventions differ by language.

---

## Rust Conventions (`src-tauri/src/`)

### Naming Patterns

**Files/Modules:**
- `snake_case` for all module files: `persistence.rs`, `scheduler/mod.rs`, `state/mod.rs`
- Platform-specific code lives in sub-modules: `platform/macos/`, `platform/windows/`

**Types/Enums/Traits:**
- `PascalCase` for structs, enums, and traits: `MacroConfig`, `ActionStep`, `InputProvider`, `StateActor`
- Enum variants: `PascalCase` — `TriggerMode::Pulse`, `TriggerMode::Hold`, `ActionType::HoldStart`

**Functions/Methods:**
- `snake_case` for all functions: `handle_intent`, `start_macro`, `inject_input`, `sanitize_name`

**Variables/Fields:**
- `snake_case` for all local vars and struct fields: `interval_ms`, `macro_id`, `action_tx`

**Constants/Globals:**
- `UPPER_SNAKE_CASE` for statics/constants (pattern from observer.rs)

### Module Organization

**Module pattern:** Single `mod.rs` per subsystem directory.
```
src-tauri/src/
├── lib.rs           # App bootstrap — registers Tauri plugins, channels, IPC handlers
├── main.rs          # Thin entry point: calls lib::run()
├── ipc/mod.rs       # All #[command] handlers — one file, grouped by subsystem
├── state/mod.rs     # AppState, StateActor, Intent enum, MacroConfig types
├── scheduler/mod.rs # Scheduler, SchedulerIntent, ActionReady, timer logic
├── persistence.rs   # ProfileManager, ProfileData, ProfileSummary
└── platform/
    ├── mod.rs       # Shared traits: InputProvider, PlatformObserver, MouseButton
    ├── macos/       # macOS-specific: input.rs, observer.rs, mod.rs
    └── windows/     # Windows-specific: mod.rs
```

**Conditional compilation:** Platform-specific code uses `#[cfg(target_os = "macos")]` and
`#[cfg(target_os = "windows")]` gates. Never put platform logic in shared modules.

### Error Handling

**Pattern:** All IPC commands return `Result<T, String>`. Errors are converted to strings
with `.map_err(|e| e.to_string())` or `format!("context: {}", e)`.

```rust
// Standard IPC error pattern (src-tauri/src/ipc/mod.rs):
state
    .send_intent(Intent::AddMacro(config))
    .await
    .map_err(|e| e.to_string())

// Context-preserving error:
serde_json::to_string_pretty(profile)
    .map_err(|e| format!("Failed to serialize profile: {}", e))
```

**Internally:** Use `let _ = sender.send(x).await;` to silently discard non-critical
channel send results (fire-and-forget intents). Reserve `?` propagation for critical paths.

**Debug logging:** Use `#[cfg(debug_assertions)]` + `eprintln!("[Module] message")` for
debug-only traces. Never use `println!` or a logging crate. Format:
```rust
#[cfg(debug_assertions)]
eprintln!("[Persistence] Saved profile '{}' → {:?}", profile.name, path);
```

### Documentation Comments

**Module-level docs:** Use `//!` (inner doc) at the top of each file with:
- `@architect:` — structural intent
- `@performance-tuner:` — perf constraints or notes
- `@safety-officer:` — safety-critical invariants

```rust
//! @architect: Saves/loads named JSON profiles to/from the user's
//! platform-specific app data directory.
//! @performance-tuner: All I/O is async via `tokio::fs`.
```

**Item-level docs:** Use `///` for public structs, methods, and IPC commands. Include
parameter meaning, side effects, and safety notes.

**Inline comments:** Use `//` for step-by-step reasoning in complex logic. Use
`// ── Section Name ───` separators for visual sectioning within long files.

**Role annotations in comments:**
- `// @safety-officer:` — marks invariants that must not be broken
- `// @scheduler-agent:` — marks scheduling constraints
- These are convention, not tooling — they are searched by code review.

### Derives

**Data types always derive:** `Debug, Clone, Serialize, Deserialize` for any type that
crosses the IPC boundary or is persisted.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroConfig { ... }
```

**Copy types:** Add `Copy` + `Eq` + `Hash` for small enum/struct types used as map keys:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputEvent { ... }
```

### Async Design

- All IPC handlers are `async` functions.
- Internal business logic (StateActor, Scheduler) runs as single long-lived Tokio tasks
  spawned with `tauri::async_runtime::spawn`.
- No per-macro spawns. The Scheduler is a single task with a `tokio::select!` loop.
- Channel backpressure: `try_send` for fire-and-forget from timer hot paths; `.await`
  sends for control-path intents.

---

## TypeScript / SolidJS Conventions (`src/`)

### Naming Patterns

**Files:**
- `PascalCase.tsx` for components: `App.tsx`
- `camelCase.ts` for non-component modules (if any)

**Types/Interfaces:**
- `PascalCase` for interfaces and type aliases: `MacroConfig`, `AppState`, `TriggerMode`
- Union type aliases for string literals: `type Tab = "dashboard" | "profiles";`
  `type TriggerMode = "Pulse" | "Hold";`

**Functions:**
- `camelCase` for helpers: `formatInputEvent`, `formatStep`
- `camelCase` prefixed with `handle` for event handlers: `handleCreateMacro`,
  `handleToggleMacro`, `handleSaveProfile`
- `camelCase` prefixed with `refresh` for data-refresh helpers: `refreshProfiles`

**Signals:**
- Signal getter functions follow SolidJS convention — `camelCase()` to read,
  `setCamelCase(value)` to write.
- Boolean signals use positive framing: `showNewMacro`, `profileLoading`, `engineActive`

### SolidJS Patterns

**Reactivity:**
- `createSignal` for all local reactive state.
- `createEffect` for side effects (data fetching, event listeners, polling).
- `onCleanup` always called inside `createEffect` for subscriptions and intervals.
- `Show` for conditional rendering (never ternary JSX with non-trivial subtrees).
- `For` for list rendering over arrays.

```tsx
// Listener pattern (src/App.tsx):
createEffect(() => {
  const unlisten = listen<AppState>("state-changed", (event) => {
    setState(event.payload);
  });
  onCleanup(() => {
    unlisten.then((fn) => fn());
  });
});
```

**Derived values:**
- Non-signal derived state uses plain functions (not `createMemo`):
```tsx
const macroList = () => {
  const s = state();
  if (!s) return [];
  return Object.values(s.macros);
};
const engineActive = () => state()?.engine_active ?? false;
```

### Import Organization

**Order:**
1. SolidJS primitives: `import { createSignal, createEffect, ... } from "solid-js"`
2. Tauri API: `import { invoke } from "@tauri-apps/api/core"`
3. Local CSS: `import "./App.css"`

No path aliases used — all imports are bare module names or relative.

### Error Handling (TypeScript)

**Pattern:** All Tauri `invoke` calls are wrapped in `try/catch`. Errors are logged with
`console.error` and a descriptive label. No user-visible error propagation from failed
invocations (silent degradation).

```tsx
try {
  await invoke("toggle_engine");
} catch (e) {
  console.error("Toggle engine failed:", e);
}
```

**User-visible errors:** Use message state with a type discriminant and auto-dismiss timer:
```tsx
const [profileMessage, setProfileMessage] = createSignal<{
  text: string;
  type: "success" | "error";
} | null>(null);

function showProfileMsg(text: string, type: "success" | "error") {
  setProfileMessage({ text, type });
  setTimeout(() => setProfileMessage(null), 3000);
}
```

### Styling

**Framework:** Tailwind CSS v4 (via `@tailwindcss/vite` plugin — no PostCSS config).
- All styling via inline `class=` Tailwind utility strings.
- No `className` (SolidJS uses `class`).
- Custom design tokens referenced with CSS variable syntax: `bg-accent`,
  `text-text-muted`, `border-border`, `shadow-[0_0_8px_var(--color-accent-glow)]`.
- Conditional class strings use template literals: `` `flex-1 ${condition ? "a" : "b"}` ``

**Component IDs:** Interactive elements have explicit `id` attributes using kebab-case:
`id="btn-create-macro"`, `id="input-macro-name"`, `id="tab-dashboard"`.
This convention supports automated testing and Tauri integration.

### TypeScript Config

- `strict: true` — all strict checks enabled.
- `noUnusedLocals: true`, `noUnusedParameters: true` — no dead code allowed.
- `noFallthroughCasesInSwitch: true`.
- `jsx: "preserve"`, `jsxImportSource: "solid-js"` — SolidJS JSX transform.
- Target: `ES2020`, module: `ESNext`.

---

## Cross-Language Conventions

### Type Mirroring

Rust types serialized via `serde` must have TypeScript mirrors in `src/App.tsx`.
The TypeScript type comments explicitly note the Rust correspondence:
```tsx
// Types (mirrors Rust state)
type TriggerMode = "Pulse" | "Hold";
interface MacroConfig { ... }
```

Rust enum variants serialize as their variant name strings (Serde default for unit
variants), so TypeScript unions use those exact strings.

### IPC Naming

- Rust `#[command]` function names use `snake_case`: `add_macro`, `set_macro_enabled`
- TypeScript `invoke` call strings use the same `snake_case` names verbatim

### Section Separators

Both languages use a consistent visual separator style for intra-file sections:

**Rust:** `// ── Section Name ─────────────────────────────`
**TypeScript:** `{/* ── Section Name ── */}` (JSX comments)

---

*Convention analysis: 2026-05-15*
