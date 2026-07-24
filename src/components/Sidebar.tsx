import type { Component, JSX } from "solid-js";
import type { Tab } from "../App";

// D-08: persistent 84px left sidebar rail, replaces the top tab bar.
// Signals-down / accessor-props pattern (RESEARCH.md Pattern 1) — props are
// read inline (props.activeTab()), never destructured, so SolidJS reactivity
// is preserved. The activeTab signal itself stays owned by App().
interface SidebarProps {
  activeTab: () => Tab;
  onSelectTab: (tab: Tab) => void;
  // Footer slot — the plan 10-03 Task 2 ThemeToggle control is mounted here
  // by App.tsx as children, keeping Sidebar decoupled from ThemeToggle's
  // implementation (Sidebar only owns the layout/spacer, not the control).
  children?: JSX.Element;
}

const Sidebar: Component<SidebarProps> = (props) => {
  return (
    <div class="w-[84px] shrink-0 flex flex-col items-center py-4 gap-1 border-r border-border sidebar-glass">
      <button
        class={`flex flex-col items-center gap-1 px-2 py-3 rounded-lg cursor-pointer transition-colors w-16 ${
          props.activeTab() === "dashboard"
            ? "bg-accent/10 text-accent"
            : "text-text-muted hover:text-text-main"
        }`}
        onClick={() => props.onSelectTab("dashboard")}
      >
        <span class="text-base">⚡</span>
        <span class="text-[11px] font-semibold">Macros</span>
      </button>
      <button
        class={`flex flex-col items-center gap-1 px-2 py-3 rounded-lg cursor-pointer transition-colors w-16 ${
          props.activeTab() === "profiles"
            ? "bg-accent/10 text-accent"
            : "text-text-muted hover:text-text-main"
        }`}
        onClick={() => props.onSelectTab("profiles")}
      >
        <span class="text-base">📁</span>
        <span class="text-[11px] font-semibold">Profiles</span>
      </button>

      <div class="flex-grow" />

      {props.children}
    </div>
  );
};

export default Sidebar;
