-- Preserve legacy result audits, but stop using Elo for new games.
ALTER TABLE profile RENAME TO legacy_profile;
CREATE TABLE profile (
 id INTEGER PRIMARY KEY CHECK(id=1),
 level INTEGER NOT NULL CHECK(level BETWEEN -40 AND 16),
 initialized INTEGER NOT NULL
);
INSERT INTO profile SELECT id, max(-40,min(16,auto_rank*2)), initialized FROM legacy_profile;
DROP TABLE legacy_profile;
CREATE TABLE rank_events (
 game_id TEXT PRIMARY KEY REFERENCES games(id), before_level INTEGER NOT NULL,
 after_level INTEGER NOT NULL, opponent_level INTEGER NOT NULL,
 result TEXT NOT NULL, algorithm TEXT NOT NULL, created_at TEXT NOT NULL
);
PRAGMA user_version = 2;
