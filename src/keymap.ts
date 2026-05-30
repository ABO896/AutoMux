// Platform detection (synchronous, Tauri WebView safe).
// Used to select the correct lookup table at module scope.
const IS_MACOS = navigator.platform.toLowerCase().includes("mac");

// ── macOS CGKeyCode lookup tables ───────────────────────────────
// Maps KeyboardEvent.code (e.g. "KeyA", "Space") → CGKeyCode integer.
// Source: HIToolbox/Events.h (Apple SDK) — verified via phracker/MacOSX-SDKs.
// Keyed by e.code (layout-independent, unambiguous) to avoid the DOM keyCode
// F12/ArrowLeft collision (both share DOM keyCode 123 in some contexts).
const DOM_KEYCODE_TO_CGKEYCODE: Record<string, number> = {
  // Letters (CGKeyCode = ANSI physical position, not ASCII value)
  KeyA: 0,
  KeyS: 1,
  KeyD: 2,
  KeyF: 3,
  KeyH: 4,
  KeyG: 5,
  KeyZ: 6,
  KeyX: 7,
  KeyC: 8,
  KeyV: 9,
  KeyB: 11,
  KeyQ: 12,
  KeyW: 13,
  KeyE: 14,
  KeyR: 15,
  KeyY: 16,
  KeyT: 17,
  KeyO: 31,
  KeyU: 32,
  KeyI: 34,
  KeyP: 35,
  KeyL: 37,
  KeyJ: 38,
  KeyK: 40,
  KeyN: 45,
  KeyM: 46,
  // Digits
  Digit1: 18,
  Digit2: 19,
  Digit3: 20,
  Digit4: 21,
  Digit6: 22,
  Digit5: 23,
  Digit9: 25,
  Digit7: 26,
  Digit8: 28,
  Digit0: 29,
  // Special keys
  Space: 49,
  Enter: 36,
  Tab: 48,
  Backspace: 51,
  Escape: 53,
  // Arrow keys
  ArrowLeft: 123,
  ArrowRight: 124,
  ArrowDown: 125,
  ArrowUp: 126,
  // Navigation
  Home: 115,
  End: 119,
  PageUp: 116,
  PageDown: 121,
  // Function keys
  F1: 122,
  F2: 120,
  F3: 99,
  F4: 118,
  F5: 96,
  F6: 97,
  F7: 98,
  F8: 100,
  F9: 101,
  F10: 109,
  F11: 103,
  F12: 111,
};

// Maps CGKeyCode integer → human-readable display name.
// Source: HIToolbox/Events.h (Apple SDK).
const CGKEYCODE_TO_NAME: Record<number, string> = {
  0: "A", 1: "S", 2: "D", 3: "F", 4: "H", 5: "G", 6: "Z", 7: "X",
  8: "C", 9: "V", 11: "B", 12: "Q", 13: "W", 14: "E", 15: "R",
  16: "Y", 17: "T", 18: "1", 19: "2", 20: "3", 21: "4", 22: "6",
  23: "5", 24: "=", 25: "9", 26: "7", 27: "-", 28: "8", 29: "0",
  30: "]", 31: "O", 32: "U", 33: "[", 34: "I", 35: "P", 36: "Return",
  37: "L", 38: "J", 39: "'", 40: "K", 41: ";", 42: "\\", 43: ",",
  44: "/", 45: "N", 46: "M", 47: ".", 48: "Tab", 49: "Space",
  50: "`", 51: "Delete", 53: "Escape",
  96: "F5", 97: "F6", 98: "F7", 99: "F3", 100: "F8", 101: "F9",
  103: "F11", 109: "F10", 111: "F12", 113: "F15", 114: "Help",
  115: "Home", 116: "Page Up", 117: "Fwd Delete", 118: "F4",
  119: "End", 120: "F2", 121: "Page Down", 122: "F1",
  123: "Left Arrow", 124: "Right Arrow", 125: "Down Arrow", 126: "Up Arrow",
  55: "Cmd", 56: "Shift", 57: "Caps Lock", 58: "Option", 59: "Control",
  60: "Right Shift", 61: "Right Option", 62: "Right Control",
};

