export const shortcuts: [string, string][] = [
  ["Super+V", "Open or close"],
  ["↑ ↓", "Move"],
  ["Shift+↑↓", "Extend selection"],
  ["Home / End", "Jump to end"],
  ["Shift+Home/End", "Extend to end"],
  ["Enter", "Paste selection or active"],
  ["Ctrl+Enter", "Paste selection"],
  ["Ctrl+A", "Select visible"],
  ["Ctrl+Click", "Toggle one"],
  ["Shift+Click", "Select range"],
  ["Space", "Toggle active"],
  ["Delete", "Remove"],
  ["F2", "Edit text"],
  ["Ctrl+P", "Pin"],
  ["Alt+1 / Alt+2", "Images / text"],
  ["Ctrl+/", "Shortcuts"],
  ["Esc", "Clear, then close"],
  ["Type", "Search"],
  ["Drag", "Drop text or image"],
];

export function ShortcutList() {
  return <>
    <ul>{shortcuts.map(([keys, action]) => <li key={keys}><kbd>{keys}</kbd><span>{action}</span></li>)}</ul>
    <p>Pinned entries stay when history is trimmed. On COSMIC, set Super+V in System Settings → Keyboard → Custom Shortcuts to the command clipboardx.</p>
  </>;
}
