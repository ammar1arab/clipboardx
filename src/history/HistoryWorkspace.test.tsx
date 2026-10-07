import { render, screen, within, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import { useState } from "react";
import { HistoryWorkspace, type HistoryActions } from "./HistoryWorkspace";
import type { ClipboardItem } from "../domain/clipboard";
import App from "../App";
import { act } from "@testing-library/react";

const native = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn(), callbacks: new Map<string, (event: { payload: unknown }) => void>() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke, convertFileSrc: (path: string) => `asset:${path}` }));
vi.mock("@tauri-apps/api/event", () => ({ listen: native.listen }));

const fixtures: ClipboardItem[] = [
  { id: "a", kind: "text", text: "Alpha source code", pinned: true, createdAt: 0, lastUsedAt: null },
  { id: "b", kind: "text", text: "Beta https://example.test", pinned: false, createdAt: 0, lastUsedAt: null },
  { id: "c", kind: "text", text: "مرحباً English", pinned: false, createdAt: 0, lastUsedAt: null },
  { id: "d", kind: "image", thumbnailUrl: "/synthetic.png", filePath: "/tmp/synthetic.png", width: 400, height: 200, pinned: false, createdAt: 0, lastUsedAt: null },
];

function setup() {
  const paste = vi.fn().mockResolvedValue(undefined);
  const dismiss = vi.fn();
  const edit = vi.fn();
  const stage = vi.fn().mockResolvedValue(undefined);
  const openSettings = vi.fn();
  function Harness() {
    const [items, setItems] = useState(fixtures);
    const actions: HistoryActions = {
      paste, dismiss, stage,
      remove: async (ids) => setItems((current) => current.filter((item) => !ids.includes(item.id))),
      pin: async (id, pinned) => setItems((current) => current.map((item) => item.id === id ? { ...item, pinned } : item)),
      edit: async (id, text) => {
        edit(id, text);
        setItems((current) => current.map((item) => item.id === id && item.kind === "text" ? { ...item, text } : item));
      },
    };
    return <HistoryWorkspace items={items} actions={actions} openSettings={openSettings} />;
  }
  render(<Harness />);
  return { user: userEvent.setup(), paste, dismiss, edit, stage, openSettings };
}
const menuFor = (name: string) => within(screen.getByRole("button", { name }).closest("li")!).getByRole("button", { name: "More actions" });

