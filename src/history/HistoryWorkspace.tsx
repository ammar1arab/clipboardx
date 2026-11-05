import { useDeferredValue, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { flushSync } from "react-dom";
import { Clipboard, X, ArrowUpRight, Trash2, Settings, Image, Type, Keyboard } from "lucide-react";
import { buildSearchIndex, searchItems, selectedItems, type ClipboardItem } from "../domain/clipboard";
import { ShortcutList } from "../shortcuts";
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
type Props = {
  items: ClipboardItem[]; actions: HistoryActions; loading?: boolean; error?: string; retry?: () => void;
  launchCount?: number; openSettings?: () => void; stageSelection?: boolean; showShortcutBar?: boolean;
};

function mixed(entries: ClipboardItem[]) {
  return entries.length > 1 && entries.some((item) => item.kind === "image");
}

export function HistoryWorkspace({ items, actions, loading, error, retry, launchCount, openSettings, stageSelection = true, showShortcutBar = true }: Props) {
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<"text" | "image" | null>(null);
  const [editing, setEditing] = useState<{ id: string; text: string } | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [staging, setStaging] = useState(false);
  const [stageReady, setStageReady] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const operationRunning = useRef(false);
  const search = useRef<HTMLInputElement>(null);
  const workspace = useRef<HTMLElement>(null);
  const shortcutBox = useRef<HTMLDivElement>(null);
  const knownIds = useRef<Set<string> | null>(null);
  const deferredQuery = useDeferredValue(query);
  const index = useMemo(() => buildSearchIndex(items), [items]);
  const visible = useMemo(() => searchItems(index, deferredQuery, kind ?? undefined), [index, deferredQuery, kind]);
  const currentIds = items.map((item) => item.id);
  const freshIds = knownIds.current === null ? new Set<string>() : new Set(currentIds.filter((id) => !knownIds.current!.has(id)));
  knownIds.current = new Set(currentIds);
  const selection = useSelection(visible.map((item) => item.id));
  const active = visible.find((item) => item.id === selection.activeId);
  const selectedKey = JSON.stringify(selectedItems(visible, selection.selected).map((item) => item.id));
  const stageToken = `${stageSelection}:${selectedKey}`;
  const stageQueue = useRef(Promise.resolve());
  const stageTicket = useRef(0);
  const publishedStage = useRef<string | undefined>(undefined);
  const seenLaunch = useRef<number | null>(null);
  const stage = useRef(actions.stage);
  const visibleItems = useRef(visible);
  stage.current = actions.stage;
  visibleItems.current = visible;
  function publishStage(token: string) {
    const ids: string[] = JSON.parse(token.slice(token.indexOf(":") + 1));
    if (!ids.length || !token.startsWith("true:")) { setStaging(false); setStageReady(false); return; }
    const chosen = selectedItems(visibleItems.current, new Set(ids));
    if (mixed(chosen)) { setStaging(false); setStageReady(false); return; }
    const ticket = ++stageTicket.current;
    setStaging(true);
    setStageReady(false);
    stageQueue.current = stageQueue.current.catch(() => {}).then(() => stage.current(ids));
    void stageQueue.current.then(() => {
      if (ticket !== stageTicket.current) return;
      setStaging(false); setStageReady(true);
    }).catch((failure) => {
      if (ticket !== stageTicket.current) return;
      setStaging(false);
      setOperationError(typeof failure === "string" ? failure : "Could not copy the selection. Please try again.");
    });
  }
  function bindWorkspace(node: HTMLElement | null) {
    workspace.current = node;
    if (!node) return;
    const launch = launchCount ?? 0;
    if (seenLaunch.current !== launch) { seenLaunch.current = launch; node.focus(); }
    if (publishedStage.current !== stageToken) { publishedStage.current = stageToken; publishStage(stageToken); }
    return () => { workspace.current = null; };
  }
  function bindShortcuts(node: HTMLDivElement | null) {
    shortcutBox.current = node;
    if (!node || !shortcutsOpen) return;
    const closeOnOutside = (event: PointerEvent) => { if (!node.contains(event.target as Node)) setShortcutsOpen(false); };
    document.addEventListener("pointerdown", closeOnOutside);
    return () => document.removeEventListener("pointerdown", closeOnOutside);
  }

  async function run(operation: () => Promise<void>) {
    if (operationRunning.current) return;
    operationRunning.current = true;
    setBusy(true); setOperationError(null);
    try { await operation(); }
    catch (failure) { setOperationError(typeof failure === "string" ? failure : "The action failed. Please try again."); }
    finally { operationRunning.current = false; setBusy(false); }
  }
  function beginEdit(item: ClipboardItem) { if (item.kind === "text") setEditing({ id: item.id, text: item.text }); }
  function removeSelected() {
    const ids = selection.selected.size ? [...selection.selected] : active ? [active.id] : [];
    if (ids.length) void run(async () => { await actions.remove(ids); selection.clear(); });
  }
  function pasteEntries(entries: ClipboardItem[]) {
    if (!entries.length) return;
    if (mixed(entries)) return;
    search.current?.blur();
    workspace.current?.focus();
    void run(() => actions.paste(entries));
  }
  function pasteSelected() {
    pasteEntries(selectedItems(visible, selection.selected));
  }
  function saveEdit() {
    if (editing) void run(async () => { await actions.edit(editing.id, editing.text); flushSync(() => setEditing(null)); workspace.current?.focus(); });
  }
  function toggleKind(next: "text" | "image") {
    setKind(kind === next ? null : next);
    selection.clear();
  }
  function onKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (editing) return;
    if ((busy || operationRunning.current) && (event.key.length === 1 || event.key === "Enter")) {
      event.preventDefault();
      return;
    }
    const target = event.target as HTMLElement;
    const inSearch = target === search.current;
    const inControl = Boolean(target.closest("button, input, textarea, select") && !inSearch && !target.closest(".item-content"));
    const modifier = event.ctrlKey || event.metaKey;
    if (event.key === "Escape") {
      event.preventDefault();
      if (shortcutsOpen) { setShortcutsOpen(false); return; }
      if (selection.selected.size) selection.clear(); else if (query) setQuery(""); else if (kind) setKind(null); else actions.dismiss();
      return;
    }
    if (modifier && event.key === "/") { event.preventDefault(); setShortcutsOpen((open) => !open); return; }
    if (event.altKey && !modifier && (event.code === "Digit1" || event.code === "Digit2")) {
      event.preventDefault();
      toggleKind(event.code === "Digit1" ? "image" : "text");
      return;
    }
    if (event.key.length === 1 && event.key !== " " && !modifier && !event.altKey && !target.closest("input, textarea, select")) {
      event.preventDefault(); setQuery(query + event.key); selection.clear(); search.current?.focus(); return;
    }
    if (inControl && !(modifier && ["a", "p", "enter"].includes(event.key.toLowerCase())) && !["Delete", "Backspace", "ArrowDown", "ArrowUp", "Home", "End", "F2"].includes(event.key)) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault(); selection.move(event.key === "ArrowDown" ? 1 : -1, event.shiftKey);
      if (!inSearch) workspace.current?.focus();
    } else if ((event.key === "Home" || event.key === "End") && !inSearch) {
      event.preventDefault();
      selection.moveTo(event.key === "Home" ? 0 : Number.POSITIVE_INFINITY, event.shiftKey);
      workspace.current?.focus();
    } else if (modifier && event.key.toLowerCase() === "a") {
      if (inSearch) return;
      event.preventDefault(); selection.selectAll(); workspace.current?.focus();
    } else if (event.key === " " && !inSearch && !inControl) {
      event.preventDefault();
      if (selection.activeId) selection.choose(selection.activeId, true, false);
    } else if (event.key === "Enter") {
      event.preventDefault(); if (modifier || selection.selected.size) pasteSelected(); else if (active) pasteEntries([active]);
    } else if ((event.key === "Delete" || event.key === "Backspace") && !inSearch) {
      event.preventDefault(); removeSelected();
    } else if (event.key === "F2" && active) {
      event.preventDefault(); beginEdit(active);
    } else if (modifier && event.key.toLowerCase() === "p" && active) {
      event.preventDefault(); void run(() => actions.pin(active.id, !active.pinned));
    }
  }
  const banner = operationError ?? error;
  const showRetry = !operationError && Boolean(error) && retry;
  return <main ref={bindWorkspace} tabIndex={-1} className="workspace" onKeyDown={onKeyDown} aria-busy={busy || loading}>
    <header className="titlebar"><div className="searchbar">
      <input ref={search} aria-label="Search clipboard history" placeholder="Search your clipboard…" value={query}
        aria-controls="history-list" aria-activedescendant={selection.activeId ? `entry-${selection.activeId}` : undefined}
        onChange={(event) => { setQuery(event.target.value); selection.clear(); }} />
      {query && <button className="icon-button" aria-label="Clear search" onClick={() => { setQuery(""); search.current?.focus(); }}><X size={14} /></button>}
    </div>
    <button className="icon-button" aria-label="Show images" aria-pressed={kind === "image"} onClick={() => toggleKind("image")}><Image size={15} /></button>
    <button className="icon-button" aria-label="Show text" aria-pressed={kind === "text"} onClick={() => toggleKind("text")}><Type size={15} /></button>
    <div className="shortcut-anchor" ref={bindShortcuts}>
      <button className="icon-button" type="button" aria-label="Shortcuts" aria-expanded={shortcutsOpen} onClick={() => setShortcutsOpen((open) => !open)}><Keyboard size={16} /></button>
      {shortcutsOpen && <div className="shortcut-popover" role="dialog" aria-label="Shortcuts"><ShortcutList /></div>}
    </div>
    {openSettings && <button className="icon-button" aria-label="Open settings" onClick={openSettings}><Settings size={16} /></button>}</header>
    {banner && <div className="error-message" role="alert">{banner}{showRetry && <button onClick={retry}>Retry</button>}</div>}
    {selection.selected.size > 0 && <div className="selection-toolbar" aria-label="Selection actions">
      <span aria-live="polite">{selection.selected.size} selected{staging ? " · Copying…" : stageReady ? " · Ctrl+V anywhere" : ""}</span>
      <button className="selection-action" disabled={busy} onClick={pasteSelected}><ArrowUpRight size={11} /> Paste</button>
      <button className="selection-action" disabled={busy} onClick={removeSelected}><Trash2 size={11} /> Delete</button>
      <button className="icon-button" aria-label="Clear selection" onClick={selection.clear}><X size={14} /></button>
    </div>}
    <div className="history-scroll">
      {loading ? <div className="empty-state" role="status">Loading clipboard history…</div> : visible.length === 0 ?
        <div className="empty-state"><Clipboard size={28} aria-hidden="true" /><strong>{query || kind ? "No matching entries" : "Your clipboard starts here"}</strong><span>{query || kind ? "Try another word or turn the filter off." : "Copy text or an image to keep it nearby."}</span></div> :
        <ul id="history-list" aria-label="Clipboard history" aria-multiselectable="true" className="history-list">
          {visible.map((item) => editing?.id === item.id ? <li key={item.id} className="inline-editor">
            <label className="editor-label" htmlFor="edit-text">Edit</label>
            <textarea id="edit-text" autoFocus dir="auto" aria-label="Edit clipboard text" value={editing.text} onChange={(event) => setEditing({ ...editing, text: event.target.value })}
              onFocus={(event) => {
                const field = event.currentTarget;
                if (field.dataset.caret === "end") return;
                field.dataset.caret = "end";
                const end = field.value.length;
                queueMicrotask(() => { field.setSelectionRange(end, end); field.scrollTop = field.scrollHeight; });
              }}
              onKeyDown={(event) => {
                if (event.key === "Escape") { event.preventDefault(); flushSync(() => setEditing(null)); workspace.current?.focus(); }
                else if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); saveEdit(); }
              }} />
            <div className="editor-actions"><span>Shift+Enter for a new line</span><button onClick={() => { flushSync(() => setEditing(null)); workspace.current?.focus(); }}>Cancel</button><button disabled={busy || !editing.text} onClick={saveEdit}>Save</button></div>
          </li> : <HistoryItemRow key={item.id} item={item} active={selection.activeId === item.id} selected={selection.selected.has(item.id)} fresh={freshIds.has(item.id)}
            activate={() => selection.activate(item.id)}
            choose={(toggle, range) => selection.choose(item.id, toggle, range)} paste={() => pasteEntries([item])}
            pin={() => void run(() => actions.pin(item.id, !item.pinned))} edit={() => beginEdit(item)} remove={() => void run(() => actions.remove([item.id]))} />)}
        </ul>}
    </div>
    <footer className="statusbar"><span>{visible.length} {visible.length === 1 ? "entry" : "entries"}</span>{showShortcutBar && <span>Ctrl+Click select <kbd>Shift+Click</kbd> range <kbd>↵</kbd> Paste <kbd>Esc</kbd> Close</span>}</footer>
  </main>;
}
