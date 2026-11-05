export type Preferences = {
  captureImages: boolean;
  pasteSeparator: string;
  windowWidth: number;
  windowHeight: number;
  stageSelection: boolean;
  showShortcutBar: boolean;
};

export function normalizePreferences(input: Partial<Preferences> | null | undefined): Preferences {
  return {
    captureImages: input?.captureImages ?? true,
    pasteSeparator: input?.pasteSeparator ?? "\n\n",
    windowWidth: input?.windowWidth ?? 460,
    windowHeight: input?.windowHeight ?? 520,
    stageSelection: input?.stageSelection ?? true,
    showShortcutBar: input?.showShortcutBar ?? true,
  };
}
