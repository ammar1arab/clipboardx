import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import { ArrowLeft } from "lucide-react";
import { errorMessage } from "../history/useHistory";

type Preferences = { captureImages: boolean; pasteSeparator: string };
type Retention = { maxItems: number; maxAgeDays: number };

export function Settings({ close }: { close: () => void }) {
  const [preferences, setPreferences] = useState<Preferences>();
  const [retention, setRetention] = useState<Retention>();
  const [autostart, setAutostart] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const [confirmClear, setConfirmClear] = useState(false);
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    let alive = true;
    void Promise.all([invoke<Preferences>("get_settings"), invoke<Retention>("get_retention"), isEnabled()])
      .then(([settings, limits, enabled]) => { if (alive) { setPreferences(settings); setRetention(limits); setAutostart(enabled); } })
      .catch((failure) => { if (alive) setError(errorMessage(failure)); });
    return () => { alive = false; };
  }, []);
  async function run(action: () => Promise<void>) {
    if (busy) return;
    setBusy(true); setError(undefined); setSaved(false);
    try { await action(); } catch (failure) { setError(errorMessage(failure)); } finally { setBusy(false); }
  }
  return <main className="workspace" onKeyDown={(event) => { if (event.key === "Escape") close(); }}>
    <header className="titlebar"><button autoFocus className="icon-button" aria-label="Back to history" onClick={close}><ArrowLeft size={18} /></button><strong>Settings</strong><span className="privacy-label">Stored locally</span></header>
    {error && <div className="error-message" role="alert">{error}</div>}
    {!preferences || !retention ? <div role="status">Loading settings…</div> : <form className="settings-form history-scroll" onSubmit={(event) => {
      event.preventDefault(); void run(async () => { await invoke("set_retention", { retention }); await invoke("set_settings", { settings: preferences }); setSaved(true); });
    }}>
      <label>History limit<input type="number" min={1} max={10000} required value={retention.maxItems} onChange={(event) => setRetention({ ...retention, maxItems: Number(event.target.value) })} /></label>
      <label>Retention days<input type="number" min={1} max={3650} required value={retention.maxAgeDays} onChange={(event) => setRetention({ ...retention, maxAgeDays: Number(event.target.value) })} /></label>
      <label className="setting-toggle">Capture images<input type="checkbox" checked={preferences.captureImages} onChange={(event) => setPreferences({ ...preferences, captureImages: event.target.checked })} /></label>
      <label>Multi-paste separator<textarea maxLength={1024} rows={2} value={preferences.pasteSeparator} onChange={(event) => setPreferences({ ...preferences, pasteSeparator: event.target.value })} /></label>
      <label className="setting-toggle">Start at login<input type="checkbox" disabled={busy} checked={autostart} onChange={() => void run(async () => { if (autostart) await disable(); else await enable(); setAutostart(await isEnabled()); })} /></label>
      <p className="settings-hint">Super+V opens history. On COSMIC, change the binding in System Settings → Keyboard → Custom Shortcuts. Pinned entries survive retention.</p>
      <button disabled={busy} type="submit">Save settings</button>{saved && <span role="status">Saved</span>}
      <div className="settings-danger"><button disabled={busy} type="button" onClick={() => void run(async () => { await invoke("clear_history", { includePinned: false }); setSaved(true); })}>Clear unpinned</button>
        <button disabled={busy} type="button" onClick={() => setConfirmClear(true)}>Clear all history</button></div>
      {confirmClear && <div role="alert"><p>Delete all history, including pinned entries?</p><button disabled={busy} type="button" onClick={() => void run(async () => { await invoke("clear_history", { includePinned: true }); setConfirmClear(false); setSaved(true); })}>Delete all</button> <button type="button" onClick={() => setConfirmClear(false)}>Cancel</button></div>}
    </form>}
    <button className="quit-button" disabled={busy} onClick={() => void run(async () => { await invoke("quit_app"); })}>Quit ClipboardX</button>
  </main>;
}
