import { describe, expect, it } from "vitest";
import { filterItems, type ClipboardItem } from "./clipboard";

const items: ClipboardItem[] = [
  { id: "a", kind: "text", text: "Café résumé", pinned: false, createdAt: 0, lastUsedAt: null },
  { id: "b", kind: "text", text: "Alpha source code", pinned: false, createdAt: 0, lastUsedAt: null },
  { id: "c", kind: "image", thumbnailUrl: "/a.png", filePath: "/tmp/a.png", width: 400, height: 200, pinned: false, createdAt: 0, lastUsedAt: null },
];

describe("filterItems", () => {
  it("matches every word in any order and ignores accents", () => {
    expect(filterItems(items, "source alpha").map((item) => item.id)).toEqual(["b"]);
    expect(filterItems(items, "cafe resume").map((item) => item.id)).toEqual(["a"]);
    expect(filterItems(items, "   ").map((item) => item.id)).toEqual(["a", "b", "c"]);
  });

  it("matches image size with x or the times sign", () => {
    expect(filterItems(items, "400x200", "image").map((item) => item.id)).toEqual(["c"]);
    expect(filterItems(items, "400×200").map((item) => item.id)).toEqual(["c"]);
    expect(filterItems(items, "400", "text")).toEqual([]);
  });
});
