# ClipboardX

A small, private clipboard history for the COSMIC desktop. Press Super+V, find what you copied, and paste it back into the app you were using. Text and images stay on this computer.

## Install

Open [the download page](https://ammar1arab.github.io/clipboardx/) and run the file it gives you.

Windows opens a setup window. Choose Next, then Install. Mac opens a window where you drag ClipboardX into Applications. Linux downloads a `.deb` package. On COSMIC, open ClipboardX once, turn on **Start at login** in the gear, and bind Super+V to the command `clipboardx` under Settings → Keyboard → Custom Shortcuts.

Closing the window hides it and keeps capture running. **Quit ClipboardX** in Settings is what stops it.

## Shortcuts

| Key | Action |
| --- | --- |
| Super+V | Open or close |
| ↑ ↓ | Move |
| Shift+↑ ↓ | Extend the selection |
| Home / End | Jump to an end |
| Enter | Paste the selection, or the active entry |
| Ctrl+Enter | Paste the selection |
| Ctrl+A | Select visible entries |
| Ctrl+Click / Shift+Click | Toggle one / select a range |
| Space | Toggle the active entry |
| Delete | Remove |
| F2 | Edit text |
| Ctrl+P | Pin or unpin |
| Alt+1 / Alt+2 | Filter images / text |
| Ctrl+/ | Show shortcuts |
| Esc | Clear selection or search, then close |
| Type | Search |
| Drag | Drop the text or image into another app |

## Develop

```bash
npm install
npm run tauri dev
```

```bash
npm test
npm run build
cd src-tauri && cargo fmt --check && cargo check && cargo clippy -- -D warnings && cargo test
```

The development window loads from `localhost:1420`. Use `npm run tauri dev` for that. The installed package does not need it.

## Stack

React and TypeScript for the popup. TanStack Query for history and settings. Tailwind CSS for the theme. Tauri 2 and Rust for the clipboard, SQLite history, and COSMIC paste.
