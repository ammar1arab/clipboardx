import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Clipboard, Search, X, ArrowUpRight, Trash2, Settings } from "lucide-react";
import { filterItems, selectedItems, type ClipboardItem } from "../domain/clipboard";
import { useSelection } from "./useSelection";
import { HistoryItemRow } from "./HistoryItemRow";

export type HistoryActions = {
  stage: (ids: string[]) => Promise<void>;
  paste: (items: ClipboardItem[]) => Promise<void>;
  remove: (ids: string[]) => Promise<void>;
  pin: (id: string, pinned: boolean) => Promise<void>;
  edit: (id: string, text: string) => Promise<void>;
  dismiss: () => void;
};
type Props = { items: ClipboardItem[]; actions: HistoryActions; loading?: boolean; error?: string; retry?: () => void; launchCount?: number; openSettings?: () => void };

export function HistoryWorkspace({ items, actions, loading, error, retry, launchCount, openSettings }: Props) {
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<{ id: string; text: string } | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [staging, setStaging] = useState(false);
  const [stageReady, setStageReady] = useState(false);
  const search = useRef<HTMLInputElement>(null);
  const visible = useMemo(() => filterItems(items, query), [items, query]);
  const selection = useSelection(visible.map((item) => item.id));
  const active = visible.find((item) => item.id === selection.activeId);
  const selectedKey = JSON.stringify(selectedItems(visible, selection.selected).map((item) => item.id));
  const stageQueue = useRef(Promise.resolve());
  const stage = useRef(actions.stage);
  stage.current = actions.stage;
  useEffect(() => { search.current?.focus(); search.current?.select(); }, [launchCount]);
  useEffect(() => {
    const ids: string[] = JSON.parse(selectedKey);
    if (!ids.length) { setStaging(false); setStageReady(false); return; }
    let cancelled = false;
    setStaging(true);
    setStageReady(false);
    stageQueue.current = stageQueue.current.catch(() => {}).then(() => stage.current(ids));
    void stageQueue.current.then(() => { if (!cancelled) { setStaging(false); setStageReady(true); } }).catch((failure) => {
      if (!cancelled) { setStaging(false); setOperationError(typeof failure === "string" ? failure : "Could not copy the selection. Please try again."); }
    });
    return () => { cancelled = true; };
  }, [selectedKey]);

  async function run(operation: () => Promise<void>) {
    if (busy) return;
    setBusy(true); setOperationError(null);
    try { await operation(); }
    catch (failure) { setOperationError(typeof failure === "string" ? failure : "The action failed. Please try again."); }
    finally { setBusy(false); }
  }
  function beginEdit(item: ClipboardItem) { if (item.kind === "text") setEditing({ id: item.id, text: item.text }); }
  function removeSelected() {
    const ids = selection.selected.size ? [...selection.selected] : active ? [active.id] : [];
    if (ids.length) void run(async () => { await actions.remove(ids); selection.clear(); });
  }
  function pasteSelected() {
    const entries = selectedItems(visible, selection.selected);
    if (!entries.length) return;
    if (entries.length > 1 && entries.some((item) => item.kind === "image")) {
      setOperationError("Paste images individually. Multi-paste supports text entries."); return;
    }
    void run(() => actions.paste(entries));
  }
  function saveEdit() {
    if (editing) void run(async () => { await actions.edit(editing.id, editing.text); setEditing(null); search.current?.focus(); });
  }
  function onKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (editing) return;
    const target = event.target as HTMLElement;
    const inSearch = target === search.current;
    const inControl = target.closest("button, input, textarea, select") && !inSearch && !target.closest(".item-content");
    const modifier = event.ctrlKey || event.metaKey;
    if (event.key === "Escape") {
      event.preventDefault();
      if (selection.selected.size) selection.clear(); else if (query) setQuery(""); else actions.dismiss();
      search.current?.focus(); return;
    }
    if (inControl && !(modifier && ["a", "p", "enter"].includes(event.key.toLowerCase())) && !["Delete", "ArrowDown", "ArrowUp", "F2"].includes(event.key)) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault(); selection.move(event.key === "ArrowDown" ? 1 : -1, event.shiftKey);
      search.current?.focus();
    } else if (modifier && event.key.toLowerCase() === "a") {
      event.preventDefault(); selection.selectAll();
    } else if (event.key === "Enter") {
      event.preventDefault(); if (modifier) pasteSelected(); else if (active) void run(() => actions.paste([active]));
    } else if (event.key === "Delete" && (selection.selected.size || !inSearch || !query)) {
      event.preventDefault(); removeSelected();
    } else if (event.key === "F2" && active) {
      event.preventDefault(); beginEdit(active);
    } else if (modifier && event.key.toLowerCase() === "p" && active) {
      event.preventDefault(); void run(() => actions.pin(active.id, !active.pinned));
    }
  }
  return <main className="workspace" onKeyDown={onKeyDown} aria-busy={busy || loading}>
    <header className="titlebar"><span className="brand"><Clipboard size={17} aria-hidden="true" /> ClipboardX</span>{openSettings ? <button className="icon-button" aria-label="Open settings" onClick={openSettings}><Settings size={16} /></button> : <span className="privacy-label">Local history</span>}</header>
    <div className="searchbar"><Search size={17} aria-hidden="true" />
      <input ref={search} autoFocus aria-label="Search clipboard history" placeholder="Search your clipboard…" value={query}
        aria-controls="history-list" aria-activedescendant={selection.activeId ? `entry-${selection.activeId}` : undefined}
        onChange={(event) => { setQuery(event.target.value); selection.clear(); }} />
      {query && <button className="icon-button" aria-label="Clear search" onClick={() => { setQuery(""); search.current?.focus(); }}><X size={15} /></button>}
    </div>
    {(error || operationError) && <div className="error-message" role="alert">{error || operationError}{retry && <button onClick={retry}>Retry</button>}</div>}
    {selection.selected.size > 0 && <div className="selection-toolbar" aria-label="Selection actions">
      <span aria-live="polite">{selection.selected.size} selected{staging ? " · Copying…" : stageReady ? " · Ctrl+V anywhere" : ""}</span>
      <button disabled={busy} onClick={pasteSelected}><ArrowUpRight size={13} /> Paste</button>
      <button disabled={busy} onClick={removeSelected}><Trash2 size={13} /> Delete</button>
      <button className="icon-button" aria-label="Clear selection" onClick={selection.clear}><X size={14} /></button>
    </div>}
    <div className="history-scroll">
      {loading ? <div className="empty-state" role="status">Loading clipboard history…</div> : visible.length === 0 ?
        <div className="empty-state"><Clipboard size={28} aria-hidden="true" /><strong>{query ? "No matching entries" : "Your clipboard starts here"}</strong><span>{query ? "Try another word or clear your search." : "Copy text or an image to keep it nearby."}</span></div> :
        <ul id="history-list" aria-label="Clipboard history" className="history-list">
          {visible.map((item) => editing?.id === item.id ? <li key={item.id} className="inline-editor">
            <label htmlFor="edit-text">Edit clipboard text</label>
            <textarea id="edit-text" autoFocus dir="auto" value={editing.text} onChange={(event) => setEditing({ ...editing, text: event.target.value })}
              onKeyDown={(event) => {
                if (event.key === "Escape") { event.preventDefault(); setEditing(null); search.current?.focus(); }
                else if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); saveEdit(); }
              }} />
            <div className="editor-actions"><span>Shift+Enter for a new line</span><button onClick={() => { setEditing(null); search.current?.focus(); }}>Cancel</button><button disabled={busy || !editing.text} onClick={saveEdit}>Save</button></div>
          </li> : <HistoryItemRow key={item.id} item={item} active={selection.activeId === item.id} selected={selection.selected.has(item.id)}
            activate={() => selection.activate(item.id)}
            choose={(toggle, range) => selection.choose(item.id, toggle, range)} paste={() => void run(() => actions.paste([item]))}
            pin={() => void run(() => actions.pin(item.id, !item.pinned))} edit={() => beginEdit(item)} remove={() => void run(() => actions.remove([item.id]))} />)}
        </ul>}
    </div>
    <footer className="statusbar"><span>{visible.length} {visible.length === 1 ? "entry" : "entries"}</span><span><kbd>↑↓</kbd> Navigate <kbd>↵</kbd> Paste <kbd>Esc</kbd> Close</span></footer>
  </main>;
}
