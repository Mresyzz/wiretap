# Palette UX & Accessibility Design Journal

## 2026-11-08 - Keyboard Navigation & Focus-Visible in API Stream List
**Learning:** For high-density developer tools like traffic inspectors, mouse-only click targets create friction and a11y violations. Adding `tabindex="0"`, keyboard Enter/Space triggers, and explicit `:focus-visible` styles enables seamless keyboard triage without redesigning DOM layouts.
**Action:** Always verify keyboard focusability on virtualized/dynamic lists and pair search boxes with standard `/` hotkeys.
