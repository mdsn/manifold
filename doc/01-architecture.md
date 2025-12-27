# Architecture Notes: Manifold

Terminal-based “super pager” for Unix manual pages.

## Portability Goal

Must work on:
- Linux (common distros)
- macOS
- Local terminal, over SSH, inside tmux

Assume a “normal” terminal that supports standard ANSI escapes. Avoid Linux-only syscalls.

## Core Design Choice

We implement our own pager/rendering loop (not embedding `less`) to enable future features:
- hyperlinking / "SEE ALSO" navigation
- structured extraction (examples, flags, sections)
- richer UI affordances (e.g. moving through "links" in the style of TUI web browsers)

`less`-mode could be added later as a backend option. For now it is out of scope.

## Workspace and Crates

We have one crate for the main binary and multiple library crates for each logical module. The project is a Cargo Workspace. All crates are in the `crates` directory:

- `crates/app/`
  - `App`: owns global state, tab list, active tab index, command palette/search mode.
  - `Action`: enum of user intents (Open, CloseTab, NextTab, ScrollDown, Search, etc.)
- `crates/ui/`
  - Stateless rendering: draw tabs, viewport, status bar.
  - No business logic.
- `crates/input/`
  - Key mapping -> `Action`
  - Mode-aware (normal vs search prompt vs command palette)
- `crates/man/`
  - `Man`: a single man page tab state
    - identity: `{ name, section? }`
    - `scroll: usize` (line index)
    - `search_state`
    - `render_cache: RenderCache`
- `crates/render/`
  - `ManRenderer`: produces formatted text for a given `(name, section, width, theme?)`
  - Initially: shell out to system `man` with a stable output mode
- `crates/platform/` (optional)
  - glue for signals, resize events, environment probing

## Data Flow

Terminal events -> `input` -> `Action` -> `App::update(action)` mutates state -> `ui::draw(frame, app_state)`.

On resize:
- UI receives resize event
- App marks all tabs “dirty”
- Renderer reruns for active tab immediately; others lazily on activation

## Rendering Strategy (important)

We need reflow on resize. Therefore:
- Store the *source identity* of the man page (name/section), not only final wrapped text.
- Cache rendered output per width:
  - `RenderCache { width: u16, lines: Vec<String> }`
- When width changes, regenerate `lines`, then clamp/translate scroll:
  - minimal approach: clamp to `min(old_scroll, new_lines.len())`
  - better later: map by “anchor” (e.g., keep heading/roff offset) — deferred

## How to Invoke `man`

We want deterministic, plain text output.

Options (choose one early, but keep behind `ManRenderer` trait):
- `man -P cat <name>` (but may bypass formatting or behave differently)
- `MANWIDTH=<w> man <name> | col -bx` (classic pipeline)
- `man <name> | ul -t ...` (avoid; terminal-dependent)

Goal: output plain UTF-8 lines without backspaces/overstrikes.

Note: macOS and Linux differ. Keep this logic tested behind an integration-style harness and allow configuration.

## UI Library Choice

Use:
- `ratatui` for layout + rendering
- `crossterm` for terminal backend + input events + resize handling

Avoid dependencies that require terminfo complexity unless necessary.

## Milestones

### M1: Skeleton + One Document
- app loop, ratatui frame
- open a fixed man page at startup, `open(2)`
- scrolling works
- resize triggers rerender and redraw

### M2: Tabs
- open/close tabs
- switch tabs, preserve scroll per tab
- lazy render non-active tabs

### M3: Search
- `/` enters search prompt
- find next match, highlight current match

### M4: “SEE ALSO” navigation (first “rich” feature)
- parse SEE ALSO section heuristically
- allow “open link under cursor” (basic)
