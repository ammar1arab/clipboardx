import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import { ArrowLeft, Keyboard } from "lucide-react";
import { errorMessage } from "../history/useHistory";
import { AppQuery } from "../query";

type Preferences = { captureImages: boolean; pasteSeparator: string; windowWidth: number; windowHeight: number };
type Retention = { maxItems: number; maxAgeDays: number };
type Loaded = { preferences: Preferences; retention: Retention; autostart: boolean };

const shortcuts = [
  ["Super+V", "Open or close"],
  ["↑ ↓", "Move"],
  ["Enter", "Paste"],
  ["Ctrl+Enter", "Paste selection"],
  ["Ctrl+A", "Select visible"],
  ["Ctrl+Click", "Toggle one"],
  ["Shift+Click", "Select range"],
  ["Delete", "Remove"],
  ["F2", "Edit text"],
  ["Ctrl+P", "Pin"],
  ["Esc", "Clear, then close"],
  ["Type", "Search"],
  ["Drag", "Drop text or image"],
];

function SettingsForm({ close }: { close: () => void }) {
  const loadedQuery = useQuery({
    queryKey: ["settings"],
    queryFn: async (): Promise<Loaded> => {
      const [preferences, retention, autostart] = await Promise.all([
        invoke<Preferences>("get_settings"),
        invoke<Retention>("get_retention"),
        isEnabled(),
      ]);
      return { preferences, retention, autostart };
    },
  });
  const [draft, setDraft] = useState<Loaded>();
  const [seen, setSeen] = useState<Loaded>();
  if (loadedQuery.data && loadedQuery.data !== seen) { setSeen(loadedQuery.data); setDraft(loadedQuery.data); }
  const operationRunning = useRef(false);
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string>();
  const [confirmClear, setConfirmClear] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const shortcutBox = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!shortcutsOpen) return;
    const closeOnOutside = (event: PointerEvent) => { if (!shortcutBox.current?.contains(event.target as Node)) setShortcutsOpen(false); };
    document.addEventListener("pointerdown", closeOnOutside);
    return () => document.removeEventListener("pointerdown", closeOnOutside);
  }, [shortcutsOpen]);
  const preferences = draft?.preferences;
  const retention = draft?.retention;
  const autostart = draft?.autostart ?? false;
  const loaded = Boolean(preferences && retention);
  const error = actionError ?? (loadedQuery.error ? errorMessage(loadedQuery.error) : undefined);
  async function run(action: () => Promise<void>) {
    if (operationRunning.current) return;
    operationRunning.current = true;
    setBusy(true); setActionError(undefined);
    try { await action(); } catch (failure) { setActionError(errorMessage(failure)); } finally { operationRunning.current = false; setBusy(false); }
  }
  return <main className="workspace" onKeyDown={(event) => {
    if (event.key !== "Escape") return;
    if (shortcutsOpen) { setShortcutsOpen(false); return; }
    close();
  }}>
    <header className="titlebar">
      <button autoFocus className="icon-button" aria-label="Back to history" onClick={close}><ArrowLeft size={18} /></button>
      <strong className="brand">Settings</strong>
      <div className="shortcut-anchor" ref={shortcutBox}>
        <button className="icon-button" type="button" aria-label="Shortcuts" aria-expanded={shortcutsOpen} onClick={() => setShortcutsOpen(!shortcutsOpen)}><Keyboard size={16} /></button>
        {shortcutsOpen && <div className="shortcut-popover" role="dialog" aria-label="Shortcuts">
          <ul>{shortcuts.map(([keys, action]) => <li key={keys}><kbd>{keys}</kbd><span>{action}</span></li>)}</ul>
          <p>Pinned entries stay when history is trimmed. On COSMIC, set Super+V in System Settings → Keyboard → Custom Shortcuts to the command clipboardx.</p>
        </div>}
      </div>
    </header>
    {error && <div className="error-message" role="alert">{error}{!loaded && <button onClick={() => void loadedQuery.refetch()}>Retry</button>}</div>}
    {!loaded ? !error && <div role="status">Loading settings…</div> : preferences && retention && <form className="settings-form" onSubmit={(event) => {
      event.preventDefault(); void run(async () => { await invoke("set_retention", { retention }); await invoke("set_settings", { settings: preferences }); });
    }}>
      <div className="settings-grid">
        <label>Width<input type="number" min={360} max={4096} required value={preferences.windowWidth} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, windowWidth: Number(event.target.value) }, retention, autostart })} /></label>
        <label>Height<input type="number" min={360} max={4096} required value={preferences.windowHeight} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, windowHeight: Number(event.target.value) }, retention, autostart })} /></label>
        <label>History limit<input type="number" min={1} max={10000} required value={retention.maxItems} onChange={(event) => setDraft({ ...draft!, retention: { ...retention, maxItems: Number(event.target.value) }, preferences, autostart })} /></label>
        <label>Keep for days<input type="number" min={1} max={3650} required value={retention.maxAgeDays} onChange={(event) => setDraft({ ...draft!, retention: { ...retention, maxAgeDays: Number(event.target.value) }, preferences, autostart })} /></label>
        <label className="setting-toggle settings-span"><span>Capture images</span><input type="checkbox" checked={preferences.captureImages} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, captureImages: event.target.checked }, retention, autostart })} /></label>
        <label className="setting-toggle settings-span"><span>Start at login</span><input type="checkbox" disabled={busy} checked={autostart} onChange={() => void run(async () => { if (autostart) await disable(); else await enable(); setDraft({ ...draft!, preferences, retention, autostart: await isEnabled() }); })} /></label>
        <label className="settings-span">Paste separator<input maxLength={1024} value={preferences.pasteSeparator} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, pasteSeparator: event.target.value }, retention, autostart })} /></label>
      </div>
      <div className="settings-footer">
        <div className="settings-actions">
          <button disabled={busy} type="submit">Save settings</button>
          <button className="quit-button" disabled={busy} type="button" onClick={() => void run(async () => { await invoke("quit_app"); })}>Quit</button>
        </div>
        <div className="settings-actions">
          <button disabled={busy} type="button" onClick={() => void run(async () => { await invoke("clear_history", { includePinned: false }); })}>Clear unpinned</button>
          <button disabled={busy} type="button" onClick={() => setConfirmClear(true)}>Clear all history</button>
        </div>
        {confirmClear && <div className="confirm-clear" role="alert"><span>Delete pinned entries too?</span><button disabled={busy} type="button" onClick={() => void run(async () => { await invoke("clear_history", { includePinned: true }); setConfirmClear(false); })}>Delete all</button><button type="button" onClick={() => setConfirmClear(false)}>Cancel</button></div>}
      </div>
    </form>}
  </main>;
}

export function Settings({ close }: { close: () => void }) {
  return <AppQuery><SettingsForm close={close} /></AppQuery>;
}
