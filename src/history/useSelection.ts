import { useState } from "react";

export function useSelection(ids: string[]) {
  const [active, setActive] = useState<string | null>(null);
  const [selection, setSelection] = useState<Set<string>>(new Set());
  const [anchor, setAnchor] = useState<string | null>(null);
  const activeId = active && ids.includes(active) ? active : ids[0] ?? null;
  const selected = new Set([...selection].filter((id) => ids.includes(id)));

  function choose(id: string, toggle: boolean, range: boolean) {
    setActive(id);
    if (range) {
      const origin = anchor && ids.includes(anchor) ? anchor : activeId ?? id;
      if (!anchor || !ids.includes(anchor)) setAnchor(origin);
      const start = ids.indexOf(origin);
      const end = ids.indexOf(id);
      setSelection(new Set(ids.slice(Math.min(start, end), Math.max(start, end) + 1)));
    } else if (toggle) {
      const next = new Set(selected);
      if (next.has(id)) next.delete(id); else next.add(id);
      setSelection(next);
      setAnchor(id);
    } else {
      setSelection(new Set());
      setAnchor(id);
    }
  }

  function moveTo(index: number, extend = false) {
    if (!ids.length) return;
    const next = ids[Math.max(0, Math.min(ids.length - 1, index))];
    choose(next, false, extend);
  }
  function move(delta: number, extend = false) {
    if (!ids.length || !activeId) return;
    moveTo(ids.indexOf(activeId) + delta, extend);
  }
  function clear() { setSelection(new Set()); setAnchor(activeId); }
  function home() { setActive(null); setSelection(new Set()); setAnchor(null); }
  function selectAll() {
    setSelection(new Set(ids));
    if (ids[0]) { setActive(ids[0]); setAnchor(ids[0]); }
  }
  return { activeId, selected, move, moveTo, choose, clear, home, activate: setActive, selectAll };
}
