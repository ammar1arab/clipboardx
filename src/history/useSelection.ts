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
      const start = ids.indexOf(anchor && ids.includes(anchor) ? anchor : activeId ?? id);
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

  function move(delta: number, extend = false) {
    if (!ids.length) return;
    const next = ids[Math.max(0, Math.min(ids.length - 1, ids.indexOf(activeId!) + delta))];
    choose(next, false, extend);
  }
  function clear() { setSelection(new Set()); setAnchor(activeId); }
  return { activeId, selected, move, choose, clear, activate: setActive, selectAll: () => setSelection(new Set(ids)) };
}
