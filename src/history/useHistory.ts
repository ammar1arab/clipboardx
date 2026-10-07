import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ClipboardItem } from "../domain/clipboard";
import type { HistoryActions } from "./HistoryWorkspace";

type StoredItem = {
  id: string; kind: "text" | "image"; text: string | null;
  imagePath: string | null; thumbnailPath: string | null; width: number | null; height: number | null;
  createdAt: number; lastUsedAt: number | null; pinned: boolean;
};
type CaptureStatus = { state: "starting" | "running" | "unavailable" | "error" | "stopped"; message?: string };

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") {
    if (error.message.includes("reading 'invoke'") || error.message.includes('reading "invoke"')) return "Open the ClipboardX window.";
    return error.message;
  }
  return "Try again.";
}

function toItem(item: StoredItem): ClipboardItem {
  const base = { id: item.id, createdAt: item.createdAt, lastUsedAt: item.lastUsedAt, pinned: item.pinned };
  if (item.kind === "text" && item.text !== null) return { ...base, kind: "text", text: item.text };
  if (item.kind === "image" && item.imagePath && item.thumbnailPath && item.width && item.height)
    return { ...base, kind: "image", thumbnailUrl: convertFileSrc(item.thumbnailPath), filePath: item.imagePath, width: item.width, height: item.height };
  throw "Clipboard history contains an invalid entry.";
}

export function useHistory() {
  const client = useQueryClient();
  const [capture, setCapture] = useState<CaptureStatus>({ state: "starting" });
  const [launchCount, setLaunchCount] = useState(0);
  const [localError, setLocalError] = useState<string>();
  const history = useQuery({ queryKey: ["history"], queryFn: async () => (await invoke<StoredItem[]>("list_history")).map(toItem) });
  const refresh = () => client.invalidateQueries({ queryKey: ["history"] });

  useEffect(() => {
    const subscriptions: (() => void)[] = [];
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    async function bind<T>(name: string, callback: (payload: T) => void) {
      try {
        const unsubscribe = await listen<T>(name, ({ payload }) => { if (!cancelled) callback(payload); });
        if (cancelled) unsubscribe(); else subscriptions.push(unsubscribe);
      } catch (failure) { if (!cancelled) setLocalError(errorMessage(failure)); }
    }
    void bind("history-changed", () => { clearTimeout(timer); timer = setTimeout(() => void refresh(), 40); });
    void bind<CaptureStatus>("capture-status", setCapture);
    void bind("window-opened", () => { setLaunchCount((value) => value + 1); void refresh(); });
    void bind("native-error", (failure) => setLocalError(errorMessage(failure)));
    void invoke<CaptureStatus>("get_capture_status").then((status) => { if (!cancelled) setCapture(status); }).catch((failure) => { if (!cancelled) setLocalError(errorMessage(failure)); });
    return () => { cancelled = true; clearTimeout(timer); subscriptions.forEach((unsubscribe) => unsubscribe()); };
  }, [client]);

  async function mutate(command: string, args: Record<string, unknown>) {
    try { await invoke(command, args); await refresh(); }
    catch (failure) { throw errorMessage(failure); }
  }
  const actions: HistoryActions = {
    stage: async (ids) => { try { await invoke("stage_selection", { ids }); } catch (failure) { throw errorMessage(failure); } },
    paste: async (entries) => { try { await invoke("paste_entries", { ids: entries.map((item) => item.id) }); } catch (failure) { throw errorMessage(failure); } },
    dismiss: () => { void invoke("hide_window").catch((failure) => setLocalError(errorMessage(failure))); },
    remove: (ids) => mutate("delete_history", { ids }),
    pin: (id, pinned) => mutate("set_pinned", { id, pinned }),
    edit: (id, text) => mutate("edit_text", { id, text }),
  };
  async function retry() {
    setLocalError(undefined);
    try { await invoke("retry_capture"); await history.refetch(); }
    catch (failure) { setLocalError(errorMessage(failure)); }
  }
  const queryError = history.error ? errorMessage(history.error) : undefined;
  return { items: history.data ?? [], actions, loading: history.isLoading, error: localError ?? queryError ?? capture.message, retry, launchCount, refresh };
}
