# ClipboardX

A small, private clipboard history for the COSMIC desktop. Press Super+V, find what you copied, and paste it back into the app you were using. Text and images stay on this computer.

## Install

Download `ClipboardX_0.1.0_amd64.deb` from the release build, then:

```bash
sudo apt install ./ClipboardX_0.1.0_amd64.deb wtype
```

Open ClipboardX once. In the gear, turn on **Start at login**. On COSMIC, bind Super+V to the command `clipboardx` under Settings → Keyboard → Custom Shortcuts.

Closing the window hides it and keeps capture running. **Quit ClipboardX** in Settings is what stops it.

## Shortcuts

| Key | Action |
| --- | --- |
| Super+V | Open or close |
| ↑ ↓ | Move |
| Enter | Paste |
| Ctrl+Enter | Paste the selection |
| Ctrl+A | Select visible entries |
| Ctrl+Click / Shift+Click | Toggle one / select a range |
| Delete | Remove |
| F2 | Edit text |
| Ctrl+P | Pin or unpin |
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
