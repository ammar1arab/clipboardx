import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ClipboardItem } from "../domain/clipboard";
import type { HistoryActions } from "./HistoryWorkspace";

type StoredItem = {
  id: string; kind: "text" | "image"; text: string | null;
  thumbnailPath: string | null; width: number | null; height: number | null;
  createdAt: number; lastUsedAt: number | null; pinned: boolean;
};
type CaptureStatus = { state: "starting" | "running" | "unavailable" | "error" | "stopped"; message?: string };

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
  return "ClipboardX could not complete the action. Please try again.";
}

export function useHistory() {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [capture, setCapture] = useState<CaptureStatus>({ state: "starting" });
  const alive = useRef(false);
  const request = useRef(0);
  const refresh = useCallback(async () => {
    const current = ++request.current;
    try {
      const stored = await invoke<StoredItem[]>("list_history");
      const entries: ClipboardItem[] = stored.map((item) => {
        const base = { id: item.id, createdAt: item.createdAt, lastUsedAt: item.lastUsedAt, pinned: item.pinned };
        if (item.kind === "text" && item.text !== null) return { ...base, kind: "text", text: item.text };
        if (item.kind === "image" && item.thumbnailPath && item.width && item.height)
          return { ...base, kind: "image", thumbnailUrl: convertFileSrc(item.thumbnailPath), width: item.width, height: item.height };
        throw "Clipboard history contains an invalid entry.";
      });
      if (alive.current && current === request.current) { setItems(entries); setError(undefined); }
    } catch (failure) {
      if (alive.current && current === request.current) setError(errorMessage(failure));
    } finally {
      if (alive.current && current === request.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    alive.current = true;
    const subscriptions: (() => void)[] = [];
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    async function bind<T>(name: string, callback: (payload: T) => void) {
      try {
        const unsubscribe = await listen<T>(name, ({ payload }) => { if (!cancelled) callback(payload); });
        if (cancelled) unsubscribe(); else subscriptions.push(unsubscribe);
      } catch (failure) { if (!cancelled) setError(errorMessage(failure)); }
    }
    void bind("history-changed", () => { clearTimeout(timer); timer = setTimeout(() => void refresh(), 40); });
    void bind<CaptureStatus>("capture-status", setCapture);
    void invoke<CaptureStatus>("get_capture_status").then((status) => { if (!cancelled) setCapture(status); }).catch((failure) => { if (!cancelled) setError(errorMessage(failure)); });
    void refresh();
    return () => { cancelled = true; alive.current = false; ++request.current; clearTimeout(timer); subscriptions.forEach((unsubscribe) => unsubscribe()); };
  }, [refresh]);

  async function mutate(command: string, args: Record<string, unknown>) {
    try { await invoke(command, args); await refresh(); }
    catch (failure) { throw errorMessage(failure); }
  }
  const actions: HistoryActions = {
    paste: async () => { throw "Native paste is not implemented yet."; },
    dismiss: () => { void invoke("hide_window").catch((failure) => setError(errorMessage(failure))); },
    remove: (ids) => mutate("delete_history", { ids }),
    pin: (id, pinned) => mutate("set_pinned", { id, pinned }),
    edit: (id, text) => mutate("edit_text", { id, text }),
  };
  async function retry() {
    try { await invoke("retry_capture"); await refresh(); }
    catch (failure) { setError(errorMessage(failure)); }
  }
  return { items, actions, loading, error: error ?? capture.message, retry };
}
