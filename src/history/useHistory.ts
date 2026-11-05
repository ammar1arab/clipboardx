import { useSyncExternalStore } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ClipboardItem } from "../domain/clipboard";
import { normalizePreferences, type Preferences } from "../domain/preferences";
import { queryClient } from "../query";
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

type DesktopSnapshot = { capture: CaptureStatus; launchCount: number; localError?: string };
let desktop: DesktopSnapshot = { capture: { state: "starting" }, launchCount: 0 };
const desktopListeners = new Set<() => void>();
let desktopActive = false;
let stopDesktop = () => {};

function publishDesktop(patch: Partial<DesktopSnapshot>) {
  desktop = { ...desktop, ...patch };
  desktopListeners.forEach((listener) => listener());
}

function refreshHistory() {
  void queryClient.invalidateQueries({ queryKey: ["history"] });
  void queryClient.invalidateQueries({ queryKey: ["settings"] });
}

function startDesktop() {
  let cancelled = false;
  const subscriptions: (() => void)[] = [];
  let timer: ReturnType<typeof setTimeout>;
  stopDesktop = () => {
    cancelled = true;
    clearTimeout(timer);
    subscriptions.forEach((unsubscribe) => unsubscribe());
  };
  async function bind<T>(name: string, callback: (payload: T) => void) {
    try {
      const unsubscribe = await listen<T>(name, ({ payload }) => { if (!cancelled) callback(payload); });
      if (cancelled) unsubscribe(); else subscriptions.push(unsubscribe);
    } catch (failure) { if (!cancelled) publishDesktop({ localError: errorMessage(failure) }); }
  }
  void bind("history-changed", () => { clearTimeout(timer); timer = setTimeout(() => refreshHistory(), 40); });
  void bind<CaptureStatus>("capture-status", (capture) => publishDesktop({ capture }));
  void bind("window-opened", () => { publishDesktop({ launchCount: desktop.launchCount + 1 }); refreshHistory(); });
  void bind("native-error", (failure) => publishDesktop({ localError: errorMessage(failure) }));
  void invoke<CaptureStatus>("get_capture_status").then((capture) => { if (!cancelled) publishDesktop({ capture }); }).catch((failure) => { if (!cancelled) publishDesktop({ localError: errorMessage(failure) }); });
}

function subscribeDesktop(listener: () => void) {
  desktopListeners.add(listener);
  if (!desktopActive) { desktopActive = true; startDesktop(); }
  return () => {
    desktopListeners.delete(listener);
    if (desktopListeners.size === 0) {
      desktopActive = false;
      stopDesktop();
      desktop = { capture: { state: "starting" }, launchCount: 0 };
    }
  };
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
  const { capture, launchCount, localError } = useSyncExternalStore(subscribeDesktop, () => desktop, () => desktop);
  const history = useQuery({ queryKey: ["history"], queryFn: async () => (await invoke<StoredItem[]>("list_history")).map(toItem) });
  const preferences = useQuery({ queryKey: ["settings"], queryFn: async () => normalizePreferences(await invoke<Partial<Preferences>>("get_settings")) });
  const refresh = () => {
    void client.invalidateQueries({ queryKey: ["history"] });
    void client.invalidateQueries({ queryKey: ["settings"] });
  };

  async function mutate(command: string, args: Record<string, unknown>) {
    try { await invoke(command, args); await refresh(); }
    catch (failure) { throw errorMessage(failure); }
  }
  const actions: HistoryActions = {
    stage: async (ids) => { try { await invoke("stage_selection", { ids }); } catch (failure) { throw errorMessage(failure); } },
    paste: async (entries) => { try { await invoke("paste_entries", { ids: entries.map((item) => item.id) }); } catch (failure) { throw errorMessage(failure); } },
    dismiss: () => { void invoke("hide_window").catch((failure) => publishDesktop({ localError: errorMessage(failure) })); },
    remove: (ids) => mutate("delete_history", { ids }),
    pin: (id, pinned) => mutate("set_pinned", { id, pinned }),
    edit: (id, text) => mutate("edit_text", { id, text }),
  };
  async function retry() {
    publishDesktop({ localError: undefined });
    try { await invoke("retry_capture"); await history.refetch(); }
    catch (failure) { publishDesktop({ localError: errorMessage(failure) }); }
  }
  const queryError = history.error ? errorMessage(history.error) : undefined;
  return {
    items: history.data ?? [], actions, loading: history.isLoading, error: localError ?? queryError ?? capture.message, retry, launchCount, refresh,
    stageSelection: preferences.data?.stageSelection ?? true,
    showShortcutBar: preferences.data?.showShortcutBar ?? true,
  };
}
