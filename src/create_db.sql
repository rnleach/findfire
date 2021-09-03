CREATE TABLE IF NOT EXISTS clusters
(
  rowid          INTEGER PRIMARY KEY AUTOINCREMENT,
  satellite      TEXT    NOT NULL,
  sector         TEXT    NOT NULL,
  mid_point_time INTEGER NOT NULL,
  lat            REAL    NOT NULL,
  lon            REAL    NOT NULL,
  power          REAL    NOT NULL,
  radius         REAL    NOT NULL,
  cell_count     INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ON clusters (satellite, sector, mid_point_time, lat, lon);