describe("history interactions", () => {
  it("stages the full selection for external Ctrl+V and deletes a filtered selection", async () => {
    const { user, stage } = setup();
    await user.keyboard("{Control>}a{/Control}");
    await waitFor(() => expect(stage).toHaveBeenCalledWith(["a", "b", "c", "d"]));
    await screen.findByText(/Ctrl\+V anywhere/);
    await user.keyboard("{Escape}");
    await user.type(screen.getByRole("textbox"), "Alpha");
    await user.keyboard("{Control>}a{/Control}{Delete}");
    expect(screen.getByText("No matching entries")).toBeVisible();
    await user.keyboard("{Escape}");
    expect(screen.getAllByRole("listitem")).toHaveLength(3);
  });

  it("exports exact text for native input drag and never pastes on drag-start", () => {
    const { paste } = setup();
    const dataTransfer = { setData: vi.fn(), effectAllowed: "none" };
    fireEvent.dragStart(screen.getByRole("button", { name: "مرحباً English" }), { dataTransfer });
    expect(dataTransfer.setData).toHaveBeenCalledWith("text/plain", "مرحباً English");
    expect(dataTransfer.effectAllowed).toBe("copy");
    expect(paste).not.toHaveBeenCalled();
    fireEvent.dragStart(screen.getByRole("button", { name: "Image, 400 by 200" }), { dataTransfer });
    expect(dataTransfer.setData).toHaveBeenCalledWith("text/uri-list", "file:///tmp/synthetic.png");
  });

  it("focuses the workspace, navigates with boundaries and pastes the active item", async () => {
    const { user, paste } = setup();
    expect(screen.getByRole("main")).toHaveFocus();
    await user.keyboard("{ArrowUp}{Enter}");
    expect(paste.mock.calls[0][0][0].id).toBe("a");
    await user.keyboard("{ArrowDown}{Enter}");
    expect(paste.mock.calls[1][0][0].id).toBe("b");
    await user.click(screen.getByRole("button", { name: "مرحباً English" }));
    expect(paste.mock.calls[2][0][0].id).toBe("c");
  });

  it("searches multilingual text and URLs and handles empty search", async () => {
    const { user } = setup();
    await user.type(screen.getByRole("textbox"), "مرحباً");
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
    await user.keyboard("{Escape}");
    await user.type(screen.getByRole("textbox"), "https:");
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
    await user.type(screen.getByRole("textbox"), "missing");
    expect(screen.getByText("No matching entries")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Clear search" }));
    expect(screen.getAllByRole("listitem")).toHaveLength(4);
  });

  it("does not autofocus search and sends typed characters into it", async () => {
    const { user } = setup();
    const search = screen.getByRole("textbox", { name: "Search clipboard history" });
    expect(search).not.toHaveFocus();
    await user.keyboard("Be");
    expect(search).toHaveFocus();
    expect(search).toHaveValue("Be");
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
  });

  it("opens settings from the toolbar gear", async () => {
    const { user, openSettings } = setup();
    await user.click(screen.getByRole("button", { name: "Open settings" }));
    expect(openSettings).toHaveBeenCalledOnce();
  });

  it("filters to images or text and clears that filter on a second press", async () => {
    const { user } = setup();
    const images = screen.getByRole("button", { name: "Show images" });
    const text = screen.getByRole("button", { name: "Show text" });
    await user.click(images);
    expect(images).toHaveAttribute("aria-pressed", "true");
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "Image, 400 by 200" })).toBeVisible();
    await user.click(images);
    expect(images).toHaveAttribute("aria-pressed", "false");
    expect(screen.getAllByRole("listitem")).toHaveLength(4);
    await user.click(text);
    expect(text).toHaveAttribute("aria-pressed", "true");
    expect(screen.getAllByRole("listitem")).toHaveLength(3);
    expect(screen.queryByRole("button", { name: "Image, 400 by 200" })).not.toBeInTheDocument();
    await user.click(images);
    expect(text).toHaveAttribute("aria-pressed", "false");
    expect(images).toHaveAttribute("aria-pressed", "true");
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
  });

  it("pins and deletes through the row menu without pasting", async () => {
    const { user, paste } = setup();
    await user.click(menuFor("Beta https://example.test"));
    await user.click(screen.getByRole("menuitem", { name: "Pin" }));
    expect(screen.getAllByText("Pinned")).toHaveLength(2);
    await user.click(menuFor("Beta https://example.test"));
    await user.click(screen.getByRole("menuitem", { name: "Delete" }));
    expect(screen.queryByRole("button", { name: "Beta https://example.test" })).not.toBeInTheDocument();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(paste).not.toHaveBeenCalled();
  });

  it("supports Ctrl+Click, range selection and deterministic multi-paste", async () => {
    const { user, paste } = setup();
    await user.keyboard("{Control>}");
    await user.click(screen.getByRole("button", { name: "Beta https://example.test" }));
    await user.click(screen.getByRole("button", { name: "Alpha source code" }));
    await user.keyboard("{Enter}{/Control}");
    expect(paste.mock.calls[0][0].map((item: ClipboardItem) => item.id)).toEqual(["a", "b"]);
    await user.keyboard("{Escape}");
    await user.keyboard("{Shift>}");
    await user.click(screen.getByRole("button", { name: "مرحباً English" }));
    await user.keyboard("{/Shift}");
    expect(screen.getByText(/3 selected/)).toBeVisible();
    expect(paste).toHaveBeenCalledTimes(1);
  });

  it("selects only filtered entries and rejects mixed multi-paste", async () => {
    const { user, paste } = setup();
    await user.keyboard("{Control>}a{Enter}{/Control}");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(paste).not.toHaveBeenCalled();
    await user.keyboard("{Escape}");
    await user.type(screen.getByRole("textbox"), "Alpha");
    await user.keyboard("{Control>}a{Enter}{/Control}");
    expect(paste.mock.calls[0][0].map((item: ClipboardItem) => item.id)).toEqual(["a"]);
  });

  it("deletes selected entries, keeps a usable active item and pins with Ctrl+P", async () => {
    const { user, paste } = setup();
    await user.keyboard("{Control>}p{/Control}");
    await waitFor(() => expect(screen.queryByText("Pinned")).not.toBeInTheDocument());
    await user.keyboard("{Delete}{Enter}");
    expect(paste.mock.calls[0][0][0].id).toBe("b");
    await user.keyboard("{Control>}a{/Control}{Delete}");
    expect(screen.getByText("Your clipboard starts here")).toBeVisible();
  });

  it("edits inline, saves with Enter and cancels with Escape", async () => {
    const { user, edit } = setup();
    await user.keyboard("{F2}");
    const editor = screen.getByRole<HTMLTextAreaElement>("textbox", { name: "Edit clipboard text" });
    await waitFor(() => expect(editor.selectionStart).toBe(editor.value.length));
    await user.clear(editor);
    await user.type(editor, "Changed synthetic text");
    await user.keyboard("{Enter}");
    expect(edit).toHaveBeenCalledWith("a", "Changed synthetic text");
    expect(screen.getByRole("main")).toHaveFocus();
    await user.keyboard("{F2}");
    await user.type(screen.getByRole("textbox", { name: "Edit clipboard text" }), " cancelled");
    await user.keyboard("{Escape}");
    expect(edit).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Changed synthetic text" })).toBeVisible();
  });

  it("does not offer text editing for images and dismisses with Escape", async () => {
    const { user, dismiss } = setup();
    await user.click(menuFor("Image, 400 by 200"));
    expect(screen.getByRole("menuitem", { name: "Delete" })).toBeVisible();
    expect(screen.queryByRole("menuitem", { name: "Edit" })).not.toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(dismiss).not.toHaveBeenCalled();
    await user.keyboard("{ArrowDown}{ArrowDown}{ArrowDown}{F2}");
    expect(screen.queryByRole("textbox", { name: "Edit clipboard text" })).not.toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(dismiss).toHaveBeenCalledOnce();
  });

  it("renders startup and recovery states without treating them as success", async () => {
    const actions = { stage: vi.fn(), paste: vi.fn(), remove: vi.fn(), pin: vi.fn(), edit: vi.fn(), dismiss: vi.fn() };
    const retry = vi.fn();
    const view = render(<HistoryWorkspace items={[]} actions={actions} loading />);
    expect(screen.getByRole("status")).toHaveTextContent("Loading");
    view.rerender(<HistoryWorkspace items={[]} actions={actions} error="History unavailable" retry={retry} />);
    expect(screen.getByRole("alert")).toHaveTextContent("History unavailable");
    await userEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledOnce();
  });
});

