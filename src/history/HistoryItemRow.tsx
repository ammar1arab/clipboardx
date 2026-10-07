import { FileText, Image, Pin, Pencil, Trash2 } from "lucide-react";
import { useEffect, useRef } from "react";
import type { ClipboardItem } from "../domain/clipboard";

type Props = {
  item: ClipboardItem; active: boolean; selected: boolean;
  choose: (toggle: boolean, range: boolean) => void;
  paste: () => void; pin: () => void; edit: () => void; remove: () => void; activate: () => void;
};

export function HistoryItemRow({ item, active, selected, choose, paste, pin, edit, remove, activate }: Props) {
  const row = useRef<HTMLLIElement>(null);
  useEffect(() => { if (active) row.current?.scrollIntoView?.({ block: "nearest" }); }, [active]);
  const label = item.kind === "text" ? item.text : `Image, ${item.width} by ${item.height}`;
  return <li ref={row} className={`history-item ${active ? "is-active" : ""} ${selected ? "is-selected" : ""}`}>
    <button id={`entry-${item.id}`} className="item-content" aria-label={label} aria-pressed={selected}
      draggable={item.kind === "text"}
      onDragStart={(event) => {
        if (item.kind === "text") { event.dataTransfer.setData("text/plain", item.text); event.dataTransfer.effectAllowed = "copy"; }
      }}
      onFocus={activate}
      onClick={(event) => {
        choose(event.ctrlKey || event.metaKey, event.shiftKey);
        if (!(event.ctrlKey || event.metaKey || event.shiftKey)) paste();
      }}>
      <span className="item-icon" aria-hidden="true">{item.kind === "text" ? <FileText size={17} /> : <Image size={17} />}</span>
      <span className="item-preview">
        {item.kind === "text" ? <span className="text-preview" dir="auto">{item.text}</span> :
          <img className="thumbnail" src={item.thumbnailUrl} alt={`${item.width} × ${item.height} clipboard image`} loading="lazy" />}
        <span className="item-meta">{item.kind === "text" ? `${item.text.length.toLocaleString()} characters` : `${item.width} × ${item.height}`}
          {item.pinned && <span className="pin-label"><Pin size={10} aria-hidden="true" /> Pinned</span>}
        </span>
      </span>
    </button>
    <div className="item-actions">
      <button className="icon-button" aria-label={item.pinned ? "Unpin item" : "Pin item"} aria-pressed={item.pinned} onClick={pin}><Pin size={14} /></button>
      {item.kind === "text" && <button className="icon-button" aria-label="Edit item" onClick={edit}><Pencil size={14} /></button>}
      <button className="icon-button" aria-label="Delete item" onClick={remove}><Trash2 size={14} /></button>
    </div>
  </li>;
}
