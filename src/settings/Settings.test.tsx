import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, test, expect } from "vitest";
import { Settings } from "./Settings";

const { invoke, enable, disable, isEnabled } = vi.hoisted(() => ({ invoke: vi.fn(), enable: vi.fn(), disable: vi.fn(), isEnabled: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: vi.fn() }));
vi.mock("@tauri-apps/plugin-autostart", () => ({ enable, disable, isEnabled }));

test("shows load errors with a working Retry instead of loading forever", async () => {
  invoke.mockReset();
  invoke.mockRejectedValueOnce("Command get_settings not found").mockImplementation(async (command) => command === "get_settings" ? { captureImages: true, pasteSeparator: "\n", windowWidth: 460, windowHeight: 520 } : { maxItems: 500, maxAgeDays: 30 });
  isEnabled.mockResolvedValue(false);
  const user = userEvent.setup();
  render(<Settings close={vi.fn()} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Command get_settings not found");
  expect(screen.queryByRole("status")).not.toBeInTheDocument();
  expect(screen.getByLabelText("Width")).toBeVisible();
  await user.click(screen.getByRole("button", { name: "Retry" }));
  expect(await screen.findByLabelText("Paste separator")).toBeVisible();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  expect(invoke.mock.calls.filter(([command]) => command === "get_settings")).toHaveLength(2);
});

test("loads real preferences, saves them, and clears all history immediately", async () => {
  invoke.mockImplementation(async (command) => command === "get_settings" ? { captureImages: true, pasteSeparator: " | ", windowWidth: 460, windowHeight: 520, stageSelection: true, showShortcutBar: true } : command === "get_retention" ? { maxItems: 500, maxAgeDays: 30 } : undefined);
  isEnabled.mockResolvedValue(true);
  const user = userEvent.setup();
  render(<Settings close={vi.fn()} />);
  const separator = await screen.findByLabelText("Paste separator");
  await waitFor(() => expect(separator).toHaveValue(" | "));
  expect(screen.getByLabelText("Start at login")).toBeChecked();
  expect(screen.getByLabelText("Stage selection for Ctrl+V")).toBeChecked();
  expect(screen.getByLabelText("Show shortcut bar")).toBeChecked();
  expect(screen.getByLabelText("Separator style")).toHaveValue("custom");
  await user.selectOptions(screen.getByLabelText("Separator style"), "line");
  expect(separator).toHaveValue("\n");
  await user.clear(separator); await user.type(separator, ", ");
  await user.click(screen.getByRole("button", { name: "Save settings" }));
  expect(screen.getByLabelText("Width")).toHaveValue(460);
  expect(screen.getByLabelText("Height")).toHaveValue(520);
  expect(screen.queryByRole("button", { name: "On top" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Always show" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Delete all" })).not.toBeInTheDocument();
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_settings", { settings: { captureImages: true, pasteSeparator: ", ", windowWidth: 460, windowHeight: 520, stageSelection: true, showShortcutBar: true } }));
  await user.click(screen.getByRole("button", { name: "Clear all history" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("clear_history", { includePinned: true }));
  await user.click(screen.getByLabelText("Start at login"));
  await waitFor(() => expect(disable).toHaveBeenCalled());
});
