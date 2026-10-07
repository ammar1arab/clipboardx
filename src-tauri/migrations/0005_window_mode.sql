ALTER TABLE preferences ADD COLUMN always_on_top INTEGER NOT NULL DEFAULT 0 CHECK(always_on_top IN (0, 1));
