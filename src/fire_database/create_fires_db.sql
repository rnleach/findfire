CREATE TABLE IF NOT EXISTS fires
(
    id            TEXT    PRIMARY KEY,
    satellite     TEXT    NOT NULL,
    last_observed INTEGER NOT NULL,
    origin_lat    REAL    NOT NULL,
    origin_lon    REAL    NOT NULL,
    perimeter     BLOB    NOT NULL
);

CREATE TABLE IF NOT EXISTS meta
(
    item_name  TEXT PRIMARY KEY,
    item_value INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS associations
(
    fire_id         TEXT    NOT NULL,
    scan_start_time INTEGER NOT NULL,
    power           REAL    NOT NULL,
    perimeter       BLOB    NOT NULL
);

