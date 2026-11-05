import { useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import { ArrowLeft, Check, Keyboard } from "lucide-react";
import { normalizePreferences, type Preferences } from "../domain/preferences";
import { errorMessage } from "../history/useHistory";
import { AppQuery } from "../query";
import { ShortcutList } from "../shortcuts";

type Retention = { maxItems: number; maxAgeDays: number };
type Loaded = { preferences: Preferences; retention: Retention; autostart: boolean };

const separatorPresets = [
  ["blank", "Blank line", "\n\n"],
  ["line", "New line", "\n"],
  ["space", "Space", " "],
] as const;

function separatorPreset(value: string) {
  return separatorPresets.find(([, , preset]) => preset === value)?.[0] ?? "custom";
}

const settingsFallback: Loaded = {
  preferences: normalizePreferences(undefined),
  retention: { maxItems: 500, maxAgeDays: 30 },
  autostart: false,
};

function SettingsForm({ close }: { close: () => void }) {
  const loadedQuery = useQuery({
    queryKey: ["settings"],
    queryFn: async (): Promise<Loaded> => {
      const [preferences, retention, autostart] = await Promise.all([
        invoke<Partial<Preferences>>("get_settings"),
        invoke<Retention>("get_retention"),
        isEnabled(),
      ]);
      return { preferences: normalizePreferences(preferences), retention, autostart };
    },
    placeholderData: settingsFallback,
  });
  const [draft, setDraft] = useState<Loaded>(settingsFallback);
  const [seen, setSeen] = useState<Loaded>();
  const settled = loadedQuery.isSuccess && !loadedQuery.isPlaceholderData;
  if (settled && loadedQuery.data && loadedQuery.data !== seen) { setSeen(loadedQuery.data); setDraft(loadedQuery.data); }
  const operationRunning = useRef(false);
  const doneTimer = useRef<number | undefined>(undefined);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string>();
  const [actionError, setActionError] = useState<string>();
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const shortcutBox = useRef<HTMLDivElement>(null);
  function bindShortcuts(node: HTMLDivElement | null) {
    shortcutBox.current = node;
    if (!node || !shortcutsOpen) return;
    const closeOnOutside = (event: PointerEvent) => { if (!node.contains(event.target as Node)) setShortcutsOpen(false); };
    document.addEventListener("pointerdown", closeOnOutside);
    return () => document.removeEventListener("pointerdown", closeOnOutside);
  }
  const preferences = draft?.preferences;
  const retention = draft?.retention;
  const autostart = draft?.autostart ?? false;
  const error = actionError ?? (loadedQuery.error ? errorMessage(loadedQuery.error) : undefined);
  async function run(action: () => Promise<void>, doneId?: string) {
    if (operationRunning.current) return;
    operationRunning.current = true;
    setBusy(true); setActionError(undefined);
    try {
      await action();
      if (doneId) {
        setDone(doneId);
        window.clearTimeout(doneTimer.current);
        doneTimer.current = window.setTimeout(() => setDone((current) => current === doneId ? undefined : current), 700);
      }
    } catch (failure) { setActionError(errorMessage(failure)); } finally { operationRunning.current = false; setBusy(false); }
  }
  return <main className="workspace" onKeyDown={(event) => {
    if (event.key !== "Escape") return;
    if (shortcutsOpen) { setShortcutsOpen(false); return; }
    close();
  }}>
    <header className="titlebar">
      <button autoFocus className="icon-button" aria-label="Back to history" onClick={close}><ArrowLeft size={18} /></button>
      <strong className="brand">Settings</strong>
      <div className="shortcut-anchor" ref={bindShortcuts}>
        <button className="icon-button" type="button" aria-label="Shortcuts" aria-expanded={shortcutsOpen} onClick={() => setShortcutsOpen(!shortcutsOpen)}><Keyboard size={16} /></button>
        {shortcutsOpen && <div className="shortcut-popover" role="dialog" aria-label="Shortcuts"><ShortcutList /></div>}
      </div>
    </header>
    {error && <div className="error-message" role="alert">{error}{!loadedQuery.isSuccess && <button onClick={() => void loadedQuery.refetch()}>Retry</button>}</div>}
    {preferences && retention && <form className="settings-form" onSubmit={(event) => {
      event.preventDefault(); void run(async () => { await invoke("set_retention", { retention }); await invoke("set_settings", { settings: preferences }); }, "save");
    }}>
      <div className="settings-grid">
        <label>Width<input type="number" min={360} max={4096} required value={preferences.windowWidth} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, windowWidth: Number(event.target.value) }, retention, autostart })} /></label>
        <label>Height<input type="number" min={360} max={4096} required value={preferences.windowHeight} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, windowHeight: Number(event.target.value) }, retention, autostart })} /></label>
        <label>History limit<input type="number" min={1} max={10000} required value={retention.maxItems} onChange={(event) => setDraft({ ...draft!, retention: { ...retention, maxItems: Number(event.target.value) }, preferences, autostart })} /></label>
        <label>Keep for days<input type="number" min={1} max={3650} required value={retention.maxAgeDays} onChange={(event) => setDraft({ ...draft!, retention: { ...retention, maxAgeDays: Number(event.target.value) }, preferences, autostart })} /></label>
        <label className="setting-toggle"><span>Capture images</span><span className="check"><input type="checkbox" checked={preferences.captureImages} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, captureImages: event.target.checked }, retention, autostart })} /></span></label>
        <label className="setting-toggle"><span>Start at login</span><span className="check"><input type="checkbox" disabled={busy || !settled} checked={autostart} onChange={() => void run(async () => { if (autostart) await disable(); else await enable(); setDraft({ ...draft!, preferences, retention, autostart: await isEnabled() }); })} /></span></label>
        <label className="setting-toggle"><span>Stage selection for Ctrl+V</span><span className="check"><input type="checkbox" checked={preferences.stageSelection} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, stageSelection: event.target.checked }, retention, autostart })} /></span></label>
        <label className="setting-toggle"><span>Show shortcut bar</span><span className="check"><input type="checkbox" checked={preferences.showShortcutBar} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, showShortcutBar: event.target.checked }, retention, autostart })} /></span></label>
        <label className="settings-span">Separator style<select aria-label="Separator style" value={separatorPreset(preferences.pasteSeparator)} onChange={(event) => {
          const preset = separatorPresets.find(([id]) => id === event.target.value);
          if (preset) setDraft({ ...draft!, preferences: { ...preferences, pasteSeparator: preset[2] }, retention, autostart });
        }}>
          {separatorPresets.map(([id, label]) => <option key={id} value={id}>{label}</option>)}
          <option value="custom">Custom</option>
        </select></label>
        <label className="settings-span">Paste separator<textarea rows={2} maxLength={1024} value={preferences.pasteSeparator} onChange={(event) => setDraft({ ...draft!, preferences: { ...preferences, pasteSeparator: event.target.value }, retention, autostart })} /></label>
      </div>
      <div className="settings-footer">
        <div className="settings-actions">
          <button className={done === "save" ? "is-done" : undefined} disabled={busy || !settled} type="submit">{done === "save" && <Check size={14} aria-hidden="true" />} Save settings</button>
          <button className={`quit-button${done === "quit" ? " is-done" : ""}`} disabled={busy} type="button" onClick={() => void run(async () => { await invoke("quit_app"); }, "quit")}>{done === "quit" && <Check size={14} aria-hidden="true" />} Quit</button>
        </div>
        <div className="settings-actions">
          <button className={done === "clear-unpinned" ? "is-done" : undefined} disabled={busy} type="button" onClick={() => void run(async () => { await invoke("clear_history", { includePinned: false }); }, "clear-unpinned")}>{done === "clear-unpinned" && <Check size={14} aria-hidden="true" />} Clear unpinned</button>
          <button className={done === "clear-all" ? "is-done" : undefined} disabled={busy} type="button" onClick={() => void run(async () => { await invoke("clear_history", { includePinned: true }); }, "clear-all")}>{done === "clear-all" && <Check size={14} aria-hidden="true" />} Clear all history</button>
        </div>
      </div>
    </form>}
  </main>;
}

export function Settings({ close }: { close: () => void }) {
  return <AppQuery><SettingsForm close={close} /></AppQuery>;
}
