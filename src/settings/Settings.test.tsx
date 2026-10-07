import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, test, expect } from "vitest";
import { Settings } from "./Settings";

const { invoke, enable, disable, isEnabled } = vi.hoisted(() => ({ invoke: vi.fn(), enable: vi.fn(), disable: vi.fn(), isEnabled: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: vi.fn() }));
vi.mock("@tauri-apps/plugin-autostart", () => ({ enable, disable, isEnabled }));

test("loads real preferences, saves separator and requires confirmation before deleting pins", async () => {
  invoke.mockImplementation(async (command) => command === "get_settings" ? { captureImages: true, pasteSeparator: " | " } : command === "get_retention" ? { maxItems: 500, maxAgeDays: 30 } : undefined);
  isEnabled.mockResolvedValue(true);
  const user = userEvent.setup();
  render(<Settings close={vi.fn()} />);
  const separator = await screen.findByLabelText("Multi-paste separator");
  expect(screen.getByLabelText("Start at login")).toBeChecked();
  await user.clear(separator); await user.type(separator, ", ");
  await user.click(screen.getByRole("button", { name: "Save settings" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_settings", { settings: { captureImages: true, pasteSeparator: ", " } }));
  await user.click(screen.getByRole("button", { name: "Clear all history" }));
  expect(invoke).not.toHaveBeenCalledWith("clear_history", { includePinned: true });
  await user.click(screen.getByRole("button", { name: "Delete all" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("clear_history", { includePinned: true }));
  await user.click(screen.getByLabelText("Start at login"));
  await waitFor(() => expect(disable).toHaveBeenCalled());
});
