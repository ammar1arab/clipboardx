import { MoreVertical, Pin } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ClipboardItem } from "../domain/clipboard";

type Props = {
  item: ClipboardItem; active: boolean; selected: boolean;
  choose: (toggle: boolean, range: boolean) => void;
  paste: () => void; pin: () => void; edit: () => void; remove: () => void; activate: () => void;
};

export function HistoryItemRow({ item, active, selected, choose, paste, pin, edit, remove, activate }: Props) {
  const menuBox = useRef<HTMLDivElement>(null);
  const [menu, setMenu] = useState(false);
  useEffect(() => {
    if (!menu) return;
    const close = (event: PointerEvent) => { if (!menuBox.current?.contains(event.target as Node)) setMenu(false); };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [menu]);
  const dragged = useRef(false);
  const pick = (action: () => void) => () => { setMenu(false); action(); };
  const label = item.kind === "text" ? item.text : `Image, ${item.width} by ${item.height}`;
  return <li ref={(node) => { if (active) node?.scrollIntoView?.({ block: "nearest" }); }} className={`history-item ${active ? "is-active" : ""} ${selected ? "is-selected" : ""}`}>
    <div id={`entry-${item.id}`} className="item-content" role="button" tabIndex={0} aria-label={label} aria-pressed={selected}
      draggable
      onDragStart={(event) => {
        dragged.current = true;
        if (item.kind === "text") event.dataTransfer.setData("text/plain", item.text);
        else {
          const uri = `file://${item.filePath}`;
          event.dataTransfer.setData("text/uri-list", uri);
          event.dataTransfer.setData("text/plain", uri);
          event.dataTransfer.setData("DownloadURL", `image/png:clipboard.png:${uri}`);
        }
        event.dataTransfer.effectAllowed = "copy";
      }}
      onDragEnd={() => { window.setTimeout(() => { dragged.current = false; }, 0); }}
      onFocus={activate}
      onKeyDown={(event) => { if (event.key === " ") { event.preventDefault(); event.currentTarget.click(); } }}
      onClick={(event) => {
        if (dragged.current) { event.preventDefault(); return; }
        choose(event.ctrlKey || event.metaKey, event.shiftKey);
        if (!(event.ctrlKey || event.metaKey || event.shiftKey)) paste();
      }}>
      {item.kind === "text" ? <span className="text-preview" dir="auto">{item.text}</span> :
        <img className="thumbnail" src={item.thumbnailUrl} alt={`${item.width} × ${item.height} clipboard image`} loading="lazy" />}
      <span className="item-meta">{item.kind === "text" ? `${item.text.length.toLocaleString()} characters` : `${item.width} × ${item.height}`}
        {item.pinned && <span className="pin-label"><Pin size={10} aria-hidden="true" /> Pinned</span>}
      </span>
    </div>
    <div ref={menuBox} className="item-menu" onKeyDown={(event) => {
      if (event.key === "Escape" && menu) { event.stopPropagation(); setMenu(false); menuBox.current?.querySelector("button")?.focus(); }
    }}>
      <button className="icon-button" aria-label="More actions" aria-haspopup="menu" aria-expanded={menu} onClick={() => setMenu(!menu)}><MoreVertical size={14} /></button>
      {menu && <div className="item-menu-list" role="menu">
        <button role="menuitem" onClick={pick(pin)}>{item.pinned ? "Unpin" : "Pin"}</button>
        {item.kind === "text" && <button role="menuitem" onClick={pick(edit)}>Edit</button>}
        <button role="menuitem" onClick={pick(remove)}>Delete</button>
      </div>}
    </div>
  </li>;
}
