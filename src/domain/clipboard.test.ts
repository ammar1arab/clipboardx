import { describe, expect, it } from "vitest";
import { filterItems, formatClipAge, type ClipboardItem } from "./clipboard";

const items: ClipboardItem[] = [
  { id: "a", kind: "text", text: "Café résumé", pinned: false, createdAt: 0, lastUsedAt: null },
  { id: "b", kind: "text", text: "Alpha source code", pinned: false, createdAt: 0, lastUsedAt: null },
  { id: "c", kind: "image", thumbnailUrl: "/a.png", filePath: "/tmp/a.png", width: 400, height: 200, pinned: false, createdAt: 0, lastUsedAt: null },
];

describe("formatClipAge", () => {
  const now = Date.parse("2026-10-07T12:00:00Z");
  it("uses a short age for recent clips and a date after a week", () => {
    expect(formatClipAge(now - 10_000, now)).toBe("Just now");
    expect(formatClipAge(now - 5 * 60_000, now)).toBe("5m ago");
    expect(formatClipAge(now - 3 * 3_600_000, now)).toBe("3h ago");
    expect(formatClipAge(now - 2 * 86_400_000, now)).toBe("2d ago");
    expect(formatClipAge(Date.parse("2026-09-01T12:00:00Z"), now)).toMatch(/Sep/);
  });
});

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
