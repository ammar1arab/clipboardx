# ClipboardX Project Profile

## Product

ClipboardX is a fast, keyboard-first desktop clipboard manager.

Primary development platform:
- Pop!_OS 24.04
- COSMIC
- Wayland

Future platform:
- Windows

## Core features

- Super+V global launcher
- Text clipboard history
- Image clipboard history with thumbnails
- Search
- Click item to paste immediately
- Enter to paste selected item
- Arrow-key navigation
- Delete item
- F2 edit text
- Ctrl+A select all
- Ctrl+Click multi-select
- Shift+Click range-select
- Ctrl+Enter paste selected items
- Pin/unpin
- Persistent clipboard history
- Duplicate prevention
- Configurable retention
- Autostart
- Fast startup
- Native desktop behavior

## Architecture

Frontend:
- React
- TypeScript
- Vite
- Tailwind CSS
- Lucide icons

Desktop/backend:
- Tauri 2
- Rust

Persistence:
- SQLite

React owns:
- rendering
- search
- selection
- keyboard UI state
- editing UI

Rust owns:
- clipboard monitoring
- native clipboard operations
- paste behavior
- window/native integration
- persistence boundary
- filesystem/app-data operations

## UI

The UI should feel like a polished native desktop utility.

Design direction:
- compact
- Apple-quality visual polish
- subtle translucency where appropriate
- rounded surfaces
- restrained animations
- excellent spacing
- strong keyboard support
- clear focus states
- no dashboard-style UI
- no unnecessary visual clutter

Use one coherent design-token system.

## Data

Clipboard contents are private local user data.

- Never send clipboard data to external services.
- Avoid logging clipboard contents.
- Validate native payloads at Rust boundaries.
- Store image files in app-data storage.
- Store image metadata and text history in SQLite.
- Do not keep large image blobs in React state.

## Engineering rules

- Inspect before editing.
- Reuse before creating.
- Keep changes scoped.
- Do not introduce dependencies without a concrete need.
- Do not claim verification that was not actually run.
- Preserve unrelated working code.

## Commands

Frontend build:

npm run build

Development:

npm run tauri dev

Rust validation:

cd src-tauri
cargo fmt --check
cargo check
cargo clippy -- -D warnings
