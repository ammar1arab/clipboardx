import { useState } from "react";
import { HistoryWorkspace } from "./history/HistoryWorkspace";
import type { ClipboardItem } from "./domain/clipboard";
import "./App.css";

const sampleItems: ClipboardItem[] = [
  { id: "sample-1", kind: "text", text: "Keep the things you copy close at hand.", pinned: true, createdAt: 0, lastUsedAt: null },
  { id: "sample-2", kind: "text", text: "https://system76.com/cosmic", pinned: false, createdAt: 0, lastUsedAt: null },
  { id: "sample-3", kind: "text", text: "const greeting = 'Hello, COSMIC';\nconsole.log(greeting);", pinned: false, createdAt: 0, lastUsedAt: null },
  { id: "sample-4", kind: "text", text: "مرحباً — your clipboard, always nearby", pinned: false, createdAt: 0, lastUsedAt: null },
];

export default function App() {
  const [items, setItems] = useState(sampleItems);
  return <HistoryWorkspace items={items} preview actions={{
    paste: async () => { throw "Native paste is not connected in this interface preview."; },
    dismiss: () => {},
    remove: async (ids) => setItems((current) => current.filter((item) => !ids.includes(item.id))),
    pin: async (id, pinned) => setItems((current) => current.map((item) => item.id === id ? { ...item, pinned } : item)),
    edit: async (id, text) => setItems((current) => current.map((item) => item.id === id && item.kind === "text" ? { ...item, text } : item)),
  }} />;
}
