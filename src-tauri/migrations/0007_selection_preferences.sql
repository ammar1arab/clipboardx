ALTER TABLE preferences ADD COLUMN stage_selection INTEGER NOT NULL DEFAULT 1 CHECK(stage_selection IN (0, 1));
ALTER TABLE preferences ADD COLUMN show_shortcut_bar INTEGER NOT NULL DEFAULT 1 CHECK(show_shortcut_bar IN (0, 1));
