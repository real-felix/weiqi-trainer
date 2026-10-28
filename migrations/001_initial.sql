PRAGMA foreign_keys = ON;
CREATE TABLE games (id TEXT PRIMARY KEY, data TEXT NOT NULL);
CREATE TABLE nodes (
 id TEXT PRIMARY KEY, game_id TEXT NOT NULL REFERENCES games(id),
 parent_id TEXT REFERENCES nodes(id), data TEXT NOT NULL
);
CREATE INDEX nodes_game ON nodes(game_id);
CREATE TABLE analysis (
 game_id TEXT NOT NULL REFERENCES games(id), node_id TEXT NOT NULL REFERENCES nodes(id),
 visits INTEGER NOT NULL, data TEXT NOT NULL, PRIMARY KEY(game_id,node_id)
);
CREATE TABLE rating_events (
 game_id TEXT PRIMARY KEY REFERENCES games(id), before_rating REAL NOT NULL,
 after_rating REAL NOT NULL, opponent_rank INTEGER NOT NULL, result TEXT NOT NULL,
 algorithm TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE profile (id INTEGER PRIMARY KEY CHECK(id=1), rating REAL NOT NULL, auto_rank INTEGER NOT NULL, initialized INTEGER NOT NULL);
INSERT INTO profile VALUES(1,1500,-5,0);
CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK(id=1), data TEXT NOT NULL);
PRAGMA user_version = 1;
