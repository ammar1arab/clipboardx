export type ClipboardItem = {
  id: string;
  pinned: boolean;
  createdAt: number;
  lastUsedAt: number | null;
} & (
  | { kind: "text"; text: string }
  | { kind: "image"; thumbnailUrl: string; filePath: string; width: number; height: number }
);

const combiningMarks = /\p{M}/gu;

export function foldSearch(value: string): string {
  return value.normalize("NFKD").replace(combiningMarks, "").toLowerCase();
}

export type SearchEntry = { kind: ClipboardItem["kind"]; text: string; item: ClipboardItem };

export function buildSearchIndex(items: ClipboardItem[]): SearchEntry[] {
  return items.map((item) => ({
    kind: item.kind,
    item,
    text: foldSearch(item.kind === "text" ? item.text : `image ${item.width}x${item.height} ${item.width}×${item.height}`),
  }));
}

export function searchItems(index: SearchEntry[], query: string, kind?: "text" | "image"): ClipboardItem[] {
  const tokens = foldSearch(query).split(/\s+/).filter(Boolean);
  const matches: ClipboardItem[] = [];
  for (const entry of index) {
    if (kind && entry.kind !== kind) continue;
    if (tokens.every((token) => entry.text.includes(token))) matches.push(entry.item);
  }
  return matches;
}

export function filterItems(items: ClipboardItem[], query: string, kind?: "text" | "image"): ClipboardItem[] {
  return searchItems(buildSearchIndex(items), query, kind);
}

export function selectedItems(items: ClipboardItem[], selected: Set<string>): ClipboardItem[] {
  return items.filter((item) => selected.has(item.id));
}
