export type ClipboardItem = {
  id: string;
  pinned: boolean;
  createdAt: number;
  lastUsedAt: number | null;
} & (
  | { kind: "text"; text: string }
  | { kind: "image"; thumbnailUrl: string; filePath: string; width: number; height: number }
);

export function filterItems(items: ClipboardItem[], query: string, kind?: "text" | "image"): ClipboardItem[] {
  const search = query.toLocaleLowerCase();
  return items.filter((item) =>
    (!kind || item.kind === kind) &&
    (item.kind === "text" ? item.text : `image ${item.width}×${item.height}`)
      .toLocaleLowerCase().includes(search),
  );
}

export function selectedItems(items: ClipboardItem[], selected: Set<string>): ClipboardItem[] {
  return items.filter((item) => selected.has(item.id));
}