describe("native history connection", () => {
  function configure(rows: { id: string; kind: string; text: string; pinned: boolean }[] = []) {
    native.callbacks.clear();
    native.listen.mockImplementation(async (name: string, callback: (event: { payload: unknown }) => void) => {
      native.callbacks.set(name, callback);
      return () => native.callbacks.delete(name);
    });
    native.invoke.mockReset();
    native.invoke.mockImplementation(async (command: string) => {
      if (command === "get_capture_status") return { state: "running" };
      if (command === "list_history") return rows.map((row) => ({ ...row, createdAt: 0, lastUsedAt: null, thumbnailPath: null, width: null, height: null }));
    });
  }

  it("starts empty without sample history and refreshes on native capture", async () => {
    configure();
    const view = render(<App />);
    await screen.findByText("Your clipboard starts here");
    expect(screen.queryByText(/sample history|Keep the things you copy/)).not.toBeInTheDocument();
    native.invoke.mockImplementation(async (command: string) => command === "list_history" ? [
      { id: "native-1", kind: "text", text: "Native synthetic capture", pinned: false, createdAt: 1, lastUsedAt: null },
    ] : { state: "running" });
    act(() => native.callbacks.get("history-changed")?.({ payload: null }));
    await screen.findByRole("button", { name: "Native synthetic capture" });
    view.unmount();
    expect(native.callbacks.size).toBe(0);
  });

  it("edits and pins through real command contracts instead of local preview mutations", async () => {
    configure([{ id: "native-2", kind: "text", text: "Stored synthetic text", pinned: false }]);
    render(<App />);
    await screen.findByRole("button", { name: "Stored synthetic text" });
    const user = userEvent.setup();
    await user.keyboard("{Control>}p{/Control}");
    expect(native.invoke).toHaveBeenCalledWith("set_pinned", { id: "native-2", pinned: true });
    await user.keyboard("{F2}");
    await user.clear(screen.getByRole("textbox", { name: "Edit clipboard text" }));
    await user.type(screen.getByRole("textbox", { name: "Edit clipboard text" }), "Edited synthetic text");
    await user.keyboard("{Enter}");
    expect(native.invoke).toHaveBeenCalledWith("edit_text", { id: "native-2", text: "Edited synthetic text" });
  });

  it("shows structured storage errors and recovers by retrying native history", async () => {
    configure();
    native.invoke.mockImplementation(async (command: string) => {
      if (command === "list_history") throw { code: "storage_error", message: "Could not read clipboard history" };
      return { state: "running" };
    });
    render(<App />);
    await screen.findByRole("alert");
    expect(screen.getByRole("alert")).toHaveTextContent("Could not read clipboard history");
    native.invoke.mockImplementation(async (command: string) => command === "list_history" ? [] : { state: "running" });
    await userEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    expect(native.invoke).toHaveBeenCalledWith("retry_capture");
  });
});
