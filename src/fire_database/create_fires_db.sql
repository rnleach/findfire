CREATE TABLE IF NOT EXISTS fires
(
    id            TEXT    PRIMARY KEY,
    satellite     TEXT    NOT NULL,
    last_observed INTEGER NOT NULL,
    origin_lat    REAL    NOT NULL,
    origin_lon    REAL    NOT NULL,
    perimeter     BLOB    NOT NULL,
    next_child    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS meta
(
    item_name  TEXT PRIMARY KEY,
    item_value INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS associations
(
    cluster_row_id INTEGER,
    fire_id        TEXT
);

