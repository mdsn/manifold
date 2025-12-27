# Manifold

## Overview

This project is a terminal-based “super pager” for Unix manual pages. Its goal is to make reading and cross-referencing multiple man pages fast and pleasant, particularly when navigating between related syscalls, commands, or concepts.

Unlike traditional usage of `man | less`, this tool allows multiple man pages to be open simultaneously in a tabbed interface, preserves scroll position per page, and properly reflows content when the terminal is resized.

The result should feel like a documentation browser embedded directly in the terminal.

---

## Motivation

Reading man pages often involves jumping back and forth between two or more entries (e.g. `open(2)`, `read(2)`, `fcntl(2)`), which is cumbersome with a single pager. Losing scroll position, reopening pages, and dealing with broken formatting after terminal resizes interrupts flow.

This tool aims to solve that by:
- Treating man pages as first-class documents
- Making navigation cheap
- Respecting terminal ergonomics

---

## Goals

- Open and view multiple man pages at once
- Switch instantly between them without losing position
- Maintain correct formatting when the terminal is resized
- Remain compatible with the system man page pipeline
- Stay fully terminal-native (no GUI, no browser)
- Portable. Works on Linux and macOS at least.

Non-goals:
- Rendering HTML or external documentation formats
- Replacing the `man` database or indexing system
- Acting as a full TUI editor or IDE

---

## Core Features

### Tabbed Interface

- Each man page is displayed in its own tab
- Tabs show the man page name (e.g. `open(2)`)
- Keyboard shortcuts to:
  - Open a new tab
  - Close the current tab
  - Switch between tabs (cycling or direct access)

### Persistent Scroll State

- Each tab maintains its own scroll offset
- Switching tabs restores the previous position exactly
- Searching within a page does not affect other tabs

### Pager Functionality

- Vertical scrolling
- Page up / page down
- Incremental search (similar to `less`)
- Jump to top / bottom

### Responsive Reflow

- On terminal resize, visible man pages are re-rendered
- Line wrapping adapts to the new terminal width
- No “janky” formatting or stale wrapping
- Reflow should preserve semantic position as much as possible

---

## Man Page Rendering Model

Man pages are not stored as plain text; they are typically written in roff/troff format and rendered by the `man` command using a preprocessing pipeline.

This tool should:
- Integrate with the system `man` pipeline rather than reimplement it
- Capture formatted output in a way that allows re-rendering
- Re-run formatting when terminal dimensions change
- Avoid relying on static, pre-wrapped text

Exact implementation details are intentionally deferred, but correctness and fidelity to `man` output are priorities.

---

## User Interface Sketch

    +--------------------------------------------------+
    | [1] ls(1)    [2] grep(1)    [3] open(2)          |
    +--------------------------------------------------+
    | (man page content here)                          |
    |                                                  |
    | / Search: _                                      |
    +--------------------------------------------------+

- Top bar: open tabs
- Main pane: man page content
- Bottom bar: commands / search / status feedback

---

## Technology Choices

### Language

- **Rust**
  - Strong fit for systems tooling
  - Good terminal ecosystem
  - Explicit control over subprocesses and I/O

### Terminal UI

Candidates:
- `ratatui` (or equivalent)
- `crossterm` for terminal control and events

The UI layer should be clearly separated from:
- Man page rendering
- Tab/document management

---

## High-Level Architecture (Draft)

- **Application Core**
  - Manages tabs
  - Tracks active document
  - Handles input dispatch

- **Document Model**
  - Represents a single man page
  - Stores scroll position and search state
  - Knows how to (re)render itself

- **Renderer**
  - Interfaces with the system `man` command
  - Produces formatted output based on terminal dimensions

- **UI Layer**
  - Draws tabs and content
  - Handles resize events
  - Delegates logic to the core

---

## Open Questions

- Best way to capture and re-run the `man` formatting pipeline
- How to preserve semantic position across reflows
- Whether to render eagerly or lazily on resize
- Keyboard shortcut conventions

These are intentionally left open for exploration during implementation.

---

## Success Criteria

- Reading multiple related man pages feels fluid
- Resizing the terminal never breaks formatting
- Navigation is fast and muscle-memory-friendly
- The tool is something the author actually wants to use daily

