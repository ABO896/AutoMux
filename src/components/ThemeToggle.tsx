import type { Component } from "solid-js";
import type { ThemePreference } from "../theme";

// D-06/D-07: 3-state theme cycle control anchored at the sidebar footer.
// Signals-down / accessor-props pattern — read via props.preference() inline,
// never destructured, matching Sidebar.tsx and RESEARCH.md Pattern 1.
interface ThemeToggleProps {
  preference: () => ThemePreference;
  onCycle: () => void;
}

const GLYPHS: Record<ThemePreference, string> = {
  system: "🖥",
  light: "☀",
  dark: "🌙",
};

// The aria-label names the NEXT action (what clicking will switch to), not
// the current state — a title tooltip alone is insufficient for a control
// whose meaning changes across the 3-state cycle (UI-SPEC Accessibility).
const NEXT_LABEL: Record<ThemePreference, string> = {
  system: "Switch to light theme",
  light: "Switch to dark theme",
  dark: "Switch to system theme",
};

const ThemeToggle: Component<ThemeToggleProps> = (props) => {
  return (
    <div
      class="flex flex-col items-center gap-1 px-2 py-3 rounded-lg cursor-pointer text-text-muted hover:text-text-main transition-colors w-16"
      onClick={() => props.onCycle()}
      title={props.preference().charAt(0).toUpperCase() + props.preference().slice(1)}
      aria-label={NEXT_LABEL[props.preference()]}
      role="button"
    >
      <span class="text-base">{GLYPHS[props.preference()]}</span>
    </div>
  );
};

export default ThemeToggle;