// ── Windows VK lookup tables ────────────────────────────────────
// Maps KeyboardEvent.code (e.g. "KeyA", "Space") → Windows VK code integer.
// Source: Microsoft Learn — Virtual-Key Codes (Winuser.h).
// Keyed by e.code string for consistency with DOM_KEYCODE_TO_CGKEYCODE.
const DOM_KEYCODE_TO_VK: Record<string, number> = {
  // Letters (VK_A-VK_Z = 0x41-0x5A)
  KeyA: 0x41, KeyB: 0x42, KeyC: 0x43, KeyD: 0x44, KeyE: 0x45, KeyF: 0x46,
  KeyG: 0x47, KeyH: 0x48, KeyI: 0x49, KeyJ: 0x4a, KeyK: 0x4b, KeyL: 0x4c,
  KeyM: 0x4d, KeyN: 0x4e, KeyO: 0x4f, KeyP: 0x50, KeyQ: 0x51, KeyR: 0x52,
  KeyS: 0x53, KeyT: 0x54, KeyU: 0x55, KeyV: 0x56, KeyW: 0x57, KeyX: 0x58,
  KeyY: 0x59, KeyZ: 0x5a,
  // Digits (VK 0x30-0x39)
  Digit0: 0x30, Digit1: 0x31, Digit2: 0x32, Digit3: 0x33, Digit4: 0x34,
  Digit5: 0x35, Digit6: 0x36, Digit7: 0x37, Digit8: 0x38, Digit9: 0x39,
  // Special keys
  Space: 0x20,
  Enter: 0x0d,
  Tab: 0x09,
  Backspace: 0x08,
  Escape: 0x1b,
  // Arrow keys
  ArrowLeft: 0x25,
  ArrowUp: 0x26,
  ArrowRight: 0x27,
  ArrowDown: 0x28,
  // Navigation
  Home: 0x24,
  End: 0x23,
  PageUp: 0x21,
  PageDown: 0x22,
  Insert: 0x2d,
  Delete: 0x2e,
  // Function keys
  F1: 0x70, F2: 0x71, F3: 0x72, F4: 0x73, F5: 0x74, F6: 0x75,
  F7: 0x76, F8: 0x77, F9: 0x78, F10: 0x79, F11: 0x7a, F12: 0x7b,
  // Numpad
  Numpad0: 0x60, Numpad1: 0x61, Numpad2: 0x62, Numpad3: 0x63, Numpad4: 0x64,
  Numpad5: 0x65, Numpad6: 0x66, Numpad7: 0x67, Numpad8: 0x68, Numpad9: 0x69,
  // Modifiers
  ShiftLeft: 0x10, ShiftRight: 0x10,
  ControlLeft: 0x11, ControlRight: 0x11,
  AltLeft: 0x12, AltRight: 0x12,
  CapsLock: 0x14,
  MetaLeft: 0x5b, MetaRight: 0x5c,
};

// Maps Windows VK code integer → human-readable display name.
// Source: Microsoft Learn — Virtual-Key Codes (Winuser.h).
const VK_TO_NAME: Record<number, string> = {
  0x41: "A", 0x42: "B", 0x43: "C", 0x44: "D", 0x45: "E", 0x46: "F",
  0x47: "G", 0x48: "H", 0x49: "I", 0x4a: "J", 0x4b: "K", 0x4c: "L",
  0x4d: "M", 0x4e: "N", 0x4f: "O", 0x50: "P", 0x51: "Q", 0x52: "R",
  0x53: "S", 0x54: "T", 0x55: "U", 0x56: "V", 0x57: "W", 0x58: "X",
  0x59: "Y", 0x5a: "Z",
  0x30: "0", 0x31: "1", 0x32: "2", 0x33: "3", 0x34: "4",
  0x35: "5", 0x36: "6", 0x37: "7", 0x38: "8", 0x39: "9",
  0x20: "Space", 0x0d: "Return", 0x09: "Tab", 0x08: "Backspace",
  0x1b: "Escape", 0x25: "Left Arrow", 0x26: "Up Arrow",
  0x27: "Right Arrow", 0x28: "Down Arrow",
  0x24: "Home", 0x23: "End", 0x21: "Page Up", 0x22: "Page Down",
  0x2d: "Insert", 0x2e: "Delete",
  0x70: "F1", 0x71: "F2", 0x72: "F3", 0x73: "F4", 0x74: "F5",
  0x75: "F6", 0x76: "F7", 0x77: "F8", 0x78: "F9", 0x79: "F10",
  0x7a: "F11", 0x7b: "F12",
  0x60: "Num 0", 0x61: "Num 1", 0x62: "Num 2", 0x63: "Num 3",
  0x64: "Num 4", 0x65: "Num 5", 0x66: "Num 6", 0x67: "Num 7",
  0x68: "Num 8", 0x69: "Num 9",
  0x10: "Shift", 0x11: "Ctrl", 0x12: "Alt",
};

/**
 * Translate a KeyboardEvent.code string (e.g. "KeyA", "Space", "F5")
 * to the platform-native integer keycode.
 *
 * macOS: returns CGKeyCode (e.g. "KeyA" → 0, "Space" → 49)
 * Windows: returns VK code (e.g. "KeyA" → 0x41, "Space" → 0x20)
 *
 * Returns null if the code string has no entry in the lookup table.
 * Call sites must pass e.code, not e.keyCode.
 */
export function domKeycodeToNative(code: string): number | null {
  const map = IS_MACOS ? DOM_KEYCODE_TO_CGKEYCODE : DOM_KEYCODE_TO_VK;
  return map[code] ?? null;
}

/**
 * Resolve a platform-native integer keycode to a human-readable display name.
 *
 * macOS: looks up CGKeyCode in CGKEYCODE_TO_NAME
 * Windows: looks up VK code in VK_TO_NAME
 *
 * Falls back to "Key {nativeCode}" when no entry exists (D-03).
 */
export function resolveKeyName(nativeCode: number): string {
  const nameMap = IS_MACOS ? CGKEYCODE_TO_NAME : VK_TO_NAME;
  return nameMap[nativeCode] ?? `Key ${nativeCode}`;
}
