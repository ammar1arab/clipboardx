CREATE TABLE preferences (singleton INTEGER PRIMARY KEY CHECK(singleton = 1), capture_images INTEGER NOT NULL DEFAULT 1 CHECK(capture_images IN (0, 1)), paste_separator TEXT NOT NULL DEFAULT (CHAR(10) || CHAR(10)));
INSERT INTO preferences (singleton) VALUES (1);
